use super::{
    deterministic_identifier,
    idempotency::{read_delete_replay, store_delete_outcome},
    ApplicationActor, ApplicationError, ApplicationIdempotency,
};
use chrono::{DateTime, Duration, Utc};
use haze_sync_common::{OperationId, RevisionId, VaultPath};
use haze_sync_core::{
    delete_guard::{DeleteGuard, DeleteGuardDecision, DeleteGuardInput, DeleteRunScope},
    tombstone_service::{
        TombstoneCreationInput, TombstoneId, TombstoneRetention, TombstoneService,
    },
};
use haze_sync_storage::{
    locks::lock_vault_path,
    models::{OperationLogRow, SyncObjectRow, TombstoneRow},
    repositories::{
        objects::{
            get_sync_object_by_path, set_current_revision_by_object_id,
            set_sync_object_deleted_at_by_object_id,
        },
        operation_log::{AppendOperationLogEntry, OperationKindName, OperationLogRepository},
        tombstones::{NewTombstone, TombstoneRepository},
    },
};
use sqlx::{PgPool, Postgres, Transaction};
use std::fmt;

const DELETE_RETENTION_DAYS: i64 = 30;
const SINGLE_DELETE_RATIO_FLOOR: u64 = 100;

/// Normalized internal guarded-delete command.
#[derive(Clone, Eq, PartialEq)]
pub(crate) struct ApplyDeleteCommand {
    pub(crate) actor: ApplicationActor,
    pub(crate) path: VaultPath,
    pub(crate) base_revision_id: Option<RevisionId>,
    pub(crate) requested_delete_count: u32,
    pub(crate) idempotency: ApplicationIdempotency,
}

impl fmt::Debug for ApplyDeleteCommand {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ApplyDeleteCommand")
            .field("actor", &self.actor)
            .field("path", &self.path)
            .field("base_revision_id", &self.base_revision_id)
            .field("requested_delete_count", &self.requested_delete_count)
            .field("idempotency", &self.idempotency)
            .finish()
    }
}

/// Typed internal result of one guarded-delete command.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ApplyDeleteOutcome {
    Tombstoned {
        path: VaultPath,
        tombstone_id: String,
        seq: i64,
        retention_until: DateTime<Utc>,
    },
    NotFound {
        path: VaultPath,
    },
    StaleBase {
        path: VaultPath,
    },
    GuardRejected {
        path: VaultPath,
    },
}

pub(super) async fn apply_delete(
    pool: &PgPool,
    command: ApplyDeleteCommand,
) -> Result<ApplyDeleteOutcome, ApplicationError> {
    if let Some(replay) = read_delete_replay(pool, &command.actor, &command.idempotency).await? {
        return Ok(replay);
    }

    let mut transaction = pool.begin().await.map_err(|_| ApplicationError::Internal)?;
    lock_vault_path(&mut *transaction, &command.path)
        .await
        .map_err(|_| ApplicationError::Internal)?;
    let outcome = apply_delete_in_transaction(&mut transaction, &command).await?;

    if let Some(replay) = store_delete_outcome(
        &mut transaction,
        &command.actor,
        &command.idempotency,
        &outcome,
    )
    .await?
    {
        transaction
            .rollback()
            .await
            .map_err(|_| ApplicationError::Internal)?;
        return Ok(replay);
    }

    transaction
        .commit()
        .await
        .map_err(|_| ApplicationError::Internal)?;
    Ok(outcome)
}

async fn apply_delete_in_transaction(
    transaction: &mut Transaction<'_, Postgres>,
    command: &ApplyDeleteCommand,
) -> Result<ApplyDeleteOutcome, ApplicationError> {
    let Some(object) = get_sync_object_by_path(&mut **transaction, &command.path)
        .await
        .map_err(|_| ApplicationError::Internal)?
    else {
        return Ok(ApplyDeleteOutcome::NotFound {
            path: command.path.clone(),
        });
    };
    let Some(current_revision_id) = active_current_revision_id(&object)? else {
        return Ok(ApplyDeleteOutcome::NotFound {
            path: command.path.clone(),
        });
    };
    if command.base_revision_id.as_ref() != Some(&current_revision_id) {
        return Ok(ApplyDeleteOutcome::StaleBase {
            path: command.path.clone(),
        });
    }

    let active_file_count = active_file_count(transaction).await?;
    let total_files_before_run = active_file_count
        .max(SINGLE_DELETE_RATIO_FLOOR)
        .max(u64::from(command.requested_delete_count));
    if !delete_guard_allows(command, total_files_before_run)? {
        return Ok(ApplyDeleteOutcome::GuardRejected {
            path: command.path.clone(),
        });
    }

    let tombstone = build_tombstone(command, &current_revision_id)?;
    let tombstone_row = TombstoneRepository::new()
        .insert(
            &mut **transaction,
            NewTombstone {
                tombstone_id: tombstone.tombstone_id.as_str(),
                path: &tombstone.path,
                deleted_revision_id: Some(tombstone.deleted_revision_id()),
                deleted_by: &tombstone.deleted_by,
                retention_until: tombstone.retention.retention_until,
            },
        )
        .await
        .map_err(|_| ApplicationError::Internal)?;
    clear_current_revision_and_mark_deleted(transaction, command, &object, &tombstone_row).await?;
    let operation = append_delete_operation(
        transaction,
        command,
        &current_revision_id,
        &tombstone_row.tombstone_id,
    )
    .await?;
    Ok(ApplyDeleteOutcome::Tombstoned {
        path: command.path.clone(),
        tombstone_id: tombstone_row.tombstone_id,
        seq: operation.seq,
        retention_until: tombstone_row.retention_until,
    })
}

fn active_current_revision_id(
    object: &SyncObjectRow,
) -> Result<Option<RevisionId>, ApplicationError> {
    if object.deleted_at.is_some() {
        return Ok(None);
    }
    object
        .current_revision_id
        .as_deref()
        .map(RevisionId::parse)
        .transpose()
        .map_err(|_| ApplicationError::Internal)
}

async fn active_file_count(
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<u64, ApplicationError> {
    let count: i64 = sqlx::query_scalar(
        "select count(*) from sync_objects where kind = 'file' and deleted_at is null and current_revision_id is not null",
    )
    .fetch_one(&mut **transaction)
    .await
    .map_err(|_| ApplicationError::Internal)?;
    u64::try_from(count).map_err(|_| ApplicationError::Internal)
}

fn delete_guard_allows(
    command: &ApplyDeleteCommand,
    total_files_before_run: u64,
) -> Result<bool, ApplicationError> {
    let scope = DeleteRunScope::new(
        command.actor.adapter_id().clone(),
        deterministic_identifier(
            "delete_run_",
            &[command.actor.adapter_id().as_str(), command.path.as_str()],
        ),
    )
    .map_err(|_| ApplicationError::Internal)?;
    let input = DeleteGuardInput::without_manual_unlock(
        scope,
        u64::from(command.requested_delete_count),
        total_files_before_run,
    );
    Ok(matches!(
        DeleteGuard::default().evaluate(&input),
        DeleteGuardDecision::Allowed
    ))
}

fn build_tombstone(
    command: &ApplyDeleteCommand,
    current_revision_id: &RevisionId,
) -> Result<haze_sync_core::tombstone_service::Tombstone, ApplicationError> {
    let created_at = Utc::now();
    let retention_until = created_at + Duration::days(DELETE_RETENTION_DAYS);
    let tombstone_id = TombstoneId::parse(&deterministic_identifier(
        "tmb_",
        &[
            command.path.as_str(),
            current_revision_id.as_str(),
            command.actor.adapter_id().as_str(),
        ],
    ))
    .map_err(|_| ApplicationError::Internal)?;
    TombstoneService::new()
        .create_tombstone(TombstoneCreationInput::new(
            tombstone_id,
            command.path.clone(),
            current_revision_id.clone(),
            Some(current_revision_id.clone()),
            command.actor.adapter_id().clone(),
            TombstoneRetention::new(retention_until, Some(DELETE_RETENTION_DAYS as u16)),
            Some(created_at),
        ))
        .map_err(|_| ApplicationError::Internal)
}

async fn clear_current_revision_and_mark_deleted(
    transaction: &mut Transaction<'_, Postgres>,
    command: &ApplyDeleteCommand,
    object: &SyncObjectRow,
    tombstone_row: &TombstoneRow,
) -> Result<(), ApplicationError> {
    set_current_revision_by_object_id(
        &mut **transaction,
        object.object_id.as_str(),
        None,
        command.actor.adapter_id(),
    )
    .await
    .map_err(|_| ApplicationError::Internal)?
    .ok_or(ApplicationError::Internal)?;
    set_sync_object_deleted_at_by_object_id(
        &mut **transaction,
        object.object_id.as_str(),
        Some(tombstone_row.deleted_at),
        command.actor.adapter_id(),
    )
    .await
    .map_err(|_| ApplicationError::Internal)?
    .ok_or(ApplicationError::Internal)?;
    Ok(())
}

async fn append_delete_operation(
    transaction: &mut Transaction<'_, Postgres>,
    command: &ApplyDeleteCommand,
    revision_id: &RevisionId,
    tombstone_id: &str,
) -> Result<OperationLogRow, ApplicationError> {
    let op_id = OperationId::parse(&deterministic_identifier(
        "op_",
        &[
            revision_id.as_str(),
            OperationKindName::DeleteFile.as_str(),
            command.path.as_str(),
            tombstone_id,
        ],
    ))
    .map_err(|_| ApplicationError::Internal)?;
    OperationLogRepository::new()
        .append(
            &mut **transaction,
            &AppendOperationLogEntry {
                op_id,
                adapter_id: command.actor.adapter_id().clone(),
                kind: OperationKindName::DeleteFile,
                path: command.path.clone(),
                revision_id: Some(revision_id.clone()),
                tombstone_id: Some(tombstone_id.to_owned()),
                conflict_id: None,
            },
        )
        .await
        .map_err(|_| ApplicationError::Internal)
}

#[cfg(test)]
mod tests {
    use super::*;
    use haze_sync_common::AdapterId;

    #[test]
    fn delete_command_debug_redacts_idempotency_key() {
        let actor = ApplicationActor::new(AdapterId::parse("worktree").unwrap());
        let path = VaultPath::parse("Notes/a.md").unwrap();
        let fingerprint = super::super::delete_request_fingerprint(&actor, &path, None, 1);
        let command = ApplyDeleteCommand {
            actor,
            path,
            base_revision_id: None,
            requested_delete_count: 1,
            idempotency: ApplicationIdempotency::new("secret-delete-key", fingerprint),
        };
        let rendered = format!("{command:?}");
        assert!(!rendered.contains("secret-delete-key"));
    }
}
