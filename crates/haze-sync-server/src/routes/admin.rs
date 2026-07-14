//! Administrative status and Worktree control routes.

use axum::{
    body::Bytes,
    extract::Extension,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use haze_sync_api::{
    auth::AdapterPrincipal,
    contracts::errors::{ErrorResponse, PublicErrorCode},
    dto::{
        primitives::TimestampDto,
        worktree::{
            WorktreeConfiguredMode, WorktreeHostLifecycle, WorktreeManualAvailability,
            WorktreeReadiness, WorktreeReadinessReason, WorktreeStatusPlatformParts,
            WorktreeSyncOnceRequest, WorktreeSyncOnceSubmissionStatus,
        },
    },
    routes::{
        admin::{
            AdapterCursorSummary, AdapterListResponse, AdapterSummary, DependencyReadinessState,
            PauseStatusSummary, ServerStatus, StatusSummaryResponse,
        },
        worktree::{
            try_worktree_status_response, worktree_sync_once_response,
            WorktreeAdminAuthRequirement,
        },
    },
};
use haze_sync_common::{AdapterId, AdapterRole as CommonAdapterRole};
use sqlx::{PgPool, Row};
use std::str::FromStr;

use crate::{
    readiness::{ReadinessComponentStatus, ReadinessReport},
    routes::auth::{authenticate_principal, AuthFailure},
    state::ServerAppState,
    worktree_http::ServerWorktreeSyncSubmission,
    worktree_status::{
        ServerWorktreeHostLifecycle, ServerWorktreeManualAvailability,
        ServerWorktreeModeCategory, ServerWorktreeReadinessCategory,
        ServerWorktreeReadinessReason, ServerWorktreeStatusSnapshot,
    },
};

pub fn router() -> axum::Router {
    axum::Router::new()
        .route("/admin/status", axum::routing::get(status_route))
        .route("/admin/adapters", axum::routing::get(adapters_route))
        .route(
            "/admin/worktree/status",
            axum::routing::get(worktree_status_route),
        )
        .route(
            "/admin/worktree/sync-once",
            axum::routing::post(worktree_sync_once_route),
        )
}

pub(super) async fn status_route(
    Extension(state): Extension<ServerAppState>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    authenticate_admin(&state, &headers).await?;
    Ok((StatusCode::OK, Json(status_from_state(&state).await)).into_response())
}

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

pub(super) async fn worktree_status_route(
    Extension(state): Extension<ServerAppState>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    authenticate_worktree_admin(&state, &headers).await?;
    let snapshot = state
        .worktree_control()
        .and_then(|control| control.snapshot())
        .ok_or_else(ApiError::worktree_unavailable)?;
    let response = try_worktree_status_response(public_worktree_status(snapshot))
        .map_err(|_error| ApiError::worktree_status_failed())?;
    Ok((StatusCode::OK, Json(response)).into_response())
}

pub(super) async fn worktree_sync_once_route(
    Extension(state): Extension<ServerAppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, ApiError> {
    authenticate_worktree_admin(&state, &headers).await?;
    let _request = serde_json::from_slice::<WorktreeSyncOnceRequest>(&body)
        .map_err(|_error| ApiError::invalid_worktree_request())?;

    let submission = state
        .worktree_control()
        .map_or(ServerWorktreeSyncSubmission::Unavailable, |control| {
            control.submit_sync_once()
        });
    let (status, public_status) = public_worktree_submission(submission);
    Ok((status, Json(worktree_sync_once_response(public_status))).into_response())
}

fn public_worktree_status(snapshot: ServerWorktreeStatusSnapshot) -> WorktreeStatusPlatformParts {
    WorktreeStatusPlatformParts {
        configured_mode: match snapshot.mode {
            ServerWorktreeModeCategory::Disabled => WorktreeConfiguredMode::Disabled,
            ServerWorktreeModeCategory::ReadOnly => WorktreeConfiguredMode::ReadOnly,
            ServerWorktreeModeCategory::ImportOnly => WorktreeConfiguredMode::ImportOnly,
            ServerWorktreeModeCategory::ExportOnly => WorktreeConfiguredMode::ExportOnly,
            ServerWorktreeModeCategory::Bidirectional => WorktreeConfiguredMode::Bidirectional,
            ServerWorktreeModeCategory::DryRun => WorktreeConfiguredMode::DryRun,
        },
        host_lifecycle: match snapshot.lifecycle {
            ServerWorktreeHostLifecycle::Disabled => WorktreeHostLifecycle::Disabled,
            ServerWorktreeHostLifecycle::Starting => WorktreeHostLifecycle::Starting,
            ServerWorktreeHostLifecycle::Running => WorktreeHostLifecycle::Running,
            ServerWorktreeHostLifecycle::Cancelling => WorktreeHostLifecycle::Cancelling,
            ServerWorktreeHostLifecycle::Shutdown => WorktreeHostLifecycle::Shutdown,
            ServerWorktreeHostLifecycle::Failed => WorktreeHostLifecycle::Failed,
        },
        readiness: match snapshot.readiness {
            ServerWorktreeReadinessCategory::Ready => WorktreeReadiness::Ready,
            ServerWorktreeReadinessCategory::NotReady => WorktreeReadiness::NotReady,
        },
        readiness_reason: match snapshot.readiness_reason {
            ServerWorktreeReadinessReason::DisabledInert => {
                WorktreeReadinessReason::DisabledInert
            }
            ServerWorktreeReadinessReason::Running => WorktreeReadinessReason::Running,
            ServerWorktreeReadinessReason::Starting => WorktreeReadinessReason::Starting,
            ServerWorktreeReadinessReason::Cancelling => WorktreeReadinessReason::Cancelling,
            ServerWorktreeReadinessReason::Shutdown => WorktreeReadinessReason::Shutdown,
            ServerWorktreeReadinessReason::Failed => WorktreeReadinessReason::Failed,
        },
        cycles_completed: snapshot.cycles_completed,
        cycles_failed: snapshot.cycles_failed,
        cycle_in_progress: snapshot.cycle_in_progress,
        pending_watcher_hints: snapshot.pending_watcher_hints,
        manual_availability: match snapshot.manual_availability {
            ServerWorktreeManualAvailability::Available => WorktreeManualAvailability::Available,
            ServerWorktreeManualAvailability::Busy => WorktreeManualAvailability::Busy,
            ServerWorktreeManualAvailability::NotStarted => {
                WorktreeManualAvailability::NotStarted
            }
            ServerWorktreeManualAvailability::Cancelling => {
                WorktreeManualAvailability::Cancelling
            }
            ServerWorktreeManualAvailability::Shutdown => WorktreeManualAvailability::Shutdown,
            ServerWorktreeManualAvailability::Unavailable => {
                WorktreeManualAvailability::Unavailable
            }
            ServerWorktreeManualAvailability::Failed => WorktreeManualAvailability::Failed,
        },
    }
}

fn public_worktree_submission(
    submission: ServerWorktreeSyncSubmission,
) -> (StatusCode, WorktreeSyncOnceSubmissionStatus) {
    match submission {
        ServerWorktreeSyncSubmission::Accepted => (
            StatusCode::ACCEPTED,
            WorktreeSyncOnceSubmissionStatus::Accepted,
        ),
        ServerWorktreeSyncSubmission::Busy => (
            StatusCode::CONFLICT,
            WorktreeSyncOnceSubmissionStatus::Busy,
        ),
        ServerWorktreeSyncSubmission::NotStarted => (
            StatusCode::SERVICE_UNAVAILABLE,
            WorktreeSyncOnceSubmissionStatus::NotStarted,
        ),
        ServerWorktreeSyncSubmission::Cancelling => (
            StatusCode::SERVICE_UNAVAILABLE,
            WorktreeSyncOnceSubmissionStatus::Cancelling,
        ),
        ServerWorktreeSyncSubmission::Shutdown => (
            StatusCode::SERVICE_UNAVAILABLE,
            WorktreeSyncOnceSubmissionStatus::Shutdown,
        ),
        ServerWorktreeSyncSubmission::Unavailable => (
            StatusCode::SERVICE_UNAVAILABLE,
            WorktreeSyncOnceSubmissionStatus::Unavailable,
        ),
        ServerWorktreeSyncSubmission::Failed => (
            StatusCode::INTERNAL_SERVER_ERROR,
            WorktreeSyncOnceSubmissionStatus::Failed,
        ),
    }
}

async fn status_from_state(state: &ServerAppState) -> StatusSummaryResponse {
    let readiness = state.readiness_state().check().await;
    let database_state = dependency_state_from_readiness(&readiness, "database");
    let object_store_state = dependency_state_from_readiness(&readiness, "object_store");
    let (last_operation_sequence, adapter_count) =
        best_effort_runtime_metadata(state, database_state).await;

    StatusSummaryResponse::from_safe_parts(
        server_status_from_readiness(&readiness),
        database_state,
        object_store_state,
        last_operation_sequence,
        adapter_count,
        PauseStatusSummary::unsupported(),
    )
}

async fn best_effort_runtime_metadata(
    state: &ServerAppState,
    database_state: DependencyReadinessState,
) -> (Option<i64>, Option<u64>) {
    if database_state != DependencyReadinessState::Ready {
        return (None, None);
    }
    let Some(pool) = state.db_pool() else {
        return (None, None);
    };
    let last_operation_sequence = query_optional_i64(pool, "select max(seq) from operation_log")
        .await
        .ok()
        .flatten();
    let adapter_count = query_count(pool, "select count(*) from sync_adapters")
        .await
        .ok();
    (last_operation_sequence, adapter_count)
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

async fn authenticate_worktree_admin(
    state: &ServerAppState,
    headers: &HeaderMap,
) -> Result<AdapterPrincipal, ApiError> {
    let principal = authenticate_principal(state, headers)
        .await
        .map_err(ApiError::from_auth_failure)?;
    WorktreeAdminAuthRequirement
        .validate_principal(&principal)
        .map_err(|_error| ApiError::forbidden_role())?;
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

    fn invalid_worktree_request() -> Self {
        Self::new(
            StatusCode::BAD_REQUEST,
            PublicErrorCode::InvalidRequest,
            "Worktree sync request must be an empty JSON object",
        )
    }

    fn worktree_unavailable() -> Self {
        Self::new(
            StatusCode::SERVICE_UNAVAILABLE,
            PublicErrorCode::InternalError,
            "Worktree runtime control is unavailable",
        )
    }

    fn worktree_status_failed() -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            PublicErrorCode::InternalError,
            "Worktree status response could not be constructed",
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
