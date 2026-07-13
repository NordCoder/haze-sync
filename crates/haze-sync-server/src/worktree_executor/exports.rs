use super::{ensure_not_cancelled, ServerWorktreeCycleExecutor, MAX_APPLICATION_CHANGE_LIMIT};
use crate::application::{AuthoritativeChange, AuthoritativeChangesQuery};
use haze_sync_storage::repositories::{
    adapter_cursors::AdapterCursorRepository,
    operation_log::OperationKindName,
    worktree_state::{
        upsert_present_state, upsert_tombstoned_state, WorktreePresentStateUpsert,
        WorktreeTombstonedStateUpsert,
    },
};
use haze_sync_worktree::{
    WorktreeCancellationToken, WorktreeMaterializationOutcome, WorktreeMaterializationRequest,
    WorktreeMaterializer, WorktreeRuntimeCycleFailure, WorktreeRuntimeCycleSummary,
    WorktreeStateSnapshot, WorktreeTombstoneMaterializationRequest, WorktreeTrashManager,
    WorktreeTrashPolicy,
};
use std::time::SystemTime;

impl ServerWorktreeCycleExecutor {
    pub(super) async fn apply_exports(
        &mut self,
        max_export_actions: usize,
        state: &mut WorktreeStateSnapshot,
        cancellation: &WorktreeCancellationToken,
        summary: &mut WorktreeRuntimeCycleSummary,
    ) -> Result<(), WorktreeRuntimeCycleFailure> {
        ensure_not_cancelled(cancellation)?;
        let cursor_repository = AdapterCursorRepository::new();
        let cursor = cursor_repository
            .initialize_if_missing(&self.pool, &self.adapter_id)
            .await
            .map_err(|_| WorktreeRuntimeCycleFailure::Export)?;
        ensure_not_cancelled(cancellation)?;

        let limit = max_export_actions.min(MAX_APPLICATION_CHANGE_LIMIT);
        let limit = u32::try_from(limit).map_err(|_| WorktreeRuntimeCycleFailure::Export)?;
        let query = AuthoritativeChangesQuery::new(cursor.last_core_seq, limit)
            .map_err(|_| WorktreeRuntimeCycleFailure::Export)?;
        let batch = self
            .services
            .authoritative_changes(query)
            .await
            .map_err(|_| WorktreeRuntimeCycleFailure::Export)?;
        if batch.from != cursor.last_core_seq {
            return Err(WorktreeRuntimeCycleFailure::Export);
        }

        let mut expected = cursor.last_core_seq;
        for change in batch.changes {
            ensure_not_cancelled(cancellation)?;
            let next = expected
                .checked_add(1)
                .ok_or(WorktreeRuntimeCycleFailure::Export)?;
            if change.seq != next {
                return Err(WorktreeRuntimeCycleFailure::Export);
            }
            let applied = self
                .apply_export_change(change, expected, state, cancellation)
                .await?;
            expected = next;
            if applied {
                summary.applied_exports += 1;
            }
        }
        Ok(())
    }

    async fn apply_export_change(
        &mut self,
        change: AuthoritativeChange,
        expected_cursor: i64,
        state: &mut WorktreeStateSnapshot,
        cancellation: &WorktreeCancellationToken,
    ) -> Result<bool, WorktreeRuntimeCycleFailure> {
        if change.kind == OperationKindName::DeleteFile {
            return self
                .apply_export_tombstone(change, expected_cursor, state, cancellation)
                .await;
        }

        if let Some(revision_id) = change.revision_id.clone() {
            let content_hash = change
                .content_hash
                .ok_or(WorktreeRuntimeCycleFailure::Export)?;
            let content = self
                .services
                .worktree_revision_content(revision_id.clone())
                .await
                .map_err(|_| WorktreeRuntimeCycleFailure::Export)?;
            if content.revision_id != revision_id || content.content_hash != content_hash {
                return Err(WorktreeRuntimeCycleFailure::Export);
            }
            let request = WorktreeMaterializationRequest {
                vault_path: content.path.clone(),
                revision_id,
                content_hash,
                bytes: content.into_bytes(),
            };
            let config = self.config.clone();
            let mut local_state = state.clone();
            let outcome = tokio::task::spawn_blocking(move || {
                WorktreeMaterializer::new(config).materialize(&mut local_state, request)
            })
            .await
            .map_err(|_| WorktreeRuntimeCycleFailure::Export)?
            .map_err(|_| WorktreeRuntimeCycleFailure::Export)?;

            let file = match outcome {
                WorktreeMaterializationOutcome::Applied { file, .. }
                | WorktreeMaterializationOutcome::AlreadyCurrent(file) => file,
                WorktreeMaterializationOutcome::NeedsImport(_) => {
                    return Err(WorktreeRuntimeCycleFailure::Export);
                }
            };

            #[cfg(test)]
            if self.failure_injection.fail_after_export_filesystem_once {
                self.failure_injection.fail_after_export_filesystem_once = false;
                return Err(WorktreeRuntimeCycleFailure::Export);
            }
            ensure_not_cancelled(cancellation)?;

            let mut transaction = self
                .pool
                .begin()
                .await
                .map_err(|_| WorktreeRuntimeCycleFailure::Export)?;
            upsert_present_state(
                &mut *transaction,
                &WorktreePresentStateUpsert {
                    adapter_id: &self.adapter_id,
                    path: &file.vault_path,
                    last_applied_revision_id: &file.revision_id,
                    content_hash: file.content_hash,
                    observation: None,
                },
            )
            .await
            .map_err(|_| WorktreeRuntimeCycleFailure::Export)?;
            AdapterCursorRepository::new()
                .advance_exact_contiguous(
                    &mut transaction,
                    &self.adapter_id,
                    expected_cursor,
                    change.seq,
                )
                .await
                .map_err(|_| WorktreeRuntimeCycleFailure::Export)?;
            transaction
                .commit()
                .await
                .map_err(|_| WorktreeRuntimeCycleFailure::Export)?;
            state.record_present(file.vault_path, file.revision_id, file.content_hash);
            return Ok(true);
        }

        // Metadata-only operations are acknowledged in sequence order but do not
        // fabricate a filesystem effect or durable path-state transition.
        self.advance_cursor_only(expected_cursor, change.seq).await?;
        Ok(false)
    }

    async fn apply_export_tombstone(
        &mut self,
        change: AuthoritativeChange,
        expected_cursor: i64,
        state: &mut WorktreeStateSnapshot,
        cancellation: &WorktreeCancellationToken,
    ) -> Result<bool, WorktreeRuntimeCycleFailure> {
        let revision_id = change
            .revision_id
            .clone()
            .ok_or(WorktreeRuntimeCycleFailure::Export)?;
        let request = WorktreeTombstoneMaterializationRequest {
            vault_path: change.path.clone(),
            tombstone_revision_id: revision_id.clone(),
            tombstoned_at: SystemTime::from(change.occurred_at),
        };
        let config = self.config.clone();
        let trash_policy = WorktreeTrashPolicy::new(self.policy.trash_retention)
            .map_err(|_| WorktreeRuntimeCycleFailure::Export)?;
        let mut local_state = state.clone();
        tokio::task::spawn_blocking(move || {
            WorktreeTrashManager::new(config, trash_policy)
                .materialize_tombstone(&mut local_state, request)
        })
        .await
        .map_err(|_| WorktreeRuntimeCycleFailure::Export)?
        .map_err(|_| WorktreeRuntimeCycleFailure::Export)?;

        #[cfg(test)]
        if self.failure_injection.fail_after_export_filesystem_once {
            self.failure_injection.fail_after_export_filesystem_once = false;
            return Err(WorktreeRuntimeCycleFailure::Export);
        }
        ensure_not_cancelled(cancellation)?;

        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| WorktreeRuntimeCycleFailure::Export)?;
        upsert_tombstoned_state(
            &mut *transaction,
            &WorktreeTombstonedStateUpsert {
                adapter_id: &self.adapter_id,
                path: &change.path,
                last_applied_revision_id: &revision_id,
            },
        )
        .await
        .map_err(|_| WorktreeRuntimeCycleFailure::Export)?;
        AdapterCursorRepository::new()
            .advance_exact_contiguous(
                &mut transaction,
                &self.adapter_id,
                expected_cursor,
                change.seq,
            )
            .await
            .map_err(|_| WorktreeRuntimeCycleFailure::Export)?;
        transaction
            .commit()
            .await
            .map_err(|_| WorktreeRuntimeCycleFailure::Export)?;
        state.record_tombstoned(change.path, revision_id);
        Ok(true)
    }

    async fn advance_cursor_only(
        &self,
        expected_cursor: i64,
        next_sequence: i64,
    ) -> Result<(), WorktreeRuntimeCycleFailure> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| WorktreeRuntimeCycleFailure::Export)?;
        AdapterCursorRepository::new()
            .advance_exact_contiguous(
                &mut transaction,
                &self.adapter_id,
                expected_cursor,
                next_sequence,
            )
            .await
            .map_err(|_| WorktreeRuntimeCycleFailure::Export)?;
        transaction
            .commit()
            .await
            .map_err(|_| WorktreeRuntimeCycleFailure::Export)
    }

    #[cfg(test)]
    pub(super) fn fail_after_export_filesystem_once(&mut self) {
        self.failure_injection.fail_after_export_filesystem_once = true;
    }
}
