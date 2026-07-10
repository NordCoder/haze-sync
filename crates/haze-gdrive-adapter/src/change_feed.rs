//! Safe Google Drive change-feed polling and reconciliation planning.
//!
//! This module keeps provider polling, cursor persistence, and work execution
//! behind injected abstractions. It performs no Core/API calls, provider
//! mutations, direct database access, or background scheduling.

use crate::config::AdapterMode;
use crate::drive::{
    normalize_drive_metadata, DriveEntryClassification, DriveEntryKind, DriveMetadata,
    ProviderError, ProviderErrorCategory, UnsupportedEntryReason,
};
use crate::scan::ImportExecution;
use crate::state::{
    DriveChangeCursor, DriveEchoObservation, EchoDecision, EchoGuard, SafeTimestamp, StateError,
};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::time::Duration;

const DEFAULT_MAX_PAGES_PER_POLL: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeFeedModelError {
    EmptyProviderId,
    EmptyPageToken,
    EmptyDriveVersion,
}

impl fmt::Display for ChangeFeedModelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EmptyProviderId => "change provider id must not be empty",
            Self::EmptyPageToken => "change page token must not be empty",
            Self::EmptyDriveVersion => "change Drive version must not be empty",
        };
        formatter.write_str(message)
    }
}

impl Error for ChangeFeedModelError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriveChangeEntry {
    provider_id: String,
    removed: bool,
    metadata: Option<DriveMetadata>,
    drive_version: Option<String>,
}

impl DriveChangeEntry {
    pub fn file(metadata: DriveMetadata) -> Result<Self, ChangeFeedModelError> {
        let provider_id = required_provider_id(metadata.id.clone())?;
        Ok(Self {
            provider_id,
            removed: false,
            metadata: Some(metadata),
            drive_version: None,
        })
    }

    pub fn removed(provider_id: impl Into<String>) -> Result<Self, ChangeFeedModelError> {
        Ok(Self {
            provider_id: required_provider_id(provider_id)?,
            removed: true,
            metadata: None,
            drive_version: None,
        })
    }

    pub fn metadata_missing(provider_id: impl Into<String>) -> Result<Self, ChangeFeedModelError> {
        Ok(Self {
            provider_id: required_provider_id(provider_id)?,
            removed: false,
            metadata: None,
            drive_version: None,
        })
    }

    pub fn with_drive_version(
        mut self,
        drive_version: impl Into<String>,
    ) -> Result<Self, ChangeFeedModelError> {
        let drive_version = drive_version.into();
        let trimmed = drive_version.trim();
        if trimmed.is_empty() {
            return Err(ChangeFeedModelError::EmptyDriveVersion);
        }
        self.drive_version = Some(trimmed.to_owned());
        Ok(self)
    }

    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }

    pub const fn is_removed(&self) -> bool {
        self.removed
    }

    pub fn metadata(&self) -> Option<&DriveMetadata> {
        self.metadata.as_ref()
    }

    pub fn drive_version(&self) -> Option<&str> {
        self.drive_version.as_deref()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct DriveChangePage {
    changes: Vec<DriveChangeEntry>,
    next_page_token: Option<String>,
    new_start_page_token: Option<String>,
}

impl DriveChangePage {
    pub fn intermediate(
        changes: Vec<DriveChangeEntry>,
        next_page_token: impl Into<String>,
    ) -> Result<Self, ChangeFeedModelError> {
        Ok(Self {
            changes,
            next_page_token: Some(required_page_token(next_page_token)?),
            new_start_page_token: None,
        })
    }

    pub fn final_page(
        changes: Vec<DriveChangeEntry>,
        new_start_page_token: impl Into<String>,
    ) -> Result<Self, ChangeFeedModelError> {
        Ok(Self {
            changes,
            next_page_token: None,
            new_start_page_token: Some(required_page_token(new_start_page_token)?),
        })
    }

    pub fn changes(&self) -> &[DriveChangeEntry] {
        &self.changes
    }
}

impl fmt::Debug for DriveChangePage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DriveChangePage")
            .field("changes", &self.changes)
            .field("has_next_page_token", &self.next_page_token.is_some())
            .field(
                "has_new_start_page_token",
                &self.new_start_page_token.is_some(),
            )
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum DriveChangePoll {
    Page(DriveChangePage),
    CursorInvalidated { new_start_page_token: String },
}

impl DriveChangePoll {
    pub fn cursor_invalidated(
        new_start_page_token: impl Into<String>,
    ) -> Result<Self, ChangeFeedModelError> {
        Ok(Self::CursorInvalidated {
            new_start_page_token: required_page_token(new_start_page_token)?,
        })
    }
}

impl fmt::Debug for DriveChangePoll {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Page(page) => formatter.debug_tuple("Page").field(page).finish(),
            Self::CursorInvalidated { .. } => formatter
                .debug_struct("CursorInvalidated")
                .field("new_start_page_token", &"<redacted-cursor-token>")
                .finish(),
        }
    }
}

pub trait DriveChangeFeedProvider {
    fn get_start_page_token(&self) -> Result<String, ProviderError>;

    fn list_changes(&self, page_token: &str) -> Result<DriveChangePoll, ProviderError>;
}

#[derive(Clone)]
pub struct FakeDriveChangeFeedProvider {
    start_page_token: String,
    polls_by_page_token: BTreeMap<String, DriveChangePoll>,
    errors_by_operation: BTreeMap<&'static str, ProviderError>,
}

impl FakeDriveChangeFeedProvider {
    pub fn new(start_page_token: impl Into<String>) -> Result<Self, ChangeFeedModelError> {
        Ok(Self {
            start_page_token: required_page_token(start_page_token)?,
            polls_by_page_token: BTreeMap::new(),
            errors_by_operation: BTreeMap::new(),
        })
    }

    pub fn with_poll(mut self, page_token: impl Into<String>, poll: DriveChangePoll) -> Self {
        self.polls_by_page_token.insert(page_token.into(), poll);
        self
    }

    pub fn with_error(mut self, operation: &'static str, error: ProviderError) -> Self {
        self.errors_by_operation.insert(operation, error);
        self
    }

    fn configured_error(&self, operation: &'static str) -> Option<ProviderError> {
        self.errors_by_operation.get(operation).cloned()
    }
}

impl fmt::Debug for FakeDriveChangeFeedProvider {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FakeDriveChangeFeedProvider")
            .field("configured_pages", &self.polls_by_page_token.len())
            .field("configured_errors", &self.errors_by_operation.len())
            .finish()
    }
}

impl DriveChangeFeedProvider for FakeDriveChangeFeedProvider {
    fn get_start_page_token(&self) -> Result<String, ProviderError> {
        if let Some(error) = self.configured_error("get_start_page_token") {
            return Err(error);
        }
        Ok(self.start_page_token.clone())
    }

    fn list_changes(&self, page_token: &str) -> Result<DriveChangePoll, ProviderError> {
        if let Some(error) = self.configured_error("list_changes") {
            return Err(error);
        }
        self.polls_by_page_token
            .get(page_token)
            .cloned()
            .ok_or_else(|| ProviderError::not_found("list_changes"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FullScanFallbackReason {
    InitialCursor,
    StoredCursorInvalidated,
    ProviderCursorInvalidated,
    RemovedEntry,
    FolderChanged,
    MissingMetadata,
    ConflictingChangeHistory,
    MetadataIdentityMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeWorkItem {
    FullScan {
        reason: FullScanFallbackReason,
        provider_id: Option<String>,
    },
    Import {
        metadata: DriveMetadata,
        execution: ImportExecution,
    },
    ConfirmExportEcho {
        observation: DriveEchoObservation,
    },
    SkipUnsupported {
        provider_id: String,
        reason: UnsupportedEntryReason,
    },
    SkipImportByMode {
        provider_id: String,
        mode: AdapterMode,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChangeWorkBatch {
    pub items: Vec<ChangeWorkItem>,
}

impl ChangeWorkBatch {
    pub fn full_scan(reason: FullScanFallbackReason) -> Self {
        Self {
            items: vec![ChangeWorkItem::FullScan {
                reason,
                provider_id: None,
            }],
        }
    }

    pub fn requires_full_scan(&self) -> bool {
        self.items
            .iter()
            .any(|item| matches!(item, ChangeWorkItem::FullScan { .. }))
    }

    fn retain_only_full_scan_work_if_required(&mut self) {
        if self.requires_full_scan() {
            self.items
                .retain(|item| matches!(item, ChangeWorkItem::FullScan { .. }));
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeProcessingError {
    safe_detail: &'static str,
}

impl ChangeProcessingError {
    pub const fn new(safe_detail: &'static str) -> Self {
        Self { safe_detail }
    }

    pub const fn safe_detail(&self) -> &'static str {
        self.safe_detail
    }
}

impl fmt::Display for ChangeProcessingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "change work processing failed: {}",
            self.safe_detail
        )
    }
}

impl Error for ChangeProcessingError {}

/// Processes one replayable unit of change-feed work.
///
/// Implementations must be idempotent because successful processing can be
/// delivered again when cursor persistence fails afterward.
pub trait ChangeWorkProcessor {
    fn process(&mut self, batch: &ChangeWorkBatch) -> Result<(), ChangeProcessingError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursorStoreError {
    operation: &'static str,
    safe_detail: &'static str,
}

impl CursorStoreError {
    pub const fn new(operation: &'static str, safe_detail: &'static str) -> Self {
        Self {
            operation,
            safe_detail,
        }
    }

    pub const fn operation(&self) -> &'static str {
        self.operation
    }
}

impl fmt::Display for CursorStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "cursor store operation {} failed: {}",
            self.operation, self.safe_detail
        )
    }
}

impl Error for CursorStoreError {}

pub trait DriveCursorStore {
    fn load_cursor(&self) -> Result<Option<DriveChangeCursor>, CursorStoreError>;

    fn save_cursor(&mut self, cursor: &DriveChangeCursor) -> Result<(), CursorStoreError>;
}

#[derive(Clone, Default, PartialEq, Eq)]
pub struct InMemoryDriveCursorStore {
    cursor: Option<DriveChangeCursor>,
    load_error: Option<CursorStoreError>,
    save_error: Option<CursorStoreError>,
    save_count: usize,
}

impl InMemoryDriveCursorStore {
    pub fn new(cursor: Option<DriveChangeCursor>) -> Self {
        Self {
            cursor,
            load_error: None,
            save_error: None,
            save_count: 0,
        }
    }

    pub fn with_load_error(mut self, error: CursorStoreError) -> Self {
        self.load_error = Some(error);
        self
    }

    pub fn with_save_error(mut self, error: CursorStoreError) -> Self {
        self.save_error = Some(error);
        self
    }

    pub fn cursor(&self) -> Option<&DriveChangeCursor> {
        self.cursor.as_ref()
    }

    pub const fn save_count(&self) -> usize {
        self.save_count
    }
}

impl DriveCursorStore for InMemoryDriveCursorStore {
    fn load_cursor(&self) -> Result<Option<DriveChangeCursor>, CursorStoreError> {
        if let Some(error) = &self.load_error {
            return Err(error.clone());
        }
        Ok(self.cursor.clone())
    }

    fn save_cursor(&mut self, cursor: &DriveChangeCursor) -> Result<(), CursorStoreError> {
        if let Some(error) = &self.save_error {
            return Err(error.clone());
        }
        self.cursor = Some(cursor.clone());
        self.save_count += 1;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetryDisposition {
    RetryAfter(Duration),
    Reauthenticate,
    DoNotRetry,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderBackoffPolicy {
    delays: [Duration; 4],
}

impl ProviderBackoffPolicy {
    pub const fn new(delays: [Duration; 4]) -> Self {
        Self { delays }
    }

    pub fn classify(&self, category: ProviderErrorCategory, attempt: usize) -> RetryDisposition {
        match category {
            ProviderErrorCategory::RateLimit
            | ProviderErrorCategory::ProviderUnavailable
            | ProviderErrorCategory::Internal => {
                let index = attempt.min(self.delays.len() - 1);
                RetryDisposition::RetryAfter(self.delays[index])
            }
            ProviderErrorCategory::Auth => RetryDisposition::Reauthenticate,
            ProviderErrorCategory::NotFound
            | ProviderErrorCategory::Unsupported
            | ProviderErrorCategory::InvalidRequest => RetryDisposition::DoNotRetry,
        }
    }
}

impl Default for ProviderBackoffPolicy {
    fn default() -> Self {
        Self::new([
            Duration::from_secs(5),
            Duration::from_secs(15),
            Duration::from_secs(60),
            Duration::from_secs(300),
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeFeedError {
    Provider {
        error: ProviderError,
        retry: RetryDisposition,
    },
    CursorStore(CursorStoreError),
    Processing(ChangeProcessingError),
    State(StateError),
    InvalidPageShape,
    RepeatedPageToken,
    PageLimitExceeded,
}

impl fmt::Display for ChangeFeedError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Provider { error, retry } => {
                write!(formatter, "{error}; retry disposition: {retry:?}")
            }
            Self::CursorStore(error) => error.fmt(formatter),
            Self::Processing(error) => error.fmt(formatter),
            Self::State(error) => error.fmt(formatter),
            Self::InvalidPageShape => formatter.write_str("provider returned invalid page shape"),
            Self::RepeatedPageToken => {
                formatter.write_str("provider returned a repeated change page token")
            }
            Self::PageLimitExceeded => {
                formatter.write_str("change feed page limit exceeded without a final cursor")
            }
        }
    }
}

impl Error for ChangeFeedError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Provider { error, .. } => Some(error),
            Self::CursorStore(error) => Some(error),
            Self::Processing(error) => Some(error),
            Self::State(error) => Some(error),
            Self::InvalidPageShape | Self::RepeatedPageToken | Self::PageLimitExceeded => None,
        }
    }
}

impl From<CursorStoreError> for ChangeFeedError {
    fn from(error: CursorStoreError) -> Self {
        Self::CursorStore(error)
    }
}

impl From<ChangeProcessingError> for ChangeFeedError {
    fn from(error: ChangeProcessingError) -> Self {
        Self::Processing(error)
    }
}

impl From<StateError> for ChangeFeedError {
    fn from(error: StateError) -> Self {
        Self::State(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeFeedCycleInput {
    pub mode: AdapterMode,
    pub dry_run: bool,
    pub polled_at: SafeTimestamp,
    pub provider_attempt: usize,
    pub max_pages_per_poll: usize,
}

impl ChangeFeedCycleInput {
    pub fn new(mode: AdapterMode, dry_run: bool, polled_at: SafeTimestamp) -> Self {
        Self {
            mode,
            dry_run,
            polled_at,
            provider_attempt: 0,
            max_pages_per_poll: DEFAULT_MAX_PAGES_PER_POLL,
        }
    }

    pub const fn with_provider_attempt(mut self, provider_attempt: usize) -> Self {
        self.provider_attempt = provider_attempt;
        self
    }

    pub const fn with_max_pages_per_poll(mut self, max_pages_per_poll: usize) -> Self {
        self.max_pages_per_poll = max_pages_per_poll;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeFeedCycleOutcome {
    pub pages_polled: usize,
    pub entries_received: usize,
    pub work_items_processed: usize,
    pub full_scan_used: bool,
    pub cursor_saved: bool,
}

pub fn run_change_feed_cycle(
    provider: &impl DriveChangeFeedProvider,
    cursor_store: &mut impl DriveCursorStore,
    processor: &mut impl ChangeWorkProcessor,
    echo_guard: &EchoGuard,
    backoff_policy: &ProviderBackoffPolicy,
    input: ChangeFeedCycleInput,
) -> Result<ChangeFeedCycleOutcome, ChangeFeedError> {
    let cursor = cursor_store.load_cursor()?;
    match cursor {
        None => recover_with_full_scan(
            provider,
            cursor_store,
            processor,
            backoff_policy,
            input,
            FullScanFallbackReason::InitialCursor,
            None,
        ),
        Some(cursor) if cursor.requires_full_scan() => {
            let reason = if cursor.invalidated_at.is_some() {
                FullScanFallbackReason::StoredCursorInvalidated
            } else {
                FullScanFallbackReason::InitialCursor
            };
            let reusable_start_token = if cursor.invalidated_at.is_none() {
                cursor.start_page_token.clone()
            } else {
                None
            };
            recover_with_full_scan(
                provider,
                cursor_store,
                processor,
                backoff_policy,
                input,
                reason,
                reusable_start_token,
            )
        }
        Some(cursor) => poll_from_cursor(
            provider,
            cursor_store,
            processor,
            echo_guard,
            backoff_policy,
            input,
            cursor,
        ),
    }
}

pub fn classify_drive_changes(
    changes: Vec<DriveChangeEntry>,
    mode: AdapterMode,
    dry_run: bool,
    echo_guard: &EchoGuard,
) -> ChangeWorkBatch {
    let mut changes_by_provider_id: BTreeMap<String, Vec<DriveChangeEntry>> = BTreeMap::new();
    for change in changes {
        changes_by_provider_id
            .entry(change.provider_id.clone())
            .or_default()
            .push(change);
    }

    let mut batch = ChangeWorkBatch::default();
    for (provider_id, provider_changes) in changes_by_provider_id {
        let mut unique_changes = Vec::new();
        for change in provider_changes {
            if !unique_changes.contains(&change) {
                unique_changes.push(change);
            }
        }

        if unique_changes.len() != 1 {
            batch.items.push(ChangeWorkItem::FullScan {
                reason: FullScanFallbackReason::ConflictingChangeHistory,
                provider_id: Some(provider_id),
            });
            continue;
        }

        let change = unique_changes.pop().expect("one unique change");
        classify_single_change(change, mode, dry_run, echo_guard, &mut batch);
    }
    batch.retain_only_full_scan_work_if_required();
    batch
}

fn recover_with_full_scan(
    provider: &impl DriveChangeFeedProvider,
    cursor_store: &mut impl DriveCursorStore,
    processor: &mut impl ChangeWorkProcessor,
    backoff_policy: &ProviderBackoffPolicy,
    input: ChangeFeedCycleInput,
    reason: FullScanFallbackReason,
    reusable_start_token: Option<String>,
) -> Result<ChangeFeedCycleOutcome, ChangeFeedError> {
    let start_page_token = match reusable_start_token {
        Some(token) => token,
        None => provider
            .get_start_page_token()
            .map_err(|error| provider_error(error, backoff_policy, input.provider_attempt))?,
    };

    let batch = ChangeWorkBatch::full_scan(reason);
    processor.process(&batch)?;

    let mut recovered_cursor = DriveChangeCursor::new(start_page_token.clone())?;
    recovered_cursor.finish_batch(start_page_token, input.polled_at)?;
    cursor_store.save_cursor(&recovered_cursor)?;

    Ok(ChangeFeedCycleOutcome {
        pages_polled: 0,
        entries_received: 0,
        work_items_processed: batch.items.len(),
        full_scan_used: true,
        cursor_saved: true,
    })
}

fn poll_from_cursor(
    provider: &impl DriveChangeFeedProvider,
    cursor_store: &mut impl DriveCursorStore,
    processor: &mut impl ChangeWorkProcessor,
    echo_guard: &EchoGuard,
    backoff_policy: &ProviderBackoffPolicy,
    input: ChangeFeedCycleInput,
    cursor: DriveChangeCursor,
) -> Result<ChangeFeedCycleOutcome, ChangeFeedError> {
    if input.max_pages_per_poll == 0 {
        return Err(ChangeFeedError::PageLimitExceeded);
    }

    let mut page_token = current_page_token(&cursor).ok_or(ChangeFeedError::InvalidPageShape)?;
    let mut seen_page_tokens = BTreeSet::new();
    let mut pages_polled = 0;
    let mut changes = Vec::new();

    let final_page_token = loop {
        if pages_polled >= input.max_pages_per_poll {
            return Err(ChangeFeedError::PageLimitExceeded);
        }
        if !seen_page_tokens.insert(page_token.clone()) {
            return Err(ChangeFeedError::RepeatedPageToken);
        }

        let poll = provider
            .list_changes(&page_token)
            .map_err(|error| provider_error(error, backoff_policy, input.provider_attempt))?;
        match poll {
            DriveChangePoll::CursorInvalidated {
                new_start_page_token,
            } => {
                let batch =
                    ChangeWorkBatch::full_scan(FullScanFallbackReason::ProviderCursorInvalidated);
                processor.process(&batch)?;

                let mut recovered_cursor = DriveChangeCursor::new(new_start_page_token.clone())?;
                recovered_cursor.finish_batch(new_start_page_token, input.polled_at)?;
                cursor_store.save_cursor(&recovered_cursor)?;

                return Ok(ChangeFeedCycleOutcome {
                    pages_polled,
                    entries_received: changes.len(),
                    work_items_processed: batch.items.len(),
                    full_scan_used: true,
                    cursor_saved: true,
                });
            }
            DriveChangePoll::Page(page) => {
                pages_polled += 1;
                changes.extend(page.changes);
                match (page.next_page_token, page.new_start_page_token) {
                    (Some(next_page_token), None) => page_token = next_page_token,
                    (None, Some(new_start_page_token)) => break new_start_page_token,
                    _ => return Err(ChangeFeedError::InvalidPageShape),
                }
            }
        }
    };

    let entries_received = changes.len();
    let batch = classify_drive_changes(changes, input.mode, input.dry_run, echo_guard);
    processor.process(&batch)?;

    let mut advanced_cursor = cursor;
    advanced_cursor.finish_batch(final_page_token, input.polled_at)?;
    cursor_store.save_cursor(&advanced_cursor)?;

    Ok(ChangeFeedCycleOutcome {
        pages_polled,
        entries_received,
        work_items_processed: batch.items.len(),
        full_scan_used: batch.requires_full_scan(),
        cursor_saved: true,
    })
}

fn classify_single_change(
    change: DriveChangeEntry,
    mode: AdapterMode,
    dry_run: bool,
    echo_guard: &EchoGuard,
    batch: &mut ChangeWorkBatch,
) {
    let provider_id = change.provider_id;
    if change.removed {
        batch.items.push(ChangeWorkItem::FullScan {
            reason: FullScanFallbackReason::RemovedEntry,
            provider_id: Some(provider_id),
        });
        return;
    }

    let Some(metadata) = change.metadata else {
        batch.items.push(ChangeWorkItem::FullScan {
            reason: FullScanFallbackReason::MissingMetadata,
            provider_id: Some(provider_id),
        });
        return;
    };

    if metadata.id != provider_id {
        batch.items.push(ChangeWorkItem::FullScan {
            reason: FullScanFallbackReason::MetadataIdentityMismatch,
            provider_id: Some(provider_id),
        });
        return;
    }

    if metadata.kind == DriveEntryKind::Folder {
        batch.items.push(ChangeWorkItem::FullScan {
            reason: FullScanFallbackReason::FolderChanged,
            provider_id: Some(provider_id),
        });
        return;
    }

    let normalized = normalize_drive_metadata(&metadata);
    match normalized.classification {
        DriveEntryClassification::Supported(_) => {}
        DriveEntryClassification::Unsupported(reason) => {
            batch.items.push(ChangeWorkItem::SkipUnsupported {
                provider_id,
                reason,
            });
            return;
        }
    }

    if let Some(observation) = echo_observation(&metadata, change.drive_version.as_deref()) {
        if echo_guard.decision_for(&observation) == EchoDecision::SuppressAdapterEcho {
            batch
                .items
                .push(ChangeWorkItem::ConfirmExportEcho { observation });
            return;
        }
    }

    match import_execution(mode, dry_run) {
        Some(execution) => batch.items.push(ChangeWorkItem::Import {
            metadata,
            execution,
        }),
        None => batch
            .items
            .push(ChangeWorkItem::SkipImportByMode { provider_id, mode }),
    }
}

fn echo_observation(
    metadata: &DriveMetadata,
    drive_version: Option<&str>,
) -> Option<DriveEchoObservation> {
    let mut observation = DriveEchoObservation::new(metadata.id.clone()).ok()?;
    if let Some(checksum) = metadata.md5_checksum.as_deref() {
        observation = observation.with_checksum(checksum).ok()?;
    }
    if let Some(drive_version) = drive_version {
        observation = observation.with_drive_version(drive_version).ok()?;
    }
    Some(observation)
}

fn import_execution(mode: AdapterMode, dry_run: bool) -> Option<ImportExecution> {
    match mode {
        AdapterMode::ImportOnly | AdapterMode::Bidirectional if dry_run => {
            Some(ImportExecution::DryRun)
        }
        AdapterMode::ImportOnly | AdapterMode::Bidirectional => Some(ImportExecution::Submit),
        AdapterMode::DryRun => Some(ImportExecution::DryRun),
        AdapterMode::Disabled | AdapterMode::ReadOnly | AdapterMode::ExportOnly => None,
    }
}

fn current_page_token(cursor: &DriveChangeCursor) -> Option<String> {
    cursor
        .next_page_token
        .as_ref()
        .or(cursor.sync_token.as_ref())
        .or(cursor.start_page_token.as_ref())
        .cloned()
}

fn provider_error(
    error: ProviderError,
    backoff_policy: &ProviderBackoffPolicy,
    attempt: usize,
) -> ChangeFeedError {
    let retry = backoff_policy.classify(error.category(), attempt);
    ChangeFeedError::Provider { error, retry }
}

fn required_provider_id(provider_id: impl Into<String>) -> Result<String, ChangeFeedModelError> {
    let provider_id = provider_id.into();
    let trimmed = provider_id.trim();
    if trimmed.is_empty() {
        return Err(ChangeFeedModelError::EmptyProviderId);
    }
    Ok(trimmed.to_owned())
}

fn required_page_token(page_token: impl Into<String>) -> Result<String, ChangeFeedModelError> {
    let page_token = page_token.into();
    let trimmed = page_token.trim();
    if trimmed.is_empty() {
        return Err(ChangeFeedModelError::EmptyPageToken);
    }
    Ok(trimmed.to_owned())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChangePollTrigger {
    PollTick,
    ProviderSignal,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebouncedPoll {
    pub trigger_count: u32,
    pub trigger_kinds: BTreeSet<ChangePollTrigger>,
    pub first_trigger_millis: u64,
    pub last_trigger_millis: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangePollDebouncer {
    debounce_window: Duration,
    pending: Option<DebouncedPoll>,
}

impl ChangePollDebouncer {
    pub const fn new(debounce_window: Duration) -> Self {
        Self {
            debounce_window,
            pending: None,
        }
    }

    pub fn record_trigger(&mut self, trigger: ChangePollTrigger, at_millis: u64) {
        match &mut self.pending {
            Some(pending) => {
                pending.trigger_count = pending.trigger_count.saturating_add(1);
                pending.trigger_kinds.insert(trigger);
                pending.last_trigger_millis = pending.last_trigger_millis.max(at_millis);
                pending.first_trigger_millis = pending.first_trigger_millis.min(at_millis);
            }
            None => {
                self.pending = Some(DebouncedPoll {
                    trigger_count: 1,
                    trigger_kinds: BTreeSet::from([trigger]),
                    first_trigger_millis: at_millis,
                    last_trigger_millis: at_millis,
                });
            }
        }
    }

    pub fn take_due(&mut self, now_millis: u64) -> Option<DebouncedPoll> {
        let pending = self.pending.as_ref()?;
        let window_millis = u64::try_from(self.debounce_window.as_millis()).unwrap_or(u64::MAX);
        if now_millis < pending.last_trigger_millis.saturating_add(window_millis) {
            return None;
        }
        self.pending.take()
    }

    pub const fn has_pending(&self) -> bool {
        self.pending.is_some()
    }
}

#[cfg(test)]
mod tests;
