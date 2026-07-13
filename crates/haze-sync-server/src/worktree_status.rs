//! Passive, secret-safe Server vocabulary for hosted Worktree status.

use crate::worktree_host::{ServerWorktreeHostLifecycle, ServerWorktreeHostStatus};
use haze_sync_worktree::WorktreeMode;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ServerWorktreeModeCategory {
    Disabled,
    ReadOnly,
    ImportOnly,
    ExportOnly,
    Bidirectional,
    DryRun,
}

impl From<WorktreeMode> for ServerWorktreeModeCategory {
    fn from(mode: WorktreeMode) -> Self {
        match mode {
            WorktreeMode::Disabled => Self::Disabled,
            WorktreeMode::ReadOnly => Self::ReadOnly,
            WorktreeMode::ImportOnly => Self::ImportOnly,
            WorktreeMode::ExportOnly => Self::ExportOnly,
            WorktreeMode::Bidirectional => Self::Bidirectional,
            WorktreeMode::DryRun => Self::DryRun,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ServerWorktreeReadinessCategory {
    Ready,
    NotReady,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ServerWorktreeReadinessReason {
    DisabledInert,
    Running,
    Starting,
    Cancelling,
    Shutdown,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ServerWorktreeManualAvailability {
    Available,
    Busy,
    NotStarted,
    Cancelling,
    Shutdown,
    Unavailable,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ServerWorktreeManualGate {
    Available,
    Busy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ServerWorktreeStatusSnapshot {
    pub(crate) mode: ServerWorktreeModeCategory,
    pub(crate) lifecycle: ServerWorktreeHostLifecycle,
    pub(crate) readiness: ServerWorktreeReadinessCategory,
    pub(crate) readiness_reason: ServerWorktreeReadinessReason,
    pub(crate) cycles_completed: u64,
    pub(crate) cycles_failed: u64,
    pub(crate) cycle_in_progress: bool,
    pub(crate) pending_watcher_hints: usize,
    pub(crate) manual_availability: ServerWorktreeManualAvailability,
}

impl ServerWorktreeStatusSnapshot {
    pub(crate) fn from_host(
        mode: WorktreeMode,
        status: ServerWorktreeHostStatus,
        manual_gate: ServerWorktreeManualGate,
    ) -> Self {
        let (readiness, readiness_reason) = readiness(status.lifecycle);
        let manual_availability = manual_availability(mode, status.lifecycle, manual_gate);
        Self {
            mode: mode.into(),
            lifecycle: status.lifecycle,
            readiness,
            readiness_reason,
            cycles_completed: status.cycles_completed,
            cycles_failed: status.cycles_failed,
            cycle_in_progress: status.cycle_in_progress,
            pending_watcher_hints: status.pending_watcher_hints,
            manual_availability,
        }
    }

    pub(crate) const fn is_ready(self) -> bool {
        matches!(self.readiness, ServerWorktreeReadinessCategory::Ready)
    }
}

const fn readiness(
    lifecycle: ServerWorktreeHostLifecycle,
) -> (
    ServerWorktreeReadinessCategory,
    ServerWorktreeReadinessReason,
) {
    match lifecycle {
        ServerWorktreeHostLifecycle::Disabled => (
            ServerWorktreeReadinessCategory::Ready,
            ServerWorktreeReadinessReason::DisabledInert,
        ),
        ServerWorktreeHostLifecycle::Running => (
            ServerWorktreeReadinessCategory::Ready,
            ServerWorktreeReadinessReason::Running,
        ),
        ServerWorktreeHostLifecycle::Starting => (
            ServerWorktreeReadinessCategory::NotReady,
            ServerWorktreeReadinessReason::Starting,
        ),
        ServerWorktreeHostLifecycle::Cancelling => (
            ServerWorktreeReadinessCategory::NotReady,
            ServerWorktreeReadinessReason::Cancelling,
        ),
        ServerWorktreeHostLifecycle::Shutdown => (
            ServerWorktreeReadinessCategory::NotReady,
            ServerWorktreeReadinessReason::Shutdown,
        ),
        ServerWorktreeHostLifecycle::Failed => (
            ServerWorktreeReadinessCategory::NotReady,
            ServerWorktreeReadinessReason::Failed,
        ),
    }
}

const fn manual_availability(
    mode: WorktreeMode,
    lifecycle: ServerWorktreeHostLifecycle,
    gate: ServerWorktreeManualGate,
) -> ServerWorktreeManualAvailability {
    match lifecycle {
        ServerWorktreeHostLifecycle::Disabled => ServerWorktreeManualAvailability::Unavailable,
        ServerWorktreeHostLifecycle::Starting => ServerWorktreeManualAvailability::NotStarted,
        ServerWorktreeHostLifecycle::Cancelling => ServerWorktreeManualAvailability::Cancelling,
        ServerWorktreeHostLifecycle::Shutdown => ServerWorktreeManualAvailability::Shutdown,
        ServerWorktreeHostLifecycle::Failed => ServerWorktreeManualAvailability::Failed,
        ServerWorktreeHostLifecycle::Running => {
            if !matches!(mode, WorktreeMode::DryRun) {
                ServerWorktreeManualAvailability::Unavailable
            } else {
                match gate {
                    ServerWorktreeManualGate::Available => {
                        ServerWorktreeManualAvailability::Available
                    }
                    ServerWorktreeManualGate::Busy => ServerWorktreeManualAvailability::Busy,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(lifecycle: ServerWorktreeHostLifecycle) -> ServerWorktreeHostStatus {
        ServerWorktreeHostStatus {
            lifecycle,
            cycles_completed: 17,
            cycles_failed: 9,
            pending_watcher_hints: 23,
            cycle_in_progress: true,
        }
    }

    #[test]
    fn readiness_mapping_is_deterministic_and_counter_independent() {
        let disabled = ServerWorktreeStatusSnapshot::from_host(
            WorktreeMode::Disabled,
            status(ServerWorktreeHostLifecycle::Disabled),
            ServerWorktreeManualGate::Available,
        );
        assert!(disabled.is_ready());
        assert_eq!(
            disabled.readiness_reason,
            ServerWorktreeReadinessReason::DisabledInert
        );

        let running = ServerWorktreeStatusSnapshot::from_host(
            WorktreeMode::DryRun,
            status(ServerWorktreeHostLifecycle::Running),
            ServerWorktreeManualGate::Busy,
        );
        assert!(running.is_ready());
        assert_eq!(
            running.manual_availability,
            ServerWorktreeManualAvailability::Busy
        );
        assert_eq!(running.cycles_failed, 9);
        assert_eq!(running.pending_watcher_hints, 23);
    }

    #[test]
    fn terminal_and_transition_states_are_not_ready() {
        for (lifecycle, reason) in [
            (
                ServerWorktreeHostLifecycle::Starting,
                ServerWorktreeReadinessReason::Starting,
            ),
            (
                ServerWorktreeHostLifecycle::Cancelling,
                ServerWorktreeReadinessReason::Cancelling,
            ),
            (
                ServerWorktreeHostLifecycle::Shutdown,
                ServerWorktreeReadinessReason::Shutdown,
            ),
            (
                ServerWorktreeHostLifecycle::Failed,
                ServerWorktreeReadinessReason::Failed,
            ),
        ] {
            let snapshot = ServerWorktreeStatusSnapshot::from_host(
                WorktreeMode::DryRun,
                status(lifecycle),
                ServerWorktreeManualGate::Available,
            );
            assert!(!snapshot.is_ready());
            assert_eq!(snapshot.readiness_reason, reason);
        }
    }

    #[test]
    fn manual_availability_is_typed_without_probe_submission() {
        let unavailable = ServerWorktreeStatusSnapshot::from_host(
            WorktreeMode::ExportOnly,
            status(ServerWorktreeHostLifecycle::Running),
            ServerWorktreeManualGate::Available,
        );
        assert_eq!(
            unavailable.manual_availability,
            ServerWorktreeManualAvailability::Unavailable
        );

        let failed = ServerWorktreeStatusSnapshot::from_host(
            WorktreeMode::DryRun,
            status(ServerWorktreeHostLifecycle::Failed),
            ServerWorktreeManualGate::Available,
        );
        assert_eq!(
            failed.manual_availability,
            ServerWorktreeManualAvailability::Failed
        );
    }

    #[test]
    fn debug_representation_contains_only_coarse_fields() {
        let rendered = format!(
            "{:?}",
            ServerWorktreeStatusSnapshot::from_host(
                WorktreeMode::DryRun,
                status(ServerWorktreeHostLifecycle::Running),
                ServerWorktreeManualGate::Available,
            )
        );
        for forbidden in [
            "/srv/",
            "postgres://",
            "payload",
            "token",
            "fingerprint",
            "idempotency",
        ] {
            assert!(!rendered.contains(forbidden));
        }
    }
}
