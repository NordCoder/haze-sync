//! Haze Sync HTTP routing entrypoint with explicit application admission.

use axum::{
    body::Body,
    extract::Extension,
    http::{Method, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use haze_sync_api::{
    contracts::errors::{ErrorResponse, PublicErrorCode},
    routes::admin::{
        DependencyReadinessState, PauseStatusSummary, ServerStatus, StatusSummaryResponse,
    },
};

use crate::{control_plane::AdmissionClass, readiness::ReadinessState, state::ServerAppState};

pub mod admin;
mod auth;
pub mod conflicts;
pub mod control_plane;
pub mod delete;
pub mod health;
pub mod v1;

pub fn build_router() -> Router {
    build_router_with_state(ServerAppState::dependency_free())
}

pub fn build_router_with_state(state: ServerAppState) -> Router {
    let readiness = state.readiness_state();
    build_router_with_state_and_readiness(state, readiness)
}

pub fn build_router_with_readiness(readiness: ReadinessState) -> Router {
    build_router_with_state_and_readiness(ServerAppState::dependency_free(), readiness)
}

fn build_router_with_state_and_readiness(
    state: ServerAppState,
    readiness: ReadinessState,
) -> Router {
    Router::new()
        .route("/health", get(health::health))
        .route("/ready", get(health::ready))
        .nest("/v1", build_v1_router())
        .layer(middleware::from_fn(operational_admission))
        .layer(Extension(readiness))
        .layer(Extension(state))
}

fn build_v1_router() -> Router {
    v1::router()
        .merge(conflicts::router())
        .merge(delete::router())
        .merge(admin::router())
        .merge(control_plane::router())
}

async fn operational_admission(
    Extension(state): Extension<ServerAppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let path = request.uri().path();
    let method = request.method();
    let class = admission_class(method, path);
    let Some(control_plane) = state.control_plane() else {
        return next.run(request).await;
    };
    if method == Method::GET && path == "/v1/admin/status" {
        let maintenance = control_plane.maintenance_state();
        let response = next.run(request).await;
        if response.status() == StatusCode::OK
            && maintenance != crate::control_plane::DurableMaintenanceState::Normal
        {
            let readiness = state.readiness_state().check().await;
            let dependency = |name: &str| {
                readiness
                    .components
                    .iter()
                    .find(|component| component.name == name)
                    .map_or(
                        DependencyReadinessState::Unknown,
                        |component| match component.status {
                            crate::readiness::ReadinessComponentStatus::Ready => {
                                DependencyReadinessState::Ready
                            }
                            crate::readiness::ReadinessComponentStatus::NotReady
                            | crate::readiness::ReadinessComponentStatus::Disabled => {
                                DependencyReadinessState::NotReady
                            }
                        },
                    )
            };
            return (
                StatusCode::OK,
                Json(StatusSummaryResponse::from_safe_parts(
                    ServerStatus::Maintenance,
                    dependency("database"),
                    dependency("object_store"),
                    None,
                    None,
                    PauseStatusSummary::supported(true),
                )),
            )
                .into_response();
        }
        return response;
    }
    match class {
        None | Some(AdmissionClass::ControlRead) => next.run(request).await,
        Some(AdmissionClass::AuthoritativeMutation) => {
            match control_plane
                .admission()
                .admit(AdmissionClass::AuthoritativeMutation)
            {
                Ok(Some(_permit)) => next.run(request).await,
                _ => maintenance_response(),
            }
        }
        Some(class) => match control_plane.admission().admit(class) {
            Ok(_) => next.run(request).await,
            Err(_) => maintenance_response(),
        },
    }
}

fn admission_class(method: &Method, path: &str) -> Option<AdmissionClass> {
    if path == "/health" || path == "/ready" || !path.starts_with("/v1") {
        return None;
    }
    if path.starts_with("/v1/admin/maintenance")
        || path.starts_with("/v1/admin/principals")
        || path.starts_with("/v1/admin/credentials")
        || path.starts_with("/v1/admin/operational-jobs")
        || (method == Method::GET && path.starts_with("/v1/admin/"))
    {
        return Some(AdmissionClass::ControlRead);
    }
    if method == Method::GET && path.starts_with("/v1/changes") {
        return Some(AdmissionClass::ChangeFeedRead);
    }
    if method == Method::PUT
        || method == Method::POST
        || method == Method::PATCH
        || method == Method::DELETE
    {
        return Some(AdmissionClass::AuthoritativeMutation);
    }
    Some(AdmissionClass::PublicRead)
}

fn maintenance_response() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ErrorResponse::new(
            PublicErrorCode::MaintenanceInProgress,
            "The requested operation is unavailable during maintenance",
        )),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use haze_sync_api::auth::{AdapterPrincipal, AdapterRole};
    use haze_sync_core::revision_service::compute_content_hash;
    use http_body_util::BodyExt as _;
    use serde_json::Value;
    use tower::ServiceExt as _;

    async fn request_json(method: &str, uri: &str, body: Body) -> (StatusCode, Value) {
        let request = Request::builder()
            .method(method)
            .uri(uri)
            .body(body)
            .expect("test request should build");
        let response = build_router()
            .oneshot(request)
            .await
            .expect("router should respond");
        let status = response.status();
        let body = response
            .into_body()
            .collect()
            .await
            .expect("response body should collect")
            .to_bytes();
        let json = serde_json::from_slice(&body).expect("response body should be JSON");
        (status, json)
    }

    async fn request_json_with_state(
        state: ServerAppState,
        method: &str,
        uri: &str,
        body: Body,
    ) -> (StatusCode, Value) {
        let request = Request::builder()
            .method(method)
            .uri(uri)
            .header("authorization", "Bearer route_test_token")
            .header("content-type", "application/json")
            .body(body)
            .expect("test request should build");
        request_json_with_custom_request(state, request).await
    }

    async fn request_json_with_custom_request(
        state: ServerAppState,
        request: Request<Body>,
    ) -> (StatusCode, Value) {
        let response = build_router_with_state(state)
            .oneshot(request)
            .await
            .expect("router should respond");
        let status = response.status();
        let body = response
            .into_body()
            .collect()
            .await
            .expect("response body should collect")
            .to_bytes();
        let json = serde_json::from_slice(&body).expect("response body should be JSON");
        (status, json)
    }

    fn static_principal_state() -> ServerAppState {
        let principal = AdapterPrincipal::new("obsidian-plugin", AdapterRole::ObsidianPlugin)
            .expect("fixture principal should be valid");
        ServerAppState::with_static_principal(principal)
    }

    fn admin_principal_state() -> ServerAppState {
        let principal = AdapterPrincipal::new("admin-cli", AdapterRole::Admin)
            .expect("fixture principal should be valid");
        ServerAppState::with_static_principal(principal)
    }

    fn non_resolver_state() -> ServerAppState {
        let principal = AdapterPrincipal::new("gdrive-adapter", AdapterRole::GdriveAdapter)
            .expect("fixture principal should be valid");
        ServerAppState::with_static_principal(principal)
    }

    fn assert_no_sensitive_route_output(json: &Value, extra_markers: &[&str]) {
        let rendered = json.to_string();
        for marker in [
            "route_test_token",
            "put-key-sensitive",
            "delete-key-sensitive",
            "sha256:test-token-hash",
            "token_hash",
            "oauth",
            "postgres://",
            "DATABASE_URL",
            "database_url",
            "object_store_root",
            "/srv/haze-sync",
            "/Users/",
            "C:\\",
            "sqlx::Error",
            "stack backtrace",
            "request-body-secret",
            "raw request body",
        ]
        .into_iter()
        .chain(extra_markers.iter().copied())
        {
            assert!(
                !rendered.contains(marker),
                "route output leaked forbidden marker {marker}: {rendered}"
            );
        }
    }

    #[test]
    fn router_builds_without_runtime_dependencies() {
        let _router = build_router();
    }

    #[tokio::test]
    async fn health_endpoint_returns_success() {
        let (status, json) = request_json("GET", "/health", Body::empty()).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["status"], "ok");
    }

    #[tokio::test]
    async fn ready_endpoint_returns_safe_not_ready_report_by_default() {
        let (status, json) = request_json("GET", "/ready", Body::empty()).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(json["status"], "not_ready");
        assert_eq!(json["components"][0]["name"], "database");
        assert!(!json.to_string().contains("postgres://"));
    }

    #[tokio::test]
    async fn server_info_returns_static_safe_json() {
        let (status, json) = request_json("GET", "/v1/server-info", Body::empty()).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["server_id"], "haze-sync-route-shell");
        assert_eq!(json["protocol_version"], 1);
    }

    #[tokio::test]
    async fn protected_file_write_requires_auth_without_runtime_state() {
        let (status, json) = request_json(
            "PUT",
            "/v1/files/Notes/a.md",
            Body::from("not persisted without runtime state"),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(json["error"]["code"], "missing_token");
    }

    #[tokio::test]
    async fn static_principal_put_error_redacts_headers_body_and_runtime_details() {
        let body = b"raw request body request-body-secret postgres://db.example/app \
            /srv/haze-sync/objects token_hash sha256:test-token-hash \
            sqlx::Error stack backtrace";
        let hash = compute_content_hash(body).to_string();
        let request = Request::builder()
            .method("PUT")
            .uri("/v1/files/Notes/a.md")
            .header("authorization", "Bearer route_test_token")
            .header("content-type", "application/octet-stream")
            .header("Idempotency-Key", "put-key-sensitive")
            .header("X-Content-SHA256", hash)
            .header("X-Base-Revision-Id", "rev_current")
            .body(Body::from(body.to_vec()))
            .expect("test request should build");
        let (status, json) =
            request_json_with_custom_request(static_principal_state(), request).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_no_sensitive_route_output(&json, &["rev_current"]);
    }

    #[tokio::test]
    async fn protected_changes_route_requires_auth_without_runtime_state() {
        let (status, json) =
            request_json("GET", "/v1/changes?since=0&limit=10", Body::empty()).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(json["error"]["code"], "missing_token");
    }

    #[tokio::test]
    async fn delete_file_route_requires_auth_without_runtime_state() {
        let (status, json) = request_json("DELETE", "/v1/files/Notes/a.md", Body::empty()).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(json["error"]["code"], "missing_token");
    }

    #[tokio::test]
    async fn admin_status_route_returns_safe_placeholder_without_runtime_state() {
        let state = admin_principal_state();
        let (status, json) =
            request_json_with_state(state, "GET", "/v1/admin/status", Body::empty()).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["server_status"], "not_ready");
        assert_no_sensitive_route_output(&json, &[]);
    }

    #[tokio::test]
    async fn non_resolver_cannot_resolve_conflict() {
        let state = non_resolver_state();
        let (status, json) = request_json_with_state(
            state,
            "POST",
            "/v1/conflicts/conflict-a/resolve",
            Body::from(r#"{"resolution":"mark_resolved"}"#),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(json["error"]["code"], "forbidden_role");
    }
}
