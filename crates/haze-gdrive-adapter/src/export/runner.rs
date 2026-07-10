use super::core::{CoreClientError, CoreClientErrorCategory, CoreExportClient};
use super::model::{
    CoreExportChange, CoreFileContent, DriveCreateTarget, ExportCycleInput, ExportCycleOutcome,
    ExportExecution, ExportPlanItem, ExportSkipReason, VerifiedExportSource,
};
use super::provider::{
    DriveCreateExportRequest, DriveExportError, DriveExportErrorCategory, DriveExportProvider,
    DriveExportReceipt, DriveTrashExportRequest, DriveUpdateExportRequest, ExportRetryDisposition,
    ExportRetryPolicy,
};
use super::state_store::{ExportStateError, ExportStateStore};
use crate::config::AdapterMode;
use crate::drive::{MIME_TEXT_MARKDOWN, MIME_TEXT_PLAIN};
use crate::hash::ContentSha256;
use crate::state::{
    CoreChangeCursor, CoreStateObservation, DriveEchoObservation, DriveStateObservation, EchoGuard,
    EchoGuardEntry, GDriveMapping, SafeTimestamp, StateError, VaultPath,
};
use std::error::Error;
use std::fmt;

const MIME_APPLICATION_OCTET_STREAM: &str = "application/octet-stream";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportError {
    Core {
        error: CoreClientError,
        retry: ExportRetryDisposition,
    },
    Provider {
        error: DriveExportError,
        retry: ExportRetryDisposition,
    },
    StateStore(ExportStateError),
    State(StateError),
    InvalidCorePage,
    PageLimitZero,
    MissingSource,
    MissingCreateTarget,
    SourcePathMismatch,
    SourceRevisionMismatch,
    SourceDeclaredHashMismatch,
    SourceDeclaredSizeMismatch,
    SourceContentHashMismatch,
    SourceContentSizeMismatch,
}

impl fmt::Display for ExportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Core { error, retry } => {
                write!(formatter, "{error}; retry disposition: {retry:?}")
            }
            Self::Provider { error, retry } => {
                write!(formatter, "{error}; retry disposition: {retry:?}")
            }
            Self::StateStore(error) => error.fmt(formatter),
            Self::State(error) => error.fmt(formatter),
            Self::InvalidCorePage => {
                formatter.write_str("Core export page does not match the requested cursor")
            }
            Self::PageLimitZero => formatter.write_str("Core export page limit must be positive"),
            Self::MissingSource => formatter.write_str("Core export source content is missing"),
            Self::MissingCreateTarget => {
                formatter.write_str("Drive create target is missing for export")
            }
            Self::SourcePathMismatch => {
                formatter.write_str("Core export source path does not match change metadata")
            }
            Self::SourceRevisionMismatch => {
                formatter.write_str("Core export source revision does not match change metadata")
            }
            Self::SourceDeclaredHashMismatch => formatter
                .write_str("Core export source declared hash does not match change metadata"),
            Self::SourceDeclaredSizeMismatch => formatter
                .write_str("Core export source declared size does not match change metadata"),
            Self::SourceContentHashMismatch => {
                formatter.write_str("Core export source bytes failed SHA-256 verification")
            }
            Self::SourceContentSizeMismatch => {
                formatter.write_str("Core export source byte length failed verification")
            }
        }
    }
}

impl Error for ExportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Core { error, .. } => Some(error),
            Self::Provider { error, .. } => Some(error),
            Self::StateStore(error) => Some(error),
            Self::State(error) => Some(error),
            Self::InvalidCorePage
            | Self::PageLimitZero
            | Self::MissingSource
            | Self::MissingCreateTarget
            | Self::SourcePathMismatch
            | Self::SourceRevisionMismatch
            | Self::SourceDeclaredHashMismatch
            | Self::SourceDeclaredSizeMismatch
            | Self::SourceContentHashMismatch
            | Self::SourceContentSizeMismatch => None,
        }
    }
}

impl From<ExportStateError> for ExportError {
    fn from(error: ExportStateError) -> Self {
        Self::StateStore(error)
    }
}

impl From<StateError> for ExportError {
    fn from(error: StateError) -> Self {
        Self::State(error)
    }
}

pub fn plan_core_export(
    change: CoreExportChange,
    mapping: Option<GDriveMapping>,
    create_target: Option<DriveCreateTarget>,
    source: Option<CoreFileContent>,
    mode: AdapterMode,
    dry_run: bool,
) -> Result<ExportPlanItem, ExportError> {
    let Some(execution) = export_execution(mode, dry_run) else {
        return Ok(ExportPlanItem::Skip {
            change,
            reason: ExportSkipReason::ModeDoesNotExport,
        });
    };

    if mapping
        .as_ref()
        .and_then(|mapping| mapping.core_sequence)
        .is_some_and(|sequence| sequence >= change.sequence())
    {
        return Ok(ExportPlanItem::Skip {
            change,
            reason: ExportSkipReason::MappingAlreadyReflectsChange,
        });
    }

    match &change {
        CoreExportChange::UpsertFile { .. } => {
            let source = verify_source(&change, source.ok_or(ExportError::MissingSource)?)?;
            match mapping {
                Some(mapping) if !mapping_is_confirmed_trashed(&mapping) => {
                    Ok(ExportPlanItem::Update {
                        change,
                        mapping,
                        source,
                        execution,
                    })
                }
                _ => Ok(ExportPlanItem::Create {
                    change,
                    target: create_target.ok_or(ExportError::MissingCreateTarget)?,
                    source,
                    execution,
                }),
            }
        }
        CoreExportChange::Tombstone { .. } => match mapping {
            Some(mapping) => Ok(ExportPlanItem::Trash {
                change,
                mapping,
                execution,
            }),
            None => Ok(ExportPlanItem::Skip {
                change,
                reason: ExportSkipReason::MissingMappingForTombstone,
            }),
        },
    }
}

pub fn run_export_cycle(
    core: &impl CoreExportClient,
    provider: &mut impl DriveExportProvider,
    state_store: &mut impl ExportStateStore,
    echo_guard: &mut EchoGuard,
    retry_policy: &ExportRetryPolicy,
    input: ExportCycleInput,
) -> Result<ExportCycleOutcome, ExportError> {
    if input.page_limit == 0 {
        return Err(ExportError::PageLimitZero);
    }

    let mut cursor = state_store.load_cursor()?;
    let page = core
        .list_changes(cursor.next_sequence, input.page_limit)
        .map_err(|error| core_error(error, retry_policy, input.provider_attempt))?;
    if page.from_sequence != cursor.next_sequence {
        return Err(ExportError::InvalidCorePage);
    }

    let should_persist_cursor =
        export_execution(input.mode, input.dry_run) == Some(ExportExecution::Submit);
    let mut outcome = ExportCycleOutcome {
        changes_received: page.changes.len(),
        work_items_planned: 0,
        provider_mutations: 0,
        mappings_saved: 0,
        echoes_recorded: 0,
        cursor_saved: false,
        next_sequence: page.next_sequence,
        has_more: page.has_more,
    };

    for change in page.changes {
        let mapping = state_store.load_mapping(change.path())?;
        let (source, create_target) = load_plan_inputs(core, state_store, &change, mapping.as_ref(), &input, retry_policy)?;
        let plan = plan_core_export(
            change,
            mapping,
            create_target,
            source,
            input.mode,
            input.dry_run,
        )?;
        outcome.work_items_planned = outcome.work_items_planned.saturating_add(1);
        let applied = apply_export_plan(
            provider,
            state_store,
            echo_guard,
            retry_policy,
            input.provider_attempt,
            input.applied_at.clone(),
            plan,
        )?;
        outcome.provider_mutations = outcome
            .provider_mutations
            .saturating_add(applied.provider_mutations);
        outcome.mappings_saved = outcome.mappings_saved.saturating_add(applied.mappings_saved);
        outcome.echoes_recorded = outcome.echoes_recorded.saturating_add(applied.echoes_recorded);
    }

    if should_persist_cursor {
        cursor.advance_to(page.next_sequence, input.applied_at)?;
        state_store.save_cursor(&cursor)?;
        outcome.cursor_saved = true;
    }

    Ok(outcome)
}

fn load_plan_inputs(
    core: &impl CoreExportClient,
    state_store: &impl ExportStateStore,
    change: &CoreExportChange,
    mapping: Option<&GDriveMapping>,
    input: &ExportCycleInput,
    retry_policy: &ExportRetryPolicy,
) -> Result<(Option<CoreFileContent>, Option<DriveCreateTarget>), ExportError> {
    if export_execution(input.mode, input.dry_run).is_none()
        || mapping
            .and_then(|mapping| mapping.core_sequence)
            .is_some_and(|sequence| sequence >= change.sequence())
    {
        return Ok((None, None));
    }

    match change {
        CoreExportChange::UpsertFile {
            path, revision_id, ..
        } => {
            let source = core
                .download_revision(path, revision_id)
                .map_err(|error| core_error(error, retry_policy, input.provider_attempt))?;
            let create_target = match mapping {
                Some(mapping) if !mapping_is_confirmed_trashed(mapping) => None,
                _ => Some(state_store.resolve_create_target(path)?),
            };
            Ok((Some(source), create_target))
        }
        CoreExportChange::Tombstone { .. } => Ok((None, None)),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct ApplyCounts {
    provider_mutations: usize,
    mappings_saved: usize,
    echoes_recorded: usize,
}

fn apply_export_plan(
    provider: &mut impl DriveExportProvider,
    state_store: &mut impl ExportStateStore,
    echo_guard: &mut EchoGuard,
    retry_policy: &ExportRetryPolicy,
    provider_attempt: usize,
    applied_at: SafeTimestamp,
    plan: ExportPlanItem,
) -> Result<ApplyCounts, ExportError> {
    match plan {
        ExportPlanItem::Skip { .. }
        | ExportPlanItem::Create {
            execution: ExportExecution::DryRun,
            ..
        }
        | ExportPlanItem::Update {
            execution: ExportExecution::DryRun,
            ..
        }
        | ExportPlanItem::Trash {
            execution: ExportExecution::DryRun,
            ..
        } => Ok(ApplyCounts::default()),
        ExportPlanItem::Create {
            change,
            target,
            source,
            execution: ExportExecution::Submit,
        } => {
            let receipt = provider
                .create_file(DriveCreateExportRequest {
                    operation_id: change.operation_id().to_owned(),
                    parent_id: target.parent_id.clone(),
                    name: target.name.clone(),
                    mime_type: mime_type_for_path(change.path()).to_owned(),
                    content_sha256: source.content_sha256,
                    content: source.into_content(),
                })
                .map_err(|error| provider_error(error, retry_policy, provider_attempt))?;
            let mapping = confirmed_create_mapping(&change, target, receipt, applied_at.clone())?;
            save_confirmed_mapping(state_store, echo_guard, mapping, applied_at)?;
            Ok(ApplyCounts {
                provider_mutations: 1,
                mappings_saved: 1,
                echoes_recorded: 1,
            })
        }
        ExportPlanItem::Update {
            change,
            mapping,
            source,
            execution: ExportExecution::Submit,
        } => {
            let receipt = provider
                .update_file(DriveUpdateExportRequest {
                    operation_id: change.operation_id().to_owned(),
                    file_id: mapping.drive_file_id.clone(),
                    expected_revision_token: mapping.drive_version.clone(),
                    mime_type: mime_type_for_path(change.path()).to_owned(),
                    content_sha256: source.content_sha256,
                    content: source.into_content(),
                })
                .map_err(|error| provider_error(error, retry_policy, provider_attempt))?;
            let mapping = confirmed_update_mapping(&change, mapping, receipt, applied_at.clone())?;
            save_confirmed_mapping(state_store, echo_guard, mapping, applied_at)?;
            Ok(ApplyCounts {
                provider_mutations: 1,
                mappings_saved: 1,
                echoes_recorded: 1,
            })
        }
        ExportPlanItem::Trash {
            change,
            mapping,
            execution: ExportExecution::Submit,
        } => {
            let receipt = provider
                .trash_file(DriveTrashExportRequest {
                    operation_id: change.operation_id().to_owned(),
                    file_id: mapping.drive_file_id.clone(),
                    expected_revision_token: mapping.drive_version.clone(),
                })
                .map_err(|error| provider_error(error, retry_policy, provider_attempt))?;
            let mapping = confirmed_trash_mapping(&change, mapping, receipt, applied_at.clone())?;
            save_confirmed_mapping(state_store, echo_guard, mapping, applied_at)?;
            Ok(ApplyCounts {
                provider_mutations: 1,
                mappings_saved: 1,
                echoes_recorded: 1,
            })
        }
    }
}

fn save_confirmed_mapping(
    state_store: &mut impl ExportStateStore,
    echo_guard: &mut EchoGuard,
    mapping: GDriveMapping,
    applied_at: SafeTimestamp,
) -> Result<(), ExportError> {
    let echo_entry = EchoGuardEntry::from_mapping(&mapping, applied_at)?;
    state_store.save_mapping(mapping)?;
    echo_guard.record_exported_write(echo_entry);
    Ok(())
}

fn confirmed_create_mapping(
    change: &CoreExportChange,
    target: DriveCreateTarget,
    receipt: DriveExportReceipt,
    applied_at: SafeTimestamp,
) -> Result<GDriveMapping, ExportError> {
    let mut mapping = GDriveMapping::new(
        change.path().clone(),
        receipt.provider_id.clone(),
        target.parent_id.clone(),
        target.name.clone(),
    )?;
    record_provider_confirmation(
        &mut mapping,
        receipt,
        target.parent_id,
        target.name,
        applied_at.clone(),
    )?;
    record_core_confirmation(&mut mapping, change, applied_at)?;
    Ok(mapping)
}

fn confirmed_update_mapping(
    change: &CoreExportChange,
    mut mapping: GDriveMapping,
    receipt: DriveExportReceipt,
    applied_at: SafeTimestamp,
) -> Result<GDriveMapping, ExportError> {
    let parent_id = mapping.parent_id.clone();
    let name = mapping.name.clone();
    record_provider_confirmation(
        &mut mapping,
        receipt,
        parent_id,
        name,
        applied_at.clone(),
    )?;
    record_core_confirmation(&mut mapping, change, applied_at)?;
    Ok(mapping)
}

fn confirmed_trash_mapping(
    change: &CoreExportChange,
    mut mapping: GDriveMapping,
    receipt: DriveExportReceipt,
    applied_at: SafeTimestamp,
) -> Result<GDriveMapping, ExportError> {
    let parent_id = mapping.parent_id.clone();
    let name = mapping.name.clone();
    record_provider_confirmation(
        &mut mapping,
        receipt,
        parent_id,
        name,
        applied_at.clone(),
    )?;
    mapping.core_revision = None;
    mapping.core_sequence = Some(change.sequence());
    mapping.last_exported_at = Some(applied_at);
    mapping.delete_candidate_since = None;
    Ok(mapping)
}

fn record_provider_confirmation(
    mapping: &mut GDriveMapping,
    receipt: DriveExportReceipt,
    parent_id: String,
    name: String,
    applied_at: SafeTimestamp,
) -> Result<(), ExportError> {
    let observation = DriveStateObservation::new(
        receipt.provider_id,
        parent_id,
        name,
        applied_at,
    )?
    .with_drive_version(receipt.revision_token)?;
    mapping.record_drive_observation(observation)?;
    Ok(())
}

fn record_core_confirmation(
    mapping: &mut GDriveMapping,
    change: &CoreExportChange,
    applied_at: SafeTimestamp,
) -> Result<(), ExportError> {
    let CoreExportChange::UpsertFile {
        revision_id, seq, ..
    } = change
    else {
        return Ok(());
    };
    mapping.record_export(
        CoreStateObservation::new(revision_id.clone(), *seq)?,
        applied_at,
    );
    Ok(())
}

fn verify_source(
    change: &CoreExportChange,
    source: CoreFileContent,
) -> Result<VerifiedExportSource, ExportError> {
    let CoreExportChange::UpsertFile {
        path,
        revision_id,
        content_sha256,
        size_bytes,
        ..
    } = change
    else {
        return Err(ExportError::MissingSource);
    };

    if source.path() != path {
        return Err(ExportError::SourcePathMismatch);
    }
    if source.revision_id() != revision_id {
        return Err(ExportError::SourceRevisionMismatch);
    }
    if source.declared_sha256() != *content_sha256 {
        return Err(ExportError::SourceDeclaredHashMismatch);
    }
    if source.declared_size_bytes() != *size_bytes {
        return Err(ExportError::SourceDeclaredSizeMismatch);
    }
    if source.content().len() as u64 != *size_bytes {
        return Err(ExportError::SourceContentSizeMismatch);
    }
    if ContentSha256::from_content(source.content()) != *content_sha256 {
        return Err(ExportError::SourceContentHashMismatch);
    }

    Ok(VerifiedExportSource::new(
        path.clone(),
        revision_id.clone(),
        *content_sha256,
        *size_bytes,
        source.into_content(),
    ))
}

fn mapping_is_confirmed_trashed(mapping: &GDriveMapping) -> bool {
    mapping.core_revision.is_none()
        && mapping.core_sequence.is_some()
        && mapping.last_exported_at.is_some()
}

fn export_execution(mode: AdapterMode, dry_run: bool) -> Option<ExportExecution> {
    match mode {
        AdapterMode::ExportOnly | AdapterMode::Bidirectional if dry_run => {
            Some(ExportExecution::DryRun)
        }
        AdapterMode::ExportOnly | AdapterMode::Bidirectional => Some(ExportExecution::Submit),
        AdapterMode::DryRun => Some(ExportExecution::DryRun),
        AdapterMode::Disabled | AdapterMode::ReadOnly | AdapterMode::ImportOnly => None,
    }
}

fn mime_type_for_path(path: &VaultPath) -> &'static str {
    let lowercase = path.as_str().to_ascii_lowercase();
    if lowercase.ends_with(".md") || lowercase.ends_with(".markdown") {
        MIME_TEXT_MARKDOWN
    } else if lowercase.ends_with(".txt") {
        MIME_TEXT_PLAIN
    } else {
        MIME_APPLICATION_OCTET_STREAM
    }
}

fn provider_error(
    error: DriveExportError,
    retry_policy: &ExportRetryPolicy,
    attempt: usize,
) -> ExportError {
    let retry = retry_policy.classify_provider(error.category(), attempt);
    ExportError::Provider { error, retry }
}

fn core_error(
    error: CoreClientError,
    retry_policy: &ExportRetryPolicy,
    attempt: usize,
) -> ExportError {
    let provider_category = match error.category() {
        CoreClientErrorCategory::Auth => DriveExportErrorCategory::Auth,
        CoreClientErrorCategory::RateLimit => DriveExportErrorCategory::RateLimit,
        CoreClientErrorCategory::Unavailable => DriveExportErrorCategory::Unavailable,
        CoreClientErrorCategory::NotFound => DriveExportErrorCategory::NotFound,
        CoreClientErrorCategory::InvalidResponse => DriveExportErrorCategory::InvalidRequest,
        CoreClientErrorCategory::Internal => DriveExportErrorCategory::Internal,
    };
    let retry = retry_policy.classify_provider(provider_category, attempt);
    ExportError::Core { error, retry }
}

pub fn echo_observation_for_mapping(mapping: &GDriveMapping) -> Result<DriveEchoObservation, StateError> {
    let mut observation = DriveEchoObservation::new(mapping.drive_file_id.clone())?;
    if let Some(checksum) = mapping.checksum.as_deref() {
        observation = observation.with_checksum(checksum)?;
    }
    if let Some(version) = mapping.drive_version.as_deref() {
        observation = observation.with_drive_version(version)?;
    }
    Ok(observation)
}

pub fn cursor_after_page(
    cursor: &CoreChangeCursor,
    next_sequence: u64,
    applied_at: SafeTimestamp,
) -> Result<CoreChangeCursor, StateError> {
    let mut cursor = cursor.clone();
    cursor.advance_to(next_sequence, applied_at)?;
    Ok(cursor)
}
