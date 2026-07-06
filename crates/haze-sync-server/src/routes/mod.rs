//! Haze Sync HTTP routing entrypoint with explicit W2/W3 route composition.
//!
//! Health remains dependency-free. Readiness and file/change routes use explicit
//! caller-owned state when supplied; no hidden global runtime state is created
//! by router construction.

use axum::{extract::Extension, routing::get, Router};

use crate::{readiness::ReadinessState, state::ServerAppState};

pub mod admin;
mod auth;
pub mod conflicts;
pub mod delete;
pub mod health;
pub mod v1;

/// Builds the Haze Sync router with safe dependency-free defaults.
///
/// Route surfaces are registered even without runtime dependencies, but handlers
/// fall back to sanitized auth/storage/not-implemented responses rather than
/// mutating anything.
pub fn build_router() -> Router {
    build_router_with_state(ServerAppState::dependency_free())
}

/// Builds the Haze Sync router with caller-supplied runtime state.
pub fn build_router_with_state(state: ServerAppState) -> Router {
    let readiness = state.readiness_state();
    build_router_with_state_and_readiness(state, readiness)
}

/// Builds the router with caller-supplied readiness checks and dependency-free
/// Core state. This preserves the W1/W2-P8 shell integration point.
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
        .layer(Extension(readiness))
        .layer(Extension(state))
}

fn build_v1_router() -> Router {
    v1::router()
        .merge(conflicts::router())
        .merge(delete::router())
        .merge(admin::router())
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
    use sqlx::postgres::PgPoolOptions;
    use std::time::Duration;
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

    async fn request_status_with_state(
        state: ServerAppState,
        method: &str,
        uri: &str,
        body: Body,
    ) -> StatusCode {
        let request = Request::builder()
            .method(method)
            .uri(uri)
            .header("authorization", "Bearer route_test_token")
            .header("content-type", "application/json")
            .body(body)
            .expect("test request should build");
        build_router_with_state(state)
            .oneshot(request)
            .await
            .expect("router should respond")
            .status()
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

    fn database_auth_state() -> ServerAppState {
        let pool = PgPoolOptions::new()
            .acquire_timeout(Duration::from_millis(50))
            .connect_lazy("postgres://haze_sync:placeholder@127.0.0.1:1/haze_sync_test")
            .expect("lazy pool should build");
        ServerAppState::new(None, None, None, crate::state::AuthState::Database { pool })
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
        assert_eq!(json["components"][0]["status"], "disabled");
        assert_eq!(json["components"][1]["name"], "object_store");
        assert_eq!(json["components"][1]["status"], "disabled");
        assert!(!json.to_string().contains("postgres://"));
        assert!(!json.to_string().contains("secret"));
        assert!(!json.to_string().contains("/srv/"));
        assert!(!json.to_string().contains("stack"));
    }

    #[tokio::test]
    async fn server_info_returns_static_safe_json() {
        let (status, json) = request_json("GET", "/v1/server-info", Body::empty()).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["server_id"], "haze-sync-route-shell");
        assert_eq!(json["protocol_version"], 1);
        assert_eq!(json["max_upload_bytes"], 52_428_800);
        assert_eq!(json["capabilities"][0], "sha256");
        assert!(!json.to_string().contains("secret"));
        assert!(!json.to_string().contains("postgres"));
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
        assert!(!json.to_string().contains("Idempotency-Key"));
        assert!(!json.to_string().contains("not persisted"));
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
        assert_eq!(json["error"]["code"], "internal_error");
        assert_no_sensitive_route_output(&json, &["rev_current"]);
    }

    #[tokio::test]
    async fn database_auth_failure_is_sanitized_for_protected_file_route() {
        let request = Request::builder()
            .method("GET")
            .uri("/v1/files/Notes/a.md")
            .header("authorization", "Bearer route_test_token")
            .body(Body::empty())
            .expect("test request should build");

        let (status, json) = request_json_with_custom_request(database_auth_state(), request).await;

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(json["error"]["code"], "internal_error");
        assert_no_sensitive_route_output(&json, &["127.0.0.1", "placeholder", "haze_sync_test"]);
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
        assert!(!json.to_string().contains("Notes/a.md"));
        assert!(!json.to_string().contains("Bearer"));
    }

    #[tokio::test]
    async fn delete_file_route_requires_idempotency_key_before_storage_mutation() {
        let state = static_principal_state();
        let (status, json) =
            request_json_with_state(state, "DELETE", "/v1/files/Notes/a.md", Body::empty()).await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(json["error"]["code"], "invalid_request");
        assert!(json.to_string().contains("Idempotency-Key"));
        assert!(!json.to_string().contains("route_test_token"));
        assert!(!json.to_string().contains("postgres://"));
        assert!(!json.to_string().contains("/srv/"));
    }

    #[tokio::test]
    async fn admin_status_route_returns_safe_placeholder_without_runtime_state() {
        let state = admin_principal_state();
        let (status, json) =
            request_json_with_state(state, "GET", "/v1/admin/status", Body::empty()).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["server_status"], "not_ready");
        assert_eq!(json["adapter_count"], Value::Null);
        assert_eq!(json["pause"]["supported"], false);
        assert!(!json.to_string().contains("route_test_token"));
        assert!(!json.to_string().contains("postgres://"));
        assert!(!json.to_string().contains("/srv/"));
        assert!(!json.to_string().contains("token_hash"));
        assert!(!json.to_string().contains("oauth"));
    }

    #[tokio::test]
    async fn adapters_list_route_returns_safe_empty_placeholder_without_runtime_state() {
        let state = admin_principal_state();
        let (status, json) =
            request_json_with_state(state, "GET", "/v1/admin/adapters", Body::empty()).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["total_count"], 0);
        assert_eq!(json["adapters"], serde_json::json!([]));
        assert!(!json.to_string().contains("route_test_token"));
        assert!(!json.to_string().contains("token_hash"));
        assert!(!json.to_string().contains("external_cursor_json"));
        assert!(!json.to_string().contains("/srv/"));
    }

    #[tokio::test]
    async fn admin_routes_reject_non_admin_roles_safely() {
        let state = static_principal_state();
        let (status, json) =
            request_json_with_state(state, "GET", "/v1/admin/status", Body::empty()).await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(json["error"]["code"], "forbidden_role");
        assert!(!json.to_string().contains("route_test_token"));
    }

    #[tokio::test]
    async fn unsupported_admin_mutation_remains_unavailable() {
        let state = admin_principal_state();
        let status = request_status_with_state(
            state,
            "POST",
            "/v1/admin/pause",
            Body::from("{\"active\":true}"),
        )
        .await;

        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn conflict_list_route_is_wired_without_storage_mutation() {
        let state = static_principal_state();
        let (status, json) =
            request_json_with_state(state, "GET", "/v1/conflicts?status=open", Body::empty()).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["conflicts"], serde_json::json!([]));
        assert!(!json.to_string().contains("postgres://"));
        assert!(!json.to_string().contains("secret"));
        assert!(!json.to_string().contains("/srv/"));
    }

    #[tokio::test]
    async fn conflict_routes_require_auth_without_runtime_state() {
        let (status, json) = request_json("GET", "/v1/conflicts?status=open", Body::empty()).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(json["error"]["code"], "missing_token");
    }

    #[tokio::test]
    async fn conflict_resolution_rejects_non_resolver_roles_safely() {
        let state = non_resolver_state();
        let body = serde_json::json!({ "resolution": "accept_current" }).to_string();
        let (status, json) = request_json_with_state(
            state,
            "POST",
            "/v1/conflicts/conf_01J/resolve",
            Body::from(body),
        )
        .await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(json["error"]["code"], "forbidden_role");
        assert!(!json.to_string().contains("route_test_token"));
    }

    #[tokio::test]
    async fn conflict_routes_use_database_auth_lookup_path_when_configured() {
        let state = database_auth_state();
        let (status, json) =
            request_json_with_state(state, "GET", "/v1/conflicts?status=open", Body::empty()).await;

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(json["error"]["code"], "internal_error");
        assert!(!json.to_string().contains("route_test_token"));
        assert!(!json.to_string().contains("postgres://"));
    }

    #[tokio::test]
    async fn unsupported_conflict_resolution_action_returns_safe_bad_request() {
        let state = static_principal_state();
        let body = serde_json::json!({ "resolution": "overwrite" }).to_string();
        let (status, json) = request_json_with_state(
            state,
            "POST",
            "/v1/conflicts/conf_01J/resolve",
            Body::from(body),
        )
        .await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(json["error"]["code"], "validation_error");
        assert!(!json.to_string().contains("overwrite"));
        assert!(!json.to_string().contains("postgres://"));
        assert!(!json.to_string().contains("secret"));
    }
}
