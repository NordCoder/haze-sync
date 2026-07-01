//! Local health and readiness routes for the server shell.
//!
//! `/health` only proves that the router is alive. `/ready` reports safe,
//! deterministic dependency readiness for PostgreSQL and object-store runtime
//! configuration without exposing URLs, filesystem paths, stack traces, or
//! secrets.

use axum::{extract::Extension, http::StatusCode, Json};
use serde::Serialize;

use crate::readiness::{ReadinessOverallStatus, ReadinessReport, ReadinessState};

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

/// Handles GET /ready.
pub async fn ready(Extension(readiness): Extension<ReadinessState>) -> (StatusCode, Json<ReadinessReport>) {
    let report = readiness.check().await;
    let status_code = match report.status {
        ReadinessOverallStatus::Ready => StatusCode::OK,
        ReadinessOverallStatus::NotReady => StatusCode::SERVICE_UNAVAILABLE,
    };

    (status_code, Json(report))
}
