//! Put-only Worktree import planning.
//!
//! Local file creates and modifications are planned here. Local absences are
//! intentionally excluded and must flow through `delete_guard`, preventing the
//! general import runner from bypassing mass-delete authorization.

use crate::import_planner::{
    WorktreeAcceptedImport, WorktreeAppliedPathState, WorktreeBaseRevision, WorktreeImportAction,
    WorktreeImportClient, WorktreeImportFile, WorktreeImportOutcome, WorktreeImportPlanError,
    WorktreePutImport, WorktreeStateSnapshot, WorktreeUnchangedFile,
};
use haze_sync_common::VaultPath;
use std::collections::BTreeMap;

/// Deterministic put-only import plan.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorktreeImportPlan {
    actions: Vec<WorktreeImportAction>,
    unchanged: Vec<WorktreeUnchangedFile>,
}

impl WorktreeImportPlan {
    /// Planned Core/API put requests in deterministic vault-path order.
    #[must_use]
    pub fn actions(&self) -> &[WorktreeImportAction] {
        &self.actions
    }

    /// Local files that already match last-applied Core state.
    #[must_use]
    pub fn unchanged(&self) -> &[WorktreeUnchangedFile] {
        &self.unchanged
    }

    /// True when there are no put requests.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }
}

/// Plans local file creates and modifications without inferring deletes.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorktreeImportPlanner;

impl WorktreeImportPlanner {
    /// Build a put-only plan from stable local files and last-applied state.
    pub fn plan(
        state: &WorktreeStateSnapshot,
        local_files: impl IntoIterator<Item = WorktreeImportFile>,
    ) -> Result<WorktreeImportPlan, WorktreeImportPlanError> {
        let local_files = stable_files_by_path(local_files)?;
        let mut actions = Vec::new();
        let mut unchanged = Vec::new();

        for (vault_path, local_file) in &local_files {
            match state.path_state(vault_path) {
                None => actions.push(put_action(local_file, WorktreeBaseRevision::Null)),
                Some(WorktreeAppliedPathState::Tombstoned(tombstone)) => {
                    actions.push(put_action(
                        local_file,
                        WorktreeBaseRevision::Known(tombstone.revision_id.clone()),
                    ));
                }
                Some(WorktreeAppliedPathState::Present(applied)) => {
                    if applied.content_hash == local_file.snapshot.content_hash {
                        unchanged.push(WorktreeUnchangedFile {
                            vault_path: vault_path.clone(),
                            revision_id: applied.revision_id.clone(),
                            content_hash: applied.content_hash,
                        });
                    } else {
                        actions.push(put_action(
                            local_file,
                            WorktreeBaseRevision::Known(applied.revision_id.clone()),
                        ));
                    }
                }
            }
        }

        Ok(WorktreeImportPlan { actions, unchanged })
    }
}

/// Submission summary for one put request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeImportSubmission {
    /// Submitted put request.
    pub action: WorktreeImportAction,
    /// Authoritative Core/API outcome.
    pub outcome: WorktreeImportOutcome,
    /// True when accepted Core state advanced local last-applied state.
    pub local_state_updated: bool,
}

/// Submission report for a complete put-only plan.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorktreeImportSubmissionReport {
    submissions: Vec<WorktreeImportSubmission>,
}

impl WorktreeImportSubmissionReport {
    /// Submitted request outcomes in request order.
    #[must_use]
    pub fn submissions(&self) -> &[WorktreeImportSubmission] {
        &self.submissions
    }

    /// True when no request was submitted.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.submissions.is_empty()
    }
}

/// Submits put-only plans through the abstract Core/API client.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorktreeImportRunner;

impl WorktreeImportRunner {
    /// Submit all put actions and update state only from accepted outcomes.
    pub fn submit_plan<C>(
        state: &mut WorktreeStateSnapshot,
        plan: WorktreeImportPlan,
        client: &mut C,
    ) -> Result<WorktreeImportSubmissionReport, C::Error>
    where
        C: WorktreeImportClient,
    {
        let mut submissions = Vec::with_capacity(plan.actions.len());
        for action in plan.actions {
            let outcome = client.submit(action.clone())?;
            let local_state_updated = update_state_from_outcome(state, &action, &outcome);
            submissions.push(WorktreeImportSubmission {
                action,
                outcome,
                local_state_updated,
            });
        }
        Ok(WorktreeImportSubmissionReport { submissions })
    }
}

fn stable_files_by_path(
    local_files: impl IntoIterator<Item = WorktreeImportFile>,
) -> Result<BTreeMap<VaultPath, WorktreeImportFile>, WorktreeImportPlanError> {
    let mut by_path = BTreeMap::new();
    for local_file in local_files {
        if !local_file.snapshot.stability.is_stable() {
            return Err(WorktreeImportPlanError::UnstableLocalFile {
                vault_path: local_file.snapshot.vault_path,
            });
        }
        let vault_path = local_file.snapshot.vault_path.clone();
        if by_path.insert(vault_path.clone(), local_file).is_some() {
            return Err(WorktreeImportPlanError::DuplicateLocalFile { vault_path });
        }
    }
    Ok(by_path)
}

fn put_action(
    local_file: &WorktreeImportFile,
    base_revision: WorktreeBaseRevision,
) -> WorktreeImportAction {
    WorktreeImportAction::Put(WorktreePutImport {
        vault_path: local_file.snapshot.vault_path.clone(),
        base_revision,
        content_hash: local_file.snapshot.content_hash,
        bytes: local_file.bytes.clone(),
    })
}

fn update_state_from_outcome(
    state: &mut WorktreeStateSnapshot,
    action: &WorktreeImportAction,
    outcome: &WorktreeImportOutcome,
) -> bool {
    let WorktreeImportAction::Put(request) = action else {
        return false;
    };
    let accepted = match outcome {
        WorktreeImportOutcome::Accepted(accepted)
        | WorktreeImportOutcome::SameContent(accepted) => accepted,
        _ => return false,
    };
    record_accepted(state, request, accepted);
    true
}

fn record_accepted(
    state: &mut WorktreeStateSnapshot,
    request: &WorktreePutImport,
    accepted: &WorktreeAcceptedImport,
) {
    state.record_present(
        request.vault_path.clone(),
        accepted.revision_id.clone(),
        accepted.content_hash,
    );
}
