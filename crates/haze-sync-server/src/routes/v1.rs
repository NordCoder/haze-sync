//! V1 Core API route shell.
//!
//! These routes reserve the HTTP API boundaries described by the contract pack.
//! Server info returns static public capabilities. File, change, and conflict
//! routes intentionally return `not_implemented` until future Core, storage,
//! auth, idempotency, conflict, and delete phases wire real behavior.

use axum::{routing::get, Json, Router};
use haze_sync_api::dto::server::{ServerCapabilityDto, ServerInfoResponse};

use crate::http::errors::{not_implemented_response, ShellErrorResponse};

/// Builds the versioned `/v1` API namespace.
pub fn router() -> Router {
    Router::new()
        .route("/server-info", get(server_info))
        .route("/changes", get(core_route_not_implemented))
        .route(
            "/files/*path",
            get(core_route_not_implemented)
                .put(core_route_not_implemented)
                .delete(core_route_not_implemented),
        )
        .route("/conflicts", get(core_route_not_implemented))
        .route(
            "/conflicts/{conflict_id}/resolve",
            axum::routing::post(core_route_not_implemented),
        )
}

/// Handles GET /v1/server-info with static, safe route-shell information.
pub async fn server_info() -> Json<ServerInfoResponse> {
    Json(ServerInfoResponse {
        server_id: "haze-sync-route-shell".to_owned(),
        protocol_version: 1,
        max_upload_bytes: 52_428_800,
        capabilities: vec![
            ServerCapabilityDto::Sha256,
            ServerCapabilityDto::OperationLog,
            ServerCapabilityDto::Tombstones,
            ServerCapabilityDto::Conflicts,
            ServerCapabilityDto::ConflictCenter,
            ServerCapabilityDto::BatchChanges,
        ],
    })
}

/// Placeholder for Core routes whose business behavior is not implemented yet.
pub async fn core_route_not_implemented() -> (axum::http::StatusCode, Json<ShellErrorResponse>) {
    not_implemented_response()
}
