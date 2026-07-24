//! HTTP transport for the passive Stage 11 operational-control API.

use axum::{
    extract::{Extension, Path},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use haze_sync_api::{
    contracts::errors::ErrorResponse,
    dto::control_plane::{
        CredentialCreateRequest, CredentialRevokeRequest, CredentialRotateRequest,
        MaintenanceActionDto, MaintenanceActionRequest, OperationalJobCancelRequest,
        OperationalJobConfirmationRequest, OperationalJobCreateRequest, PrincipalMutationRequest,
    },
    routes::control_plane::{
        validate_confirmation_request, validate_credential_create_request,
        validate_credential_rotate_request, validate_operational_job_create_request,
        ControlPlaneRouteError, ValidatedIdempotencyKey, IDEMPOTENCY_KEY_HEADER,
    },
};

use crate::{
    control_plane::{AuthenticatedIdentity, ControlPlaneError, ControlPlaneServices},
    routes::auth::{authenticate_identity, AuthFailure},
    state::ServerAppState,
};

pub fn router() -> Router {
    Router::new()
        .route("/admin/maintenance", get(maintenance_status))
        .route("/admin/maintenance/quiesce", post(quiesce))
        .route(
            "/admin/maintenance/enter-maintenance",
            post(enter_maintenance),
        )
        .route("/admin/maintenance/resume", post(resume))
        .route(
            "/admin/principals/:principal_id",
            get(principal_status).patch(mutate_principal),
        )
        .route(
            "/admin/principals/:principal_id/credentials",
            get(list_credentials).post(create_credential),
        )
        .route(
            "/admin/principals/:principal_id/credentials/rotate",
            post(rotate_credential),
        )
        .route("/admin/credentials/:credential_id", get(credential_status))
        .route(
            "/admin/credentials/:credential_id/revoke",
            post(revoke_credential),
        )
        .route("/admin/operational-jobs", post(create_operational_job))
        .route(
            "/admin/operational-jobs/:operation_id",
            get(operational_job_status),
        )
        .route(
            "/admin/operational-jobs/:operation_id/confirm",
            post(confirm_operational_job),
        )
        .route(
            "/admin/operational-jobs/:operation_id/cancel",
            post(cancel_operational_job),
        )
}

async fn maintenance_status(
    Extension(state): Extension<ServerAppState>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let actor = authenticate(&state, &headers).await?;
    require_admin(&actor)?;
    let response = services(&state)?.maintenance_status().await?;
    Ok((StatusCode::OK, Json(response)).into_response())
}

async fn quiesce(
    Extension(state): Extension<ServerAppState>,
    headers: HeaderMap,
    Json(request): Json<MaintenanceActionRequest>,
) -> Result<Response, ApiError> {
    maintenance_action(&state, &headers, request, MaintenanceActionDto::Quiesce).await
}

async fn enter_maintenance(
    Extension(state): Extension<ServerAppState>,
    headers: HeaderMap,
    Json(request): Json<MaintenanceActionRequest>,
) -> Result<Response, ApiError> {
    maintenance_action(
        &state,
        &headers,
        request,
        MaintenanceActionDto::EnterMaintenance,
    )
    .await
}

async fn resume(
    Extension(state): Extension<ServerAppState>,
    headers: HeaderMap,
    Json(request): Json<MaintenanceActionRequest>,
) -> Result<Response, ApiError> {
    maintenance_action(&state, &headers, request, MaintenanceActionDto::Resume).await
}

async fn maintenance_action(
    state: &ServerAppState,
    headers: &HeaderMap,
    request: MaintenanceActionRequest,
    action: MaintenanceActionDto,
) -> Result<Response, ApiError> {
    let actor = authenticate(state, headers).await?;
    let key = idempotency_key(headers)?;
    let response = services(state)?
        .request_maintenance_action(
            &actor,
            key.expose_for_private_digest(),
            action,
            request.expected_maintenance_generation,
        )
        .await?;
    Ok((StatusCode::OK, Json(response)).into_response())
}

async fn principal_status(
    Extension(state): Extension<ServerAppState>,
    Path(principal_id): Path<String>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let actor = authenticate(&state, &headers).await?;
    let value = services(&state)?
        .principal_status(&actor, &principal_id)
        .await?;
    Ok((StatusCode::OK, Json(value)).into_response())
}

async fn mutate_principal(
    Extension(state): Extension<ServerAppState>,
    Path(principal_id): Path<String>,
    headers: HeaderMap,
    Json(request): Json<PrincipalMutationRequest>,
) -> Result<Response, ApiError> {
    let actor = authenticate(&state, &headers).await?;
    let key = idempotency_key(&headers)?;
    let response = services(&state)?
        .mutate_principal(
            &actor,
            &principal_id,
            key.expose_for_private_digest(),
            &request,
        )
        .await?;
    Ok((StatusCode::OK, Json(response)).into_response())
}

async fn list_credentials(
    Extension(state): Extension<ServerAppState>,
    Path(principal_id): Path<String>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let actor = authenticate(&state, &headers).await?;
    let response = services(&state)?
        .list_credentials(&actor, &principal_id)
        .await?;
    Ok((StatusCode::OK, Json(response)).into_response())
}

async fn create_credential(
    Extension(state): Extension<ServerAppState>,
    Path(principal_id): Path<String>,
    headers: HeaderMap,
    Json(request): Json<CredentialCreateRequest>,
) -> Result<Response, ApiError> {
    validate_credential_create_request(&request)?;
    let actor = authenticate(&state, &headers).await?;
    let key = idempotency_key(&headers)?;
    let response = services(&state)?
        .create_credential(
            &actor,
            &principal_id,
            key.expose_for_private_digest(),
            &request,
        )
        .await?;
    Ok((StatusCode::CREATED, Json(response)).into_response())
}

async fn rotate_credential(
    Extension(state): Extension<ServerAppState>,
    Path(principal_id): Path<String>,
    headers: HeaderMap,
    Json(request): Json<CredentialRotateRequest>,
) -> Result<Response, ApiError> {
    validate_credential_rotate_request(&request)?;
    let actor = authenticate(&state, &headers).await?;
    let key = idempotency_key(&headers)?;
    let response = services(&state)?
        .rotate_credential(
            &actor,
            &principal_id,
            key.expose_for_private_digest(),
            &request,
        )
        .await?;
    Ok((StatusCode::OK, Json(response)).into_response())
}

async fn credential_status(
    Extension(state): Extension<ServerAppState>,
    Path(credential_id): Path<String>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let actor = authenticate(&state, &headers).await?;
    let response = services(&state)?
        .credential_status(&actor, &credential_id)
        .await?;
    Ok((StatusCode::OK, Json(response)).into_response())
}

async fn revoke_credential(
    Extension(state): Extension<ServerAppState>,
    Path(credential_id): Path<String>,
    headers: HeaderMap,
    Json(request): Json<CredentialRevokeRequest>,
) -> Result<Response, ApiError> {
    let actor = authenticate(&state, &headers).await?;
    let key = idempotency_key(&headers)?;
    let response = services(&state)?
        .revoke_credential(
            &actor,
            &credential_id,
            key.expose_for_private_digest(),
            &request,
        )
        .await?;
    Ok((StatusCode::OK, Json(response)).into_response())
}

async fn create_operational_job(
    Extension(state): Extension<ServerAppState>,
    headers: HeaderMap,
    Json(request): Json<OperationalJobCreateRequest>,
) -> Result<Response, ApiError> {
    validate_operational_job_create_request(&request)?;
    let actor = authenticate(&state, &headers).await?;
    let key = idempotency_key(&headers)?;
    let response = services(&state)?
        .create_operational_job(&actor, key.expose_for_private_digest(), &request)
        .await?;
    Ok((StatusCode::ACCEPTED, Json(response)).into_response())
}

async fn operational_job_status(
    Extension(state): Extension<ServerAppState>,
    Path(operation_id): Path<String>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let actor = authenticate(&state, &headers).await?;
    let response = services(&state)?
        .operational_job_status(&actor, &operation_id)
        .await?;
    Ok((StatusCode::OK, Json(response)).into_response())
}

async fn confirm_operational_job(
    Extension(state): Extension<ServerAppState>,
    Path(operation_id): Path<String>,
    headers: HeaderMap,
    Json(request): Json<OperationalJobConfirmationRequest>,
) -> Result<Response, ApiError> {
    validate_confirmation_request(&request)?;
    let actor = authenticate(&state, &headers).await?;
    let response = services(&state)?
        .confirm_operational_job(&actor, &operation_id, &request)
        .await?;
    Ok((StatusCode::OK, Json(response)).into_response())
}

async fn cancel_operational_job(
    Extension(state): Extension<ServerAppState>,
    Path(operation_id): Path<String>,
    headers: HeaderMap,
    Json(request): Json<OperationalJobCancelRequest>,
) -> Result<Response, ApiError> {
    let actor = authenticate(&state, &headers).await?;
    let response = services(&state)?
        .cancel_operational_job(&actor, &operation_id, &request)
        .await?;
    Ok((StatusCode::OK, Json(response)).into_response())
}

fn services(state: &ServerAppState) -> Result<&ControlPlaneServices, ApiError> {
    state.control_plane().ok_or_else(ApiError::internal)
}

async fn authenticate(
    state: &ServerAppState,
    headers: &HeaderMap,
) -> Result<AuthenticatedIdentity, ApiError> {
    authenticate_identity(state, headers)
        .await
        .map_err(ApiError::from_auth_failure)
}

fn require_admin(actor: &AuthenticatedIdentity) -> Result<(), ApiError> {
    if actor.principal.role().can_admin() {
        Ok(())
    } else {
        Err(ControlPlaneRouteError::Forbidden.into())
    }
}

fn idempotency_key(headers: &HeaderMap) -> Result<ValidatedIdempotencyKey, ApiError> {
    let value = headers
        .get(IDEMPOTENCY_KEY_HEADER)
        .map(|value| value.to_str())
        .transpose()
        .map_err(|_| ControlPlaneRouteError::InvalidIdempotencyKey)?;
    ValidatedIdempotencyKey::parse(value).map_err(Into::into)
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    body: ErrorResponse,
}

impl ApiError {
    fn internal() -> Self {
        ControlPlaneRouteError::Internal.into()
    }

    fn from_auth_failure(error: AuthFailure) -> Self {
        match error {
            AuthFailure::MissingToken => Self {
                status: StatusCode::UNAUTHORIZED,
                body: ErrorResponse::new(
                    haze_sync_api::contracts::errors::PublicErrorCode::MissingToken,
                    "Missing bearer token",
                ),
            },
            AuthFailure::InvalidToken => Self {
                status: StatusCode::UNAUTHORIZED,
                body: ErrorResponse::new(
                    haze_sync_api::contracts::errors::PublicErrorCode::InvalidToken,
                    "Invalid bearer token",
                ),
            },
            AuthFailure::Internal => ControlPlaneRouteError::Internal.into(),
        }
    }
}

impl From<ControlPlaneRouteError> for ApiError {
    fn from(error: ControlPlaneRouteError) -> Self {
        Self {
            status: StatusCode::from_u16(error.status_code())
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            body: error.error_response(),
        }
    }
}

impl From<ControlPlaneError> for ApiError {
    fn from(error: ControlPlaneError) -> Self {
        let route = match error {
            ControlPlaneError::InvalidInput => ControlPlaneRouteError::InvalidRequest,
            ControlPlaneError::InvalidAuthentication => ControlPlaneRouteError::Internal,
            ControlPlaneError::Forbidden => ControlPlaneRouteError::Forbidden,
            ControlPlaneError::NotFound => ControlPlaneRouteError::NotFound,
            ControlPlaneError::Conflict => ControlPlaneRouteError::Conflict,
            ControlPlaneError::MaintenanceInProgress => {
                ControlPlaneRouteError::MaintenanceInProgress
            }
            ControlPlaneError::ControlTransitionInProgress => {
                ControlPlaneRouteError::ControlTransitionInProgress
            }
            ControlPlaneError::InvalidControlTransition => {
                ControlPlaneRouteError::InvalidControlTransition
            }
            ControlPlaneError::StaleControlGeneration => {
                ControlPlaneRouteError::StaleControlGeneration
            }
            ControlPlaneError::StaleRecordVersion => ControlPlaneRouteError::StaleRecordVersion,
            ControlPlaneError::StaleExecutorFence => ControlPlaneRouteError::StaleExecutorFence,
            ControlPlaneError::IdempotencyConflict => ControlPlaneRouteError::IdempotencyConflict,
            ControlPlaneError::IdempotencyInProgress => {
                ControlPlaneRouteError::IdempotencyInProgress
            }
            ControlPlaneError::CredentialSecretNotReplayable => {
                ControlPlaneRouteError::CredentialSecretNotReplayable
            }
            ControlPlaneError::OperationInProgress => ControlPlaneRouteError::OperationInProgress,
            ControlPlaneError::ConfirmationRequired => ControlPlaneRouteError::ConfirmationRequired,
            ControlPlaneError::ConfirmationExpired => ControlPlaneRouteError::ConfirmationExpired,
            ControlPlaneError::OperationNotCancellable => {
                ControlPlaneRouteError::OperationNotCancellable
            }
            ControlPlaneError::UnsupportedOperationKind => {
                ControlPlaneRouteError::UnsupportedOperationKind
            }
            ControlPlaneError::InvalidStoredState
            | ControlPlaneError::AdapterControlUnavailable
            | ControlPlaneError::QuiescenceTimeout
            | ControlPlaneError::Internal => ControlPlaneRouteError::Internal,
        };
        route.into()
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

    #[test]
    fn control_errors_never_render_secrets_or_internal_storage_details() {
        for error in [
            ControlPlaneError::MaintenanceInProgress,
            ControlPlaneError::StaleExecutorFence,
            ControlPlaneError::IdempotencyInProgress,
            ControlPlaneError::ConfirmationExpired,
            ControlPlaneError::Internal,
        ] {
            let api = ApiError::from(error);
            let rendered = serde_json::to_string(&api.body).unwrap();
            assert!(!rendered.contains("postgres://"));
            assert!(!rendered.contains("verifier_material"));
            assert!(!rendered.contains("confirmation_digest"));
            assert!(!rendered.contains("lease_token"));
            assert!(!rendered.contains("sqlx"));
        }
    }
}
