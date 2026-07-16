//! Conflict route implementation preserved under a composition wrapper.

use axum::{
    body::Bytes,
    extract::{Extension, Path, Query},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use haze_sync_api::{
    contracts::errors::{ErrorResponse, PublicErrorCode},
    dto::{
        conflicts::{
            ConflictDetailResponse, ConflictListQuery, ConflictListResponse,
            ConflictResolutionRequest, ConflictResolutionResponse,
        },
        primitives::{ConflictIdDto, RevisionIdDto, TimestampDto, VaultPathDto},
    },
    routes::conflicts::{
        parse_conflict_list_query, parse_conflict_resolution_request,
        try_conflict_detail_response, try_conflict_list_response,
        try_conflict_resolution_response, ConflictsRouteError,
    },
};
use haze_sync_common::{AdapterRole, ConflictId};
use haze_sync_storage::repositories::conflicts::{
    get_conflict, list_conflicts, resolve_conflict, ConflictRepositoryError,
};
use serde::Deserialize;
use sqlx::PgPool;

use crate::{
    routes::auth::{authenticate_principal, AuthFailure},
    state::ServerAppState,
};

#[derive(Debug, Deserialize)]
struct ConflictListParams {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    offset: Option<u32>,
}

pub fn router() -> axum::Router {
    axum::Router::new()
        .route("/conflicts", axum::routing::get(list_conflicts_route))
        .route("/conflicts/:conflict_id", axum::routing::get(get_conflict_route))
        .route(
            "/conflicts/:conflict_id/resolve",
            axum::routing::post(resolve_conflict_route),
        )
}

async fn list_conflicts_route(
    Extension(state): Extension<ServerAppState>,
    Query(params): Query<ConflictListParams>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    authenticate_reader(&state, &headers).await?;
    let query = parse_conflict_list_query(ConflictListQuery {
        status: params.status,
        path: params.path,
        limit: params.limit,
        offset: params.offset,
    })
    .map_err(ApiError::from)?;
    let pool = state
        .db_pool()
        .ok_or_else(ApiError::unavailable)?;
    let rows = list_conflicts(pool, &query)
        .await
        .map_err(ApiError::from_repository)?;
    let response = try_conflict_list_response(rows)
        .map_err(ApiError::from)?;
    Ok((StatusCode::OK, Json(response)).into_response())
}

async fn get_conflict_route(
    Extension(state): Extension<ServerAppState>,
    Path(conflict_id): Path<String>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    authenticate_reader(&state, &headers).await?;
    let conflict_id = ConflictId::parse(&conflict_id)
        .map_err(|_| ApiError::invalid_request())?;
    let pool = state
        .db_pool()
        .ok_or_else(ApiError::unavailable)?;
    let row = get_conflict(pool, &conflict_id)
        .await
        .map_err(ApiError::from_repository)?
        .ok_or_else(ApiError::not_found)?;
    let response = try_conflict_detail_response(row)
        .map_err(ApiError::from)?;
    Ok((StatusCode::OK, Json(response)).into_response())
}

async fn resolve_conflict_route(
    Extension(state): Extension<ServerAppState>,
    Path(conflict_id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, ApiError> {
    let principal = authenticate_principal(&state, &headers)
        .await
        .map_err(ApiError::from_auth_failure)?;
    if !principal.role().can_admin() {
        return Err(ApiError::forbidden_role());
    }
    let conflict_id = ConflictId::parse(&conflict_id)
        .map_err(|_| ApiError::invalid_request())?;
    let request = serde_json::from_slice::<ConflictResolutionRequest>(&body)
        .map_err(|_| ApiError::invalid_request())?;
    let parsed = parse_conflict_resolution_request(request)
        .map_err(ApiError::from)?;
    let pool = state
        .db_pool()
        .ok_or_else(ApiError::unavailable)?;
    let row = resolve_conflict(pool, &conflict_id, &parsed)
        .await
        .map_err(ApiError::from_repository)?;
    let response = try_conflict_resolution_response(row)
        .map_err(ApiError::from)?;
    Ok((StatusCode::OK, Json(response)).into_response())
}

async fn authenticate_reader(
    state: &ServerAppState,
    headers: &HeaderMap,
) -> Result<(), ApiError> {
    let principal = authenticate_principal(state, headers)
        .await
        .map_err(ApiError::from_auth_failure)?;
    match principal.role() {
        AdapterRole::Admin
        | AdapterRole::ObsidianAdapter
        | AdapterRole::GdriveAdapter
        | AdapterRole::WorktreeAdapter => Ok(()),
    }
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

    fn invalid_request() -> Self {
        Self::new(
            StatusCode::BAD_REQUEST,
            PublicErrorCode::InvalidRequest,
            "Conflict request failed validation",
        )
    }

    fn not_found() -> Self {
        Self::new(
            StatusCode::NOT_FOUND,
            PublicErrorCode::NotFound,
            "Conflict was not found",
        )
    }

    fn unavailable() -> Self {
        Self::new(
            StatusCode::SERVICE_UNAVAILABLE,
            PublicErrorCode::Unavailable,
            "Conflict storage is unavailable",
        )
    }

    fn internal() -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            PublicErrorCode::InternalError,
            "Conflict request failed",
        )
    }

    fn from_repository(error: ConflictRepositoryError) -> Self {
        match error {
            ConflictRepositoryError::InvalidIdentifier
            | ConflictRepositoryError::InvalidPath
            | ConflictRepositoryError::InvalidStatus
            | ConflictRepositoryError::InvalidResolution => Self::invalid_request(),
            ConflictRepositoryError::NotFound => Self::not_found(),
            ConflictRepositoryError::AlreadyResolved => Self::new(
                StatusCode::CONFLICT,
                PublicErrorCode::Conflict,
                "Conflict is already resolved",
            ),
            ConflictRepositoryError::DatabaseOperationFailed => Self::internal(),
        }
    }

    fn from_auth_failure(error: AuthFailure) -> Self {
        match error {
            AuthFailure::MissingToken => Self::missing_token(),
            AuthFailure::InvalidToken => Self::invalid_token(),
            AuthFailure::Internal => Self::internal(),
        }
    }
}

impl From<ConflictsRouteError> for ApiError {
    fn from(error: ConflictsRouteError) -> Self {
        Self {
            status: StatusCode::from_u16(error.status_code())
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
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
#[path = "conflicts/tests.rs"]
mod tests;
