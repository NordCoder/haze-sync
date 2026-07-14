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
        match snapshot.manual_availability {
            ServerWorktreeManualAvailability::Failed => {
                return ServerWorktreeSyncSubmission::Failed
            }
            ServerWorktreeManualAvailability::Unavailable => {
                return ServerWorktreeSyncSubmission::Unavailable
            }
            ServerWorktreeManualAvailability::NotStarted => {
                return ServerWorktreeSyncSubmission::NotStarted
            }
            ServerWorktreeManualAvailability::Cancelling => {
                return ServerWorktreeSyncSubmission::Cancelling
            }
            ServerWorktreeManualAvailability::Shutdown => {
                return ServerWorktreeSyncSubmission::Shutdown
            }
            ServerWorktreeManualAvailability::Busy => {
                return ServerWorktreeSyncSubmission::Busy
            }
            ServerWorktreeManualAvailability::Available => {}
        }

        match host.submit_manual(WorktreeRuntimeManualRequest::dry_run(self.manual_budget)) {
            WorktreeRuntimeManualSubmission::Accepted(ticket) => {
                drop(ticket);
                ServerWorktreeSyncSubmission::Accepted
            }
            WorktreeRuntimeManualSubmission::Busy => ServerWorktreeSyncSubmission::Busy,
            WorktreeRuntimeManualSubmission::NotStarted => {
                ServerWorktreeSyncSubmission::NotStarted
            }
            WorktreeRuntimeManualSubmission::Cancelling => {
                ServerWorktreeSyncSubmission::Cancelling
            }
            WorktreeRuntimeManualSubmission::Shutdown => ServerWorktreeSyncSubmission::Shutdown,
        }
    }
}
