//! Haze Sync HTTP route shell.
//!
//! This module defines route boundaries for the V1 Core API surface. Health is
//! dependency-free; readiness is backed by explicit safe dependency checks when
//! supplied. Core file/change/conflict routes remain placeholders and do not read
//! or write storage, mutate the filesystem, run Core algorithms, bypass future
//! auth/idempotency/conflict/delete semantics, or contact providers.

use axum::{routing::get, Extension, Router};

use crate::readiness::ReadinessState;

pub mod health;
pub mod v1;

/// Builds the Haze Sync server shell router with safe not-ready dependency defaults.
///
/// The default router remains free of live database connections, object-store
/// creation, provider clients, auth middleware, and production listener startup.
pub fn build_router() -> Router {
    build_router_with_readiness(ReadinessState::dependency_free_not_ready())
}

/// Builds the Haze Sync server shell router with caller-supplied readiness checks.
///
/// This is the integration point future startup code can use after explicitly
/// creating a database pool and validating object-store configuration. Supplying
/// readiness state does not wire file/changelog routes to Core services.
pub fn build_router_with_readiness(readiness: ReadinessState) -> Router {
    Router::new()
        .route("/health", get(health::health))
        .route("/ready", get(health::ready))
        .nest("/v1", v1::router())
        .layer(Extension(readiness))
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
    async fn placeholder_file_write_returns_explicit_not_implemented_error() {
        let (status, json) = request_json(
            "PUT",
            "/v1/files/Notes/a.md",
            Body::from("not persisted by route shell"),
        )
        .await;

        assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
        assert_eq!(json["error"]["code"], "not_implemented");
        assert!(json["error"]["message"]
            .as_str()
            .expect("message should be a string")
            .contains("route shell"));
        assert!(!json.to_string().contains("accepted"));
        assert!(!json.to_string().contains("persisted"));
    }

    #[tokio::test]
    async fn placeholder_changes_route_returns_not_implemented_error() {
        let (status, json) =
            request_json("GET", "/v1/changes?since=0&limit=10", Body::empty()).await;

        assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
        assert_eq!(json["error"]["code"], "not_implemented");
    }
}
