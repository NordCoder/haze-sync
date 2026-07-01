//! Haze Sync HTTP route shell and W2 Core file-operation fan-in.
//!
//! Health remains dependency-free. Readiness and Core file/change routes use
//! explicit caller-owned state when supplied; no hidden global runtime state is
//! created by router construction.

use axum::{routing::get, Extension, Router};

use crate::{readiness::ReadinessState, state::ServerAppState};

pub mod health;
pub mod v1;

/// Builds the Haze Sync router with safe not-ready dependency defaults.
///
/// Protected W2 Core routes are registered, but without runtime state they
/// return sanitized auth/storage errors rather than mutating anything.
pub fn build_router() -> Router {
    build_router_with_state(ServerAppState::dependency_free())
}

/// Builds the Haze Sync router with caller-supplied runtime state.
#[must_use]
pub fn build_router_with_state(state: ServerAppState) -> Router {
    let readiness = state.readiness_state();
    build_router_with_state_and_readiness(state, readiness)
}

/// Builds the router with caller-supplied readiness checks and dependency-free
/// Core state. This preserves the W1/W2-P8 shell integration point.
pub fn build_router_with_readiness(readiness: ReadinessState) -> Router {
    build_router_with_state_and_readiness(ServerAppState::dependency_free(), readiness)
}

fn build_router_with_state_and_readiness(state: ServerAppState, readiness: ReadinessState) -> Router {
    Router::new()
        .route("/health", get(health::health))
        .route("/ready", get(health::ready))
        .nest("/v1", v1::router())
        .layer(Extension(readiness))
        .layer(Extension(state))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
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
}
