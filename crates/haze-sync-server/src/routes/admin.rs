//! Minimal safe admin/status route fan-in.
//!
//! The handlers in this module expose read-only operational summaries built from
//! already-sanitized W3-P6 DTOs. They do not mutate adapter state, pause/resume
//! runtime work, call providers, or reveal sensitive runtime details.

use axum::{
    extract::Extension,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use haze_sync_api::{
    auth::AdapterPrincipal,
    contracts::errors::{ErrorResponse, PublicErrorCode},
    dto::primitives::TimestampDto,
    routes::admin::{
        AdapterCursorSummary, AdapterListResponse, AdapterSummary, DependencyReadinessState,
        PauseStatusSummary, ServerStatus, StatusSummaryResponse,
    },
};
use haze_sync_common::{AdapterId, AdapterRole as CommonAdapterRole};
use sqlx::{PgPool, Row};
use std::str::FromStr;

use crate::{
    readiness::{ReadinessComponentStatus, ReadinessReport},
    routes::auth::{authenticate_principal, AuthFailure},
    state::ServerAppState,
};

pub fn router() -> axum::Router {
    axum::Router::new()
        .route("/admin/status", axum::routing::get(status_route))
        .route("/admin/adapters", axum::routing::get(adapters_route))
}

/// Handles GET /v1/admin/status.
pub(super) async fn status_route(
    Extension(state): Extension<ServerAppState>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    authenticate_admin(&state, &headers).await?;

    let response = status_from_state(&state).await?;

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// Handles GET /v1/admin/adapters.
pub(super) async fn adapters_route(
    Extension(state): Extension<ServerAppState>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    authenticate_admin(&state, &headers).await?;

    let response = if let Some(pool) = state.db_pool() {
        adapters_from_runtime_state(pool).await?
    } else {
        AdapterListResponse::empty()
    };

    Ok((StatusCode::OK, Json(response)).into_response())
}

async fn status_from_state(state: &ServerAppState) -> Result<StatusSummaryResponse, ApiError> {
    let readiness = state.readiness_state().check().await;
    let (last_operation_sequence, adapter_count) = if let Some(pool) = state.db_pool() {
        (
            query_optional_i64(pool, "select max(seq) from operation_log").await?,
            Some(query_count(pool, "select count(*) from sync_adapters").await?),
        )
    } else {
        (None, None)
    };

    Ok(StatusSummaryResponse::from_safe_parts(
        server_status_from_readiness(&readiness),
        dependency_state_from_readiness(&readiness, "database"),
        dependency_state_from_readiness(&readiness, "object_store"),
        last_operation_sequence,
        adapter_count,
        PauseStatusSummary::unsupported(),
    ))
}

async fn adapters_from_runtime_state(pool: &PgPool) -> Result<AdapterListResponse, ApiError> {
    let rows = sqlx::query(
        "select a.adapter_id, a.role, a.enabled, a.last_seen_at, \
                c.last_core_seq, c.last_success_at, \
                c.external_cursor_json is not null as has_external_cursor \
         from sync_adapters a \
         left join adapter_cursors c on c.adapter_id = a.adapter_id \
         order by a.adapter_id",
    )
    .fetch_all(pool)
    .await
    .map_err(|_error| ApiError::internal())?;

    let mut adapters = Vec::with_capacity(rows.len());
    for row in rows {
        let adapter_id: String = row
            .try_get("adapter_id")
            .map_err(|_error| ApiError::internal())?;
        let role: String = row.try_get("role").map_err(|_error| ApiError::internal())?;
        let enabled: bool = row
            .try_get("enabled")
            .map_err(|_error| ApiError::internal())?;
        let last_seen_at: Option<DateTime<Utc>> = row
            .try_get("last_seen_at")
            .map_err(|_error| ApiError::internal())?;
        let last_core_seq: Option<i64> = row
            .try_get("last_core_seq")
            .map_err(|_error| ApiError::internal())?;
        let last_success_at: Option<DateTime<Utc>> = row
            .try_get("last_success_at")
            .map_err(|_error| ApiError::internal())?;
        let has_external_cursor: bool = row
            .try_get("has_external_cursor")
            .map_err(|_error| ApiError::internal())?;

        let adapter_id = AdapterId::parse(&adapter_id).map_err(|_error| ApiError::internal())?;
        let role = CommonAdapterRole::from_str(&role).ok();
        let cursor = AdapterCursorSummary::new(
            last_core_seq,
            last_success_at.map(timestamp_dto),
            has_external_cursor,
        );
        let mut adapter = AdapterSummary::new(adapter_id, role, enabled).with_cursor(cursor);
        if let Some(last_seen_at) = last_seen_at {
            adapter = adapter.with_last_seen_at(timestamp_dto(last_seen_at));
        }
        adapters.push(adapter);
    }

    Ok(AdapterListResponse::new(adapters))
}

fn server_status_from_readiness(readiness: &ReadinessReport) -> ServerStatus {
    if readiness.is_ready() {
        return ServerStatus::Ready;
    }

    if readiness
        .components
        .iter()
        .any(|component| matches!(component.status, ReadinessComponentStatus::Ready))
    {
        ServerStatus::Degraded
    } else {
        ServerStatus::NotReady
    }
}

fn dependency_state_from_readiness(
    readiness: &ReadinessReport,
    component_name: &str,
) -> DependencyReadinessState {
    let Some(component) = readiness
        .components
        .iter()
        .find(|component| component.name == component_name)
    else {
        return DependencyReadinessState::Unknown;
    };

    match component.status {
        ReadinessComponentStatus::Ready => DependencyReadinessState::Ready,
        ReadinessComponentStatus::NotReady | ReadinessComponentStatus::Disabled => {
            DependencyReadinessState::NotReady
        }
    }
}

async fn query_optional_i64(pool: &PgPool, query: &str) -> Result<Option<i64>, ApiError> {
    sqlx::query_scalar(query)
        .fetch_one(pool)
        .await
        .map_err(|_error| ApiError::internal())
}

async fn query_count(pool: &PgPool, query: &str) -> Result<u64, ApiError> {
    let count: i64 = sqlx::query_scalar(query)
        .fetch_one(pool)
        .await
        .map_err(|_error| ApiError::internal())?;
    u64::try_from(count).map_err(|_error| ApiError::internal())
}

async fn authenticate_admin(
    state: &ServerAppState,
    headers: &HeaderMap,
) -> Result<AdapterPrincipal, ApiError> {
    let principal = authenticate_principal(state, headers)
        .await
        .map_err(ApiError::from_auth_failure)?;

    if !principal.role().can_admin() {
        return Err(ApiError::forbidden_role());
    }

    Ok(principal)
}

fn timestamp_dto(timestamp: DateTime<Utc>) -> TimestampDto {
    TimestampDto::from(timestamp.format("%Y-%m-%dT%H:%M:%SZ").to_string())
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

    fn internal() -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            PublicErrorCode::InternalError,
            "Admin status request failed",
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

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}
#[cfg(test)]
mod tests;
