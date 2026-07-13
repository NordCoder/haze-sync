use super::{ensure_not_cancelled, ServerWorktreeCycleExecutor};
use chrono::{DateTime, Utc};
use haze_sync_common::{ContentHash, RevisionId, VaultPath};
use haze_sync_storage::{
    models::WorktreeStateRow,
    repositories::worktree_state::{
        load_snapshot, load_worktree_instance, update_reconciliation_observation,
        WorktreeObservationUpdate, WorktreeReconciliationObservation,
        WORKTREE_STATE_FORMAT_VERSION,
    },
};
use haze_sync_worktree::{
    WorktreeCancellationToken, WorktreeObservedFileState, WorktreeReconciliationState,
    WorktreeReconciliationStateTransition, WorktreeRuntimeCycleFailure, WorktreeStateSnapshot,
};
use std::time::SystemTime;

pub(super) struct LoadedState {
    pub(super) applied: WorktreeStateSnapshot,
    pub(super) reconciliation: WorktreeReconciliationState,
}

impl ServerWorktreeCycleExecutor {
    pub(super) async fn verify_required_binding(
        &self,
    ) -> Result<(), WorktreeRuntimeCycleFailure> {
        let binding = load_worktree_instance(&self.pool, &self.adapter_id)
            .await
            .map_err(|_| WorktreeRuntimeCycleFailure::Plan)?
            .ok_or(WorktreeRuntimeCycleFailure::Plan)?;
        if binding.state_format_version != WORKTREE_STATE_FORMAT_VERSION
            || binding.root_fingerprint != self.root_fingerprint.to_string()
        {
            return Err(WorktreeRuntimeCycleFailure::Plan);
        }
        Ok(())
    }

    pub(super) async fn load_durable_state(
        &self,
        cancellation: &WorktreeCancellationToken,
    ) -> Result<LoadedState, WorktreeRuntimeCycleFailure> {
        let mut rows = Vec::new();
        let mut after_path = None;

        loop {
            ensure_not_cancelled(cancellation)?;
            let page = load_snapshot(
                &self.pool,
                &self.adapter_id,
                after_path.as_ref(),
                self.policy.state_page_size,
            )
            .await
            .map_err(|_| WorktreeRuntimeCycleFailure::Plan)?;
            ensure_not_cancelled(cancellation)?;
            if rows.len().saturating_add(page.states.len()) > self.policy.max_state_paths {
                return Err(WorktreeRuntimeCycleFailure::Plan);
            }
            rows.extend(page.states);
            match page.next_after_path {
                Some(next) => {
                    if after_path.as_ref() == Some(&next) {
                        return Err(WorktreeRuntimeCycleFailure::Plan);
                    }
                    after_path = Some(next);
                }
                None => break,
            }
        }

        decode_loaded_state(rows)
    }

    pub(super) async fn persist_observation_transitions(
        &self,
        transitions: &[WorktreeReconciliationStateTransition],
        cancellation: &WorktreeCancellationToken,
    ) -> Result<(), WorktreeRuntimeCycleFailure> {
        for transition in transitions {
            ensure_not_cancelled(cancellation)?;
            let observation = WorktreeReconciliationObservation::new(
                transition.current.size,
                transition.current.modified.map(DateTime::<Utc>::from),
            );
            let input = WorktreeObservationUpdate {
                adapter_id: &self.adapter_id,
                path: &transition.vault_path,
                expected_revision_id: &transition.current.revision_id,
                expected_content_hash: transition.current.content_hash,
                observation,
            };
            // A concurrent authoritative state change may make the guarded update a no-op.
            // That is safe and will be reconciled by a later cycle.
            let _ = update_reconciliation_observation(&self.pool, &input)
                .await
                .map_err(|_| WorktreeRuntimeCycleFailure::Plan)?;
        }
        Ok(())
    }
}

fn decode_loaded_state(
    rows: Vec<WorktreeStateRow>,
) -> Result<LoadedState, WorktreeRuntimeCycleFailure> {
    let mut applied = WorktreeStateSnapshot::new();
    let mut observations = Vec::new();

    for row in rows {
        let path = VaultPath::parse(&row.path).map_err(|_| WorktreeRuntimeCycleFailure::Plan)?;
        let revision_id = RevisionId::parse(&row.last_applied_revision_id)
            .map_err(|_| WorktreeRuntimeCycleFailure::Plan)?;
        match row.state_kind.as_str() {
            "present" => {
                let content_hash = row
                    .content_sha256
                    .as_deref()
                    .ok_or(WorktreeRuntimeCycleFailure::Plan)
                    .and_then(|value| {
                        ContentHash::parse(value).map_err(|_| WorktreeRuntimeCycleFailure::Plan)
                    })?;
                applied.record_present(path.clone(), revision_id.clone(), content_hash);
                if let (Some(_), Some(size)) =
                    (row.observation_schema_version, row.observed_size_bytes)
                {
                    let size =
                        u64::try_from(size).map_err(|_| WorktreeRuntimeCycleFailure::Plan)?;
                    observations.push((
                        path,
                        WorktreeObservedFileState {
                            revision_id,
                            content_hash,
                            size,
                            modified: row.observed_mtime.map(SystemTime::from),
                        },
                    ));
                }
            }
            "tombstoned" => applied.record_tombstoned(path, revision_id),
            _ => return Err(WorktreeRuntimeCycleFailure::Plan),
        }
    }

    let mut reconciliation = WorktreeReconciliationState::new(applied.clone());
    for (path, observation) in observations {
        reconciliation.record_observation(path, observation);
    }
    Ok(LoadedState {
        applied,
        reconciliation,
    })
}
