//! Cloneable, bounded HTTP control boundary for the uniquely owned Worktree host.

use std::sync::Weak;

use haze_sync_worktree::{
    WorktreeRuntimeCycleBudget, WorktreeRuntimeManualRequest, WorktreeRuntimeManualSubmission,
};

use crate::{
    worktree_host::ServerWorktreeRuntimeHost,
    worktree_status::{ServerWorktreeManualAvailability, ServerWorktreeStatusSnapshot},
};

#[derive(Clone, Debug)]
pub(crate) struct ServerWorktreeHttpControl {
    host: Weak<ServerWorktreeRuntimeHost>,
    manual_budget: WorktreeRuntimeCycleBudget,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ServerWorktreeSyncSubmission {
    Accepted,
    Busy,
    NotStarted,
    Cancelling,
    Shutdown,
    Unavailable,
    Failed,
}

impl ServerWorktreeHttpControl {
    pub(crate) const fn new(
        host: Weak<ServerWorktreeRuntimeHost>,
        manual_budget: WorktreeRuntimeCycleBudget,
    ) -> Self {
        Self {
            host,
            manual_budget,
        }
    }

    pub(crate) fn snapshot(&self) -> Option<ServerWorktreeStatusSnapshot> {
        Some(self.host.upgrade()?.snapshot())
    }

    pub(crate) fn submit_sync_once(&self) -> ServerWorktreeSyncSubmission {
        let Some(host) = self.host.upgrade() else {
            return ServerWorktreeSyncSubmission::Unavailable;
        };
        let snapshot = host.snapshot();
        submit_from_snapshot(snapshot.manual_availability, || {
            map_submission(host.submit_manual(WorktreeRuntimeManualRequest::dry_run(
                self.manual_budget,
            )))
        })
    }
}

fn submit_from_snapshot(
    availability: ServerWorktreeManualAvailability,
    submit: impl FnOnce() -> ServerWorktreeSyncSubmission,
) -> ServerWorktreeSyncSubmission {
    match availability {
        ServerWorktreeManualAvailability::Failed => ServerWorktreeSyncSubmission::Failed,
        ServerWorktreeManualAvailability::Unavailable => ServerWorktreeSyncSubmission::Unavailable,
        ServerWorktreeManualAvailability::Available
        | ServerWorktreeManualAvailability::Busy
        | ServerWorktreeManualAvailability::NotStarted
        | ServerWorktreeManualAvailability::Cancelling
        | ServerWorktreeManualAvailability::Shutdown => submit(),
    }
}

fn map_submission(submission: WorktreeRuntimeManualSubmission) -> ServerWorktreeSyncSubmission {
    match submission {
        WorktreeRuntimeManualSubmission::Accepted(ticket) => {
            drop(ticket);
            ServerWorktreeSyncSubmission::Accepted
        }
        WorktreeRuntimeManualSubmission::Busy => ServerWorktreeSyncSubmission::Busy,
        WorktreeRuntimeManualSubmission::NotStarted => ServerWorktreeSyncSubmission::NotStarted,
        WorktreeRuntimeManualSubmission::Cancelling => ServerWorktreeSyncSubmission::Cancelling,
        WorktreeRuntimeManualSubmission::Shutdown => ServerWorktreeSyncSubmission::Shutdown,
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    #[test]
    fn stale_busy_snapshot_uses_later_authoritative_accepted_result_once() {
        let calls = Cell::new(0);
        let result = submit_from_snapshot(ServerWorktreeManualAvailability::Busy, || {
            calls.set(calls.get() + 1);
            ServerWorktreeSyncSubmission::Accepted
        });
        assert_eq!(result, ServerWorktreeSyncSubmission::Accepted);
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn lifecycle_snapshots_do_not_bypass_typed_submission() {
        for availability in [
            ServerWorktreeManualAvailability::NotStarted,
            ServerWorktreeManualAvailability::Cancelling,
            ServerWorktreeManualAvailability::Shutdown,
        ] {
            let calls = Cell::new(0);
            let result = submit_from_snapshot(availability, || {
                calls.set(calls.get() + 1);
                ServerWorktreeSyncSubmission::Busy
            });
            assert_eq!(result, ServerWorktreeSyncSubmission::Busy);
            assert_eq!(calls.get(), 1);
        }
    }

    #[test]
    fn authoritative_outcomes_are_preserved_without_retry() {
        for expected in [
            ServerWorktreeSyncSubmission::Busy,
            ServerWorktreeSyncSubmission::Cancelling,
            ServerWorktreeSyncSubmission::Shutdown,
        ] {
            let calls = Cell::new(0);
            let result = submit_from_snapshot(ServerWorktreeManualAvailability::Available, || {
                calls.set(calls.get() + 1);
                expected
            });
            assert_eq!(result, expected);
            assert_eq!(calls.get(), 1);
        }
    }

    #[test]
    fn failed_and_unavailable_remain_snapshot_only() {
        for (availability, expected) in [
            (
                ServerWorktreeManualAvailability::Failed,
                ServerWorktreeSyncSubmission::Failed,
            ),
            (
                ServerWorktreeManualAvailability::Unavailable,
                ServerWorktreeSyncSubmission::Unavailable,
            ),
        ] {
            let calls = Cell::new(0);
            let result = submit_from_snapshot(availability, || {
                calls.set(calls.get() + 1);
                ServerWorktreeSyncSubmission::Accepted
            });
            assert_eq!(result, expected);
            assert_eq!(calls.get(), 0);
        }
    }
}
