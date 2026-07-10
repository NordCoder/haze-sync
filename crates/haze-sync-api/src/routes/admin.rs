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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusSummaryRequest {
    /// Include unknown placeholder values when live dependency state is absent.
    pub include_placeholders: bool,
}

impl Default for StatusSummaryRequest {
    fn default() -> Self {
        Self {
            include_placeholders: true,
        }
    }
}

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
    /// No conclusive readiness result is available.
    Unknown,
    /// The dependency check succeeded.
    Ready,
    /// The dependency check failed without exposing raw internal errors.
    NotReady,
}

/// Honest execution state for passive doctor/status check summaries.
///
/// These values distinguish a real pass/fail result from checks that were
/// skipped, not executed, or represented only by a placeholder. They carry no
/// raw error text or runtime payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationalCheckStatus {
    /// A real check ran and succeeded.
    Passed,
    /// A real check ran and failed; details remain private to runtime logs.
    Failed,
    /// Runtime intentionally skipped the check.
    Skipped,
    /// Runtime did not execute the check.
    NotRun,
    /// No live check exists and the entry is an explicit placeholder.
    Placeholder,
}

impl OperationalCheckStatus {
    /// Map execution state into the coarse readiness vocabulary.
    #[must_use]
    pub const fn readiness_state(self) -> DependencyReadinessState {
        match self {
            Self::Passed => DependencyReadinessState::Ready,
            Self::Failed => DependencyReadinessState::NotReady,
            Self::Skipped | Self::NotRun | Self::Placeholder => {
                DependencyReadinessState::Unknown
            }
        }
    }
}

/// Fixed public doctor-check names.
///
/// The enum prevents runtime paths, provider names, SQL text, or arbitrary error
/// labels from becoming public check identifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DoctorCheckKind {
    /// Database readiness summary.
    Database,
    /// Content/object-store readiness summary.
    ObjectStore,
    /// Operation-log availability summary.
    OperationLog,
    /// Adapter registry/configuration summary.
    AdapterRegistry,
}

/// One sanitized doctor-facing check result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoctorCheckSummary {
    /// Stable public check name.
    pub check: DoctorCheckKind,
    /// Whether the check passed, failed, was skipped, was not run, or is a placeholder.
    pub status: OperationalCheckStatus,
    /// Coarse readiness derived from status.
    pub readiness_state: DependencyReadinessState,
    /// Check timestamp only when a real pass/fail check ran.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checked_at: Option<TimestampDto>,
}

impl DoctorCheckSummary {
    /// Build a successful live-check summary without runtime details.
    #[must_use]
    pub fn passed(check: DoctorCheckKind, checked_at: TimestampDto) -> Self {
        Self::observed(check, OperationalCheckStatus::Passed, checked_at)
    }

    /// Build a failed live-check summary without exposing its raw error.
    #[must_use]
    pub fn failed(check: DoctorCheckKind, checked_at: TimestampDto) -> Self {
        Self::observed(check, OperationalCheckStatus::Failed, checked_at)
    }

    /// Build an explicitly skipped check summary.
    #[must_use]
    pub const fn skipped(check: DoctorCheckKind) -> Self {
        Self::unobserved(check, OperationalCheckStatus::Skipped)
    }

    /// Build an explicitly not-run check summary.
    #[must_use]
    pub const fn not_run(check: DoctorCheckKind) -> Self {
        Self::unobserved(check, OperationalCheckStatus::NotRun)
    }

    /// Build an explicit placeholder for a check that has no live implementation.
    #[must_use]
    pub const fn placeholder(check: DoctorCheckKind) -> Self {
        Self::unobserved(check, OperationalCheckStatus::Placeholder)
    }

    fn observed(
        check: DoctorCheckKind,
        status: OperationalCheckStatus,
        checked_at: TimestampDto,
    ) -> Self {
        Self {
            check,
            status,
            readiness_state: status.readiness_state(),
            checked_at: Some(checked_at),
        }
    }

    const fn unobserved(check: DoctorCheckKind, status: OperationalCheckStatus) -> Self {
        Self {
            check,
            status,
            readiness_state: status.readiness_state(),
            checked_at: None,
        }
    }
}

/// Passive doctor-facing status response.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoctorStatusResponse {
    /// Sanitized high-level server state.
    pub server_status: ServerStatus,
    /// Fixed-name check summaries with no raw errors or runtime payloads.
    pub checks: Vec<DoctorCheckSummary>,
}

impl DoctorStatusResponse {
    /// Build a response from already sanitized public summaries.
    #[must_use]
    pub fn new(server_status: ServerStatus, checks: Vec<DoctorCheckSummary>) -> Self {
        Self {
            server_status,
            checks,
        }
    }

    /// Build the explicit dependency-free placeholder used before live checks exist.
    #[must_use]
    pub fn placeholder() -> Self {
        Self::new(
            ServerStatus::NotReady,
            vec![
                DoctorCheckSummary::placeholder(DoctorCheckKind::Database),
                DoctorCheckSummary::placeholder(DoctorCheckKind::ObjectStore),
                DoctorCheckSummary::placeholder(DoctorCheckKind::OperationLog),
                DoctorCheckSummary::placeholder(DoctorCheckKind::AdapterRegistry),
            ],
        )
    }
}

/// Safe global or adapter pause summary.
///
/// API models support/state only. It does not expose pause/resume mutations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PauseStatusSummary {
    /// Whether pause status is supported by the current schema/runtime.
    pub supported: bool,
    /// Whether the system or adapter is actively paused, when supported.
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

    /// Return a supported pause summary from an already sanitized runtime flag.
    #[must_use]
    pub const fn supported(active: bool) -> Self {
        Self {
            supported: true,
            active: Some(active),
        }
    }

    /// Whether the public support/state pair is internally consistent.
    #[must_use]
    pub const fn is_consistent(self) -> bool {
        matches!((self.supported, self.active), (false, None) | (true, Some(_)))
    }
}

/// Passive status summary response model.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusSummaryResponse {
    /// High-level server status only.
    pub server_status: ServerStatus,
    /// Database readiness without raw database errors or URLs.
    pub db_readiness_state: DependencyReadinessState,
    /// Object-store readiness without local filesystem paths.
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

/// Public external-cursor presence without the cursor value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CursorPresence {
    /// Cursor state was not observed or the cursor summary is unavailable.
    Unknown,
    /// Runtime observed that no private external cursor exists.
    Absent,
    /// Runtime observed that a private external cursor exists.
    Present,
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

    /// Return only public cursor presence, never its private value.
    #[must_use]
    pub const fn external_cursor_presence(&self) -> CursorPresence {
        if self.has_external_cursor {
            CursorPresence::Present
        } else {
            CursorPresence::Absent
        }
    }
}

/// Sanitized adapter runtime state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterRuntimeState {
    /// No conclusive runtime state is available.
    Unknown,
    /// Adapter is disabled by safe configuration metadata.
    Disabled,
    /// Adapter is enabled but not currently processing work.
    Idle,
    /// Adapter is actively processing work.
    Running,
    /// Adapter is active with a degraded sanitized state.
    Degraded,
}

/// Sanitized runtime observation for an adapter.
///
/// This type cannot contain provider payloads, raw errors, local paths, or cursor
/// values. `observation_status` states whether runtime state was actually checked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterRuntimeSummary {
    /// Whether runtime observation ran, was skipped, was not run, or is a placeholder.
    pub observation_status: OperationalCheckStatus,
    /// Sanitized runtime state when it was observed.
    pub state: Option<AdapterRuntimeState>,
    /// Pause support/state only; no mutation intent is represented.
    pub pause: PauseStatusSummary,
}

impl AdapterRuntimeSummary {
    /// Build a real sanitized runtime observation.
    #[must_use]
    pub const fn observed(state: AdapterRuntimeState, pause: PauseStatusSummary) -> Self {
        Self {
            observation_status: OperationalCheckStatus::Passed,
            state: Some(state),
            pause,
        }
    }

    /// Build an explicitly skipped runtime observation.
    #[must_use]
    pub const fn skipped() -> Self {
        Self::unobserved(OperationalCheckStatus::Skipped)
    }

    /// Build an explicitly not-run runtime observation.
    #[must_use]
    pub const fn not_run() -> Self {
        Self::unobserved(OperationalCheckStatus::NotRun)
    }

    /// Build an explicit placeholder before runtime observation is implemented.
    #[must_use]
    pub const fn placeholder() -> Self {
        Self::unobserved(OperationalCheckStatus::Placeholder)
    }

    const fn unobserved(observation_status: OperationalCheckStatus) -> Self {
        Self {
            observation_status,
            state: None,
            pause: PauseStatusSummary::unsupported(),
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

    /// Return external-cursor presence without exposing its private value.
    #[must_use]
    pub const fn external_cursor_presence(&self) -> CursorPresence {
        match &self.cursor {
            Some(cursor) => cursor.external_cursor_presence(),
            None => CursorPresence::Unknown,
        }
    }
}

/// Adapter identity/config summary paired with sanitized runtime observation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterOperationalSummary {
    /// Existing safe adapter identity/config/cursor summary.
    pub adapter: AdapterSummary,
    /// Sanitized runtime observation with honest execution status.
    pub runtime: AdapterRuntimeSummary,
}

impl AdapterOperationalSummary {
    /// Pair already sanitized adapter and runtime summaries.
    #[must_use]
    pub const fn new(adapter: AdapterSummary, runtime: AdapterRuntimeSummary) -> Self {
        Self { adapter, runtime }
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
        let total_count = adapters.len() as u64;
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
    fn adapters_list_response_serialization_is_stable_and_cursor_safe() {
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
        assert_eq!(adapter.external_cursor_presence(), CursorPresence::Present);
        let response = AdapterListResponse::new(vec![adapter]);

        let json = serde_json::to_string(&response).unwrap();

        assert_eq!(
            json,
            "{\"total_count\":1,\"adapters\":[{\"adapter_id\":\"gdrive-adapter\",\"role\":\"gdrive_adapter\",\"mode\":\"import_only\",\"enabled\":true,\"last_seen_at\":\"2026-07-02T10:00:00Z\",\"cursor\":{\"last_core_seq\":42,\"last_success_at\":\"2026-07-02T10:01:00Z\",\"has_external_cursor\":true}}]}"
        );
        assert_no_secret_bearing_fields(&json);
    }

    #[test]
    fn cursor_presence_distinguishes_unknown_absent_and_present_without_values() {
        let unknown = AdapterSummary::new(
            AdapterId::parse("obsidian-plugin").unwrap(),
            Some(AdapterRole::ObsidianPlugin),
            true,
        );
        let absent = AdapterCursorSummary::new(None, None, false);
        let present = AdapterCursorSummary::new(Some(7), None, true);

        assert_eq!(unknown.external_cursor_presence(), CursorPresence::Unknown);
        assert_eq!(absent.external_cursor_presence(), CursorPresence::Absent);
        assert_eq!(present.external_cursor_presence(), CursorPresence::Present);
        assert_eq!(serde_json::to_string(&CursorPresence::Present).unwrap(), "\"present\"");
    }

    #[test]
    fn doctor_status_distinguishes_live_skipped_not_run_and_placeholder_checks() {
        let response = DoctorStatusResponse::new(
            ServerStatus::Degraded,
            vec![
                DoctorCheckSummary::passed(
                    DoctorCheckKind::Database,
                    TimestampDto::from("2026-07-10T09:00:00Z"),
                ),
                DoctorCheckSummary::failed(
                    DoctorCheckKind::ObjectStore,
                    TimestampDto::from("2026-07-10T09:00:01Z"),
                ),
                DoctorCheckSummary::skipped(DoctorCheckKind::OperationLog),
                DoctorCheckSummary::not_run(DoctorCheckKind::AdapterRegistry),
            ],
        );

        assert_eq!(
            response.checks[0].readiness_state,
            DependencyReadinessState::Ready
        );
        assert_eq!(
            response.checks[1].readiness_state,
            DependencyReadinessState::NotReady
        );
        assert_eq!(
            response.checks[2].readiness_state,
            DependencyReadinessState::Unknown
        );
        assert_eq!(response.checks[2].checked_at, None);
        assert_eq!(response.checks[3].checked_at, None);

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"status\":\"passed\""));
        assert!(json.contains("\"status\":\"failed\""));
        assert!(json.contains("\"status\":\"skipped\""));
        assert!(json.contains("\"status\":\"not_run\""));
        assert!(!json.contains("raw_error"));
        assert_no_secret_bearing_fields(&json);

        let placeholder_json = serde_json::to_string(&DoctorStatusResponse::placeholder()).unwrap();
        assert_eq!(placeholder_json.matches("\"status\":\"placeholder\"").count(), 4);
        assert!(!placeholder_json.contains("checked_at"));
    }

    #[test]
    fn adapter_runtime_summary_is_sanitized_and_honest() {
        let adapter = AdapterSummary::new(
            AdapterId::parse("worktree-adapter").unwrap(),
            Some(AdapterRole::WorktreeAdapter),
            true,
        );
        let observed = AdapterOperationalSummary::new(
            adapter.clone(),
            AdapterRuntimeSummary::observed(
                AdapterRuntimeState::Running,
                PauseStatusSummary::supported(false),
            ),
        );
        let placeholder = AdapterOperationalSummary::new(
            adapter,
            AdapterRuntimeSummary::placeholder(),
        );

        assert!(observed.runtime.pause.is_consistent());
        assert_eq!(
            placeholder.runtime.observation_status,
            OperationalCheckStatus::Placeholder
        );
        assert_eq!(placeholder.runtime.state, None);
        assert!(placeholder.runtime.pause.is_consistent());

        let json = serde_json::to_string(&vec![observed, placeholder]).unwrap();
        assert!(json.contains("\"state\":\"running\""));
        assert!(json.contains("\"observation_status\":\"placeholder\""));
        assert!(!json.contains("pause_intent"));
        assert!(!json.contains("resume_intent"));
        assert_no_secret_bearing_fields(&json);
    }

    #[test]
    fn request_shapes_are_deterministic() {
        assert_eq!(
            serde_json::to_string(&StatusSummaryRequest::default()).unwrap(),
            "{\"include_placeholders\":true}"
        );
        assert_eq!(
            serde_json::to_string(&AdapterListRequest::default()).unwrap(),
            "{\"include_disabled\":true}"
        );
    }

    #[test]
    fn pause_resume_request_model_is_not_exposed_without_public_mutation_contract() {
        let json = serde_json::to_string(&StatusSummaryResponse::placeholder()).unwrap();

        assert!(json.contains("\"pause\":{\"supported\":false,\"active\":null}"));
        assert!(PauseStatusSummary::unsupported().is_consistent());
        assert!(PauseStatusSummary::supported(true).is_consistent());
        assert!(!json.contains("pause_intent"));
        assert!(!json.contains("resume_intent"));
    }

    fn assert_no_secret_bearing_fields(json: &str) {
        let json = json.to_ascii_lowercase();
        for forbidden in [
            "bearer ",
            "token_hash",
            "oauth",
            "secret",
            "database_url",
            "db_url",
            "postgres://",
            "mysql://",
            "provider_payload",
            "raw_error",
            "runtime_error",
            "external_cursor_json",
            "raw_cursor",
            "cursor_value",
            "page_token",
            "object_store_root",
            "/home/",
            "/users/",
            "/srv/",
            "c:\\",
            "stack_trace",
            "backtrace",
        ] {
            assert!(
                !json.contains(forbidden),
                "admin/status output leaked forbidden field {forbidden}: {json}"
            );
        }
    }
}
