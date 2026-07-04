use haze_sync_api::{auth::AdapterPrincipal, routes::files::PutFileRouteRequest};
use haze_sync_common::{ContentHash, OperationId, RevisionId, VaultPath};
use haze_sync_core::revision_service::{
    AppendOperationRequest, ContentStore, InsertRevisionRequest, OperationKind, OperationLog,
    OperationLogEntry, RevisionRepository, RevisionService, RevisionServiceError, StoredContent,
    StoredRevision, UpsertFileRequest, UpsertOutcome,
};
use haze_sync_storage::{
    object_store::ObjectStore, repositories::operation_log::OperationKindName, LocalObjectStore,
};

use super::{deterministic_identifier, map_revision_service_error, ApiError};

pub(super) fn run_core_normal_upsert(
    request: &PutFileRouteRequest,
    principal: &AdapterPrincipal,
    current_revision: Option<StoredRevision>,
    object_store: &LocalObjectStore,
) -> Result<UpsertOutcome, ApiError> {
    let repository = PlanningRevisionRepository { current_revision };
    let content_store = PlanningContentStore { object_store };
    let operation_log = PlanningOperationLog;
    let mut service = RevisionService::new(repository, content_store, operation_log);

    let mut upsert_request = UpsertFileRequest::new(
        request.path().clone(),
        principal.common_adapter_id().clone(),
        request.base_revision_id().cloned(),
        request.content_sha256(),
        request.body().to_vec(),
    );
    upsert_request = upsert_request.with_idempotency_key(request.idempotency_key().as_str());

    service
        .upsert_file(upsert_request)
        .map_err(map_revision_service_error)
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
        let content_hash_string = request.content_hash.to_string();
        let revision_id = RevisionId::parse(&deterministic_identifier(
            "rev_",
            &[
                request.path.as_str(),
                request
                    .parent_revision_id
                    .as_ref()
                    .map(RevisionId::as_str)
                    .unwrap_or("null"),
                content_hash_string.as_str(),
                request.adapter_id.as_str(),
            ],
        ))
        .map_err(|_error| RevisionServiceError::repository("build_revision_id"))?;

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
            .map_err(|_error| RevisionServiceError::content_store("put_content"))?;

        Ok(StoredContent {
            hash: expected_hash,
            size_bytes: u64::try_from(bytes.len())
                .map_err(|_error| RevisionServiceError::content_store("content_size"))?,
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
        .map_err(|_error| RevisionServiceError::operation_log("build_operation_id"))?;

        Ok(OperationLogEntry {
            operation_id: op_id,
            seq: 0,
        })
    }
}
