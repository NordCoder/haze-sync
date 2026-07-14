//! Secret-safe public DTOs for hosted Worktree status and manual submission.
//!
//! These types serialize already-sanitized Server results. They do not derive
//! lifecycle policy, inspect the filesystem, execute cycles, or expose runtime
//! tickets, paths, errors, cursors, tokens, or request internals.

use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

/// Configured Worktree mode exposed by the public status contract.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorktreeConfiguredMode {
    Disabled,
    ReadOnly,
    ImportOnly,
    ExportOnly,
    Bidirectional,
    DryRun,
}

/// Hosted Worktree lifecycle exposed without private runtime details.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorktreeHostLifecycle {
    Disabled,
    Starting,
    Running,
    Cancelling,
    Shutdown,
    Failed,
}

/// Coarse public readiness category for the hosted Worktree runtime.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorktreeReadiness {
    Ready,
    NotReady,
}

/// Stable reason paired with the public readiness category.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorktreeReadinessReason {
    DisabledInert,
    Running,
    Starting,
    Cancelling,
    Shutdown,
    Failed,
}

/// Already-sanitized Server result for manual-cycle submission availability.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorktreeManualAvailability {
    Available,
    Busy,
    NotStarted,
    Cancelling,
    Shutdown,
    Unavailable,
    Failed,
}

/// Safe public values used to construct a Worktree status response.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WorktreeStatusSafeParts {
    pub configured_mode: WorktreeConfiguredMode,
    pub host_lifecycle: WorktreeHostLifecycle,
    pub readiness: WorktreeReadiness,
    pub readiness_reason: WorktreeReadinessReason,
    pub cycles_completed: u64,
    pub cycles_failed: u64,
    pub cycle_in_progress: bool,
    pub pending_watcher_hints: u64,
    pub manual_availability: WorktreeManualAvailability,
}

/// Safe public values with a platform-width watcher-hint count.
///
/// This helper shape supports future Server mapping without making platform
/// width part of the JSON contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WorktreeStatusPlatformParts {
    pub configured_mode: WorktreeConfiguredMode,
    pub host_lifecycle: WorktreeHostLifecycle,
    pub readiness: WorktreeReadiness,
    pub readiness_reason: WorktreeReadinessReason,
    pub cycles_completed: u64,
    pub cycles_failed: u64,
    pub cycle_in_progress: bool,
    pub pending_watcher_hints: usize,
    pub manual_availability: WorktreeManualAvailability,
}

/// Public status response for a future `GET /v1/admin/worktree/status` handler.
///
/// Readiness and manual availability are accepted as sanitized Server facts.
/// Counters and watcher hints are informational and never derive readiness.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorktreeStatusResponse {
    pub configured_mode: WorktreeConfiguredMode,
    pub host_lifecycle: WorktreeHostLifecycle,
    pub readiness: WorktreeReadiness,
    pub readiness_reason: WorktreeReadinessReason,
    pub cycles_completed: u64,
    pub cycles_failed: u64,
    pub cycle_in_progress: bool,
    pub pending_watcher_hints: u64,
    pub manual_availability: WorktreeManualAvailability,
}

impl WorktreeStatusResponse {
    /// Build a response from already-sanitized public values.
    #[must_use]
    pub const fn from_safe_parts(parts: WorktreeStatusSafeParts) -> Self {
        Self {
            configured_mode: parts.configured_mode,
            host_lifecycle: parts.host_lifecycle,
            readiness: parts.readiness,
            readiness_reason: parts.readiness_reason,
            cycles_completed: parts.cycles_completed,
            cycles_failed: parts.cycles_failed,
            cycle_in_progress: parts.cycle_in_progress,
            pending_watcher_hints: parts.pending_watcher_hints,
            manual_availability: parts.manual_availability,
        }
    }

    /// Build a response while checking conversion of a platform-width hint count.
    pub fn try_from_platform_parts(
        parts: WorktreeStatusPlatformParts,
    ) -> Result<Self, WorktreeStatusBuildError> {
        let pending_watcher_hints = u64::try_from(parts.pending_watcher_hints)
            .map_err(|_| WorktreeStatusBuildError::PendingWatcherHintsOutOfRange)?;

        Ok(Self::from_safe_parts(WorktreeStatusSafeParts {
            configured_mode: parts.configured_mode,
            host_lifecycle: parts.host_lifecycle,
            readiness: parts.readiness,
            readiness_reason: parts.readiness_reason,
            cycles_completed: parts.cycles_completed,
            cycles_failed: parts.cycles_failed,
            cycle_in_progress: parts.cycle_in_progress,
            pending_watcher_hints,
            manual_availability: parts.manual_availability,
        }))
    }

    /// Return the supplied coarse readiness fact without recalculating policy.
    #[must_use]
    pub const fn is_ready(self) -> bool {
        matches!(self.readiness, WorktreeReadiness::Ready)
    }
}

/// Safe construction failure with no rejected value or backend detail.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorktreeStatusBuildError {
    PendingWatcherHintsOutOfRange,
}

impl fmt::Display for WorktreeStatusBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Worktree status count is outside the public integer range")
    }
}

impl Error for WorktreeStatusBuildError {}

/// Bodyless administrative request marker for a future sync-once endpoint.
///
/// Unknown fields are rejected so clients cannot smuggle force flags, paths,
/// budgets, mode overrides, tickets, or other runtime policy into the request.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorktreeSyncOnceRequest {}

/// Coarse result of submitting one manual cycle request to Server.
///
/// `Accepted` means accepted or queued for submission only. It does not state
/// that a cycle started or completed successfully.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorktreeSyncOnceSubmissionStatus {
    Accepted,
    Busy,
    NotStarted,
    Cancelling,
    Shutdown,
    Unavailable,
    Failed,
}

/// Public response for a future `POST /v1/admin/worktree/sync-once` handler.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorktreeSyncOnceResponse {
    pub status: WorktreeSyncOnceSubmissionStatus,
}

impl WorktreeSyncOnceResponse {
    /// Build a response from an already-sanitized Server submission outcome.
    #[must_use]
    pub const fn submitted(status: WorktreeSyncOnceSubmissionStatus) -> Self {
        Self { status }
    }

    /// Whether Server accepted or queued the submission.
    ///
    /// This is intentionally not named as a completion or success check.
    #[must_use]
    pub const fn was_accepted(self) -> bool {
        matches!(self.status, WorktreeSyncOnceSubmissionStatus::Accepted)
    }
}

#[cfg(test)]
mod tests {
    use serde::{de::DeserializeOwned, Serialize};

    use super::*;

    fn assert_roundtrip<T>(value: T)
    where
        T: Copy + fmt::Debug + DeserializeOwned + Eq + Serialize,
    {
        let json = serde_json::to_string(&value).expect("value must serialize");
        let decoded = serde_json::from_str::<T>(&json).expect("value must deserialize");
        assert_eq!(decoded, value);
    }

    #[test]
    fn enum_wire_values_are_stable_and_roundtrip() {
        for (value, wire) in [
            (WorktreeConfiguredMode::Disabled, "disabled"),
            (WorktreeConfiguredMode::ReadOnly, "read_only"),
            (WorktreeConfiguredMode::ImportOnly, "import_only"),
            (WorktreeConfiguredMode::ExportOnly, "export_only"),
            (WorktreeConfiguredMode::Bidirectional, "bidirectional"),
            (WorktreeConfiguredMode::DryRun, "dry_run"),
        ] {
            assert_eq!(
                serde_json::to_string(&value).unwrap(),
                format!("\"{wire}\"")
            );
            assert_roundtrip(value);
        }

        for value in [
            WorktreeHostLifecycle::Disabled,
            WorktreeHostLifecycle::Starting,
            WorktreeHostLifecycle::Running,
            WorktreeHostLifecycle::Cancelling,
            WorktreeHostLifecycle::Shutdown,
            WorktreeHostLifecycle::Failed,
        ] {
            assert_roundtrip(value);
        }

        for value in [WorktreeReadiness::Ready, WorktreeReadiness::NotReady] {
            assert_roundtrip(value);
        }

        for value in [
            WorktreeReadinessReason::DisabledInert,
            WorktreeReadinessReason::Running,
            WorktreeReadinessReason::Starting,
            WorktreeReadinessReason::Cancelling,
            WorktreeReadinessReason::Shutdown,
            WorktreeReadinessReason::Failed,
        ] {
            assert_roundtrip(value);
        }

        for value in [
            WorktreeManualAvailability::Available,
            WorktreeManualAvailability::Busy,
            WorktreeManualAvailability::NotStarted,
            WorktreeManualAvailability::Cancelling,
            WorktreeManualAvailability::Shutdown,
            WorktreeManualAvailability::Unavailable,
            WorktreeManualAvailability::Failed,
        ] {
            assert_roundtrip(value);
        }
    }

    #[test]
    fn representative_status_json_is_stable_and_secret_safe() {
        let response = WorktreeStatusResponse::from_safe_parts(WorktreeStatusSafeParts {
            configured_mode: WorktreeConfiguredMode::DryRun,
            host_lifecycle: WorktreeHostLifecycle::Running,
            readiness: WorktreeReadiness::Ready,
            readiness_reason: WorktreeReadinessReason::Running,
            cycles_completed: 17,
            cycles_failed: 2,
            cycle_in_progress: true,
            pending_watcher_hints: 23,
            manual_availability: WorktreeManualAvailability::Busy,
        });

        let json = serde_json::to_string(&response).unwrap();
        assert_eq!(
            json,
            "{\"configured_mode\":\"dry_run\",\"host_lifecycle\":\"running\",\"readiness\":\"ready\",\"readiness_reason\":\"running\",\"cycles_completed\":17,\"cycles_failed\":2,\"cycle_in_progress\":true,\"pending_watcher_hints\":23,\"manual_availability\":\"busy\"}"
        );
        assert!(response.is_ready());
        assert_no_private_worktree_material(&json);
        assert_roundtrip(response);
    }

    #[test]
    fn accepted_semantic_combinations_remain_explicit_public_facts() {
        let disabled = WorktreeStatusResponse::from_safe_parts(WorktreeStatusSafeParts {
            configured_mode: WorktreeConfiguredMode::Disabled,
            host_lifecycle: WorktreeHostLifecycle::Disabled,
            readiness: WorktreeReadiness::Ready,
            readiness_reason: WorktreeReadinessReason::DisabledInert,
            cycles_completed: 0,
            cycles_failed: 0,
            cycle_in_progress: false,
            pending_watcher_hints: 0,
            manual_availability: WorktreeManualAvailability::Unavailable,
        });
        assert!(disabled.is_ready());

        let busy = WorktreeStatusResponse::from_safe_parts(WorktreeStatusSafeParts {
            configured_mode: WorktreeConfiguredMode::DryRun,
            host_lifecycle: WorktreeHostLifecycle::Running,
            readiness: WorktreeReadiness::Ready,
            readiness_reason: WorktreeReadinessReason::Running,
            cycles_completed: 1,
            cycles_failed: u64::MAX,
            cycle_in_progress: true,
            pending_watcher_hints: u64::MAX,
            manual_availability: WorktreeManualAvailability::Busy,
        });
        assert!(busy.is_ready());

        let failed = WorktreeStatusResponse::from_safe_parts(WorktreeStatusSafeParts {
            configured_mode: WorktreeConfiguredMode::DryRun,
            host_lifecycle: WorktreeHostLifecycle::Failed,
            readiness: WorktreeReadiness::NotReady,
            readiness_reason: WorktreeReadinessReason::Failed,
            cycles_completed: u64::MAX,
            cycles_failed: 0,
            cycle_in_progress: false,
            pending_watcher_hints: 0,
            manual_availability: WorktreeManualAvailability::Failed,
        });
        assert!(!failed.is_ready());
    }

    #[test]
    fn empty_sync_request_rejects_client_controlled_runtime_policy() {
        assert_eq!(
            serde_json::to_string(&WorktreeSyncOnceRequest::default()).unwrap(),
            "{}"
        );
        assert!(serde_json::from_str::<WorktreeSyncOnceRequest>("{\"force\":true}").is_err());
        assert!(serde_json::from_str::<WorktreeSyncOnceRequest>("{\"path\":\"Notes\"}").is_err());
    }

    #[test]
    fn submission_outcomes_roundtrip_and_accepted_is_not_completion() {
        for status in [
            WorktreeSyncOnceSubmissionStatus::Accepted,
            WorktreeSyncOnceSubmissionStatus::Busy,
            WorktreeSyncOnceSubmissionStatus::NotStarted,
            WorktreeSyncOnceSubmissionStatus::Cancelling,
            WorktreeSyncOnceSubmissionStatus::Shutdown,
            WorktreeSyncOnceSubmissionStatus::Unavailable,
            WorktreeSyncOnceSubmissionStatus::Failed,
        ] {
            let response = WorktreeSyncOnceResponse::submitted(status);
            assert_roundtrip(response);
            assert_eq!(
                response.was_accepted(),
                status == WorktreeSyncOnceSubmissionStatus::Accepted
            );
        }
    }

    fn assert_no_private_worktree_material(json: &str) {
        let lowercase = json.to_ascii_lowercase();
        for forbidden in [
            "raw_error",
            "root_fingerprint",
            "database_url",
            "provider_payload",
            "request_payload",
            "cursor_value",
            "bearer ",
            "token_hash",
            "idempotency",
            "runtime_ticket",
            "generation_id",
            "/home/",
            "/users/",
            "/srv/",
            "c:\\",
        ] {
            assert!(!lowercase.contains(forbidden), "leaked {forbidden}: {json}");
        }
    }
}
