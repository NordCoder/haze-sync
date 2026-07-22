use crate::config::AdapterMode;
use crate::drive::{DriveMetadata, UnsupportedEntryReason};
use crate::scan::ImportExecution;
use crate::state::{DriveChangeCursor, DriveEchoObservation, SafeTimestamp};
use std::error::Error;
use std::fmt;

pub(super) const DEFAULT_MAX_PAGES_PER_POLL: usize = 128;

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

    pub(super) fn into_parts(self) -> (String, bool, Option<DriveMetadata>, Option<String>) {
        (
            self.provider_id,
            self.removed,
            self.metadata,
            self.drive_version,
        )
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

    pub(super) fn into_parts(self) -> (Vec<DriveChangeEntry>, Option<String>, Option<String>) {
        (
            self.changes,
            self.next_page_token,
            self.new_start_page_token,
        )
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

    pub(super) fn retain_only_full_scan_work_if_required(&mut self) {
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

fn required_provider_id(provider_id: impl Into<String>) -> Result<String, ChangeFeedModelError> {
    let provider_id = provider_id.into();
    let trimmed = provider_id.trim();
    if trimmed.is_empty() {
        return Err(ChangeFeedModelError::EmptyProviderId);
    }
    Ok(trimmed.to_owned())
}

pub(super) fn required_page_token(
    page_token: impl Into<String>,
) -> Result<String, ChangeFeedModelError> {
    let page_token = page_token.into();
    let trimmed = page_token.trim();
    if trimmed.is_empty() {
        return Err(ChangeFeedModelError::EmptyPageToken);
    }
    Ok(trimmed.to_owned())
}
