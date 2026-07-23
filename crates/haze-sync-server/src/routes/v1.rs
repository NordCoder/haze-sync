//! V1 HTTP transport wiring for file operations and authoritative changes.
//!
//! Routes own parsing, authentication, authorization, and public response mapping.
//! Reusable application services own transaction, locking, idempotency, Core,
//! object-store, and Storage choreography.

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
            accepted_upload_response, conflict_saved_upload_response,
            ignored_same_content_response, parse_get_file_request, parse_put_file_request,
            FileDownloadRouteHeaders, FileRouteError, GetFileRouteRequestParts,
            PutFileRouteRequestParts, APPLICATION_OCTET_STREAM, X_REVISION_ID_HEADER,
            X_SIZE_BYTES_HEADER,
        },
    },
};
use haze_sync_storage::repositories::operation_log::OperationKindName;
use std::collections::HashMap;

use crate::{
    application::{
        file_request_fingerprint, ApplicationActor, ApplicationError, ApplicationIdempotency,
        ApplyFileCommand, ApplyFileOutcome, AuthoritativeChange, AuthoritativeChangesQuery,
        RevisionContentQuery, ServerApplicationServices,
    },
    http::errors::{not_implemented_response, ShellErrorResponse},
    routes::auth::{authenticate_principal, AuthFailure},
    state::ServerAppState,
};

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

    let actor = ApplicationActor::new(principal.common_adapter_id().clone());
    let fingerprint = file_request_fingerprint(
        &actor,
        request.path(),
        request.base_revision_id(),
        request.content_sha256(),
        request.body(),
    );
    let command = ApplyFileCommand {
        actor,
        path: request.path().clone(),
        base_revision_id: request.base_revision_id().cloned(),
        content_hash: request.content_sha256(),
        bytes: request.body().to_vec(),
        idempotency: ApplicationIdempotency::new(request.idempotency_key().as_str(), fingerprint),
    };
    let outcome = application_services(&state)
        .await?
        .apply_file(command)
        .await
        .map_err(map_file_application_error)?;
    Ok((StatusCode::OK, Json(put_response(outcome))).into_response())
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
    let content = application_services(&state)
        .await?
        .revision_content(RevisionContentQuery {
            path: request.path().clone(),
            revision_id: request.revision_id().cloned(),
        })
        .await
        .map_err(map_file_application_error)?;
    let headers = FileDownloadRouteHeaders::new(
        content.revision_id.clone(),
        content.content_hash,
        content.size_bytes,
    );
    raw_file_response(content.into_bytes(), headers)
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
    let internal_query =
        AuthoritativeChangesQuery::new(request.since_value(), request.limit_value())
            .map_err(|_| ChangesRouteError::invalid_response_page())?;
    let batch = application_services(&state)
        .await?
        .authoritative_changes(internal_query)
        .await
        .map_err(map_changes_application_error)?;
    let changes = batch
        .changes
        .into_iter()
        .map(change_entry_dto)
        .collect::<Result<Vec<_>, _>>()?;
    let response = changes_response_from_parts(batch.from, batch.to, batch.has_more, changes)?;
    Ok((StatusCode::OK, Json(response)).into_response())
}

/// Placeholder for future Core routes outside the currently wired W2/W3 surface.
pub async fn core_route_not_implemented() -> (StatusCode, Json<ShellErrorResponse>) {
    not_implemented_response()
}

async fn application_services(
    state: &ServerAppState,
) -> Result<ServerApplicationServices, ApiError> {
    state
        .application_services()
        .ok_or_else(ApiError::storage_unavailable)
}

fn put_response(outcome: ApplyFileOutcome) -> PutFileResponse {
    match outcome {
        ApplyFileOutcome::Accepted {
            path,
            revision_id,
            seq,
            ..
        } => accepted_upload_response(path, revision_id, seq),
        ApplyFileOutcome::SameContent { path, .. } => ignored_same_content_response(path),
        ApplyFileOutcome::ConflictSaved {
            path,
            conflict_id,
            materialized_path,
            policy_applied,
            seq,
        } => conflict_saved_upload_response(
            path,
            conflict_id,
            materialized_path,
            policy_dto(policy_applied),
            seq,
        ),
    }
}

fn change_entry_dto(change: AuthoritativeChange) -> Result<ChangeEntryDto, ApiError> {
    Ok(ChangeEntryDto {
        seq: change.seq,
        kind: operation_kind_dto(change.kind),
        path: VaultPathDto::from(change.path),
        revision_id: change.revision_id.map(RevisionIdDto::from),
        content_sha256: change.content_hash.map(ContentSha256Dto::from),
        size_bytes: change.size_bytes,
        tombstone_id: change.tombstone_id.map(TombstoneIdDto::from),
        conflict_id: change.conflict_id.map(ConflictIdDto::from),
        updated_by: AdapterIdDto::from(change.actor_id),
        updated_at: TimestampDto::from(change.occurred_at.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
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

fn insert_header(headers: &mut HeaderMap, name: &'static str, value: &str) -> Result<(), ApiError> {
    let name = HeaderName::from_bytes(name.as_bytes()).map_err(|_| ApiError::internal())?;
    let value = HeaderValue::from_str(value).map_err(|_| ApiError::internal())?;
    headers.insert(name, value);
    Ok(())
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
            Err(ApiError::forbidden_role())
        }
        FilePermission::Write if !principal.role().can_write_files() => {
            Err(ApiError::forbidden_role())
        }
        _ => Ok(principal),
    }
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
            value.to_str().map_err(|_| match kind {
                HeaderErrorKind::Idempotency => FileRouteError::InvalidIdempotencyKey.into(),
                HeaderErrorKind::ContentSha256 => FileRouteError::InvalidContentSha256.into(),
                HeaderErrorKind::BaseRevision => FileRouteError::InvalidBaseRevision.into(),
            })
        })
        .transpose()
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

const fn policy_dto(policy: haze_sync_core::conflict_service::ConflictPolicy) -> ConflictPolicyDto {
    match policy {
        haze_sync_core::conflict_service::ConflictPolicy::PreserveBoth => {
            ConflictPolicyDto::PreserveBoth
        }
        haze_sync_core::conflict_service::ConflictPolicy::CurrentWinsWithIncomingBackup => {
            ConflictPolicyDto::CurrentWinsWithIncomingBackup
        }
    }
}

fn map_file_application_error(error: ApplicationError) -> ApiError {
    match error {
        ApplicationError::DependenciesUnavailable => ApiError::storage_unavailable(),
        ApplicationError::NotFound | ApplicationError::ContentUnavailable => {
            FileRouteError::NotFound.into()
        }
        ApplicationError::Conflict => FileRouteError::Conflict.into(),
        ApplicationError::InvalidContentHash => FileRouteError::InvalidContentSha256.into(),
        ApplicationError::IdempotencyMismatch => FileRouteError::IdempotencyMismatch.into(),
        ApplicationError::InvalidInput
        | ApplicationError::ContentCorrupt
        | ApplicationError::Internal => ApiError::internal(),
    }
}

fn map_changes_application_error(error: ApplicationError) -> ApiError {
    match error {
        ApplicationError::DependenciesUnavailable => ApiError::storage_unavailable(),
        _ => ChangesRouteError::core_unavailable().into(),
    }
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

    pub(super) fn invalid_token() -> Self {
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
