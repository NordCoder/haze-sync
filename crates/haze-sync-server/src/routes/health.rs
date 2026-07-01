//! Local health route for the server shell.
//!
//! This endpoint only proves that the router is alive. It does not perform
//! readiness checks, database checks, object-store checks, provider checks, or
//! adapter runtime checks.

use axum::Json;
use serde::Serialize;

/// Safe JSON response for GET /health.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct HealthResponse {
    /// Local process/router health status.
    pub status: &'static str,
}

/// Handles GET /health.
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}
