//! Local health and readiness routes for the server shell.
//!
//! `/health` only proves that the router is alive. `/ready` is an explicit safe
//! not-ready placeholder until later phases wire non-secret runtime dependency
//! checks for PostgreSQL, migrations, object storage, and adapters.

use axum::{http::StatusCode, Json};
use serde::Serialize;

/// Safe JSON response for GET /health.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct HealthResponse {
    /// Local process/router health status.
    pub status: &'static str,
}

/// Safe JSON response for GET /ready.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ReadyResponse {
    /// Readiness status for dependency-backed API behavior.
    pub status: &'static str,
    /// Safe public explanation for the placeholder state.
    pub reason: &'static str,
}

/// Handles GET /health.
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

/// Handles GET /ready.
///
/// The Wave 1 route shell is intentionally not dependency-backed, so readiness
/// remains false instead of implying that future Core/storage behavior is active.
pub async fn ready() -> (StatusCode, Json<ReadyResponse>) {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ReadyResponse {
            status: "not_ready",
            reason: "runtime dependency checks are not wired in the route shell",
        }),
    )
}
