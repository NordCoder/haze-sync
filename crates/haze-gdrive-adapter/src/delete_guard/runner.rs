use super::core::{
    CoreDeleteError, CoreDeleteGateway, CoreDeleteGuardRequest, CoreDeleteRequest,
    CoreDeleteResponse,
};
use super::model::{
    CompleteDeleteScan, ConfirmedDeleteCandidate, CoreDeleteRejectedReason, DeleteBlockReason,
    DeleteExecution, DeleteReconciliationInput, DeleteReconciliationOutcome, DeleteRejection,
    DeleteSafetyNotice, DeleteScanIssue, DeleteScanObservation, RecoveredDeleteCandidate,
    GDRIVE_ADAPTER_ID,
};
use super::state_store::{DeleteCandidateStateStore, DeleteMappingState, DeleteStateError};
use crate::config::{AdapterMode, DeleteSafetyConfig};
use crate::hash::ContentSha256;
use crate::scan::DeleteCandidatePlan;
use crate::state::{SafeTimestamp, VaultPath};
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeleteReconciliationError {
    Core(CoreDeleteError),
    State(DeleteStateError),
}

impl fmt::Display for DeleteReconciliationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Core(error) => error.fmt(formatter),
            Self::State(error) => error.fmt(formatter),
        }
    }
}

impl Error for DeleteReconciliationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Core(error) => Some(error),
            Self::State(error) => Some(error),
        }
    }
}

impl From<CoreDeleteError> for DeleteReconciliationError {
    fn from(error: CoreDeleteError) -> Self {
        Self::Core(error)
    }
}

impl From<DeleteStateError> for DeleteReconciliationError {
    fn from(error: DeleteStateError) -> Self {
        Self::State(error)
    }
}

pub fn run_delete_reconciliation(
    core: &mut impl CoreDeleteGateway,
    state_store: &mut impl DeleteCandidateStateStore,
    delete_safety: DeleteSafetyConfig,
    input: DeleteReconciliationInput,
) -> Result<DeleteReconciliationOutcome, DeleteReconciliationError> {
    let mut outcome = DeleteReconciliationOutcome::default();
    let Some(execution) = delete_execution(input.mode, input.dry_run) else {
        outcome.blocked = Some(DeleteBlockReason::ModeDoesNotImportDeletes { mode: input.mode });
        return Ok(outcome);
    };
    outcome.execution = Some(execution);

    let scan = match input.observation {
        DeleteScanObservation::Complete(scan) => scan,
        DeleteScanObservation::Unreliable(issue) => {
            outcome.blocked = Some(DeleteBlockReason::ScanUnreliable(issue));
            return Ok(outcome);
        }
    };

    if scan_evidence_is_ambiguous(&scan, &input.observed_at) {
        outcome.blocked = Some(DeleteBlockReason::ScanUnreliable(
            DeleteScanIssue::IncompleteScan,
        ));
        return Ok(outcome);
    }

    let total_mapped_files = match validate_scan_against_state(state_store, &scan) {
        Ok(total_mapped_files) => total_mapped_files,
        Err(error) if error.is_state_drift() => {
            outcome.blocked = Some(DeleteBlockReason::StateChangedSinceScan);
            return Ok(outcome);
        }
        Err(error) => return Err(error.into()),
    };

    for moved in scan.moved_provider_identities {
        outcome
            .notices
            .push(DeleteSafetyNotice::ProviderIdentityMoved(moved));
    }

    apply_recoveries(
        state_store,
        scan.recovered_candidates,
        execution,
        &mut outcome,
    )?;

    let mut confirmed = Vec::new();
    for candidate in scan.delete_candidates {
        classify_candidate(
            candidate,
            execution,
            state_store,
            &mut outcome,
            &mut confirmed,
        )?;
    }
    confirmed.sort_by(|left, right| left.path.cmp(&right.path));
    outcome.confirmed_candidates = confirmed.len();

    if confirmed.is_empty() {
        return Ok(outcome);
    }

    if state_changed_before_guard(state_store, total_mapped_files, &confirmed)? {
        outcome.blocked = Some(DeleteBlockReason::StateChangedSinceScan);
        return Ok(outcome);
    }

    let proposed_delete_count = confirmed.len() as u64;
    if proposed_delete_count > u64::from(delete_safety.max_deletes_per_run) {
        outcome.blocked = Some(DeleteBlockReason::AdapterDeleteCountExceeded {
            proposed_delete_count,
            max_deletes_per_run: delete_safety.max_deletes_per_run,
        });
        return Ok(outcome);
    }
    if delete_ratio_exceeded(
        proposed_delete_count,
        total_mapped_files,
        delete_safety.max_delete_ratio_percent,
    ) {
        outcome.blocked = Some(DeleteBlockReason::AdapterDeleteRatioExceeded {
            proposed_delete_count,
            total_files_before_run: total_mapped_files,
            max_delete_ratio_percent: delete_safety.max_delete_ratio_percent,
        });
        return Ok(outcome);
    }

    let guard_request = CoreDeleteGuardRequest {
        adapter_id: GDRIVE_ADAPTER_ID.to_owned(),
        run_id: input.run_id,
        proposed_delete_count,
        total_files_before_run: total_mapped_files,
    };
    let guard_decision = core.evaluate_delete_guard(&guard_request)?;
    outcome.core_guard_decision = Some(guard_decision.clone());
    if !guard_decision.is_allowed() {
        outcome.blocked = Some(DeleteBlockReason::CoreGuardBlocked(guard_decision));
        return Ok(outcome);
    }

    if execution == DeleteExecution::DryRun {
        outcome.delete_previews = confirmed.len();
        return Ok(outcome);
    }

    apply_confirmed_deletes(core, state_store, confirmed, &mut outcome)?;
    Ok(outcome)
}

fn validate_scan_against_state(
    state_store: &impl DeleteCandidateStateStore,
    scan: &CompleteDeleteScan,
) -> Result<u64, DeleteStateError> {
    let current_mapping_count = state_store.current_mapping_count()?;
    if current_mapping_count != scan.total_mapped_files {
        return Err(DeleteStateError::MappingNotFound);
    }

    for recovered in &scan.recovered_candidates {
        let state = state_store.mapping_state(&recovered.provider_id, &recovered.path)?;
        if state.delete_candidate_since.as_ref() != Some(&recovered.first_detected_at) {
            return Err(DeleteStateError::CandidateTimestampMismatch);
        }
    }
    for candidate in &scan.delete_candidates {
        let state = state_store.mapping_state(&candidate.provider_id, &candidate.path)?;
        validate_candidate_snapshot(candidate, &state)?;
    }

    Ok(current_mapping_count)
}

fn validate_candidate_snapshot(
    candidate: &DeleteCandidatePlan,
    state: &DeleteMappingState,
) -> Result<(), DeleteStateError> {
    if state.base_revision_id != candidate.base_revision_id {
        return Err(DeleteStateError::CoreRevisionMismatch);
    }
    if state.delete_candidate_since != candidate.previously_detected_at {
        return Err(DeleteStateError::CandidateTimestampMismatch);
    }
    Ok(())
}

fn apply_recoveries(
    state_store: &mut impl DeleteCandidateStateStore,
    recoveries: Vec<RecoveredDeleteCandidate>,
    execution: DeleteExecution,
    outcome: &mut DeleteReconciliationOutcome,
) -> Result<(), DeleteStateError> {
    for recovered in recoveries {
        match execution {
            DeleteExecution::Submit => {
                state_store.clear_candidate(
                    &recovered.provider_id,
                    &recovered.path,
                    &recovered.first_detected_at,
                )?;
                outcome.candidates_cleared = outcome.candidates_cleared.saturating_add(1);
            }
            DeleteExecution::DryRun => {
                outcome.candidate_clear_previews =
                    outcome.candidate_clear_previews.saturating_add(1);
            }
        }
        outcome.notices.push(DeleteSafetyNotice::Recovered {
            path: recovered.path,
        });
    }
    Ok(())
}

fn classify_candidate(
    candidate: DeleteCandidatePlan,
    execution: DeleteExecution,
    state_store: &mut impl DeleteCandidateStateStore,
    outcome: &mut DeleteReconciliationOutcome,
    confirmed: &mut Vec<ConfirmedDeleteCandidate>,
) -> Result<(), DeleteStateError> {
    match candidate.previously_detected_at {
        None => {
            match execution {
                DeleteExecution::Submit => {
                    state_store.mark_candidate(
                        &candidate.provider_id,
                        &candidate.path,
                        &candidate.detected_at,
                    )?;
                    outcome.candidates_marked = outcome.candidates_marked.saturating_add(1);
                }
                DeleteExecution::DryRun => {
                    outcome.candidate_mark_previews =
                        outcome.candidate_mark_previews.saturating_add(1);
                }
            }
            outcome
                .notices
                .push(DeleteSafetyNotice::FirstAbsenceRecorded {
                    path: candidate.path,
                });
        }
        Some(first_detected_at) if first_detected_at == candidate.detected_at => {
            outcome
                .notices
                .push(DeleteSafetyNotice::FirstAbsenceRecorded {
                    path: candidate.path,
                });
        }
        Some(first_detected_at) => {
            let operation_id = delete_operation_id(
                &candidate.provider_id,
                &candidate.path,
                candidate.base_revision_id.as_deref(),
                first_detected_at.as_str(),
            );
            outcome.notices.push(DeleteSafetyNotice::ConfirmedAbsence {
                path: candidate.path.clone(),
            });
            confirmed.push(ConfirmedDeleteCandidate {
                provider_id: candidate.provider_id,
                path: candidate.path,
                base_revision_id: candidate.base_revision_id,
                first_detected_at,
                confirmed_at: candidate.detected_at,
                operation_id,
            });
        }
    }
    Ok(())
}

fn state_changed_before_guard(
    state_store: &impl DeleteCandidateStateStore,
    expected_mapping_count: u64,
    confirmed: &[ConfirmedDeleteCandidate],
) -> Result<bool, DeleteStateError> {
    if state_store.current_mapping_count()? != expected_mapping_count {
        return Ok(true);
    }
    for candidate in confirmed {
        match validate_confirmed_candidate(state_store, candidate) {
            Ok(()) => {}
            Err(error) if error.is_state_drift() => return Ok(true),
            Err(error) => return Err(error),
        }
    }
    Ok(false)
}

fn validate_confirmed_candidate(
    state_store: &impl DeleteCandidateStateStore,
    candidate: &ConfirmedDeleteCandidate,
) -> Result<(), DeleteStateError> {
    let state = state_store.mapping_state(&candidate.provider_id, &candidate.path)?;
    if state.base_revision_id != candidate.base_revision_id {
        return Err(DeleteStateError::CoreRevisionMismatch);
    }
    if state.delete_candidate_since.as_ref() != Some(&candidate.first_detected_at) {
        return Err(DeleteStateError::CandidateTimestampMismatch);
    }
    Ok(())
}

fn apply_confirmed_deletes(
    core: &mut impl CoreDeleteGateway,
    state_store: &mut impl DeleteCandidateStateStore,
    confirmed: Vec<ConfirmedDeleteCandidate>,
    outcome: &mut DeleteReconciliationOutcome,
) -> Result<(), DeleteReconciliationError> {
    for candidate in confirmed {
        match validate_confirmed_candidate(state_store, &candidate) {
            Ok(()) => {}
            Err(error) if error.is_state_drift() => {
                outcome.blocked = Some(DeleteBlockReason::StateChangedSinceScan);
                break;
            }
            Err(error) => return Err(error.into()),
        }

        let response = core.delete_file(CoreDeleteRequest {
            operation_id: candidate.operation_id,
            path: candidate.path.clone(),
            base_revision_id: candidate.base_revision_id.clone(),
        })?;
        outcome.delete_submissions = outcome.delete_submissions.saturating_add(1);
        match response {
            CoreDeleteResponse::Tombstoned | CoreDeleteResponse::NotFound => {
                state_store.retire_mapping(
                    &candidate.provider_id,
                    &candidate.path,
                    &candidate.first_detected_at,
                    candidate.base_revision_id.as_deref(),
                )?;
                outcome.mappings_retired = outcome.mappings_retired.saturating_add(1);
            }
            CoreDeleteResponse::Rejected {
                reason: CoreDeleteRejectedReason::UnsafeDelete,
            } => {
                outcome.rejections.push(DeleteRejection {
                    path: candidate.path,
                    reason: CoreDeleteRejectedReason::UnsafeDelete,
                });
                outcome.blocked = Some(DeleteBlockReason::CoreRejectedUnsafeDelete);
                break;
            }
            CoreDeleteResponse::Rejected { reason } => {
                outcome.rejections.push(DeleteRejection {
                    path: candidate.path,
                    reason,
                });
            }
        }
    }
    Ok(())
}

fn scan_evidence_is_ambiguous(scan: &CompleteDeleteScan, observed_at: &SafeTimestamp) -> bool {
    let mut recovered_paths = BTreeSet::new();
    let mut recovered_provider_ids = BTreeSet::new();
    for recovered in &scan.recovered_candidates {
        if !recovered_paths.insert(&recovered.path)
            || !recovered_provider_ids.insert(recovered.provider_id.as_str())
        {
            return true;
        }
    }

    let mut candidate_paths = BTreeSet::new();
    let mut candidate_provider_ids = BTreeSet::new();
    for candidate in &scan.delete_candidates {
        if candidate.detected_at != *observed_at
            || candidate
                .previously_detected_at
                .as_ref()
                .is_some_and(|first_detected_at| candidate.detected_at < *first_detected_at)
            || !candidate_paths.insert(&candidate.path)
            || !candidate_provider_ids.insert(candidate.provider_id.as_str())
            || recovered_paths.contains(&candidate.path)
            || recovered_provider_ids.contains(candidate.provider_id.as_str())
        {
            return true;
        }
    }

    let mut moved_provider_ids = BTreeSet::new();
    for moved in &scan.moved_provider_identities {
        if !moved_provider_ids.insert(moved.provider_id.as_str()) {
            return true;
        }
    }
    candidate_provider_ids
        .iter()
        .any(|provider_id| moved_provider_ids.contains(provider_id))
}

fn delete_execution(mode: AdapterMode, dry_run: bool) -> Option<DeleteExecution> {
    match mode {
        AdapterMode::ImportOnly | AdapterMode::Bidirectional if dry_run => {
            Some(DeleteExecution::DryRun)
        }
        AdapterMode::ImportOnly | AdapterMode::Bidirectional => Some(DeleteExecution::Submit),
        AdapterMode::DryRun => Some(DeleteExecution::DryRun),
        AdapterMode::Disabled | AdapterMode::ReadOnly | AdapterMode::ExportOnly => None,
    }
}

fn delete_ratio_exceeded(delete_count: u64, total_count: u64, max_percent: u8) -> bool {
    if delete_count == 0 {
        return false;
    }
    if total_count == 0 {
        return true;
    }
    u128::from(delete_count).saturating_mul(100)
        > u128::from(total_count).saturating_mul(u128::from(max_percent))
}

fn delete_operation_id(
    provider_id: &str,
    path: &VaultPath,
    base_revision_id: Option<&str>,
    first_detected_at: &str,
) -> String {
    let mut material = Vec::new();
    append_hash_field(&mut material, Some(provider_id));
    append_hash_field(&mut material, Some(path.as_str()));
    append_hash_field(&mut material, base_revision_id);
    append_hash_field(&mut material, Some(first_detected_at));
    format!(
        "gdrive-delete-{}",
        ContentSha256::from_content(&material).as_hex()
    )
}

fn append_hash_field(material: &mut Vec<u8>, value: Option<&str>) {
    match value {
        Some(value) => {
            material.push(1);
            material.extend_from_slice(&(value.len() as u64).to_be_bytes());
            material.extend_from_slice(value.as_bytes());
        }
        None => material.push(0),
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn ratio_check_is_strictly_greater_than_threshold() {
        assert!(!delete_ratio_exceeded(5, 100, 5));
        assert!(delete_ratio_exceeded(6, 100, 5));
        assert!(delete_ratio_exceeded(1, 0, 100));
    }

    #[test]
    fn operation_id_hashing_distinguishes_field_boundaries() {
        let first = delete_operation_id("a\nb", &VaultPath::new("c.md").unwrap(), None, "d");
        let second = delete_operation_id("a", &VaultPath::new("b/c.md").unwrap(), None, "d");

        assert_ne!(first, second);
    }
}
