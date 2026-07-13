//! Passive, secret-safe Server vocabulary for hosted Worktree status.

use crate::worktree_host::{ServerWorktreeHostLifecycle, ServerWorktreeHostStatus};
use haze_sync_worktree::{WorktreeMode, WorktreeRuntimeLifecycle, WorktreeRuntimeManualStatus};

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
        manual_status: Option<WorktreeRuntimeManualStatus>,
    ) -> Self {
        let (readiness, readiness_reason) = match status.lifecycle {
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
        };
        let manual_availability = manual_availability(mode, status.lifecycle, manual_status);
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

const fn manual_availability(
    mode: WorktreeMode,
    host_lifecycle: ServerWorktreeHostLifecycle,
    manual_status: Option<WorktreeRuntimeManualStatus>,
) -> ServerWorktreeManualAvailability {
    if matches!(host_lifecycle, ServerWorktreeHostLifecycle::Failed) {
        return ServerWorktreeManualAvailability::Failed;
    }
    if matches!(host_lifecycle, ServerWorktreeHostLifecycle::Disabled) {
        return ServerWorktreeManualAvailability::Unavailable;
    }
    let Some(status) = manual_status else {
        return match host_lifecycle {
            ServerWorktreeHostLifecycle::Starting => ServerWorktreeManualAvailability::NotStarted,
            ServerWorktreeHostLifecycle::Cancelling => ServerWorktreeManualAvailability::Cancelling,
            ServerWorktreeHostLifecycle::Shutdown => ServerWorktreeManualAvailability::Shutdown,
            _ => ServerWorktreeManualAvailability::Unavailable,
        };
    };
    match status.lifecycle {
        WorktreeRuntimeLifecycle::Created => ServerWorktreeManualAvailability::NotStarted,
        WorktreeRuntimeLifecycle::Cancelling => ServerWorktreeManualAvailability::Cancelling,
        WorktreeRuntimeLifecycle::Shutdown => ServerWorktreeManualAvailability::Shutdown,
        WorktreeRuntimeLifecycle::Running if !matches!(mode, WorktreeMode::DryRun) => {
            ServerWorktreeManualAvailability::Unavailable
        }
        WorktreeRuntimeLifecycle::Running if status.busy => ServerWorktreeManualAvailability::Busy,
        WorktreeRuntimeLifecycle::Running => ServerWorktreeManualAvailability::Available,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host(lifecycle: ServerWorktreeHostLifecycle) -> ServerWorktreeHostStatus {
        ServerWorktreeHostStatus {
            lifecycle,
            cycles_completed: 17,
            cycles_failed: 9,
            pending_watcher_hints: 23,
            cycle_in_progress: true,
        }
    }

    fn manual(lifecycle: WorktreeRuntimeLifecycle, busy: bool) -> WorktreeRuntimeManualStatus {
        WorktreeRuntimeManualStatus { lifecycle, busy }
    }

    #[test]
    fn busy_is_ready_and_counters_are_informational() {
        let snapshot = ServerWorktreeStatusSnapshot::from_host(
            WorktreeMode::DryRun,
            host(ServerWorktreeHostLifecycle::Running),
            Some(manual(WorktreeRuntimeLifecycle::Running, true)),
        );
        assert!(snapshot.is_ready());
        assert_eq!(
            snapshot.manual_availability,
            ServerWorktreeManualAvailability::Busy
        );
        assert_eq!(snapshot.cycles_failed, 9);
        assert_eq!(snapshot.pending_watcher_hints, 23);
    }

    #[test]
    fn lifecycle_overrides_are_deterministic() {
        let failed = ServerWorktreeStatusSnapshot::from_host(
            WorktreeMode::DryRun,
            host(ServerWorktreeHostLifecycle::Failed),
            Some(manual(WorktreeRuntimeLifecycle::Running, true)),
        );
        assert!(!failed.is_ready());
        assert_eq!(
            failed.manual_availability,
            ServerWorktreeManualAvailability::Failed
        );

        for (lifecycle, expected) in [
            (
                WorktreeRuntimeLifecycle::Created,
                ServerWorktreeManualAvailability::NotStarted,
            ),
            (
                WorktreeRuntimeLifecycle::Cancelling,
                ServerWorktreeManualAvailability::Cancelling,
            ),
            (
                WorktreeRuntimeLifecycle::Shutdown,
                ServerWorktreeManualAvailability::Shutdown,
            ),
        ] {
            let snapshot = ServerWorktreeStatusSnapshot::from_host(
                WorktreeMode::DryRun,
                host(ServerWorktreeHostLifecycle::Running),
                Some(manual(lifecycle, true)),
            );
            assert_eq!(snapshot.manual_availability, expected);
        }
    }

    #[test]
    fn non_dry_run_manual_state_is_unavailable() {
        let snapshot = ServerWorktreeStatusSnapshot::from_host(
            WorktreeMode::ExportOnly,
            host(ServerWorktreeHostLifecycle::Running),
            Some(manual(WorktreeRuntimeLifecycle::Running, true)),
        );
        assert_eq!(
            snapshot.manual_availability,
            ServerWorktreeManualAvailability::Unavailable
        );
    }
}
