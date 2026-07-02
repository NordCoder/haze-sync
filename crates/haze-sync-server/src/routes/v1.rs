//! V1 Core API route wiring for W2 normal file operations.
//!
//! This module wires already-merged W2 API helpers, storage repositories,
//! idempotency primitives, object store primitives, and readiness/auth state into
//! the normal PUT/GET/changes slice. Conflict preservation, delete behavior, and
//! adapter runtimes remain future-phase work.

use axum::{
    body::{Body, Bytes},
    extract::{Path, Query},
    http::{header, HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Extension, Json, Router,
};
use haze_sync_api::{
    auth::{AdapterPrincipal, AdapterRole, BearerToken},
    contracts::{
        errors::{ErrorResponse, PublicErrorCode},
        headers::{
            AUTHORIZATION_HEADER, IDEMPOTENCY_KEY_HEADER, X_BASE_REVISION_ID_HEADER,
            X_CONTENT_SHA256_HEADER,
        },
    },
    dto::{
        changes::ChangeEntryDto,
        common::OperationKindDto,
        files::PutFileResponse,
        primitives::{
            AdapterIdDto, ConflictIdDto, ContentSha256Dto, RevisionIdDto, TimestampDto,
            TombstoneIdDto, VaultPathDto,
        },
        server::{ServerCapabilityDto, ServerInfoResponse},
    },
    routes::{
        changes::{changes_response_from_parts, parse_changes_query, ChangesRouteError},
        files::{
            accepted_upload_response, ignored_same_content_response, parse_get_file_request,
            parse_put_file_request, FileDownloadRouteHeaders, FileRouteError,
            GetFileRouteRequestParts, PutFileRouteRequest, PutFileRouteRequestParts,
            APPLICATION_OCTET_STREAM, X_REVISION_ID_HEADER, X_SIZE_BYTES_HEADER,
        },
    },
};
use haze_sync_common::{AdapterId, ContentHash, OperationId, RevisionId, VaultPath};
use haze_sync_core::{
    idempotency::{RequestFingerprint, StoredIdempotencyResponse},
    revision_service::{
        compute_content_hash, AppendOperationRequest, ContentStore, InsertRevisionRequest,
        OperationKind, OperationLog, OperationLogEntry, RevisionRepository, RevisionService,
        RevisionServiceError, StoredContent, StoredRevision, UpsertFileRequest, UpsertOutcome,
    },
};
use haze_sync_storage::{
    locks::lock_vault_path,
    models::FileRevisionRow,
    object_store::ObjectStore,
    repositories::{
        idempotency::{
            compare_request_fingerprint, insert_idempotency_record, read_idempotency_record,
            IdempotencyRecordInput, IdempotencyRequestComparison, IdempotencyStoreOutcome,
        },
        objects::{
            create_or_find_sync_object_by_path, set_current_revision_by_object_id, NewSyncObject,
            SyncObjectKind,
        },
        operation_log::{
            AppendOperationLogEntry, ChangeFeedRow, OperationKindName, OperationLogRepository,
        },
        revisions::{
            get_current_revision_by_path, get_file_revision_by_id, insert_file_revision,
            NewFileRevision,
        },
        RepositoryError,
    },
    LocalObjectStore, ObjectStoreError,
};
use serde_json::json;
use sha2::{Digest, Sha256 as Sha256Digest};
use sqlx::{PgPool, Row};
use std::{collections::HashMap, str::FromStr};

use crate::{
    http::errors::{not_implemented_response, ShellErrorResponse},
    state::{AuthState, ServerAppState},
};

const MAX_UPLOAD_BYTES: u64 = 52_428_800;

/// Builds the versioned `/v1` API namespace.
pub fn router() -> Router {
    Router::new()
        .route("/server-info", get(server_info))
        .route("/changes", get(changes_route))
        .route(
            "/files/*path",
            get(get_file_route)
                .put(put_file_route)
                .delete(core_route_not_implemented),
        )
        .route("/conflicts", get(core_route_not_implemented))
        .route(
            "/conflicts/{conflict_id}/resolve",
            axum::routing::post(core_route_not_implemented),
        )
}

/// Handles GET /v1/server-info with static, safe route-shell information.
pub async fn server_info() -> Json<ServerInfoResponse> {
    Json(ServerInfoResponse {
        server_id: "haze-sync-route-shell".to_owned(),
        protocol_version: 1,
        max_upload_bytes: MAX_UPLOAD_BYTES,
        capabilities: vec![
            ServerCapabilityDto::Sha256,
            ServerCapabilityDto::OperationLog,
            ServerCapabilityDto::Tombstones,
            ServerCapabilityDto::Conflicts,
            ServerCapabilityDto::ConflictCenter,
            ServerCapabilityDto::BatchChanges,
        ],
    })
}

async fn put_file_route(
    Extension(state): Extension<ServerAppState>,
    Path(route_path): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, ApiError> {
    let principal = authenticate(&state, &headers, FilePermission::Write).await?;
    let request = parse_put_file_request(PutFileRouteRequestParts {
        route_path: route_path.as_str(),
        idempotency_key: optional_header(
            &headers,
            IDEMPOTENCY_KEY_HEADER,
            HeaderErrorKind::Idempotency,
        )?,
        content_sha256: optional_header(
            &headers,
            X_CONTENT_SHA256_HEADER,
            HeaderErrorKind::ContentSha256,
        )?,
        base_revision_id: optional_header(
            &headers,
            X_BASE_REVISION_ID_HEADER,
            HeaderErrorKind::BaseRevision,
        )?,
        body: body.to_vec(),
        max_upload_bytes: Some(MAX_UPLOAD_BYTES),
    })?;

    let pool = runtime_pool(&state)?;
    let object_store = runtime_object_store(&state)?;
    let fingerprint = request_fingerprint(&request, &principal);

    if let Some(replay) = read_existing_idempotency(
        pool,
        &principal,
        request.idempotency_key().as_str(),
        fingerprint,
    )
    .await?
    {
        return replay_response(replay);
    }

    let mut transaction = pool.begin().await.map_err(|_error| ApiError::internal())?;
    lock_vault_path(&mut *transaction, request.path())
        .await
        .map_err(map_repository_error)?;

    let current_revision = get_current_revision_by_path(&mut *transaction, request.path())
        .await
        .map_err(map_repository_error)?
        .map(stored_revision_from_row)
        .transpose()?;

    let outcome = run_core_normal_upsert(&request, &principal, current_revision, object_store)?;
    let response_body = apply_upsert_outcome(&mut transaction, &principal, outcome).await?;

    if let Some(replay) = store_successful_idempotency(
        &mut transaction,
        &principal,
        request.idempotency_key().as_str(),
        fingerprint,
        &response_body,
    )
    .await?
    {
        transaction
            .rollback()
            .await
            .map_err(|_error| ApiError::internal())?;
        return replay_response(replay);
    }

    transaction
        .commit()
        .await
        .map_err(|_error| ApiError::internal())?;

    Ok((StatusCode::OK, Json(response_body)).into_response())
}

async fn get_file_route(
    Extension(state): Extension<ServerAppState>,
    Path(route_path): Path<String>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    authenticate(&state, &headers, FilePermission::Read).await?;
    let request = parse_get_file_request(GetFileRouteRequestParts {
        route_path: route_path.as_str(),
        revision_id: query.get("revision_id").map(String::as_str),
    })?;

    let pool = runtime_pool(&state)?;
    let object_store = runtime_object_store(&state)?;

    let row = if let Some(revision_id) = request.revision_id() {
        let row = get_file_revision_by_id(pool, revision_id)
            .await
            .map_err(map_repository_error)?
            .ok_or(FileRouteError::NotFound)?;
        if row.path != request.path().as_str() {
            return Err(FileRouteError::NotFound.into());
        }
        row
    } else {
        get_current_revision_by_path(pool, request.path())
            .await
            .map_err(map_repository_error)?
            .ok_or(FileRouteError::NotFound)?
    };

    let revision = stored_revision_from_row(row)?;
    let bytes = object_store
        .get_bytes(revision.content_hash)
        .map_err(map_object_store_get_error)?;
    if u64::try_from(bytes.len()).map_err(|_| ApiError::internal())? != revision.size_bytes {
        return Err(ApiError::internal());
    }

    raw_file_response(
        bytes,
        FileDownloadRouteHeaders::new(
            revision.revision_id,
            revision.content_hash,
            revision.size_bytes,
        ),
    )
}

async fn changes_route(
    Extension(state): Extension<ServerAppState>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    authenticate(&state, &headers, FilePermission::Read).await?;
    let request = parse_changes_query(
        query.get("since").map(String::as_str),
        query.get("limit").map(String::as_str),
    )?;
    let pool = runtime_pool(&state)?;
    let page = OperationLogRepository::new()
        .changes_since(pool, request.since_value(), request.limit_value())
        .await
        .map_err(map_repository_error)?;

    let changes = page
        .changes
        .into_iter()
        .map(change_entry_from_row)
        .collect::<Result<Vec<_>, _>>()?;
    let response = changes_response_from_parts(page.from_seq, page.to_seq, page.has_more, changes)?;

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// Placeholder for future Core routes whose business behavior is outside W2-F1.
pub async fn core_route_not_implemented() -> (StatusCode, Json<ShellErrorResponse>) {
    not_implemented_response()
}

#[derive(Clone, Copy)]
enum FilePermission {
    Read,
    Write,
}

async fn authenticate(
    state: &ServerAppState,
    headers: &HeaderMap,
    permission: FilePermission,
) -> Result<AdapterPrincipal, ApiError> {
    let header = required_auth_header(headers)?;
    let token = BearerToken::parse_authorization_header(header)
        .map_err(|_error| ApiError::invalid_token())?;

    let principal = match state.auth() {
        AuthState::Disabled => return Err(ApiError::invalid_token()),
        AuthState::StaticPrincipal { principal } => principal.clone(),
        AuthState::Database { pool } => lookup_principal_by_token(pool, &token).await?,
    };

    match permission {
        FilePermission::Read if !principal.role().can_read_files() => {
            return Err(ApiError::forbidden_role());
        }
        FilePermission::Write if !principal.role().can_write_files() => {
            return Err(ApiError::forbidden_role());
        }
        _ => {}
    }

    Ok(principal)
}

async fn lookup_principal_by_token(
    pool: &PgPool,
    token: &BearerToken,
) -> Result<AdapterPrincipal, ApiError> {
    let hash = token.sha256_hash();
    let prefixed_hash = format!("sha256:{}", hash.digest_hex());
    let row = sqlx::query(
        "select adapter_id, role from sync_adapters \
         where enabled = true and (token_hash = $1 or token_hash = $2) \
         limit 1",
    )
    .bind(hash.digest_hex())
    .bind(prefixed_hash)
    .fetch_optional(pool)
    .await
    .map_err(|_error| ApiError::internal())?
    .ok_or_else(ApiError::invalid_token)?;

    let adapter_id: String = row
        .try_get("adapter_id")
        .map_err(|_error| ApiError::internal())?;
    let role: String = row.try_get("role").map_err(|_error| ApiError::internal())?;
    let role = AdapterRole::from_str(&role).map_err(|_error| ApiError::invalid_token())?;

    AdapterPrincipal::new(adapter_id, role).map_err(|_error| ApiError::invalid_token())
}

fn required_auth_header(headers: &HeaderMap) -> Result<&str, ApiError> {
    let value = headers
        .get(AUTHORIZATION_HEADER)
        .ok_or_else(ApiError::missing_token)?;
    value.to_str().map_err(|_error| ApiError::invalid_token())
}

#[derive(Clone, Copy)]
enum HeaderErrorKind {
    Idempotency,
    ContentSha256,
    BaseRevision,
}

fn optional_header<'a>(
    headers: &'a HeaderMap,
    name: &'static str,
    kind: HeaderErrorKind,
) -> Result<Option<&'a str>, ApiError> {
    headers
        .get(name)
        .map(|value| {
            value.to_str().map_err(|_error| match kind {
                HeaderErrorKind::Idempotency => FileRouteError::InvalidIdempotencyKey.into(),
                HeaderErrorKind::ContentSha256 => FileRouteError::InvalidContentSha256.into(),
                HeaderErrorKind::BaseRevision => FileRouteError::InvalidBaseRevision.into(),
            })
        })
        .transpose()
}

fn runtime_pool(state: &ServerAppState) -> Result<&PgPool, ApiError> {
    state.db_pool().ok_or_else(ApiError::storage_unavailable)
}

fn runtime_object_store(state: &ServerAppState) -> Result<&LocalObjectStore, ApiError> {
    state
        .object_store()
        .ok_or_else(ApiError::storage_unavailable)
}

fn request_fingerprint(
    request: &PutFileRouteRequest,
    principal: &AdapterPrincipal,
) -> RequestFingerprint {
    let metadata = json!({
        "method": "PUT",
        "path": request.path().as_str(),
        "base_revision_id": request
            .base_revision_id()
            .map(RevisionId::as_str)
            .unwrap_or("null"),
        "content_sha256": request.content_sha256().to_string(),
        "adapter_id": principal.adapter_id(),
        "scope": "adapter",
    });
    RequestFingerprint::from_safe_json_metadata(&metadata)
}

async fn read_existing_idempotency(
    pool: &PgPool,
    principal: &AdapterPrincipal,
    idempotency_key: &str,
    fingerprint: RequestFingerprint,
) -> Result<Option<StoredIdempotencyResponse>, ApiError> {
    let mut connection = pool
        .acquire()
        .await
        .map_err(|_error| ApiError::internal())?;
    let Some(record) = read_idempotency_record(
        &mut *connection,
        principal.common_adapter_id(),
        idempotency_key,
    )
    .await
    .map_err(|_error| ApiError::internal())?
    else {
        return Ok(None);
    };

    match compare_request_fingerprint(&record, fingerprint.as_sha256())
        .map_err(|_error| ApiError::internal())?
    {
        IdempotencyRequestComparison::SameRequest => {
            let response = serde_json::from_value(record.response_json)
                .map_err(|_error| ApiError::internal())?;
            Ok(Some(response))
        }
        IdempotencyRequestComparison::DifferentRequest => {
            Err(FileRouteError::IdempotencyMismatch.into())
        }
    }
}

async fn store_successful_idempotency(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    principal: &AdapterPrincipal,
    idempotency_key: &str,
    fingerprint: RequestFingerprint,
    response_body: &PutFileResponse,
) -> Result<Option<StoredIdempotencyResponse>, ApiError> {
    let stored_response = StoredIdempotencyResponse::json(
        StatusCode::OK.as_u16(),
        serde_json::to_value(response_body).map_err(|_error| ApiError::internal())?,
    )
    .map_err(|_error| ApiError::internal())?;
    let input = IdempotencyRecordInput::new(
        principal.common_adapter_id().clone(),
        idempotency_key,
        *fingerprint.as_sha256(),
        serde_json::to_value(stored_response).map_err(|_error| ApiError::internal())?,
    )
    .map_err(|_error| ApiError::internal())?;

    match insert_idempotency_record(&mut **transaction, &input)
        .await
        .map_err(|_error| ApiError::internal())?
    {
        IdempotencyStoreOutcome::Stored { .. } => Ok(None),
        IdempotencyStoreOutcome::AlreadyExists { record } => {
            match compare_request_fingerprint(&record, fingerprint.as_sha256())
                .map_err(|_error| ApiError::internal())?
            {
                IdempotencyRequestComparison::SameRequest => {
                    let response = serde_json::from_value(record.response_json)
                        .map_err(|_error| ApiError::internal())?;
                    Ok(Some(response))
                }
                IdempotencyRequestComparison::DifferentRequest => {
                    Err(FileRouteError::IdempotencyMismatch.into())
                }
            }
        }
    }
}

fn run_core_normal_upsert(
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

async fn apply_upsert_outcome(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    principal: &AdapterPrincipal,
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
        UpsertOutcome::RejectedStaleOrUnknownBase { .. } => Err(FileRouteError::Conflict.into()),
    }
}

/// Planning repository used to let the W2-P4 service make the normal-upsert decision.
///
/// The actual DB writes happen after the service returns an accepted outcome, inside
/// the caller's transaction. The service remains the decision authority for accepted,
/// ignored, hash-rejected, and stale-base-rejected outcomes.
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

/// Content-store adapter for the W2-P4 service.
///
/// File bytes are written before DB metadata is committed because the local
/// filesystem object store is not transaction-aware. This ordering guarantees no
/// metadata row can point at missing content when the store write fails. If a later
/// DB operation fails or the transaction rolls back, the safe fallback is an
/// unreferenced content-addressed blob; W2-F1 intentionally does not add garbage
/// collection, retention workers, or cleanup jobs.
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

async fn persist_accepted_revision(
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

fn stored_revision_from_row(row: FileRevisionRow) -> Result<StoredRevision, ApiError> {
    let revision_id =
        RevisionId::parse(row.revision_id.as_str()).map_err(|_error| ApiError::internal())?;
    let path = VaultPath::parse(row.path.as_str()).map_err(|_error| ApiError::internal())?;
    let parent_revision_id = row
        .parent_revision_id
        .as_deref()
        .map(RevisionId::parse)
        .transpose()
        .map_err(|_error| ApiError::internal())?;
    let content_hash =
        ContentHash::parse(row.content_sha256.as_str()).map_err(|_error| ApiError::internal())?;
    let size_bytes = u64::try_from(row.size_bytes).map_err(|_| ApiError::internal())?;
    let created_by =
        AdapterId::parse(row.created_by.as_str()).map_err(|_error| ApiError::internal())?;

    Ok(StoredRevision {
        revision_id,
        path,
        parent_revision_id,
        content_hash,
        size_bytes,
        created_by,
    })
}

fn raw_file_response(
    bytes: Vec<u8>,
    headers: FileDownloadRouteHeaders,
) -> Result<Response, ApiError> {
    let mut response = Response::new(Body::from(bytes));
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(APPLICATION_OCTET_STREAM),
    );
    insert_header(
        response.headers_mut(),
        X_REVISION_ID_HEADER,
        headers.revision_id().as_str(),
    )?;
    insert_header(
        response.headers_mut(),
        X_CONTENT_SHA256_HEADER,
        &headers.content_sha256().to_string(),
    )?;
    insert_header(
        response.headers_mut(),
        X_SIZE_BYTES_HEADER,
        &headers.size_bytes().to_string(),
    )?;

    Ok(response)
}

fn replay_response(stored: StoredIdempotencyResponse) -> Result<Response, ApiError> {
    let status =
        StatusCode::from_u16(stored.status_code()).map_err(|_error| ApiError::internal())?;
    let mut response = (status, Json(stored.body().clone())).into_response();

    for (name, value) in stored.headers() {
        let name =
            HeaderName::from_bytes(name.as_bytes()).map_err(|_error| ApiError::internal())?;
        let value = HeaderValue::from_str(value).map_err(|_error| ApiError::internal())?;
        response.headers_mut().insert(name, value);
    }

    Ok(response)
}

fn insert_header(headers: &mut HeaderMap, name: &'static str, value: &str) -> Result<(), ApiError> {
    let name = HeaderName::from_bytes(name.as_bytes()).map_err(|_error| ApiError::internal())?;
    let value = HeaderValue::from_str(value).map_err(|_error| ApiError::internal())?;
    headers.insert(name, value);
    Ok(())
}

fn change_entry_from_row(row: ChangeFeedRow) -> Result<ChangeEntryDto, ApiError> {
    let kind = OperationKindName::from_str(row.kind.as_str()).map_err(map_repository_error)?;
    Ok(ChangeEntryDto {
        seq: row.seq,
        kind: operation_kind_dto(kind),
        path: VaultPathDto::from(row.path),
        revision_id: row.revision_id.map(RevisionIdDto::from),
        content_sha256: row.content_sha256.map(ContentSha256Dto::from),
        size_bytes: row
            .size_bytes
            .map(u64::try_from)
            .transpose()
            .map_err(|_error| ApiError::internal())?,
        tombstone_id: row.tombstone_id.map(TombstoneIdDto::from),
        conflict_id: row.conflict_id.map(ConflictIdDto::from),
        updated_by: AdapterIdDto::from(row.adapter_id),
        updated_at: TimestampDto::from(row.created_at.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
    })
}

const fn operation_kind_dto(kind: OperationKindName) -> OperationKindDto {
    match kind {
        OperationKindName::UpsertFile => OperationKindDto::UpsertFile,
        OperationKindName::DeleteFile => OperationKindDto::DeleteFile,
        OperationKindName::RestoreFile => OperationKindDto::RestoreFile,
        OperationKindName::ConflictCreated => OperationKindDto::ConflictCreated,
        OperationKindName::ConflictResolved => OperationKindDto::ConflictResolved,
        OperationKindName::BackupCreated => OperationKindDto::BackupCreated,
    }
}

fn map_object_store_get_error(error: ObjectStoreError) -> ApiError {
    match error {
        ObjectStoreError::MissingBlob { .. } => FileRouteError::NotFound.into(),
        _ => ApiError::internal(),
    }
}

fn map_repository_error(_error: RepositoryError) -> ApiError {
    ApiError::internal()
}

fn map_revision_service_error(_error: RevisionServiceError) -> ApiError {
    ApiError::internal()
}

fn deterministic_identifier(prefix: &str, parts: &[&str]) -> String {
    let mut hasher = Sha256Digest::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update([0]);
    }
    let digest = hasher.finalize();
    format!("{prefix}{}", lower_hex(&digest[..16]))
}

fn lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    body: ErrorResponse,
}

impl ApiError {
    fn new(status: StatusCode, code: PublicErrorCode, message: &'static str) -> Self {
        Self {
            status,
            body: ErrorResponse::new(code, message),
        }
    }

    fn missing_token() -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            PublicErrorCode::MissingToken,
            "Missing bearer token",
        )
    }

    fn invalid_token() -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            PublicErrorCode::InvalidToken,
            "Invalid bearer token",
        )
    }

    fn forbidden_role() -> Self {
        Self::new(
            StatusCode::FORBIDDEN,
            PublicErrorCode::ForbiddenRole,
            "Authenticated adapter role is not allowed for this operation",
        )
    }

    fn storage_unavailable() -> Self {
        Self::new(
            StatusCode::SERVICE_UNAVAILABLE,
            PublicErrorCode::InternalError,
            "Core storage dependencies are not configured",
        )
    }

    fn internal() -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            PublicErrorCode::InternalError,
            "Core file operation failed",
        )
    }
}

impl From<FileRouteError> for ApiError {
    fn from(error: FileRouteError) -> Self {
        let status = StatusCode::from_u16(error.http_status_code())
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        Self {
            status,
            body: error.to_error_response(),
        }
    }
}

impl From<ChangesRouteError> for ApiError {
    fn from(error: ChangesRouteError) -> Self {
        let status =
            StatusCode::from_u16(error.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        Self {
            status,
            body: error.error_response(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use haze_sync_api::auth::AdapterRole;
    use serde_json::Value;
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn parsed_request(
        path: &str,
        base_revision_id: Option<&str>,
        content_sha256: &str,
    ) -> PutFileRouteRequest {
        parse_put_file_request(PutFileRouteRequestParts {
            route_path: path,
            idempotency_key: Some("test-key"),
            content_sha256: Some(content_sha256),
            base_revision_id,
            body: b"hello".to_vec(),
            max_upload_bytes: Some(MAX_UPLOAD_BYTES),
        })
        .unwrap()
    }

    fn parsed_request_with_body(
        path: &str,
        base_revision_id: Option<&str>,
        body: &[u8],
    ) -> PutFileRouteRequest {
        let hash = compute_content_hash(body).to_string();
        parse_put_file_request(PutFileRouteRequestParts {
            route_path: path,
            idempotency_key: Some("test-key"),
            content_sha256: Some(&hash),
            base_revision_id,
            body: body.to_vec(),
            max_upload_bytes: Some(MAX_UPLOAD_BYTES),
        })
        .unwrap()
    }

    fn principal() -> AdapterPrincipal {
        AdapterPrincipal::new("obsidian-plugin", AdapterRole::ObsidianPlugin).unwrap()
    }

    fn temp_store(label: &str) -> (LocalObjectStore, PathBuf) {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "haze-sync-w2f1-{label}-{}-{suffix}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        (LocalObjectStore::new(root.clone()), root)
    }

    fn stored_revision(
        revision_id: &str,
        path: &str,
        parent_revision_id: Option<&str>,
        content: &[u8],
    ) -> StoredRevision {
        StoredRevision {
            revision_id: RevisionId::parse(revision_id).unwrap(),
            path: VaultPath::parse(path).unwrap(),
            parent_revision_id: parent_revision_id.map(|value| RevisionId::parse(value).unwrap()),
            content_hash: compute_content_hash(content),
            size_bytes: content.len() as u64,
            created_by: AdapterId::parse("obsidian-plugin").unwrap(),
        }
    }

    fn body_json(response: &PutFileResponse) -> Value {
        serde_json::to_value(response).unwrap()
    }

    #[test]
    fn id_fixtures_have_required_prefixes() {
        let revision_id = deterministic_identifier("rev_", &["Notes/a.md", "sha256"]);
        let op_id = deterministic_identifier("op_", &["rev_1", "upsert_file"]);

        assert!(RevisionId::parse(&revision_id).is_ok());
        assert!(OperationId::parse(&op_id).is_ok());
    }

    #[test]
    fn put_new_file_accepted_and_get_latest_returns_same_bytes_hash() {
        let (store, root) = temp_store("new-file");
        let request = parsed_request_with_body("Notes/a.md", None, b"hello");
        let outcome = run_core_normal_upsert(&request, &principal(), None, &store).unwrap();

        let revision = match outcome {
            UpsertOutcome::AcceptedNewFile {
                revision,
                operation,
            } => {
                assert_eq!(operation.seq, 0);
                revision
            }
            other => panic!("unexpected outcome: {other:?}"),
        };

        assert_eq!(revision.path.as_str(), "Notes/a.md");
        assert_eq!(revision.content_hash, compute_content_hash(b"hello"));
        assert_eq!(
            store.get_bytes(revision.content_hash).unwrap(),
            b"hello".to_vec()
        );

        let response = raw_file_response(
            b"hello".to_vec(),
            FileDownloadRouteHeaders::new(
                revision.revision_id.clone(),
                revision.content_hash,
                revision.size_bytes,
            ),
        )
        .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            APPLICATION_OCTET_STREAM
        );
        assert_eq!(
            response.headers().get(X_REVISION_ID_HEADER).unwrap(),
            revision.revision_id.as_str()
        );
        assert_eq!(
            response.headers().get(X_CONTENT_SHA256_HEADER).unwrap(),
            revision.content_hash.to_string().as_str()
        );
        assert_eq!(
            response.headers().get(X_SIZE_BYTES_HEADER).unwrap(),
            revision.size_bytes.to_string().as_str()
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn put_second_revision_accepted_with_current_base_and_explicit_revision_get_works() {
        let (store, root) = temp_store("second-revision");
        let current = stored_revision("rev_current", "Notes/a.md", None, b"old");
        store.put_bytes(current.content_hash, b"old").unwrap();

        let request = parsed_request_with_body("Notes/a.md", Some("rev_current"), b"new");
        let outcome =
            run_core_normal_upsert(&request, &principal(), Some(current.clone()), &store).unwrap();

        let revision = match outcome {
            UpsertOutcome::AcceptedNewRevision { revision, .. } => revision,
            other => panic!("unexpected outcome: {other:?}"),
        };

        assert_eq!(
            revision.parent_revision_id.as_ref().unwrap().as_str(),
            "rev_current"
        );
        assert_eq!(
            store.get_bytes(revision.content_hash).unwrap(),
            b"new".to_vec()
        );

        let explicit = FileDownloadRouteHeaders::new(
            revision.revision_id.clone(),
            revision.content_hash,
            revision.size_bytes,
        );
        let response =
            raw_file_response(store.get_bytes(revision.content_hash).unwrap(), explicit).unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(X_REVISION_ID_HEADER).unwrap(),
            revision.revision_id.as_str()
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn same_content_duplicate_is_ignored_without_extra_content_write() {
        let (store, root) = temp_store("duplicate");
        let current = stored_revision("rev_current", "Notes/a.md", None, b"hello");
        let request = parsed_request_with_body("Notes/a.md", Some("rev_current"), b"hello");

        let outcome =
            run_core_normal_upsert(&request, &principal(), Some(current.clone()), &store).unwrap();

        match outcome {
            UpsertOutcome::IgnoredDuplicateSameContent { current_revision } => {
                assert_eq!(current_revision.revision_id, current.revision_id);
            }
            other => panic!("unexpected outcome: {other:?}"),
        }
        assert!(store.get_bytes(current.content_hash).is_err());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn stale_unknown_or_null_base_with_different_content_does_not_overwrite() {
        let (store, root) = temp_store("stale");
        let current = stored_revision("rev_current", "Notes/a.md", None, b"old");

        let null_base = parsed_request_with_body("Notes/a.md", None, b"new");
        let null_outcome =
            run_core_normal_upsert(&null_base, &principal(), Some(current.clone()), &store)
                .unwrap();
        assert!(matches!(
            null_outcome,
            UpsertOutcome::RejectedStaleOrUnknownBase { .. }
        ));
        assert!(store.get_bytes(compute_content_hash(b"new")).is_err());

        let stale_base = parsed_request_with_body("Notes/a.md", Some("rev_stale"), b"newer");
        let stale_outcome =
            run_core_normal_upsert(&stale_base, &principal(), Some(current), &store).unwrap();
        assert!(matches!(
            stale_outcome,
            UpsertOutcome::RejectedStaleOrUnknownBase { .. }
        ));
        assert!(store.get_bytes(compute_content_hash(b"newer")).is_err());

        let unknown_base =
            parsed_request_with_body("Notes/missing.md", Some("rev_missing"), b"new");
        let unknown_outcome =
            run_core_normal_upsert(&unknown_base, &principal(), None, &store).unwrap();
        assert!(matches!(
            unknown_outcome,
            UpsertOutcome::RejectedStaleOrUnknownBase { .. }
        ));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn invalid_content_hash_is_rejected_before_content_store_write() {
        let (store, root) = temp_store("hash-mismatch");
        let wrong_hash = compute_content_hash(b"other").to_string();
        let request = parse_put_file_request(PutFileRouteRequestParts {
            route_path: "Notes/a.md",
            idempotency_key: Some("test-key"),
            content_sha256: Some(&wrong_hash),
            base_revision_id: Some("rev_current"),
            body: b"hello".to_vec(),
            max_upload_bytes: Some(MAX_UPLOAD_BYTES),
        })
        .unwrap();

        let outcome = run_core_normal_upsert(&request, &principal(), None, &store).unwrap();
        assert!(matches!(
            outcome,
            UpsertOutcome::RejectedHashMismatch { .. }
        ));
        assert!(store.get_bytes(compute_content_hash(b"hello")).is_err());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn idempotency_same_key_same_request_replays_and_different_request_rejects() {
        let first = parsed_request_with_body("Notes/a.md", None, b"hello");
        let same = parsed_request_with_body("Notes/a.md", None, b"hello");
        let different = parsed_request_with_body("Notes/a.md", None, b"changed");
        let principal = principal();

        let first_fingerprint = request_fingerprint(&first, &principal);
        let same_fingerprint = request_fingerprint(&same, &principal);
        let different_fingerprint = request_fingerprint(&different, &principal);

        let response = accepted_upload_response(
            first.path().clone(),
            RevisionId::parse("rev_replayed").unwrap(),
            1,
        );
        let stored =
            StoredIdempotencyResponse::json(StatusCode::OK.as_u16(), body_json(&response)).unwrap();

        assert_eq!(first_fingerprint, same_fingerprint);
        assert_eq!(stored.status_code(), StatusCode::OK.as_u16());
        assert_eq!(stored.body(), &body_json(&response));
        assert_ne!(first_fingerprint, different_fingerprint);
    }

    #[test]
    fn changes_mapping_preserves_upsert_kind_and_safe_metadata() {
        assert_eq!(
            operation_kind_dto(OperationKindName::UpsertFile),
            OperationKindDto::UpsertFile
        );
    }

    #[test]
    fn missing_file_maps_to_safe_not_found_response() {
        let error: ApiError = FileRouteError::NotFound.into();
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn invalid_headers_or_paths_map_to_safe_errors() {
        let missing_idempotency = parse_put_file_request(PutFileRouteRequestParts {
            route_path: "Notes/a.md",
            idempotency_key: None,
            content_sha256: Some(&compute_content_hash(b"hello").to_string()),
            base_revision_id: Some("null"),
            body: b"hello".to_vec(),
            max_upload_bytes: Some(MAX_UPLOAD_BYTES),
        })
        .unwrap_err();
        let response = ApiError::from(missing_idempotency).into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let invalid_path = parse_get_file_request(GetFileRouteRequestParts {
            route_path: "../secret.md",
            revision_id: None,
        })
        .unwrap_err();
        let response = ApiError::from(invalid_path).into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn request_fingerprint_uses_safe_metadata_not_body() {
        let hash = compute_content_hash(b"hello").to_string();
        let first = parsed_request("Notes/a.md", Some("rev_current"), &hash);
        let second = parse_put_file_request(PutFileRouteRequestParts {
            route_path: "Notes/a.md",
            idempotency_key: Some("test-key"),
            content_sha256: Some(&hash),
            base_revision_id: Some("rev_current"),
            body: b"different body would already fail hash verification".to_vec(),
            max_upload_bytes: Some(MAX_UPLOAD_BYTES),
        })
        .unwrap();
        let principal =
            AdapterPrincipal::new("obsidian-plugin", AdapterRole::ObsidianPlugin).unwrap();

        assert_eq!(
            request_fingerprint(&first, &principal),
            request_fingerprint(&second, &principal)
        );
    }

    #[test]
    fn request_fingerprint_changes_with_base_revision() {
        let hash = compute_content_hash(b"hello").to_string();
        let current = parsed_request("Notes/a.md", Some("rev_current"), &hash);
        let other = parsed_request("Notes/a.md", Some("rev_other"), &hash);
        let principal =
            AdapterPrincipal::new("obsidian-plugin", AdapterRole::ObsidianPlugin).unwrap();

        assert_ne!(
            request_fingerprint(&current, &principal),
            request_fingerprint(&other, &principal)
        );
    }

    #[test]
    fn public_auth_errors_do_not_echo_tokens() {
        let error = ApiError::invalid_token().into_response();
        assert_eq!(error.status(), StatusCode::UNAUTHORIZED);
    }
}
