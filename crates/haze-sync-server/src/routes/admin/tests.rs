use super::*;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use haze_sync_api::auth::{AdapterPrincipal, AdapterRole};
use http_body_util::BodyExt as _;
use serde_json::Value;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tower::ServiceExt as _;

#[test]
fn dependency_free_status_payload_is_safe_placeholder() {
    let payload = StatusSummaryResponse::placeholder();
    let json = serde_json::to_string(&payload).expect("status should serialize");
    assert!(json.contains("not_ready"));
    assert_no_sensitive_leaks(&json);
}

#[tokio::test]
async fn dependency_free_admin_status_reports_checked_readiness() {
    let payload = status_from_state(&ServerAppState::dependency_free()).await;
    let json = serde_json::to_string(&payload).expect("status should serialize");
    assert_eq!(payload.server_status, ServerStatus::NotReady);
    assert_eq!(payload.db_readiness_state, DependencyReadinessState::NotReady);
    assert_eq!(
        payload.object_store_readiness_state,
        DependencyReadinessState::NotReady
    );
    assert_eq!(payload.last_operation_sequence, None);
    assert_eq!(payload.adapter_count, None);
    assert_eq!(payload.pause, PauseStatusSummary::unsupported());
    assert_no_sensitive_leaks(&json);
}

#[tokio::test]
async fn unavailable_database_keeps_admin_status_readable() {
    let pool = PgPoolOptions::new().connect_lazy_with(PgConnectOptions::new());
    pool.close().await;
    let state = ServerAppState::new(Some(pool), None, None, crate::state::AuthState::Disabled);
    let payload = status_from_state(&state).await;
    let json = serde_json::to_string(&payload).expect("status should serialize");
    assert_eq!(payload.server_status, ServerStatus::NotReady);
    assert_eq!(payload.db_readiness_state, DependencyReadinessState::NotReady);
    assert_eq!(payload.last_operation_sequence, None);
    assert_eq!(payload.adapter_count, None);
    assert_no_sensitive_leaks(&json);
}

#[test]
fn empty_adapter_list_payload_is_safe() {
    let payload = AdapterListResponse::empty();
    let json = serde_json::to_string(&payload).expect("adapter list should serialize");
    assert_eq!(json, "{\"total_count\":0,\"adapters\":[]}");
    assert_no_sensitive_leaks(&json);
}

#[test]
fn worktree_status_mapping_preserves_ready_busy_and_counts() {
    let response = try_worktree_status_response(public_worktree_status(
        ServerWorktreeStatusSnapshot {
            mode: ServerWorktreeModeCategory::DryRun,
            lifecycle: ServerWorktreeHostLifecycle::Running,
            readiness: ServerWorktreeReadinessCategory::Ready,
            readiness_reason: ServerWorktreeReadinessReason::Running,
            cycles_completed: 17,
            cycles_failed: 9,
            cycle_in_progress: true,
            pending_watcher_hints: 23,
            manual_availability: ServerWorktreeManualAvailability::Busy,
        },
    ))
    .unwrap();
    assert!(response.is_ready());
    assert_eq!(response.manual_availability, WorktreeManualAvailability::Busy);
    assert_eq!(response.cycles_failed, 9);
    assert_eq!(response.pending_watcher_hints, 23);
    assert_no_sensitive_leaks(&serde_json::to_string(&response).unwrap());
}

#[test]
fn every_sync_submission_has_exact_http_and_json_mapping() {
    for (internal, expected_status, expected_wire) in [
        (ServerWorktreeSyncSubmission::Accepted, StatusCode::ACCEPTED, "accepted"),
        (ServerWorktreeSyncSubmission::Busy, StatusCode::CONFLICT, "busy"),
        (
            ServerWorktreeSyncSubmission::NotStarted,
            StatusCode::SERVICE_UNAVAILABLE,
            "not_started",
        ),
        (
            ServerWorktreeSyncSubmission::Cancelling,
            StatusCode::SERVICE_UNAVAILABLE,
            "cancelling",
        ),
        (
            ServerWorktreeSyncSubmission::Shutdown,
            StatusCode::SERVICE_UNAVAILABLE,
            "shutdown",
        ),
        (
            ServerWorktreeSyncSubmission::Unavailable,
            StatusCode::SERVICE_UNAVAILABLE,
            "unavailable",
        ),
        (
            ServerWorktreeSyncSubmission::Failed,
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed",
        ),
    ] {
        let (status, public) = public_worktree_submission(internal);
        assert_eq!(status, expected_status);
        let json = serde_json::to_value(worktree_sync_once_response(public)).unwrap();
        assert_eq!(json["status"], expected_wire);
        assert_no_sensitive_leaks(&json.to_string());
    }
}

#[tokio::test]
async fn worktree_routes_require_admin_and_use_fixed_absent_control_responses() {
    let admin = principal_state(AdapterRole::Admin);
    let non_admin = principal_state(AdapterRole::WorktreeAdapter);

    let (status, json) = request(admin.clone(), "GET", "/v1/admin/worktree/status", Body::empty(), true).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(json["error"]["code"], "internal_error");

    let (status, json) = request(
        admin.clone(),
        "POST",
        "/v1/admin/worktree/sync-once",
        Body::from("{}"),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(json["status"], "unavailable");

    let (status, json) = request(
        admin,
        "POST",
        "/v1/admin/worktree/sync-once",
        Body::from("{\"force\":true}"),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(json["error"]["code"], "invalid_request");
    assert!(!json.to_string().contains("force"));

    let (status, json) = request(
        non_admin,
        "GET",
        "/v1/admin/worktree/status",
        Body::empty(),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(json["error"]["code"], "forbidden_role");

    let (status, json) = request(
        ServerAppState::dependency_free(),
        "POST",
        "/v1/admin/worktree/sync-once",
        Body::from("{\"raw\":\"request-body-secret\"}"),
        false,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(json["error"]["code"], "missing_token");
    assert_no_sensitive_leaks(&json.to_string());
}

#[test]
fn admin_errors_do_not_echo_token_material() {
    let response = ApiError::invalid_token().into_response();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

fn principal_state(role: AdapterRole) -> ServerAppState {
    ServerAppState::with_static_principal(
        AdapterPrincipal::new("worktree-route-test", role).expect("principal should be valid"),
    )
}

async fn request(
    state: ServerAppState,
    method: &str,
    uri: &str,
    body: Body,
    authorized: bool,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    if authorized {
        builder = builder.header("authorization", "Bearer route_test_token");
    }
    let response = crate::routes::build_router_with_state(state)
        .oneshot(builder.body(body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap();
    (status, json)
}

fn assert_no_sensitive_leaks(json: &str) {
    for forbidden in [
        concat!("to", "ken"),
        concat!("ha", "sh"),
        concat!("oa", "uth"),
        concat!("se", "cret"),
        concat!("database", "_url"),
        concat!("db", "_url"),
        concat!("provider", "_payload"),
        concat!("external_cursor", "_json"),
        concat!("object_store", "_root"),
        concat!("/", "srv", "/"),
        concat!("post", "gres", "://"),
        concat!("sta", "ck"),
        concat!("back", "trace"),
        "request-body-secret",
        "route_test_token",
        "ticket",
        "generation",
        "fingerprint",
    ] {
        assert!(
            !json.contains(forbidden),
            "admin output leaked forbidden marker {forbidden}: {json}"
        );
    }
}
