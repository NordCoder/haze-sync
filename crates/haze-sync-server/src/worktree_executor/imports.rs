use super::scan::LocalObservation;
use super::{ensure_not_cancelled, ServerWorktreeCycleExecutor};
use crate::application::{
    derive_worktree_delete_idempotency, derive_worktree_put_idempotency, ApplicationActor,
    ApplyDeleteCommand, ApplyDeleteOutcome, ApplyFileCommand, ApplyFileOutcome,
    RevisionContentQuery,
};
use chrono::{DateTime, Utc};
use haze_sync_common::{RevisionId, VaultPath};
use haze_sync_storage::repositories::worktree_state::{
    upsert_present_state, upsert_tombstoned_state, WorktreePresentStateUpsert,
    WorktreeReconciliationObservation, WorktreeTombstonedStateUpsert,
};
use haze_sync_worktree::{
    WorktreeBaseRevision, WorktreeCancellationToken, WorktreeDeleteAuthorization,
    WorktreeDeleteCandidate, WorktreeDeleteGuardDecision, WorktreeDeleteGuardPolicy,
    WorktreeDeleteScan, WorktreeGuardedDeletePlan, WorktreeImportAction,
    WorktreeRuntimeCycleFailure, WorktreeRuntimeCycleSummary, WorktreeStateSnapshot,
};
use std::collections::BTreeMap;

impl ServerWorktreeCycleExecutor {
    pub(super) fn authorize_delete_candidates(
        &self,
        scan: WorktreeDeleteScan,
        max_delete_candidates: usize,
    ) -> Result<Vec<WorktreeDeleteCandidate>, WorktreeRuntimeCycleFailure> {
        if scan.candidates().len() > max_delete_candidates {
            return Err(WorktreeRuntimeCycleFailure::Plan);
        }
        let policy = WorktreeDeleteGuardPolicy::new(
            max_delete_candidates,
            self.policy.max_delete_ratio_basis_points,
        )
        .map_err(|_| WorktreeRuntimeCycleFailure::Plan)?;
        let plan =
            WorktreeGuardedDeletePlan::evaluate(scan, policy, WorktreeDeleteAuthorization::Guarded);
        match plan.decision() {
            WorktreeDeleteGuardDecision::Allowed(_) => Ok(plan.candidates().to_vec()),
            WorktreeDeleteGuardDecision::Blocked { .. } => Err(WorktreeRuntimeCycleFailure::Plan),
        }
    }

    pub(super) async fn submit_imports(
        &mut self,
        actions: &[WorktreeImportAction],
        observations: &BTreeMap<VaultPath, LocalObservation>,
        state: &mut WorktreeStateSnapshot,
        cancellation: &WorktreeCancellationToken,
        summary: &mut WorktreeRuntimeCycleSummary,
    ) -> Result<(), WorktreeRuntimeCycleFailure> {
        let actor = ApplicationActor::new(self.adapter_id.clone());

        for action in actions {
            ensure_not_cancelled(cancellation)?;
            let WorktreeImportAction::Put(put) = action else {
                return Err(WorktreeRuntimeCycleFailure::Plan);
            };
            let base_revision_id = base_revision_option(&put.base_revision);
            let command = ApplyFileCommand {
                actor: actor.clone(),
                path: put.vault_path.clone(),
                base_revision_id: base_revision_id.clone(),
                content_hash: put.content_hash,
                bytes: put.bytes.clone(),
                idempotency: derive_worktree_put_idempotency(
                    &self.adapter_id,
                    &put.vault_path,
                    base_revision_id.as_ref(),
                    put.content_hash,
                    &put.bytes,
                ),
            };
            let outcome = self
                .services
                .apply_file(command)
                .await
                .map_err(|_| WorktreeRuntimeCycleFailure::Submit)?;
            summary.submitted_imports += 1;

            #[cfg(test)]
            if self.failure_injection.fail_after_import_submission_once {
                self.failure_injection.fail_after_import_submission_once = false;
                return Err(WorktreeRuntimeCycleFailure::Submit);
            }

            ensure_not_cancelled(cancellation)?;
            let accepted = match outcome {
                ApplyFileOutcome::Accepted {
                    revision_id,
                    content_hash,
                    ..
                } => Some((revision_id, content_hash.unwrap_or(put.content_hash))),
                ApplyFileOutcome::SameContent {
                    revision_id: Some(revision_id),
                    content_hash: Some(content_hash),
                    ..
                } => Some((revision_id, content_hash)),
                ApplyFileOutcome::SameContent { .. } => {
                    let content = self
                        .services
                        .revision_content(RevisionContentQuery {
                            path: put.vault_path.clone(),
                            revision_id: None,
                        })
                        .await
                        .map_err(|_| WorktreeRuntimeCycleFailure::Submit)?;
                    Some((content.revision_id, content.content_hash))
                }
                ApplyFileOutcome::ConflictSaved { .. } => None,
            };

            if let Some((revision_id, content_hash)) = accepted {
                let local = observations.get(&put.vault_path);
                let fallback_size = u64::try_from(put.bytes.len())
                    .map_err(|_| WorktreeRuntimeCycleFailure::Submit)?;
                let observation = WorktreeReconciliationObservation::new(
                    local.map_or(fallback_size, |value| value.size),
                    local
                        .and_then(|value| value.modified)
                        .map(DateTime::<Utc>::from),
                );
                upsert_present_state(
                    &self.pool,
                    &WorktreePresentStateUpsert {
                        adapter_id: &self.adapter_id,
                        path: &put.vault_path,
                        last_applied_revision_id: &revision_id,
                        content_hash,
                        observation: Some(observation),
                    },
                )
                .await
                .map_err(|_| WorktreeRuntimeCycleFailure::Submit)?;
                state.record_present(put.vault_path.clone(), revision_id, content_hash);
            }

            #[cfg(test)]
            if self.failure_injection.cancel_after_import_commit_once {
                self.failure_injection.cancel_after_import_commit_once = false;
                cancellation.cancel();
            }
            ensure_not_cancelled(cancellation)?;
        }
        Ok(())
    }

    pub(super) async fn submit_deletes(
        &mut self,
        candidates: &[WorktreeDeleteCandidate],
        state: &mut WorktreeStateSnapshot,
        cancellation: &WorktreeCancellationToken,
    ) -> Result<(), WorktreeRuntimeCycleFailure> {
        let actor = ApplicationActor::new(self.adapter_id.clone());
        let requested_delete_count =
            u32::try_from(candidates.len()).map_err(|_| WorktreeRuntimeCycleFailure::Plan)?;
        let mut accepted = Vec::new();

        for candidate in candidates {
            ensure_not_cancelled(cancellation)?;
            let base_revision_id = base_revision_option(&candidate.base_revision);
            let command = ApplyDeleteCommand {
                actor: actor.clone(),
                path: candidate.vault_path.clone(),
                base_revision_id: base_revision_id.clone(),
                requested_delete_count,
                idempotency: derive_worktree_delete_idempotency(
                    &self.adapter_id,
                    &candidate.vault_path,
                    base_revision_id.as_ref(),
                    requested_delete_count,
                ),
            };
            let outcome = self
                .services
                .apply_delete(command)
                .await
                .map_err(|_| WorktreeRuntimeCycleFailure::Submit)?;
            ensure_not_cancelled(cancellation)?;

            if matches!(outcome, ApplyDeleteOutcome::Tombstoned { .. }) {
                let revision_id = base_revision_id.ok_or(WorktreeRuntimeCycleFailure::Submit)?;
                accepted.push((candidate.vault_path.clone(), revision_id));
            }
        }

        if accepted.is_empty() {
            return Ok(());
        }
        ensure_not_cancelled(cancellation)?;
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| WorktreeRuntimeCycleFailure::Submit)?;
        for (path, revision_id) in &accepted {
            upsert_tombstoned_state(
                &mut *transaction,
                &WorktreeTombstonedStateUpsert {
                    adapter_id: &self.adapter_id,
                    path,
                    last_applied_revision_id: revision_id,
                },
            )
            .await
            .map_err(|_| WorktreeRuntimeCycleFailure::Submit)?;
        }
        transaction
            .commit()
            .await
            .map_err(|_| WorktreeRuntimeCycleFailure::Submit)?;
        for (path, revision_id) in accepted {
            state.record_tombstoned(path, revision_id);
        }
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn fail_after_import_submission_once(&mut self) {
        self.failure_injection.fail_after_import_submission_once = true;
    }

    #[cfg(test)]
    pub(super) fn cancel_after_import_commit_once(&mut self) {
        self.failure_injection.cancel_after_import_commit_once = true;
    }
}

fn base_revision_option(base: &WorktreeBaseRevision) -> Option<RevisionId> {
    match base {
        WorktreeBaseRevision::Known(revision_id) => Some(revision_id.clone()),
        WorktreeBaseRevision::Null => None,
    }
}
