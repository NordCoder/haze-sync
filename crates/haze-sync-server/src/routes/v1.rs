//! V1 Core API route wiring for W2 normal file operations.
//!
//! This module wires already-merged W2 API helpers, storage repositories,
//! idempotency primitives, object store primitives, and readiness/auth state into
//! the normal PUT/GET/changes slice.
//!
//! W3 delete/conflict/admin behavior is composed alongside this router at the
//! parent `/v1` namespace.

mod persistence;
mod planning;
#[cfg(test)]
mod tests;

use axum::{
    body::{Body, Bytes},
    extract::{Path, Query},
    http::{header, HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Extension, Json, Router,
};
use haze_sync_api::{
    auth::AdapterPrincipal,
    contracts::{
        errors::{ErrorResponse, PublicErrorCode},
        headers::{IDEMPOTENCY_KEY_HEADER, X_BASE_REVISION_ID_HEADER, X_CONTENT_SHA256_HEADER},
    },
    dto::{
        changes::ChangeEntryDto,
        common::{ConflictPolicyDto, OperationKindDto},
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
            parse_get_file_request, parse_put_file_request, FileDownloadRouteHeaders,
            FileRouteError, GetFileRouteRequestParts, PutFileRouteRequest,
            PutFileRouteRequestParts, APPLICATION_OCTET_STREAM, X_REVISION_ID_HEADER,
            X_SIZE_BYTES_HEADER,
        },
    },
};
use haze_sync_common::{AdapterId, ContentHash, OperationId, RevisionId, VaultPath};
#[cfg(test)]
use haze_sync_core::revision_service::compute_content_hash;
use haze_sync_core::{
    conflict_saved_planner::ConflictSavedPreservationPlan,
    conflict_service::ConflictPolicy,
    idempotency::{RequestFingerprint, StoredIdempotencyResponse},
    revision_service::{RevisionServiceError, StoredRevision},
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
        operation_log::{ChangeFeedRow, OperationKindName, OperationLogRepository},
        revisions::{get_current_revision_by_path, get_file_revision_by_id},
        RepositoryError,
    },
    LocalObjectStore, ObjectStoreError,
};
use serde_json::json;
use sha2::{Digest, Sha256 as Sha256Digest};
use sqlx::PgPool;
use std::{collections::HashMap, str::FromStr};

use crate::{
    http::errors::{not_implemented_response, ShellErrorResponse},
    routes::auth::{authenticate_principal, AuthFailure},
    state::ServerAppState,
};

use self::{persistence::apply_upsert_outcome, planning::run_core_normal_upsert};

const MAX_UPLOAD_BYTES: u64 = 52_428_800;

/// Builds the versioned `/v1` API namespace.
pub fn router() -> Router {
    Router::new()
        .route("/server-info", get(server_info))
        .route("/changes", get(changes_route))
        .route("/files/*path", get(get_file_route).put(put_file_route))
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
    let response_body =
        apply_upsert_outcome(&mut transaction, &principal, object_store, outcome).await?;

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

/// Placeholder for future Core routes outside the currently wired W2/W3 surface.
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
    let principal = authenticate_principal(state, headers)
        .await
        .map_err(ApiError::from_auth_failure)?;

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
        "body_sha256": body_content_hash(request.body()).to_string(),
        "adapter_id": principal.adapter_id(),
        "scope": "adapter",
    });
    RequestFingerprint::from_safe_json_metadata(&metadata)
}

fn body_content_hash(bytes: &[u8]) -> ContentHash {
    let digest = Sha256Digest::digest(bytes);
    let mut hash_bytes = [0_u8; 32];
    hash_bytes.copy_from_slice(&digest);
    ContentHash::from_bytes(hash_bytes)
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
        &mut connection,
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

    match insert_idempotency_record(transaction, &input)
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

fn policy_dto(policy: ConflictPolicy) -> ConflictPolicyDto {
    match policy {
        ConflictPolicy::PreserveBoth => ConflictPolicyDto::PreserveBoth,
        ConflictPolicy::CurrentWinsWithIncomingBackup => {
            ConflictPolicyDto::CurrentWinsWithIncomingBackup
        }
    }
}

fn incoming_conflict_revision_id(
    plan: &ConflictSavedPreservationPlan,
) -> Result<RevisionId, ApiError> {
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
    .map_err(|_error| ApiError::internal())
}

fn conflict_id_from_plan(
    plan: &ConflictSavedPreservationPlan,
) -> Result<haze_sync_common::ConflictId, ApiError> {
    let hash = plan.incoming_content_hash.to_string();
    haze_sync_common::ConflictId::parse(&deterministic_identifier(
        "conf_",
        &[
            plan.original_path.as_str(),
            plan.materialized_path.as_str(),
            plan.current_revision_id.as_str(),
            hash.as_str(),
            plan.incoming_adapter_id.as_str(),
        ],
    ))
    .map_err(|_error| ApiError::internal())
}

fn conflict_created_operation_id(
    conflict_id: &haze_sync_common::ConflictId,
    incoming_revision_id: &RevisionId,
    plan: &ConflictSavedPreservationPlan,
) -> Result<OperationId, ApiError> {
    OperationId::parse(&deterministic_identifier(
        "op_",
        &[
            conflict_id.as_str(),
            incoming_revision_id.as_str(),
            OperationKindName::ConflictCreated.as_str(),
            plan.original_path.as_str(),
        ],
    ))
    .map_err(|_error| ApiError::internal())
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

pub(super) fn map_conflict_saved_planning_error(
    _error: haze_sync_core::conflict_saved_planner::ConflictSavedPlanningError,
) -> ApiError {
    FileRouteError::Conflict.into()
}

pub(super) fn deterministic_identifier(prefix: &str, parts: &[&str]) -> String {
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
pub(super) struct ApiError {
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

    pub(super) fn internal() -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            PublicErrorCode::InternalError,
            "Core file operation failed",
        )
    }

    fn from_auth_failure(error: AuthFailure) -> Self {
        match error {
            AuthFailure::MissingToken => Self::missing_token(),
            AuthFailure::InvalidToken => Self::invalid_token(),
            AuthFailure::Internal => Self::internal(),
        }
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
