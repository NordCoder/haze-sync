//! HTTP transport wiring for guarded file deletion.
//!
//! This route owns parsing, authentication, authorization, and public response
//! mapping. The reusable application service owns transaction, locking,
//! idempotency, Core delete guard/tombstone, and Storage choreography.

use axum::{
    extract::{Extension, Path},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use haze_sync_api::{
    auth::AdapterPrincipal,
    contracts::{
        errors::{ErrorResponse, PublicErrorCode},
        headers::{IDEMPOTENCY_KEY_HEADER, X_BASE_REVISION_ID_HEADER},
    },
    dto::files::DeleteFileResponse,
    routes::delete::{
        not_found_delete_response, parse_delete_file_request, stale_base_delete_response,
        tombstoned_delete_response, unsafe_delete_response, DeleteFileRouteRequestParts,
        DeleteRouteError,
    },
};

use crate::{
    application::{
        delete_request_fingerprint, ApplicationActor, ApplicationError, ApplicationIdempotency,
        ApplyDeleteCommand, ApplyDeleteOutcome, ServerApplicationServices,
    },
    routes::auth::{authenticate_principal, AuthFailure},
    state::ServerAppState,
};

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

    let actor = ApplicationActor::new(principal.common_adapter_id().clone());
    let requested_delete_count = request.delete_guard().requested_delete_count;
    let fingerprint = delete_request_fingerprint(
        &actor,
        request.path(),
        request.base_revision_id(),
        requested_delete_count,
    );
    let command = ApplyDeleteCommand {
        actor,
        path: request.path().clone(),
        base_revision_id: request.base_revision_id().cloned(),
        requested_delete_count,
        idempotency: ApplicationIdempotency::new(request.idempotency_key().as_str(), fingerprint),
    };
    let outcome = application_services(&state)?
        .apply_delete(command)
        .await
        .map_err(map_application_error)?;
    let response = delete_response(outcome);
    let status = delete_response_status(&response);
    Ok((status, Json(response)).into_response())
}

fn application_services(state: &ServerAppState) -> Result<ServerApplicationServices, ApiError> {
    state
        .application_services()
        .ok_or_else(ApiError::storage_unavailable)
}

fn delete_response(outcome: ApplyDeleteOutcome) -> DeleteFileResponse {
    match outcome {
        ApplyDeleteOutcome::Tombstoned {
            path,
            tombstone_id,
            seq,
            retention_until,
        } => tombstoned_delete_response(
            path,
            tombstone_id,
            seq,
            retention_until.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        ),
        ApplyDeleteOutcome::NotFound { path } => not_found_delete_response(path),
        ApplyDeleteOutcome::StaleBase { path } => stale_base_delete_response(path),
        ApplyDeleteOutcome::GuardRejected { path } => unsafe_delete_response(path),
    }
}

fn delete_response_status(response: &DeleteFileResponse) -> StatusCode {
    match response {
        DeleteFileResponse::Tombstoned { .. } => StatusCode::OK,
        DeleteFileResponse::NotFound { .. } => StatusCode::NOT_FOUND,
        DeleteFileResponse::Rejected { .. } => StatusCode::CONFLICT,
    }
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
            value.to_str().map_err(|_| match kind {
                HeaderErrorKind::Idempotency => DeleteRouteError::InvalidIdempotencyKey.into(),
                HeaderErrorKind::BaseRevision => DeleteRouteError::InvalidBaseRevision.into(),
            })
        })
        .transpose()
}

fn map_application_error(error: ApplicationError) -> ApiError {
    match error {
        ApplicationError::DependenciesUnavailable => ApiError::storage_unavailable(),
        ApplicationError::NotFound => DeleteRouteError::NotFound.into(),
        ApplicationError::Conflict => DeleteRouteError::StaleBaseConflict.into(),
        ApplicationError::IdempotencyMismatch => DeleteRouteError::IdempotencyMismatch.into(),
        ApplicationError::InvalidInput
        | ApplicationError::InvalidContentHash
        | ApplicationError::ContentUnavailable
        | ApplicationError::ContentCorrupt
        | ApplicationError::Internal => ApiError::internal(),
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
