//! Safe Worktree doctor facts and non-destructive repair planning.
//!
//! Doctor output is derived from authoritative scan/reconciliation/runtime facts,
//! never from watcher hints. Repair plans describe possible actions and risks but
//! never carry execution authorization.

use crate::{
    WorktreeEchoStatus, WorktreeReconciliationEntry, WorktreeReconciliationKind,
    WorktreeReconciliationSummary, WorktreeRuntimeLifecycle, WorktreeRuntimeStatus,
    WorktreeRuntimeWatcherState, WorktreeScanSkipReason,
};
use haze_sync_common::VaultPath;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorktreeDoctorHealth {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WorktreeDoctorIssueKind {
    MissingFile,
    DirtyFile,
    ContentHashMismatch,
    ReservedPathViolation,
    SkippedSymlink,
    SkippedSpecialFile,
    SkippedUnsafePath,
    SkippedFilesystemEntry,
    StaleEcho,
    ExpiredEcho,
    PartialScan,
    RuntimeDegraded,
}

impl WorktreeDoctorIssueKind {
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::MissingFile => "missing_file",
            Self::DirtyFile => "dirty_file",
            Self::ContentHashMismatch => "content_hash_mismatch",
            Self::ReservedPathViolation => "reserved_path_violation",
            Self::SkippedSymlink => "skipped_symlink",
            Self::SkippedSpecialFile => "skipped_special_file",
            Self::SkippedUnsafePath => "skipped_unsafe_path",
            Self::SkippedFilesystemEntry => "skipped_filesystem_entry",
            Self::StaleEcho => "stale_echo",
            Self::ExpiredEcho => "expired_echo",
            Self::PartialScan => "partial_scan",
            Self::RuntimeDegraded => "runtime_degraded",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeDoctorIssue {
    pub kind: WorktreeDoctorIssueKind,
    pub vault_path: Option<VaultPath>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct WorktreeDoctorSummary {
    pub missing_files: usize,
    pub dirty_files: usize,
    pub content_hash_mismatches: usize,
    pub reserved_path_violations: usize,
    pub skipped_symlinks: usize,
    pub skipped_special_files: usize,
    pub skipped_unsafe_paths: usize,
    pub skipped_filesystem_entries: usize,
    pub stale_echoes: usize,
    pub expired_echoes: usize,
    pub partial_scan: bool,
    pub runtime_degraded: bool,
}

impl WorktreeDoctorSummary {
    #[must_use]
    pub const fn issue_count(self) -> usize {
        self.missing_files
            + self.dirty_files
            + self.content_hash_mismatches
            + self.reserved_path_violations
            + self.skipped_symlinks
            + self.skipped_special_files
            + self.skipped_unsafe_paths
            + self.skipped_filesystem_entries
            + self.stale_echoes
            + self.expired_echoes
            + self.partial_scan as usize
            + self.runtime_degraded as usize
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeDoctorReport {
    health: WorktreeDoctorHealth,
    summary: WorktreeDoctorSummary,
    issues: Vec<WorktreeDoctorIssue>,
}

impl WorktreeDoctorReport {
    #[must_use]
    pub const fn health(&self) -> WorktreeDoctorHealth {
        self.health
    }

    #[must_use]
    pub const fn summary(&self) -> WorktreeDoctorSummary {
        self.summary
    }

    #[must_use]
    pub fn issues(&self) -> &[WorktreeDoctorIssue] {
        &self.issues
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeDoctorSnapshot {
    pub reconciliation_entries: Vec<WorktreeReconciliationEntry>,
    pub reconciliation_summary: WorktreeReconciliationSummary,
    pub runtime_status: WorktreeRuntimeStatus,
}

pub trait WorktreeDoctorFactSource {
    type Error;

    fn load_snapshot(&mut self) -> Result<WorktreeDoctorSnapshot, Self::Error>;
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorktreeDoctor;

impl WorktreeDoctor {
    #[must_use]
    pub fn diagnose(snapshot: &WorktreeDoctorSnapshot) -> WorktreeDoctorReport {
        let mut summary = WorktreeDoctorSummary {
            expired_echoes: snapshot.reconciliation_summary.echo_expired,
            runtime_degraded: runtime_is_degraded(snapshot.runtime_status),
            ..WorktreeDoctorSummary::default()
        };
        let mut issues = Vec::new();
        for entry in &snapshot.reconciliation_entries {
            classify_entry(entry, &mut summary, &mut issues);
        }
        push_aggregate_issue(summary.expired_echoes > 0, WorktreeDoctorIssueKind::ExpiredEcho, &mut issues);
        push_aggregate_issue(summary.partial_scan, WorktreeDoctorIssueKind::PartialScan, &mut issues);
        push_aggregate_issue(summary.runtime_degraded, WorktreeDoctorIssueKind::RuntimeDegraded, &mut issues);
        issues.sort_by(|left, right| {
            left.vault_path
                .as_ref()
                .map(VaultPath::as_str)
                .cmp(&right.vault_path.as_ref().map(VaultPath::as_str))
                .then_with(|| left.kind.cmp(&right.kind))
        });
        WorktreeDoctorReport {
            health: health_for(summary),
            summary,
            issues,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorktreeDoctorRunner;

impl WorktreeDoctorRunner {
    pub fn run<S>(source: &mut S) -> Result<WorktreeDoctorReport, S::Error>
    where
        S: WorktreeDoctorFactSource,
    {
        source
            .load_snapshot()
            .map(|snapshot| WorktreeDoctor::diagnose(&snapshot))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorktreeRepairActionKind {
    Rescan,
    RestoreMissingFromCore,
    RestoreAuthoritativeContent,
    MoveReservedEntry,
    LeaveSkippedEntryUnchanged,
    ClearStaleEchoMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorktreeRepairRisk {
    None,
    Overwrite,
    Move,
    Trash,
    Delete,
}

impl WorktreeRepairRisk {
    #[must_use]
    pub const fn requires_confirmation(self) -> bool {
        !matches!(self, Self::None)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeRepairAction {
    pub kind: WorktreeRepairActionKind,
    pub issue_kind: WorktreeDoctorIssueKind,
    pub vault_path: Option<VaultPath>,
    pub risk: WorktreeRepairRisk,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorktreeRepairPlan {
    actions: Vec<WorktreeRepairAction>,
}

impl WorktreeRepairPlan {
    #[must_use]
    pub fn actions(&self) -> &[WorktreeRepairAction] {
        &self.actions
    }

    #[must_use]
    pub fn confirmation_required(&self) -> usize {
        self.actions
            .iter()
            .filter(|action| action.risk.requires_confirmation())
            .count()
    }

    #[must_use]
    pub fn requires_confirmation(&self) -> bool {
        self.confirmation_required() > 0
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorktreeRepairPlanner;

impl WorktreeRepairPlanner {
    #[must_use]
    pub fn plan(report: &WorktreeDoctorReport) -> WorktreeRepairPlan {
        WorktreeRepairPlan {
            actions: report.issues.iter().map(repair_for_issue).collect(),
        }
    }
}

fn classify_entry(
    entry: &WorktreeReconciliationEntry,
    summary: &mut WorktreeDoctorSummary,
    issues: &mut Vec<WorktreeDoctorIssue>,
) {
    let issue_kind = match entry.kind {
        WorktreeReconciliationKind::Clean
        | WorktreeReconciliationKind::Extra
        | WorktreeReconciliationKind::ConflictMaterialized => None,
        WorktreeReconciliationKind::Missing => {
            summary.missing_files += 1;
            Some(WorktreeDoctorIssueKind::MissingFile)
        }
        WorktreeReconciliationKind::Dirty => {
            summary.dirty_files += 1;
            if entry.expected_content_hash.is_some()
                && entry.observed_content_hash.is_some()
                && entry.expected_content_hash != entry.observed_content_hash
            {
                summary.content_hash_mismatches += 1;
                Some(WorktreeDoctorIssueKind::ContentHashMismatch)
            } else {
                Some(WorktreeDoctorIssueKind::DirtyFile)
            }
        }
        WorktreeReconciliationKind::Skipped => classify_skip(entry, summary),
    };
    if let Some(kind) = issue_kind {
        issues.push(WorktreeDoctorIssue {
            kind,
            vault_path: entry.vault_path.clone(),
        });
    }
    if entry.echo_status == WorktreeEchoStatus::Stale {
        summary.stale_echoes += 1;
        issues.push(WorktreeDoctorIssue {
            kind: WorktreeDoctorIssueKind::StaleEcho,
            vault_path: entry.vault_path.clone(),
        });
    }
}

fn classify_skip(
    entry: &WorktreeReconciliationEntry,
    summary: &mut WorktreeDoctorSummary,
) -> Option<WorktreeDoctorIssueKind> {
    match entry.skip_reason? {
        WorktreeScanSkipReason::ReservedPath => {
            entry.vault_path.as_ref()?;
            summary.reserved_path_violations += 1;
            Some(WorktreeDoctorIssueKind::ReservedPathViolation)
        }
        WorktreeScanSkipReason::Symlink => {
            summary.skipped_symlinks += 1;
            Some(WorktreeDoctorIssueKind::SkippedSymlink)
        }
        WorktreeScanSkipReason::SpecialFile => {
            summary.skipped_special_files += 1;
            Some(WorktreeDoctorIssueKind::SkippedSpecialFile)
        }
        WorktreeScanSkipReason::UnsafePath => {
            summary.skipped_unsafe_paths += 1;
            summary.partial_scan = true;
            Some(WorktreeDoctorIssueKind::SkippedUnsafePath)
        }
        WorktreeScanSkipReason::FilesystemError
        | WorktreeScanSkipReason::ReadError
        | WorktreeScanSkipReason::UnstableFile => {
            summary.skipped_filesystem_entries += 1;
            summary.partial_scan = true;
            Some(WorktreeDoctorIssueKind::SkippedFilesystemEntry)
        }
    }
}

fn push_aggregate_issue(
    present: bool,
    kind: WorktreeDoctorIssueKind,
    issues: &mut Vec<WorktreeDoctorIssue>,
) {
    if present {
        issues.push(WorktreeDoctorIssue {
            kind,
            vault_path: None,
        });
    }
}

fn runtime_is_degraded(status: WorktreeRuntimeStatus) -> bool {
    matches!(
        status.watcher,
        WorktreeRuntimeWatcherState::Closed | WorktreeRuntimeWatcherState::Failed(_)
    ) || status.lifecycle == WorktreeRuntimeLifecycle::Cancelling
}

fn health_for(summary: WorktreeDoctorSummary) -> WorktreeDoctorHealth {
    if summary.missing_files > 0 || summary.dirty_files > 0 || summary.reserved_path_violations > 0 {
        WorktreeDoctorHealth::Unhealthy
    } else if summary.issue_count() > 0 {
        WorktreeDoctorHealth::Degraded
    } else {
        WorktreeDoctorHealth::Healthy
    }
}

fn repair_for_issue(issue: &WorktreeDoctorIssue) -> WorktreeRepairAction {
    let (kind, risk) = match issue.kind {
        WorktreeDoctorIssueKind::MissingFile => (
            WorktreeRepairActionKind::RestoreMissingFromCore,
            WorktreeRepairRisk::None,
        ),
        WorktreeDoctorIssueKind::DirtyFile | WorktreeDoctorIssueKind::ContentHashMismatch => (
            WorktreeRepairActionKind::RestoreAuthoritativeContent,
            WorktreeRepairRisk::Overwrite,
        ),
        WorktreeDoctorIssueKind::ReservedPathViolation => (
            WorktreeRepairActionKind::MoveReservedEntry,
            WorktreeRepairRisk::Move,
        ),
        WorktreeDoctorIssueKind::SkippedSymlink
        | WorktreeDoctorIssueKind::SkippedSpecialFile
        | WorktreeDoctorIssueKind::SkippedUnsafePath
        | WorktreeDoctorIssueKind::SkippedFilesystemEntry => (
            WorktreeRepairActionKind::LeaveSkippedEntryUnchanged,
            WorktreeRepairRisk::None,
        ),
        WorktreeDoctorIssueKind::StaleEcho => (
            WorktreeRepairActionKind::ClearStaleEchoMetadata,
            WorktreeRepairRisk::Delete,
        ),
        WorktreeDoctorIssueKind::ExpiredEcho
        | WorktreeDoctorIssueKind::PartialScan
        | WorktreeDoctorIssueKind::RuntimeDegraded => {
            (WorktreeRepairActionKind::Rescan, WorktreeRepairRisk::None)
        }
    };
    WorktreeRepairAction {
        kind,
        issue_kind: issue.kind,
        vault_path: issue.vault_path.clone(),
        risk,
    }
}
