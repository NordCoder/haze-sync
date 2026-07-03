//! Haze Sync HTTP route shell and W2 Core file-operation fan-in.
//!
//! Health remains dependency-free. Readiness and Core file/change routes use
//! explicit caller-owned state when supplied; no hidden global runtime state is
//! created by router construction.

use axum::{
    body::{to_bytes, Body},
    extract::{Extension, Path, Query},
    http::{HeaderMap, Method, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use haze_sync_api::contracts::errors::{ErrorResponse, PublicErrorCode};
use serde_json::Value;
use std::collections::HashMap;

use crate::{readiness::ReadinessState, state::ServerAppState};

pub mod conflicts;
pub mod health;
#[cfg_attr(not(test), allow(unused_imports))]
pub mod v1;

/// Builds the Haze Sync router with safe not-ready dependency defaults.
///
/// Protected W2 Core routes are registered, but without runtime state they
/// return sanitized auth/storage errors rather than mutating anything.
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
        .nest("/v1", v1::router())
        .fallback(conflict_fallback)
        .layer(middleware::from_fn(conflict_route_intercept))
        .layer(Extension(readiness))
        .layer(Extension(state))
}

async fn conflict_route_intercept(request: Request<Body>, next: Next) -> Response {
    let Some(state) = request.extensions().get::<ServerAppState>().cloned() else {
        return next.run(request).await;
    };

    if request.method() == Method::GET && request.uri().path() == "/v1/conflicts" {
        let headers = request.headers().clone();
        let query = query_map(request.uri().query());
        return conflict_response(
            conflicts::list_conflicts_route(Extension(state), Query(query), headers).await,
        );
    }

    next.run(request).await
}

async fn conflict_fallback(request: Request<Body>) -> Response {
    let Some(state) = request.extensions().get::<ServerAppState>().cloned() else {
        return StatusCode::NOT_FOUND.into_response();
    };

    if request.method() != Method::POST {
        return StatusCode::NOT_FOUND.into_response();
    }

    let Some(conflict_id) = conflict_resolve_id(request.uri().path()) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let headers = request.headers().clone();
    let body = request.into_body();
    let bytes = match to_bytes(body, 16_384).await {
        Ok(bytes) => bytes,
        Err(_error) => return invalid_conflict_json_response(),
    };
    let value = match serde_json::from_slice::<Value>(&bytes) {
        Ok(value) => value,
        Err(_error) => Value::Null,
    };

    conflict_response(
        conflicts::resolve_conflict_route(
            Extension(state),
            Path(conflict_id),
            headers,
            Json(value),
        )
        .await,
    )
}

fn conflict_response(result: Result<Response, conflicts::ApiError>) -> Response {
    match result {
        Ok(response) => response,
        Err(error) => error.into_response(),
    }
}

fn invalid_conflict_json_response() -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse::new(
            PublicErrorCode::ValidationError,
            "Invalid conflict resolution request",
        )),
    )
        .into_response()
}

fn query_map(query: Option<&str>) -> HashMap<String, String> {
    let mut output = HashMap::new();
    let Some(query) = query else {
        return output;
    };

    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        output.insert(key.to_owned(), value.to_owned());
    }

    output
}

fn conflict_resolve_id(path: &str) -> Option<String> {
    let value = path
        .strip_prefix("/v1/conflicts/")?
        .strip_suffix("/resolve")?;
    if value.is_empty() || value.contains('/') {
        return None;
    }

    Some(value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use haze_sync_api::auth::{AdapterPrincipal, AdapterRole};
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
    async fn protected_changes_route_requires_auth_without_runtime_state() {
        let (status, json) =
            request_json("GET", "/v1/changes?since=0&limit=10", Body::empty()).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(json["error"]["code"], "missing_token");
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
