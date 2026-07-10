use super::model::{
    ChangeFeedCycleInput, ChangeFeedCycleOutcome, ChangeProcessingError, ChangeWorkBatch,
    ChangeWorkItem, CursorStoreError, DriveChangeEntry, DriveChangePoll, DriveCursorStore,
    FullScanFallbackReason,
};
use super::policy::{ProviderBackoffPolicy, RetryDisposition};
use super::provider::DriveChangeFeedProvider;
use crate::config::AdapterMode;
use crate::drive::{
    normalize_drive_metadata, DriveEntryClassification, DriveEntryKind, DriveMetadata,
    ProviderError,
};
use crate::scan::ImportExecution;
use crate::state::{
    DriveChangeCursor, DriveEchoObservation, EchoDecision, EchoGuard, StateError,
};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

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

pub fn run_change_feed_cycle(
    provider: &impl DriveChangeFeedProvider,
    cursor_store: &mut impl DriveCursorStore,
    processor: &mut impl super::model::ChangeWorkProcessor,
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
            .entry(change.provider_id().to_owned())
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
    processor: &mut impl super::model::ChangeWorkProcessor,
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
    processor: &mut impl super::model::ChangeWorkProcessor,
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
                let (page_changes, next_page_token, new_start_page_token) = page.into_parts();
                changes.extend(page_changes);
                match (next_page_token, new_start_page_token) {
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
    let (provider_id, removed, metadata, drive_version) = change.into_parts();
    if removed {
        batch.items.push(ChangeWorkItem::FullScan {
            reason: FullScanFallbackReason::RemovedEntry,
            provider_id: Some(provider_id),
        });
        return;
    }

    let Some(metadata) = metadata else {
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

    if let Some(observation) = echo_observation(&metadata, drive_version.as_deref()) {
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
