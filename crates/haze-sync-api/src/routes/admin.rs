//! Passive admin/status API contract helpers.
//!
//! This module defines typed request and response shapes for future
//! admin/status endpoints. It deliberately does not register handlers,
//! authenticate tokens, create database pools, read storage, call providers,
//! mutate adapter state, expose raw cursors, or include secrets.

use haze_sync_common::{AdapterId, AdapterMode, AdapterRole};
use serde::{Deserialize, Serialize};

use crate::dto::primitives::TimestampDto;

/// Request shape for a future passive status summary endpoint.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusSummaryRequest {}

/// Request shape for a future passive adapters-list endpoint.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterListRequest {
    /// Include disabled adapters in the response.
    pub include_disabled: bool,
}

impl Default for AdapterListRequest {
    fn default() -> Self {
        Self {
            include_disabled: true,
        }
    }
}

/// Safe high-level server status values for public admin output.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerStatus {
    /// The process is up, but dependencies are not yet known ready.
    NotReady,
    /// The process and required dependencies are ready.
    Ready,
    /// The process is up with one or more degraded dependencies.
    Degraded,
    /// The process is intentionally in maintenance mode.
    Maintenance,
}

/// Safe dependency readiness state for public admin output.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyReadinessState {
    /// The dependency has not been checked by the future handler yet.
    Unknown,
    /// The dependency check succeeded.
    Ready,
    /// The dependency check failed without exposing raw internal errors.
    NotReady,
}

/// Safe global pause summary.
///
/// W3-P6 does not implement pause or resume behavior because the current schema
/// has no paused flag. Future phases may set `supported=true` and `active` from
/// a guarded runtime/status source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PauseStatusSummary {
    /// Whether global pause status is supported by the current schema/runtime.
    pub supported: bool,
    /// Whether the system is actively paused, when supported.
    pub active: Option<bool>,
}

impl PauseStatusSummary {
    /// Return an explicit unsupported placeholder.
    #[must_use]
    pub const fn unsupported() -> Self {
        Self {
            supported: false,
            active: None,
        }
    }
}

/// Passive status summary response model.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusSummaryResponse {
    /// High-level server status only.
    pub server_status: ServerStatus,
    /// Placeholder for database readiness without raw database errors or URLs.
    pub db_readiness_state: DependencyReadinessState,
    /// Placeholder for object-store readiness without local filesystem paths.
    pub object_store_readiness_state: DependencyReadinessState,
    /// Last operation sequence if the future handler has a sanitized value.
    pub last_operation_sequence: Option<i64>,
    /// Number of configured adapters if the future handler has a sanitized value.
    pub adapter_count: Option<u64>,
    /// Safe pause-status placeholder.
    pub pause: PauseStatusSummary,
}

impl StatusSummaryResponse {
    /// Build the deterministic dependency-free placeholder used by tests and CLI
    /// scaffolding before server handlers exist.
    #[must_use]
    pub const fn placeholder() -> Self {
        Self {
            server_status: ServerStatus::NotReady,
            db_readiness_state: DependencyReadinessState::Unknown,
            object_store_readiness_state: DependencyReadinessState::Unknown,
            last_operation_sequence: None,
            adapter_count: None,
            pause: PauseStatusSummary::unsupported(),
        }
    }

    /// Build a response from already-sanitized public values.
    #[must_use]
    pub const fn from_safe_parts(
        server_status: ServerStatus,
        db_readiness_state: DependencyReadinessState,
        object_store_readiness_state: DependencyReadinessState,
        last_operation_sequence: Option<i64>,
        adapter_count: Option<u64>,
        pause: PauseStatusSummary,
    ) -> Self {
        Self {
            server_status,
            db_readiness_state,
            object_store_readiness_state,
            last_operation_sequence,
            adapter_count,
            pause,
        }
    }
}

/// Public adapter cursor summary.
///
/// Raw provider cursors, JSON payloads, token hashes, OAuth tokens, and local
/// runtime details are intentionally not representable in this shape.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterCursorSummary {
    /// Last Core operation sequence safely recorded for this adapter.
    pub last_core_seq: Option<i64>,
    /// Last successful adapter activity timestamp, if known.
    pub last_success_at: Option<TimestampDto>,
    /// Whether a private external cursor exists, without exposing its content.
    pub has_external_cursor: bool,
}

impl AdapterCursorSummary {
    /// Build a sanitized cursor summary without raw external cursor content.
    #[must_use]
    pub fn new(
        last_core_seq: Option<i64>,
        last_success_at: Option<TimestampDto>,
        has_external_cursor: bool,
    ) -> Self {
        Self {
            last_core_seq,
            last_success_at,
            has_external_cursor,
        }
    }
}

/// Public adapter summary for admin/status output.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterSummary {
    /// Stable adapter id.
    pub adapter_id: AdapterId,
    /// Adapter role when available from safe metadata.
    pub role: Option<AdapterRole>,
    /// Adapter rollout mode when available from safe config/status metadata.
    pub mode: Option<AdapterMode>,
    /// Whether the adapter is enabled.
    pub enabled: bool,
    /// Last time the adapter was seen, if known.
    pub last_seen_at: Option<TimestampDto>,
    /// Sanitized cursor summary, if contract-ready.
    pub cursor: Option<AdapterCursorSummary>,
}

impl AdapterSummary {
    /// Build a minimal adapter summary from safe fields.
    #[must_use]
    pub fn new(adapter_id: AdapterId, role: Option<AdapterRole>, enabled: bool) -> Self {
        Self {
            adapter_id,
            role,
            mode: None,
            enabled,
            last_seen_at: None,
            cursor: None,
        }
    }

    /// Attach a safe rollout mode.
    #[must_use]
    pub fn with_mode(mut self, mode: AdapterMode) -> Self {
        self.mode = Some(mode);
        self
    }

    /// Attach a safe last-seen timestamp.
    #[must_use]
    pub fn with_last_seen_at(mut self, last_seen_at: TimestampDto) -> Self {
        self.last_seen_at = Some(last_seen_at);
        self
    }

    /// Attach a sanitized cursor summary.
    #[must_use]
    pub fn with_cursor(mut self, cursor: AdapterCursorSummary) -> Self {
        self.cursor = Some(cursor);
        self
    }
}

/// Passive adapters-list response model.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterListResponse {
    /// Total number of adapters represented in this response.
    pub total_count: u64,
    /// Safe adapter summaries.
    pub adapters: Vec<AdapterSummary>,
}

impl AdapterListResponse {
    /// Build a deterministic adapters-list response from safe summaries.
    #[must_use]
    pub fn new(adapters: Vec<AdapterSummary>) -> Self {
        let total_count = u64::try_from(adapters.len()).unwrap_or(u64::MAX);
        Self {
            total_count,
            adapters,
        }
    }

    /// Return an explicit empty placeholder.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            total_count: 0,
            adapters: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_response_serialization_is_stable_and_safe() {
        let response = StatusSummaryResponse::placeholder();

        let json = serde_json::to_string(&response).unwrap();

        assert_eq!(
            json,
            "{\"server_status\":\"not_ready\",\"db_readiness_state\":\"unknown\",\"object_store_readiness_state\":\"unknown\",\"last_operation_sequence\":null,\"adapter_count\":null,\"pause\":{\"supported\":false,\"active\":null}}"
        );
        assert_no_secret_bearing_fields(&json);
    }

    #[test]
    fn adapters_list_response_serialization_is_stable_and_safe() {
        let adapter = AdapterSummary::new(
            AdapterId::parse("gdrive-adapter").unwrap(),
            Some(AdapterRole::GdriveAdapter),
            true,
        )
        .with_mode(AdapterMode::ImportOnly)
        .with_last_seen_at(TimestampDto::from("2026-07-02T10:00:00Z"))
        .with_cursor(AdapterCursorSummary::new(
            Some(42),
            Some(TimestampDto::from("2026-07-02T10:01:00Z")),
            true,
        ));
        let response = AdapterListResponse::new(vec![adapter]);

        let json = serde_json::to_string(&response).unwrap();

        assert_eq!(
            json,
            "{\"total_count\":1,\"adapters\":[{\"adapter_id\":\"gdrive-adapter\",\"role\":\"gdrive_adapter\",\"mode\":\"import_only\",\"enabled\":true,\"last_seen_at\":\"2026-07-02T10:00:00Z\",\"cursor\":{\"last_core_seq\":42,\"last_success_at\":\"2026-07-02T10:01:00Z\",\"has_external_cursor\":true}}]}"
        );
        assert_no_secret_bearing_fields(&json);
    }

    #[test]
    fn request_shapes_are_deterministic() {
        assert_eq!(serde_json::to_string(&StatusSummaryRequest {}).unwrap(), "{}");
        assert_eq!(
            serde_json::to_string(&AdapterListRequest::default()).unwrap(),
            "{\"include_disabled\":true}"
        );
    }

    #[test]
    fn pause_resume_request_model_is_not_exposed_without_schema_support() {
        let json = serde_json::to_string(&StatusSummaryResponse::placeholder()).unwrap();

        assert!(json.contains("\"pause\":{\"supported\":false,\"active\":null}"));
        assert!(!json.contains("pause_intent"));
        assert!(!json.contains("resume_intent"));
    }

    fn assert_no_secret_bearing_fields(json: &str) {
        for forbidden in [
            "token",
            "hash",
            "oauth",
            "secret",
            "database_url",
            "db_url",
            "provider_payload",
            "external_cursor_json",
            "object_store_root",
            "/srv/",
            "stack",
            "backtrace",
        ] {
            assert!(
                !json.contains(forbidden),
                "admin/status output leaked forbidden field {forbidden}: {json}"
            );
        }
    }
}
