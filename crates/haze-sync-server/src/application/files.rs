use super::{
    deterministic_identifier,
    idempotency::{read_file_replay, store_file_outcome},
    ApplicationActor, ApplicationError, ApplicationIdempotency,
};
use chrono::Utc;
use haze_sync_common::{
    AdapterId, ConflictId, ContentHash, OperationId, RevisionId, VaultPath,
};
use haze_sync_core::{
    conflict_saved_planner::{require_upsert_conflict_saved_plan, ConflictSavedPreservationPlan},
    conflict_service::ConflictPolicy,
    revision_service::{
        AppendOperationRequest, ContentStore, InsertRevisionRequest, OperationKind, OperationLog,
        OperationLogEntry, RevisionRepository, RevisionService, RevisionServiceError, StoredContent,
        StoredRevision, UpsertFileRequest, UpsertOutcome,
    },
};
use haze_sync_storage::{
    locks::lock_vault_path,
    models::FileRevisionRow,
    object_store::ObjectStore,
    repositories::{
        objects::{
            create_or_find_sync_object_by_path, set_current_revision_by_object_id, NewSyncObject,
            SyncObjectKind,
        },
        operation_log::{AppendOperationLogEntry, OperationKindName, OperationLogRepository},
        revisions::{
            get_current_revision_by_path, get_file_revision_by_id, insert_file_revision,
            NewFileRevision,
        },
    },
    LocalObjectStore, ObjectStoreError,
};
use sqlx::{PgPool, Postgres, Transaction};
use std::fmt;

/// Normalized internal file create/update command.
#[derive(Clone, Eq, PartialEq)]
pub(crate) struct ApplyFileCommand {
    pub(crate) actor: ApplicationActor,
    pub(crate) path: VaultPath,
    pub(crate) base_revision_id: Option<RevisionId>,
    pub(crate) content_hash: ContentHash,
    pub(crate) bytes: Vec<u8>,
    pub(crate) idempotency: ApplicationIdempotency,
}

impl fmt::Debug for ApplyFileCommand {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ApplyFileCommand")
            .field("actor", &self.actor)
            .field("path", &self.path)
            .field("base_revision_id", &self.base_revision_id)
            .field("content_hash", &self.content_hash)
            .field("bytes", &"[REDACTED]")
            .field("idempotency", &self.idempotency)
            .finish()
    }
}

/// Typed internal result of one authoritative file command.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ApplyFileOutcome {
    Accepted {
        path: VaultPath,
        revision_id: RevisionId,
        /// Present for all service-returned accepted outcomes, including replays.
        content_hash: Option<ContentHash>,
        seq: i64,
    },
    SameContent {
        path: VaultPath,
        /// Present for all service-returned same-content outcomes, including replays.
        revision_id: Option<RevisionId>,
        /// Present for all service-returned same-content outcomes, including replays.
        content_hash: Option<ContentHash>,
    },
    ConflictSaved {
        path: VaultPath,
        conflict_id: ConflictId,
        materialized_path: VaultPath,
        policy_applied: ConflictPolicy,
        seq: i64,
    },
}

/// Query for current or explicit authoritative revision content.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RevisionContentQuery {
    pub(crate) path: VaultPath,
    pub(crate) revision_id: Option<RevisionId>,
}

/// Verified authoritative revision metadata plus bytes.
#[derive(Clone, Eq, PartialEq)]
pub(crate) struct AuthoritativeRevisionContent {
    pub(crate) path: VaultPath,
    pub(crate) revision_id: RevisionId,
    pub(crate) content_hash: ContentHash,
    pub(crate) size_bytes: u64,
    pub(crate) created_by: AdapterId,
    bytes: Vec<u8>,
}

impl AuthoritativeRevisionContent {
    #[must_use]
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    #[must_use]
    pub(crate) fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

impl fmt::Debug for AuthoritativeRevisionContent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthoritativeRevisionContent")
            .field("path", &self.path)
            .field("revision_id", &self.revision_id)
            .field("content_hash", &self.content_hash)
            .field("size_bytes", &self.size_bytes)
            .field("created_by", &self.created_by)
            .field("bytes", &"[REDACTED]")
            .finish()
    }
}

pub(super) async fn apply_file(
    pool: &PgPool,
    object_store: &LocalObjectStore,
    command: ApplyFileCommand,
) -> Result<ApplyFileOutcome, ApplicationError> {
    if let Some(replay) = read_file_replay(pool, &command.actor, &command.idempotency).await? {
        return enrich_replayed_outcome(pool, replay).await;
    }

    let mut transaction = pool.begin().await.map_err(|_| ApplicationError::Internal)?;
    lock_vault_path(&mut *transaction, &command.path)
        .await
        .map_err(|_| ApplicationError::Internal)?;

    let current_revision = get_current_revision_by_path(&mut *transaction, &command.path)
        .await
        .map_err(|_| ApplicationError::Internal)?
        .map(stored_revision_from_row)
        .transpose()?;
    let planned = run_core_upsert(&command, current_revision, object_store)?;
    let outcome = persist_upsert_outcome(
        &mut transaction,
        &command.actor,
        object_store,
        planned,
    )
    .await?;

    if let Some(replay) = store_file_outcome(
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
        return enrich_replayed_outcome(pool, replay).await;
    }

    transaction
        .commit()
        .await
        .map_err(|_| ApplicationError::Internal)?;
    Ok(outcome)
}

pub(super) async fn revision_content(
    pool: &PgPool,
    object_store: &LocalObjectStore,
    query: RevisionContentQuery,
) -> Result<AuthoritativeRevisionContent, ApplicationError> {
    let row = if let Some(revision_id) = &query.revision_id {
        let row = get_file_revision_by_id(pool, revision_id)
            .await
            .map_err(|_| ApplicationError::Internal)?
            .ok_or(ApplicationError::NotFound)?;
        if row.path != query.path.as_str() {
            return Err(ApplicationError::NotFound);
        }
        row
    } else {
        get_current_revision_by_path(pool, &query.path)
            .await
            .map_err(|_| ApplicationError::Internal)?
            .ok_or(ApplicationError::NotFound)?
    };
    let revision = stored_revision_from_row(row)?;
    let bytes = object_store
        .get_bytes(revision.content_hash)
        .map_err(map_object_store_read_error)?;
    if u64::try_from(bytes.len()).map_err(|_| ApplicationError::ContentCorrupt)?
        != revision.size_bytes
    {
        return Err(ApplicationError::ContentCorrupt);
    }
    Ok(AuthoritativeRevisionContent {
        path: revision.path,
        revision_id: revision.revision_id,
        content_hash: revision.content_hash,
        size_bytes: revision.size_bytes,
        created_by: revision.created_by,
        bytes,
    })
}

fn run_core_upsert(
    command: &ApplyFileCommand,
    current_revision: Option<StoredRevision>,
    object_store: &LocalObjectStore,
) -> Result<UpsertOutcome, ApplicationError> {
    let repository = PlanningRevisionRepository { current_revision };
    let content_store = PlanningContentStore { object_store };
    let operation_log = PlanningOperationLog;
    let mut service = RevisionService::new(repository, content_store, operation_log);
    let request = UpsertFileRequest::new(
        command.path.clone(),
        command.actor.adapter_id().clone(),
        command.base_revision_id.clone(),
        command.content_hash,
        command.bytes.clone(),
    )
    .with_idempotency_key(command.idempotency.key());
    service
        .upsert_file(request)
        .map_err(|_| ApplicationError::Internal)
}

async fn persist_upsert_outcome(
    transaction: &mut Transaction<'_, Postgres>,
    actor: &ApplicationActor,
    object_store: &LocalObjectStore,
    outcome: UpsertOutcome,
) -> Result<ApplyFileOutcome, ApplicationError> {
    match outcome {
        UpsertOutcome::AcceptedNewFile { revision, .. }
        | UpsertOutcome::AcceptedNewRevision { revision, .. } => {
            let seq = persist_accepted_revision(transaction, actor, &revision).await?;
            Ok(ApplyFileOutcome::Accepted {
                path: revision.path,
                revision_id: revision.revision_id,
                content_hash: Some(revision.content_hash),
                seq,
            })
        }
        UpsertOutcome::IgnoredDuplicateSameContent { current_revision } => {
            Ok(ApplyFileOutcome::SameContent {
                path: current_revision.path,
                revision_id: Some(current_revision.revision_id),
                content_hash: Some(current_revision.content_hash),
            })
        }
        UpsertOutcome::RejectedHashMismatch { .. } => Err(ApplicationError::InvalidContentHash),
        rejected @ UpsertOutcome::RejectedStaleOrUnknownBase {
            conflict_saved: Some(_),
            ..
        } => persist_conflict_saved_outcome(transaction, actor, object_store, &rejected).await,
        UpsertOutcome::RejectedStaleOrUnknownBase { .. } => Err(ApplicationError::Conflict),
    }
}

pub(crate) async fn persist_accepted_revision(
    transaction: &mut Transaction<'_, Postgres>,
    actor: &ApplicationActor,
    revision: &StoredRevision,
) -> Result<i64, ApplicationError> {
    insert_content_blob(transaction, revision.content_hash, revision.size_bytes).await?;
    let object_id = deterministic_identifier("obj_", &[revision.path.as_str()]);
    let object = create_or_find_sync_object_by_path(
        &mut **transaction,
        NewSyncObject {
            object_id: object_id.as_str(),
            path: &revision.path,
            kind: SyncObjectKind::File,
            updated_by: actor.adapter_id(),
        },
    )
    .await
    .map_err(|_| ApplicationError::Internal)?;
    insert_file_revision(
        &mut **transaction,
        NewFileRevision {
            revision_id: &revision.revision_id,
            object_id: object.object_id.as_str(),
            path: &revision.path,
            parent_revision_id: revision.parent_revision_id.as_ref(),
            content_hash: revision.content_hash,
            size_bytes: revision.size_bytes,
            created_by: actor.adapter_id(),
        },
    )
    .await
    .map_err(|_| ApplicationError::Internal)?;
    set_current_revision_by_object_id(
        &mut **transaction,
        object.object_id.as_str(),
        Some(&revision.revision_id),
        actor.adapter_id(),
    )
    .await
    .map_err(|_| ApplicationError::Internal)?
    .ok_or(ApplicationError::Internal)?;

    let content_hash = revision.content_hash.to_string();
    let op_id = OperationId::parse(&deterministic_identifier(
        "op_",
        &[
            revision.revision_id.as_str(),
            OperationKindName::UpsertFile.as_str(),
            revision.path.as_str(),
            content_hash.as_str(),
        ],
    ))
    .map_err(|_| ApplicationError::Internal)?;
    let operation = OperationLogRepository::new()
        .append(
            &mut **transaction,
            &AppendOperationLogEntry {
                op_id,
                adapter_id: actor.adapter_id().clone(),
                kind: OperationKindName::UpsertFile,
                path: revision.path.clone(),
                revision_id: Some(revision.revision_id.clone()),
                tombstone_id: None,
                conflict_id: None,
            },
        )
        .await
        .map_err(|_| ApplicationError::Internal)?;
    Ok(operation.seq)
}

async fn persist_conflict_saved_outcome(
    transaction: &mut Transaction<'_, Postgres>,
    actor: &ApplicationActor,
    object_store: &LocalObjectStore,
    outcome: &UpsertOutcome,
) -> Result<ApplyFileOutcome, ApplicationError> {
    let plan = require_upsert_conflict_saved_plan(outcome, Utc::now())
        .map_err(|_| ApplicationError::Conflict)?;
    let conflict_saved = outcome.conflict_saved().ok_or(ApplicationError::Conflict)?;
    object_store
        .put_bytes(
            plan.incoming_content_hash,
            conflict_saved.incoming_content.content.as_slice(),
        )
        .map_err(|_| ApplicationError::Internal)?;
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
            updated_by: actor.adapter_id(),
        },
    )
    .await
    .map_err(|_| ApplicationError::Internal)?;
    insert_file_revision(
        &mut **transaction,
        NewFileRevision {
            revision_id: &incoming_revision_id,
            object_id: conflict_object.object_id.as_str(),
            path: &plan.materialized_path,
            parent_revision_id: Some(&plan.current_revision_id),
            content_hash: plan.incoming_content_hash,
            size_bytes: plan.incoming_size_bytes,
            created_by: actor.adapter_id(),
        },
    )
    .await
    .map_err(|_| ApplicationError::Internal)?;
    set_current_revision_by_object_id(
        &mut **transaction,
        conflict_object.object_id.as_str(),
        Some(&incoming_revision_id),
        actor.adapter_id(),
    )
    .await
    .map_err(|_| ApplicationError::Internal)?
    .ok_or(ApplicationError::Internal)?;

    let conflict_id = conflict_id_from_plan(&plan)?;
    insert_conflict_row(transaction, &conflict_id, &incoming_revision_id, &plan).await?;
    let operation = OperationLogRepository::new()
        .append(
            &mut **transaction,
            &AppendOperationLogEntry {
                op_id: conflict_created_operation_id(&conflict_id, &incoming_revision_id, &plan)?,
                adapter_id: actor.adapter_id().clone(),
                kind: OperationKindName::ConflictCreated,
                path: plan.original_path.clone(),
                revision_id: Some(incoming_revision_id),
                tombstone_id: None,
                conflict_id: Some(conflict_id.clone()),
            },
        )
        .await
        .map_err(|_| ApplicationError::Internal)?;
    Ok(ApplyFileOutcome::ConflictSaved {
        path: plan.original_path,
        conflict_id,
        materialized_path: plan.materialized_path,
        policy_applied: plan.policy_applied,
        seq: operation.seq,
    })
}

async fn enrich_replayed_outcome(
    pool: &PgPool,
    outcome: ApplyFileOutcome,
) -> Result<ApplyFileOutcome, ApplicationError> {
    match outcome {
        ApplyFileOutcome::Accepted {
            path,
            revision_id,
            content_hash: _,
            seq,
        } => {
            let row = get_file_revision_by_id(pool, &revision_id)
                .await
                .map_err(|_| ApplicationError::Internal)?
                .ok_or(ApplicationError::Internal)?;
            let revision = stored_revision_from_row(row)?;
            Ok(ApplyFileOutcome::Accepted {
                path,
                revision_id,
                content_hash: Some(revision.content_hash),
                seq,
            })
        }
        ApplyFileOutcome::SameContent {
            path,
            revision_id: _,
            content_hash: _,
        } => {
            let row = get_current_revision_by_path(pool, &path)
                .await
                .map_err(|_| ApplicationError::Internal)?
                .ok_or(ApplicationError::Internal)?;
            let revision = stored_revision_from_row(row)?;
            Ok(ApplyFileOutcome::SameContent {
                path,
                revision_id: Some(revision.revision_id),
                content_hash: Some(revision.content_hash),
            })
        }
        conflict @ ApplyFileOutcome::ConflictSaved { .. } => Ok(conflict),
    }
}

fn stored_revision_from_row(row: FileRevisionRow) -> Result<StoredRevision, ApplicationError> {
    Ok(StoredRevision {
        revision_id: RevisionId::parse(row.revision_id.as_str())
            .map_err(|_| ApplicationError::Internal)?,
        path: VaultPath::parse(row.path.as_str()).map_err(|_| ApplicationError::Internal)?,
        parent_revision_id: row
            .parent_revision_id
            .as_deref()
            .map(RevisionId::parse)
            .transpose()
            .map_err(|_| ApplicationError::Internal)?,
        content_hash: ContentHash::parse(row.content_sha256.as_str())
            .map_err(|_| ApplicationError::Internal)?,
        size_bytes: u64::try_from(row.size_bytes).map_err(|_| ApplicationError::Internal)?,
        created_by: AdapterId::parse(row.created_by.as_str())
            .map_err(|_| ApplicationError::Internal)?,
    })
}

fn incoming_conflict_revision_id(
    plan: &ConflictSavedPreservationPlan,
) -> Result<RevisionId, ApplicationError> {
    let hash = plan.incoming_content_hash.to_string();
    RevisionId::parse(&deterministic_identifier(
        "rev_",
        &[
            plan.materialized_path.as_str(),
            plan.current_revision_id.as_str(),
            hash.as_str(),
            plan.incoming_adapter_id.as_str(),
        ],
    ))
    .map_err(|_| ApplicationError::Internal)
}

fn conflict_id_from_plan(
    plan: &ConflictSavedPreservationPlan,
) -> Result<ConflictId, ApplicationError> {
    let hash = plan.incoming_content_hash.to_string();
    ConflictId::parse(&deterministic_identifier(
        "conf_",
        &[
            plan.original_path.as_str(),
            plan.materialized_path.as_str(),
            plan.current_revision_id.as_str(),
            hash.as_str(),
            plan.incoming_adapter_id.as_str(),
        ],
    ))
    .map_err(|_| ApplicationError::Internal)
}

fn conflict_created_operation_id(
    conflict_id: &ConflictId,
    incoming_revision_id: &RevisionId,
    plan: &ConflictSavedPreservationPlan,
) -> Result<OperationId, ApplicationError> {
    OperationId::parse(&deterministic_identifier(
        "op_",
        &[
            conflict_id.as_str(),
            incoming_revision_id.as_str(),
            OperationKindName::ConflictCreated.as_str(),
            plan.original_path.as_str(),
        ],
    ))
    .map_err(|_| ApplicationError::Internal)
}

async fn insert_conflict_row(
    transaction: &mut Transaction<'_, Postgres>,
    conflict_id: &ConflictId,
    incoming_revision_id: &RevisionId,
    plan: &ConflictSavedPreservationPlan,
) -> Result<(), ApplicationError> {
    sqlx::query(
        "insert into conflicts \
         (conflict_id, original_path, base_revision_id, current_revision_id, incoming_revision_id, \
          incoming_adapter_id, policy_applied, materialized_path, status) \
         values ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(conflict_id.as_str())
    .bind(plan.original_path.as_str())
    .bind(plan.provided_base_revision_id.as_ref().map(RevisionId::as_str))
    .bind(plan.current_revision_id.as_str())
    .bind(incoming_revision_id.as_str())
    .bind(plan.incoming_adapter_id.as_str())
    .bind(plan.policy_applied.as_str())
    .bind(plan.materialized_path.as_str())
    .bind("open")
    .execute(&mut **transaction)
    .await
    .map_err(|_| ApplicationError::Internal)?;
    Ok(())
}

async fn insert_content_blob(
    transaction: &mut Transaction<'_, Postgres>,
    content_hash: ContentHash,
    size_bytes: u64,
) -> Result<(), ApplicationError> {
    let size_bytes = i64::try_from(size_bytes).map_err(|_| ApplicationError::Internal)?;
    let object_store_path = LocalObjectStore::relative_blob_path(content_hash)
        .to_string_lossy()
        .replace('\\', "/");
    sqlx::query(
        "insert into content_blobs (sha256, size_bytes, object_store_path) \
         values ($1, $2, $3) on conflict (sha256) do nothing",
    )
    .bind(content_hash.to_prefixed_string())
    .bind(size_bytes)
    .bind(object_store_path)
    .execute(&mut **transaction)
    .await
    .map_err(|_| ApplicationError::Internal)?;
    Ok(())
}

fn map_object_store_read_error(error: ObjectStoreError) -> ApplicationError {
    match error {
        ObjectStoreError::MissingBlob { .. } => ApplicationError::ContentUnavailable,
        ObjectStoreError::HashMismatch { .. } | ObjectStoreError::SizeMismatch { .. } => {
            ApplicationError::ContentCorrupt
        }
        _ => ApplicationError::Internal,
    }
}

struct PlanningRevisionRepository {
    current_revision: Option<StoredRevision>,
}

impl RevisionRepository for PlanningRevisionRepository {
    fn current_revision(
        &mut self,
        _path: &VaultPath,
    ) -> Result<Option<StoredRevision>, RevisionServiceError> {
        Ok(self.current_revision.clone())
    }

    fn insert_revision(
        &mut self,
        request: InsertRevisionRequest,
    ) -> Result<StoredRevision, RevisionServiceError> {
        let content_hash = request.content_hash.to_string();
        let revision_id = RevisionId::parse(&deterministic_identifier(
            "rev_",
            &[
                request.path.as_str(),
                request
                    .parent_revision_id
                    .as_ref()
                    .map(RevisionId::as_str)
                    .unwrap_or("null"),
                content_hash.as_str(),
                request.adapter_id.as_str(),
            ],
        ))
        .map_err(|_| RevisionServiceError::repository("build_revision_id"))?;
        Ok(StoredRevision {
            revision_id,
            path: request.path,
            parent_revision_id: request.parent_revision_id,
            content_hash: request.content_hash,
            size_bytes: request.size_bytes,
            created_by: request.adapter_id,
        })
    }
}

struct PlanningContentStore<'a> {
    object_store: &'a LocalObjectStore,
}

impl ContentStore for PlanningContentStore<'_> {
    fn put_content(
        &mut self,
        expected_hash: ContentHash,
        bytes: &[u8],
    ) -> Result<StoredContent, RevisionServiceError> {
        self.object_store
            .put_bytes(expected_hash, bytes)
            .map_err(|_| RevisionServiceError::content_store("put_content"))?;
        Ok(StoredContent {
            hash: expected_hash,
            size_bytes: u64::try_from(bytes.len())
                .map_err(|_| RevisionServiceError::content_store("content_size"))?,
        })
    }
}

struct PlanningOperationLog;

impl OperationLog for PlanningOperationLog {
    fn append_operation(
        &mut self,
        request: AppendOperationRequest,
    ) -> Result<OperationLogEntry, RevisionServiceError> {
        let kind = match request.kind {
            OperationKind::UpsertFile => OperationKindName::UpsertFile.as_str(),
        };
        let op_id = OperationId::parse(&deterministic_identifier(
            "op_",
            &[request.revision_id.as_str(), kind, request.path.as_str()],
        ))
        .map_err(|_| RevisionServiceError::operation_log("build_operation_id"))?;
        Ok(OperationLogEntry {
            operation_id: op_id,
            seq: 0,
        })
    }
}
