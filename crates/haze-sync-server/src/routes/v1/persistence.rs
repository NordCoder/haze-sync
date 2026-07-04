use chrono::Utc;
use haze_sync_api::{
    auth::AdapterPrincipal,
    dto::files::PutFileResponse,
    routes::files::{
        accepted_upload_response, conflict_saved_upload_response, ignored_same_content_response,
        FileRouteError,
    },
};
use haze_sync_common::{ContentHash, OperationId, RevisionId};
use haze_sync_core::{
    conflict_saved_planner::{require_upsert_conflict_saved_plan, ConflictSavedPreservationPlan},
    revision_service::{StoredRevision, UpsertOutcome},
};
use haze_sync_storage::{
    object_store::ObjectStore,
    repositories::{
        objects::{
            create_or_find_sync_object_by_path, set_current_revision_by_object_id, NewSyncObject,
            SyncObjectKind,
        },
        operation_log::{AppendOperationLogEntry, OperationKindName, OperationLogRepository},
        revisions::{insert_file_revision, NewFileRevision},
    },
    LocalObjectStore,
};

use super::{
    conflict_created_operation_id, conflict_id_from_plan, deterministic_identifier,
    incoming_conflict_revision_id, map_conflict_saved_planning_error, map_repository_error,
    policy_dto, ApiError,
};

pub(super) async fn apply_upsert_outcome(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    principal: &AdapterPrincipal,
    object_store: &LocalObjectStore,
    outcome: UpsertOutcome,
) -> Result<PutFileResponse, ApiError> {
    match outcome {
        UpsertOutcome::AcceptedNewFile { revision, .. }
        | UpsertOutcome::AcceptedNewRevision { revision, .. } => {
            let seq = persist_accepted_revision(transaction, principal, &revision).await?;
            Ok(accepted_upload_response(
                revision.path,
                revision.revision_id,
                seq,
            ))
        }
        UpsertOutcome::IgnoredDuplicateSameContent { current_revision } => {
            Ok(ignored_same_content_response(current_revision.path))
        }
        UpsertOutcome::RejectedHashMismatch { .. } => {
            Err(FileRouteError::InvalidContentSha256.into())
        }
        rejected @ UpsertOutcome::RejectedStaleOrUnknownBase {
            conflict_saved: Some(_),
            ..
        } => persist_conflict_saved_outcome(transaction, principal, object_store, &rejected).await,
        UpsertOutcome::RejectedStaleOrUnknownBase { .. } => Err(FileRouteError::Conflict.into()),
    }
}

pub(super) async fn persist_accepted_revision(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    principal: &AdapterPrincipal,
    revision: &StoredRevision,
) -> Result<i64, ApiError> {
    insert_content_blob(transaction, revision.content_hash, revision.size_bytes).await?;

    let object_id = deterministic_identifier("obj_", &[revision.path.as_str()]);
    let object = create_or_find_sync_object_by_path(
        &mut **transaction,
        NewSyncObject {
            object_id: object_id.as_str(),
            path: &revision.path,
            kind: SyncObjectKind::File,
            updated_by: principal.common_adapter_id(),
        },
    )
    .await
    .map_err(map_repository_error)?;

    let _revision_row = insert_file_revision(
        &mut **transaction,
        NewFileRevision {
            revision_id: &revision.revision_id,
            object_id: object.object_id.as_str(),
            path: &revision.path,
            parent_revision_id: revision.parent_revision_id.as_ref(),
            content_hash: revision.content_hash,
            size_bytes: revision.size_bytes,
            created_by: principal.common_adapter_id(),
        },
    )
    .await
    .map_err(map_repository_error)?;

    let _updated_object = set_current_revision_by_object_id(
        &mut **transaction,
        object.object_id.as_str(),
        Some(&revision.revision_id),
        principal.common_adapter_id(),
    )
    .await
    .map_err(map_repository_error)?
    .ok_or_else(ApiError::internal)?;

    let content_hash_string = revision.content_hash.to_string();
    let op_id = OperationId::parse(&deterministic_identifier(
        "op_",
        &[
            revision.revision_id.as_str(),
            OperationKindName::UpsertFile.as_str(),
            revision.path.as_str(),
            content_hash_string.as_str(),
        ],
    ))
    .map_err(|_error| ApiError::internal())?;

    let operation = OperationLogRepository::new()
        .append(
            &mut **transaction,
            &AppendOperationLogEntry {
                op_id,
                adapter_id: principal.common_adapter_id().clone(),
                kind: OperationKindName::UpsertFile,
                path: revision.path.clone(),
                revision_id: Some(revision.revision_id.clone()),
                tombstone_id: None,
                conflict_id: None,
            },
        )
        .await
        .map_err(map_repository_error)?;

    Ok(operation.seq)
}

async fn persist_conflict_saved_outcome(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    principal: &AdapterPrincipal,
    object_store: &LocalObjectStore,
    outcome: &UpsertOutcome,
) -> Result<PutFileResponse, ApiError> {
    let plan = require_upsert_conflict_saved_plan(outcome, Utc::now())
        .map_err(map_conflict_saved_planning_error)?;
    let conflict_saved = outcome.conflict_saved().ok_or(FileRouteError::Conflict)?;

    object_store
        .put_bytes(
            plan.incoming_content_hash,
            conflict_saved.incoming_content.content.as_slice(),
        )
        .map_err(|_error| ApiError::internal())?;
    insert_content_blob(
        transaction,
        plan.incoming_content_hash,
        plan.incoming_size_bytes,
    )
    .await?;

    let incoming_revision_id = incoming_conflict_revision_id(&plan)?;
    let conflict_object_id = deterministic_identifier("obj_", &[plan.materialized_path.as_str()]);
    let conflict_object = create_or_find_sync_object_by_path(
        &mut **transaction,
        NewSyncObject {
            object_id: conflict_object_id.as_str(),
            path: &plan.materialized_path,
            kind: SyncObjectKind::File,
            updated_by: principal.common_adapter_id(),
        },
    )
    .await
    .map_err(map_repository_error)?;

    let _incoming_revision = insert_file_revision(
        &mut **transaction,
        NewFileRevision {
            revision_id: &incoming_revision_id,
            object_id: conflict_object.object_id.as_str(),
            path: &plan.materialized_path,
            parent_revision_id: Some(&plan.current_revision_id),
            content_hash: plan.incoming_content_hash,
            size_bytes: plan.incoming_size_bytes,
            created_by: principal.common_adapter_id(),
        },
    )
    .await
    .map_err(map_repository_error)?;

    let _updated_conflict_object = set_current_revision_by_object_id(
        &mut **transaction,
        conflict_object.object_id.as_str(),
        Some(&incoming_revision_id),
        principal.common_adapter_id(),
    )
    .await
    .map_err(map_repository_error)?
    .ok_or_else(ApiError::internal)?;

    let conflict_id = conflict_id_from_plan(&plan)?;
    insert_conflict_row(transaction, &conflict_id, &incoming_revision_id, &plan).await?;

    let operation = OperationLogRepository::new()
        .append(
            &mut **transaction,
            &AppendOperationLogEntry {
                op_id: conflict_created_operation_id(&conflict_id, &incoming_revision_id, &plan)?,
                adapter_id: principal.common_adapter_id().clone(),
                kind: OperationKindName::ConflictCreated,
                path: plan.original_path.clone(),
                revision_id: Some(incoming_revision_id.clone()),
                tombstone_id: None,
                conflict_id: Some(conflict_id.clone()),
            },
        )
        .await
        .map_err(map_repository_error)?;

    Ok(conflict_saved_upload_response(
        plan.original_path,
        conflict_id,
        plan.materialized_path,
        policy_dto(plan.policy_applied),
        operation.seq,
    ))
}

async fn insert_conflict_row(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    conflict_id: &haze_sync_common::ConflictId,
    incoming_revision_id: &RevisionId,
    plan: &ConflictSavedPreservationPlan,
) -> Result<(), ApiError> {
    sqlx::query(
        "insert into conflicts \
         (conflict_id, original_path, base_revision_id, current_revision_id, incoming_revision_id, \
          incoming_adapter_id, policy_applied, materialized_path, status) \
         values ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(conflict_id.as_str())
    .bind(plan.original_path.as_str())
    .bind(
        plan.provided_base_revision_id
            .as_ref()
            .map(RevisionId::as_str),
    )
    .bind(plan.current_revision_id.as_str())
    .bind(incoming_revision_id.as_str())
    .bind(plan.incoming_adapter_id.as_str())
    .bind(plan.policy_applied.as_str())
    .bind(plan.materialized_path.as_str())
    .bind("open")
    .execute(&mut **transaction)
    .await
    .map_err(|_error| ApiError::internal())?;

    Ok(())
}

async fn insert_content_blob(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    content_hash: ContentHash,
    size_bytes: u64,
) -> Result<(), ApiError> {
    let size_bytes = i64::try_from(size_bytes).map_err(|_| ApiError::internal())?;
    let object_store_path = LocalObjectStore::relative_blob_path(content_hash)
        .to_string_lossy()
        .replace('\\', "/");

    sqlx::query(
        "insert into content_blobs (sha256, size_bytes, object_store_path) \
         values ($1, $2, $3) \
         on conflict (sha256) do nothing",
    )
    .bind(content_hash.to_prefixed_string())
    .bind(size_bytes)
    .bind(object_store_path)
    .execute(&mut **transaction)
    .await
    .map_err(|_error| ApiError::internal())?;

    Ok(())
}
