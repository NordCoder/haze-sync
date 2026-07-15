use crate::config::AdapterMode;
use crate::drive::ProviderErrorCategory;
use crate::scan::{
    DeleteCandidatePlan, FullScanError, FullScanPlan, ScanSkipReason, SkippedScanEntry,
};
use crate::state::{GDriveMapping, SafeTimestamp, VaultPath};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

pub const GDRIVE_ADAPTER_ID: &str = "gdrive-adapter";
const MAX_DELETE_RUN_ID_LEN: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteExecution {
    Submit,
    DryRun,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteScanIssue {
    PermissionLoss,
    RootOrFolderUnavailable,
    ProviderFailure,
    IncompleteScan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovedProviderIdentity {
    pub provider_id: String,
    pub observed_path: Option<VaultPath>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RecoveredDeleteCandidate {
    pub provider_id: String,
    pub path: VaultPath,
    pub first_detected_at: SafeTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompleteDeleteScan {
    pub delete_candidates: Vec<DeleteCandidatePlan>,
    pub recovered_candidates: Vec<RecoveredDeleteCandidate>,
    pub moved_provider_identities: Vec<MovedProviderIdentity>,
    pub total_mapped_files: u64,
}

impl CompleteDeleteScan {
    #[must_use]
    pub fn from_full_scan(plan: FullScanPlan, mappings: &[GDriveMapping]) -> DeleteScanObservation {
        if plan
            .unsupported
            .iter()
            .any(scan_entry_makes_scan_incomplete)
        {
            return DeleteScanObservation::Unreliable(DeleteScanIssue::IncompleteScan);
        }

        let Some(mapping_snapshot) = MappingSnapshot::new(mappings) else {
            return DeleteScanObservation::Unreliable(DeleteScanIssue::IncompleteScan);
        };

        let mut recovered_candidates = Vec::new();
        for planned_import in &plan.imports {
            if planned_import.clear_delete_candidate {
                let Some(recovered) = mapping_snapshot
                    .recovered_candidate(&planned_import.provider_id, &planned_import.request.path)
                else {
                    return DeleteScanObservation::Unreliable(DeleteScanIssue::IncompleteScan);
                };
                recovered_candidates.push(recovered);
            }
        }
        for unchanged in &plan.unchanged {
            if unchanged.clear_delete_candidate {
                let Some(recovered) =
                    mapping_snapshot.recovered_candidate(&unchanged.provider_id, &unchanged.path)
                else {
                    return DeleteScanObservation::Unreliable(DeleteScanIssue::IncompleteScan);
                };
                recovered_candidates.push(recovered);
            }
        }

        let mut moved_provider_identities = plan
            .unsupported
            .iter()
            .filter_map(|entry| match entry.reason {
                ScanSkipReason::MappingIdentityPathChanged => Some(MovedProviderIdentity {
                    provider_id: entry.provider_id.clone(),
                    observed_path: entry.path.clone(),
                }),
                _ => None,
            })
            .collect::<Vec<_>>();
        moved_provider_identities.sort_by(|left, right| {
            left.provider_id
                .cmp(&right.provider_id)
                .then_with(|| left.observed_path.cmp(&right.observed_path))
        });
        moved_provider_identities.dedup();
        for moved in &moved_provider_identities {
            let Some(mapping) = mapping_snapshot.by_provider_id(moved.provider_id.as_str()) else {
                return DeleteScanObservation::Unreliable(DeleteScanIssue::IncompleteScan);
            };
            if let Some(first_detected_at) = mapping.delete_candidate_since.clone() {
                recovered_candidates.push(RecoveredDeleteCandidate {
                    provider_id: mapping.drive_file_id.clone(),
                    path: mapping.vault_path.clone(),
                    first_detected_at,
                });
            }
        }
        recovered_candidates.sort();
        recovered_candidates.dedup();

        let mut delete_candidates = plan.delete_candidates;
        delete_candidates.sort_by(|left, right| {
            left.path
                .cmp(&right.path)
                .then_with(|| left.provider_id.cmp(&right.provider_id))
        });
        if !mapping_snapshot.delete_candidates_match(&delete_candidates)
            || scan_actions_overlap(&delete_candidates, &recovered_candidates)
        {
            return DeleteScanObservation::Unreliable(DeleteScanIssue::IncompleteScan);
        }

        DeleteScanObservation::Complete(Self {
            delete_candidates,
            recovered_candidates,
            moved_provider_identities,
            total_mapped_files: mappings.len() as u64,
        })
    }
}

struct MappingSnapshot<'a> {
    by_provider_id: BTreeMap<&'a str, &'a GDriveMapping>,
    by_path: BTreeMap<&'a VaultPath, &'a GDriveMapping>,
}

impl<'a> MappingSnapshot<'a> {
    fn new(mappings: &'a [GDriveMapping]) -> Option<Self> {
        let mut by_provider_id = BTreeMap::new();
        let mut by_path = BTreeMap::new();
        for mapping in mappings {
            if by_provider_id
                .insert(mapping.drive_file_id.as_str(), mapping)
                .is_some()
                || by_path.insert(&mapping.vault_path, mapping).is_some()
            {
                return None;
            }
        }
        Some(Self {
            by_provider_id,
            by_path,
        })
    }

    fn by_provider_id(&self, provider_id: &str) -> Option<&'a GDriveMapping> {
        self.by_provider_id.get(provider_id).copied()
    }

    fn recovered_candidate(
        &self,
        provider_id: &str,
        path: &VaultPath,
    ) -> Option<RecoveredDeleteCandidate> {
        let mapping = self.by_provider_id(provider_id)?;
        if mapping.vault_path != *path || self.by_path.get(path).copied()? != mapping {
            return None;
        }
        Some(RecoveredDeleteCandidate {
            provider_id: mapping.drive_file_id.clone(),
            path: mapping.vault_path.clone(),
            first_detected_at: mapping.delete_candidate_since.clone()?,
        })
    }

    fn delete_candidates_match(&self, candidates: &[DeleteCandidatePlan]) -> bool {
        let mut provider_ids = BTreeSet::new();
        let mut paths = BTreeSet::new();
        candidates.iter().all(|candidate| {
            let Some(mapping) = self.by_provider_id(&candidate.provider_id) else {
                return false;
            };
            provider_ids.insert(candidate.provider_id.as_str())
                && paths.insert(&candidate.path)
                && mapping.vault_path == candidate.path
                && mapping.core_revision == candidate.base_revision_id
                && mapping.delete_candidate_since == candidate.previously_detected_at
        })
    }
}

fn scan_actions_overlap(
    delete_candidates: &[DeleteCandidatePlan],
    recovered_candidates: &[RecoveredDeleteCandidate],
) -> bool {
    let recovered_provider_ids = recovered_candidates
        .iter()
        .map(|candidate| candidate.provider_id.as_str())
        .collect::<BTreeSet<_>>();
    let recovered_paths = recovered_candidates
        .iter()
        .map(|candidate| &candidate.path)
        .collect::<BTreeSet<_>>();
    delete_candidates.iter().any(|candidate| {
        recovered_provider_ids.contains(candidate.provider_id.as_str())
            || recovered_paths.contains(&candidate.path)
    })
}

fn scan_entry_makes_scan_incomplete(entry: &SkippedScanEntry) -> bool {
    matches!(
        entry.reason,
        ScanSkipReason::DuplicateProviderEntry | ScanSkipReason::FolderCycle
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeleteScanObservation {
    Complete(CompleteDeleteScan),
    Unreliable(DeleteScanIssue),
}

impl DeleteScanObservation {
    #[must_use]
    pub fn from_full_scan_result(
        result: Result<FullScanPlan, FullScanError>,
        mappings: &[GDriveMapping],
    ) -> Self {
        match result {
            Ok(plan) => CompleteDeleteScan::from_full_scan(plan, mappings),
            Err(error) => Self::Unreliable(classify_full_scan_error(&error)),
        }
    }
}

fn classify_full_scan_error(error: &FullScanError) -> DeleteScanIssue {
    match error {
        FullScanError::Provider(provider_error) => match provider_error.category() {
            ProviderErrorCategory::Auth => DeleteScanIssue::PermissionLoss,
            ProviderErrorCategory::NotFound => DeleteScanIssue::RootOrFolderUnavailable,
            ProviderErrorCategory::RateLimit
            | ProviderErrorCategory::ProviderUnavailable
            | ProviderErrorCategory::Internal => DeleteScanIssue::ProviderFailure,
            ProviderErrorCategory::Unsupported | ProviderErrorCategory::InvalidRequest => {
                DeleteScanIssue::IncompleteScan
            }
        },
        FullScanError::InvalidRootFolder
        | FullScanError::DuplicateMappingDriveFileId
        | FullScanError::DuplicateMappingPath
        | FullScanError::ContentSizeMismatch
        | FullScanError::ContentHashVerificationFailed => DeleteScanIssue::IncompleteScan,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteReconciliationInput {
    pub run_id: String,
    pub mode: AdapterMode,
    pub dry_run: bool,
    pub observed_at: SafeTimestamp,
    pub observation: DeleteScanObservation,
}

impl DeleteReconciliationInput {
    pub fn new(
        run_id: impl Into<String>,
        mode: AdapterMode,
        dry_run: bool,
        observed_at: SafeTimestamp,
        observation: DeleteScanObservation,
    ) -> Result<Self, DeleteModelError> {
        let run_id = run_id.into();
        validate_run_id(&run_id)?;
        Ok(Self {
            run_id,
            mode,
            dry_run,
            observed_at,
            observation,
        })
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ConfirmedDeleteCandidate {
    pub provider_id: String,
    pub path: VaultPath,
    pub base_revision_id: Option<String>,
    pub first_detected_at: SafeTimestamp,
    pub confirmed_at: SafeTimestamp,
    pub operation_id: String,
}

impl fmt::Debug for ConfirmedDeleteCandidate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ConfirmedDeleteCandidate")
            .field("provider_id", &self.provider_id)
            .field("path", &self.path)
            .field("base_revision_id", &self.base_revision_id)
            .field("first_detected_at", &self.first_detected_at)
            .field("confirmed_at", &self.confirmed_at)
            .field("operation_id", &"<redacted-idempotency-key>")
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManualDeleteUnlockAvailability {
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreDeleteGuardBlockReason {
    TooManyDeletes,
    DeleteRatio,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreDeleteGuardDecision {
    Allowed,
    BlockedTooManyDeletes {
        proposed_delete_count: u64,
        max_deletes_per_run: u64,
    },
    BlockedDeleteRatio {
        proposed_delete_count: u64,
        total_files_before_run: u64,
        max_delete_ratio_numerator: u64,
        max_delete_ratio_denominator: u64,
    },
    BlockedRequiresManualUnlock {
        proposed_delete_count: u64,
        total_files_before_run: u64,
        reason: CoreDeleteGuardBlockReason,
    },
}

impl CoreDeleteGuardDecision {
    #[must_use]
    pub const fn is_allowed(&self) -> bool {
        matches!(self, Self::Allowed)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeleteBlockReason {
    ScanUnreliable(DeleteScanIssue),
    StateChangedSinceScan,
    ModeDoesNotImportDeletes {
        mode: AdapterMode,
    },
    AdapterDeleteCountExceeded {
        proposed_delete_count: u64,
        max_deletes_per_run: u32,
    },
    AdapterDeleteRatioExceeded {
        proposed_delete_count: u64,
        total_files_before_run: u64,
        max_delete_ratio_percent: u8,
    },
    CoreGuardBlocked(CoreDeleteGuardDecision),
    CoreRejectedUnsafeDelete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreDeleteRejectedReason {
    StaleBaseRevision,
    UnsafeDelete,
    IdempotencyConflict,
    ValidationError,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteRejection {
    pub path: VaultPath,
    pub reason: CoreDeleteRejectedReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeleteSafetyNotice {
    ProviderIdentityMoved(MovedProviderIdentity),
    FirstAbsenceRecorded { path: VaultPath },
    Recovered { path: VaultPath },
    ConfirmedAbsence { path: VaultPath },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteReconciliationOutcome {
    pub execution: Option<DeleteExecution>,
    pub candidates_marked: usize,
    pub candidate_mark_previews: usize,
    pub candidates_cleared: usize,
    pub candidate_clear_previews: usize,
    pub confirmed_candidates: usize,
    pub delete_submissions: usize,
    pub delete_previews: usize,
    pub mappings_retired: usize,
    pub rejections: Vec<DeleteRejection>,
    pub notices: Vec<DeleteSafetyNotice>,
    pub blocked: Option<DeleteBlockReason>,
    pub core_guard_decision: Option<CoreDeleteGuardDecision>,
    pub manual_unlock: ManualDeleteUnlockAvailability,
}

impl Default for DeleteReconciliationOutcome {
    fn default() -> Self {
        Self {
            execution: None,
            candidates_marked: 0,
            candidate_mark_previews: 0,
            candidates_cleared: 0,
            candidate_clear_previews: 0,
            confirmed_candidates: 0,
            delete_submissions: 0,
            delete_previews: 0,
            mappings_retired: 0,
            rejections: Vec::new(),
            notices: Vec::new(),
            blocked: None,
            core_guard_decision: None,
            manual_unlock: ManualDeleteUnlockAvailability::Unavailable,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteModelError {
    InvalidRunId,
}

impl fmt::Display for DeleteModelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidRunId => "delete reconciliation run id is invalid",
        })
    }
}

impl Error for DeleteModelError {}

fn validate_run_id(run_id: &str) -> Result<(), DeleteModelError> {
    if run_id.is_empty()
        || run_id.len() > MAX_DELETE_RUN_ID_LEN
        || run_id.as_bytes().contains(&0)
        || !run_id.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.')
        })
    {
        return Err(DeleteModelError::InvalidRunId);
    }
    Ok(())
}
