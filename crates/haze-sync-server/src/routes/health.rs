//! Local health and truthful readiness routes.

use axum::{extract::Extension, http::StatusCode, Json};
use serde::Serialize;

use crate::{
    control_plane::DurableMaintenanceState,
    readiness::{
        ReadinessComponent, ReadinessComponentStatus, ReadinessOverallStatus, ReadinessReport,
        ReadinessState,
    },
    state::ServerAppState,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
}

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

pub async fn ready(
    Extension(readiness): Extension<ReadinessState>,
    Extension(state): Extension<ServerAppState>,
) -> (StatusCode, Json<ReadinessReport>) {
    let mut report = readiness.check().await;
    if let Some(control_plane) = state.control_plane() {
        let maintenance = control_plane.maintenance_state();
        report.components.push(if maintenance == DurableMaintenanceState::Normal {
            ReadinessComponent {
                name: "operational_control",
                status: ReadinessComponentStatus::Ready,
                code: "operational_control_ready",
                message: "operational control is in normal state",
            }
        } else {
            ReadinessComponent {
                name: "operational_control",
                status: ReadinessComponentStatus::NotReady,
                code: "operational_control_not_ready",
                message: "operational control is not in normal state",
            }
        });
        report = ReadinessReport::new(report.components);
    } else if state.db_pool().is_some() {
        report.components.push(ReadinessComponent {
            name: "operational_control",
            status: ReadinessComponentStatus::NotReady,
            code: "operational_control_unavailable",
            message: "operational control could not be initialized",
        });
        report = ReadinessReport::new(report.components);
    }
    let status_code = match report.status {
        ReadinessOverallStatus::Ready => StatusCode::OK,
        ReadinessOverallStatus::NotReady => StatusCode::SERVICE_UNAVAILABLE,
    };
    (status_code, Json(report))
}
