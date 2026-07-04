//! Server wiring for the W3 DELETE tombstone fan-in slice.
//!
//! This module wires the already-merged passive DELETE route helpers, delete
//! guard primitives, tombstone service primitives, storage repositories, and
//! idempotency storage into a narrow server route. It records tombstone metadata
//! only and never hard-deletes database rows, content blobs, or filesystem paths.

use axum::{
    extract::{Extension, Path},
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{Duration, Utc};
use haze_sync_api::{
    auth::AdapterPrincipal,
    contracts::{
        errors::{ErrorResponse, PublicErrorCode},
        headers::{IDEMPOTENCY_KEY_HEADER, X_BASE_REVISION_ID_HEADER},
    },
    dto::files::DeleteFileResponse,
    routes::delete::{
        not_found_delete_response, parse_delete_file_request, stale_base_delete_response,
        tombstoned_delete_response, unsafe_delete_response, DeleteFileRouteRequest,
        DeleteFileRouteRequestParts, DeleteRouteError,
    },
};
use haze_sync_common::{OperationId, RevisionId, VaultPath};
use haze_sync_core::{
    delete_guard::{DeleteGuard, DeleteGuardDecision, DeleteGuardInput, DeleteRunScope},
    idempotency::{RequestFingerprint, StoredIdempotencyResponse},
    tombstone_service::{
        TombstoneCreationInput, TombstoneId, TombstoneRetention, TombstoneService,
    },
};
use haze_sync_storage::{
    locks::lock_vault_path,
    models::{OperationLogRow, SyncObjectRow, TombstoneRow},
    repositories::{
        idempotency::{
            compare_request_fingerprint, insert_idempotency_record, read_idempotency_record,
            IdempotencyRecordInput, IdempotencyRequestComparison, IdempotencyStoreOutcome,
        },
        objects::{
            get_sync_object_by_path, set_current_revision_by_object_id,
            set_sync_object_deleted_at_by_object_id,
        },
        operation_log::{AppendOperationLogEntry, OperationKindName, OperationLogRepository},
        tombstones::{NewTombstone, TombstoneRepository},
        RepositoryError,
    },
};
use serde_json::json;
use sha2::{Digest, Sha256 as Sha256Digest};
use sqlx::PgPool;

use crate::{
    routes::auth::{authenticate_principal, AuthFailure},
    state::ServerAppState,
};

const DELETE_RETENTION_DAYS: i64 = 30;
const ROUTE_SINGLE_DELETE_RATIO_FLOOR: u64 = 100;

pub fn router() -> axum::Router {
    axum::Router::new().route("/files/*path", axum::routing::delete(delete_file_route))
}

pub(super) async fn delete_file_route(
    Extension(state): Extension<ServerAppState>,
    Path(route_path): Path<String>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let principal = authenticate(&state, &headers).await?;
    let request = parse_delete_file_request(DeleteFileRouteRequestParts {
        route_path: route_path.as_str(),
        idempotency_key: optional_header(
            &headers,
            IDEMPOTENCY_KEY_HEADER,
            HeaderErrorKind::Idempotency,
        )?,
        base_revision_id: optional_header(
            &headers,
            X_BASE_REVISION_ID_HEADER,
            HeaderErrorKind::BaseRevision,
        )?,
        requested_delete_count: None,
    })?;
    request.auth_requirement().validate_role(principal.role())?;

    let pool = runtime_pool(&state)?;
    let fingerprint = delete_request_fingerprint(&request, &principal);

    if let Some(replay) = read_existing_delete_idempotency(
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

    let response_body = apply_delete_request(&mut transaction, &principal, &request).await?;
    let status = delete_response_status(&response_body);

    if let Some(replay) = store_delete_idempotency(
        &mut transaction,
        &principal,
        request.idempotency_key().as_str(),
        fingerprint,
        status,
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

    Ok((status, Json(response_body)).into_response())
}

async fn apply_delete_request(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    principal: &AdapterPrincipal,
    request: &DeleteFileRouteRequest,
) -> Result<DeleteFileResponse, ApiError> {
    let Some(object) = get_sync_object_by_path(&mut **transaction, request.path())
        .await
        .map_err(map_repository_error)?
    else {
        return Ok(not_found_delete_response(request.path().clone()));
    };

    let Some(current_revision_id) = active_current_revision_id(&object)? else {
        return Ok(not_found_delete_response(request.path().clone()));
    };

    if !delete_base_is_current(request, &current_revision_id) {
        return Ok(stale_base_delete_response(request.path().clone()));
    }

    let active_file_count = active_file_count(transaction).await?;
    let total_files_before_run = delete_guard_total_files(
        active_file_count,
        u64::from(request.delete_guard().requested_delete_count),
    );
    if !delete_guard_allows(principal, request, total_files_before_run)? {
        return Ok(unsafe_delete_response(request.path().clone()));
    }

    let tombstone = build_tombstone(principal, request.path(), &current_revision_id)?;
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
        .map_err(map_repository_error)?;

    clear_current_revision_and_mark_deleted(transaction, principal, &object, &tombstone_row)
        .await?;
    let operation = append_delete_operation(
        transaction,
        principal,
        request.path(),
        &current_revision_id,
        &tombstone_row.tombstone_id,
    )
    .await?;

    Ok(tombstoned_delete_response(
        request.path().clone(),
        tombstone_row.tombstone_id,
        operation.seq,
        timestamp_string(tombstone_row.retention_until),
    ))
}

fn active_current_revision_id(object: &SyncObjectRow) -> Result<Option<RevisionId>, ApiError> {
    if object.deleted_at.is_some() {
        return Ok(None);
    }

    object
        .current_revision_id
        .as_deref()
        .map(RevisionId::parse)
        .transpose()
        .map_err(|_error| ApiError::internal())
}

fn delete_base_is_current(
    request: &DeleteFileRouteRequest,
    current_revision_id: &RevisionId,
) -> bool {
    request.base_revision_id() == Some(current_revision_id)
}

async fn active_file_count(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<u64, ApiError> {
    let count: i64 = sqlx::query_scalar(
        "select count(*) from sync_objects where kind = 'file' and deleted_at is null and current_revision_id is not null",
    )
    .fetch_one(&mut **transaction)
    .await
    .map_err(|_error| ApiError::internal())?;

    u64::try_from(count).map_err(|_error| ApiError::internal())
}

fn delete_guard_total_files(active_file_count: u64, proposed_delete_count: u64) -> u64 {
    active_file_count
        .max(ROUTE_SINGLE_DELETE_RATIO_FLOOR)
        .max(proposed_delete_count)
}

fn delete_guard_allows(
    principal: &AdapterPrincipal,
    request: &DeleteFileRouteRequest,
    total_files_before_run: u64,
) -> Result<bool, ApiError> {
    let scope = DeleteRunScope::new(
        principal.common_adapter_id().clone(),
        deterministic_identifier(
            "delete_run_",
            &[principal.adapter_id(), request.path().as_str()],
        ),
    )
    .map_err(|_error| ApiError::internal())?;
    let input = DeleteGuardInput::without_manual_unlock(
        scope,
        u64::from(request.delete_guard().requested_delete_count),
        total_files_before_run,
    );

    Ok(matches!(
        DeleteGuard::default().evaluate(&input),
        DeleteGuardDecision::Allowed
    ))
}

fn build_tombstone(
    principal: &AdapterPrincipal,
    path: &VaultPath,
    current_revision_id: &RevisionId,
) -> Result<haze_sync_core::tombstone_service::Tombstone, ApiError> {
    let created_at = Utc::now();
    let retention_until = created_at + Duration::days(DELETE_RETENTION_DAYS);
    let tombstone_id = TombstoneId::parse(&deterministic_identifier(
        "tmb_",
        &[
            path.as_str(),
            current_revision_id.as_str(),
            principal.adapter_id(),
        ],
    ))
    .map_err(|_error| ApiError::internal())?;
    let input = TombstoneCreationInput::new(
        tombstone_id,
        path.clone(),
        current_revision_id.clone(),
        Some(current_revision_id.clone()),
        principal.common_adapter_id().clone(),
        TombstoneRetention::new(retention_until, Some(DELETE_RETENTION_DAYS as u16)),
        Some(created_at),
    );

    TombstoneService::new()
        .create_tombstone(input)
        .map_err(|_error| ApiError::internal())
}

async fn clear_current_revision_and_mark_deleted(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    principal: &AdapterPrincipal,
    object: &SyncObjectRow,
    tombstone_row: &TombstoneRow,
) -> Result<(), ApiError> {
    set_current_revision_by_object_id(
        &mut **transaction,
        object.object_id.as_str(),
        None,
        principal.common_adapter_id(),
    )
    .await
    .map_err(map_repository_error)?
    .ok_or_else(ApiError::internal)?;

    set_sync_object_deleted_at_by_object_id(
        &mut **transaction,
        object.object_id.as_str(),
        Some(tombstone_row.deleted_at),
        principal.common_adapter_id(),
    )
    .await
    .map_err(map_repository_error)?
    .ok_or_else(ApiError::internal)?;

    Ok(())
}

async fn append_delete_operation(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    principal: &AdapterPrincipal,
    path: &VaultPath,
    revision_id: &RevisionId,
    tombstone_id: &str,
) -> Result<OperationLogRow, ApiError> {
    let op_id = OperationId::parse(&deterministic_identifier(
        "op_",
        &[
            revision_id.as_str(),
            OperationKindName::DeleteFile.as_str(),
            path.as_str(),
            tombstone_id,
        ],
    ))
    .map_err(|_error| ApiError::internal())?;

    OperationLogRepository::new()
        .append(
            &mut **transaction,
            &AppendOperationLogEntry {
                op_id,
                adapter_id: principal.common_adapter_id().clone(),
                kind: OperationKindName::DeleteFile,
                path: path.clone(),
                revision_id: Some(revision_id.clone()),
                tombstone_id: Some(tombstone_id.to_owned()),
                conflict_id: None,
            },
        )
        .await
        .map_err(map_repository_error)
}

fn delete_response_status(response: &DeleteFileResponse) -> StatusCode {
    match response {
        DeleteFileResponse::Tombstoned { .. } => StatusCode::OK,
        DeleteFileResponse::NotFound { .. } => StatusCode::NOT_FOUND,
        DeleteFileResponse::Rejected { .. } => StatusCode::CONFLICT,
    }
}

fn delete_request_fingerprint(
    request: &DeleteFileRouteRequest,
    principal: &AdapterPrincipal,
) -> RequestFingerprint {
    let metadata = json!({
        "method": "DELETE",
        "path": request.path().as_str(),
        "base_revision_id": request
            .base_revision_id()
            .map(RevisionId::as_str)
            .unwrap_or("null"),
        "requested_delete_count": request.delete_guard().requested_delete_count,
        "adapter_id": principal.adapter_id(),
        "scope": "adapter",
    });
    RequestFingerprint::from_safe_json_metadata(&metadata)
}

async fn read_existing_delete_idempotency(
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
            Err(DeleteRouteError::IdempotencyMismatch.into())
        }
    }
}

async fn store_delete_idempotency(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    principal: &AdapterPrincipal,
    idempotency_key: &str,
    fingerprint: RequestFingerprint,
    status: StatusCode,
    response_body: &DeleteFileResponse,
) -> Result<Option<StoredIdempotencyResponse>, ApiError> {
    let stored_response = StoredIdempotencyResponse::json(
        status.as_u16(),
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
                    Err(DeleteRouteError::IdempotencyMismatch.into())
                }
            }
        }
    }
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

async fn authenticate(
    state: &ServerAppState,
    headers: &HeaderMap,
) -> Result<AdapterPrincipal, ApiError> {
    authenticate_principal(state, headers)
        .await
        .map_err(ApiError::from_auth_failure)
}

#[derive(Clone, Copy)]
enum HeaderErrorKind {
    Idempotency,
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
                HeaderErrorKind::Idempotency => DeleteRouteError::InvalidIdempotencyKey.into(),
                HeaderErrorKind::BaseRevision => DeleteRouteError::InvalidBaseRevision.into(),
            })
        })
        .transpose()
}

fn runtime_pool(state: &ServerAppState) -> Result<&PgPool, ApiError> {
    state.db_pool().ok_or_else(ApiError::storage_unavailable)
}

fn timestamp_string(value: chrono::DateTime<Utc>) -> String {
    value.format("%Y-%m-%dT%H:%M:%SZ").to_string()
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

fn map_repository_error(_error: RepositoryError) -> ApiError {
    ApiError::internal()
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
            "Core delete operation failed",
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

impl From<DeleteRouteError> for ApiError {
    fn from(error: DeleteRouteError) -> Self {
        let status = StatusCode::from_u16(error.http_status_code())
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        Self {
            status,
            body: error.to_error_response(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}
#[cfg(test)]
mod tests;
