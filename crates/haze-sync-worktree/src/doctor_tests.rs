use crate::*;
use haze_sync_common::{ContentHash, VaultPath};

fn path(input: &str) -> VaultPath {
    VaultPath::parse(input).unwrap()
}

fn hash(byte: u8) -> ContentHash {
    ContentHash::from_bytes([byte; 32])
}

fn runtime_status(watcher: WorktreeRuntimeWatcherState) -> WorktreeRuntimeStatus {
    WorktreeRuntimeStatus {
        lifecycle: WorktreeRuntimeLifecycle::Running,
        mode: WorktreeMode::Bidirectional,
        watcher,
        startup_cycle_pending: false,
        cycle_in_progress: false,
        pending_watcher_hints: 0,
        watcher_hints_observed: 0,
        cycles_completed: 1,
        cycles_failed: 0,
        last_cycle_cause: Some(WorktreeRuntimeCycleCause::Periodic),
        last_cycle: Some(WorktreeRuntimeLastCycle::Completed(
            WorktreeRuntimeCycleSummary {
                full_scan_completed: true,
                ..WorktreeRuntimeCycleSummary::default()
            },
        )),
    }
}

fn entry(
    vault_path: Option<&str>,
    kind: WorktreeReconciliationKind,
    expected: Option<ContentHash>,
    observed: Option<ContentHash>,
    echo_status: WorktreeEchoStatus,
    skip_reason: Option<WorktreeScanSkipReason>,
) -> WorktreeReconciliationEntry {
    WorktreeReconciliationEntry {
        vault_path: vault_path.map(path),
        kind,
        expected_revision: None,
        expected_content_hash: expected,
        observed_content_hash: observed,
        expected_size: None,
        observed_size: None,
        expected_modified: None,
        observed_modified: None,
        modification_facts_match: None,
        echo_status,
        skip_reason,
    }
}

fn snapshot(entries: Vec<WorktreeReconciliationEntry>) -> WorktreeDoctorSnapshot {
    WorktreeDoctorSnapshot {
        reconciliation_entries: entries,
        reconciliation_summary: WorktreeReconciliationSummary::default(),
        runtime_status: runtime_status(WorktreeRuntimeWatcherState::Running),
    }
}

#[test]
fn healthy_snapshot_has_no_issues_or_repairs() {
    let report = WorktreeDoctor::diagnose(&snapshot(vec![
        entry(
            Some("clean.md"),
            WorktreeReconciliationKind::Clean,
            Some(hash(1)),
            Some(hash(1)),
            WorktreeEchoStatus::NoMarker,
            None,
        ),
        entry(
            None,
            WorktreeReconciliationKind::Skipped,
            None,
            None,
            WorktreeEchoStatus::NotChecked,
            Some(WorktreeScanSkipReason::ReservedPath),
        ),
    ]));

    assert_eq!(report.health(), WorktreeDoctorHealth::Healthy);
    assert_eq!(report.summary().issue_count(), 0);
    assert!(report.issues().is_empty());
    assert!(WorktreeRepairPlanner::plan(&report).actions().is_empty());
}

#[test]
fn doctor_classifies_missing_dirty_and_hash_mismatch_separately() {
    let report = WorktreeDoctor::diagnose(&snapshot(vec![
        entry(
            Some("missing.md"),
            WorktreeReconciliationKind::Missing,
            Some(hash(1)),
            None,
            WorktreeEchoStatus::NotChecked,
            None,
        ),
        entry(
            Some("dirty.md"),
            WorktreeReconciliationKind::Dirty,
            None,
            None,
            WorktreeEchoStatus::NoMarker,
            None,
        ),
        entry(
            Some("mismatch.md"),
            WorktreeReconciliationKind::Dirty,
            Some(hash(2)),
            Some(hash(3)),
            WorktreeEchoStatus::NoMarker,
            None,
        ),
    ]));

    let summary = report.summary();
    assert_eq!(report.health(), WorktreeDoctorHealth::Unhealthy);
    assert_eq!(summary.missing_files, 1);
    assert_eq!(summary.dirty_files, 2);
    assert_eq!(summary.content_hash_mismatches, 1);
    assert!(report.issues().iter().any(|issue| {
        issue.kind == WorktreeDoctorIssueKind::ContentHashMismatch
            && issue.vault_path.as_ref() == Some(&path("mismatch.md"))
    }));
}

#[test]
fn reserved_symlink_special_and_partial_scan_facts_are_safe_and_counted() {
    let report = WorktreeDoctor::diagnose(&snapshot(vec![
        entry(
            Some("reserved-collision"),
            WorktreeReconciliationKind::Skipped,
            None,
            None,
            WorktreeEchoStatus::NotChecked,
            Some(WorktreeScanSkipReason::ReservedPath),
        ),
        entry(
            None,
            WorktreeReconciliationKind::Skipped,
            None,
            None,
            WorktreeEchoStatus::NotChecked,
            Some(WorktreeScanSkipReason::ReservedPath),
        ),
        entry(
            Some("link.md"),
            WorktreeReconciliationKind::Skipped,
            None,
            None,
            WorktreeEchoStatus::NotChecked,
            Some(WorktreeScanSkipReason::Symlink),
        ),
        entry(
            Some("device"),
            WorktreeReconciliationKind::Skipped,
            None,
            None,
            WorktreeEchoStatus::NotChecked,
            Some(WorktreeScanSkipReason::SpecialFile),
        ),
        entry(
            None,
            WorktreeReconciliationKind::Skipped,
            None,
            None,
            WorktreeEchoStatus::NotChecked,
            Some(WorktreeScanSkipReason::FilesystemError),
        ),
    ]));

    let summary = report.summary();
    assert_eq!(summary.reserved_path_violations, 1);
    assert_eq!(summary.skipped_symlinks, 1);
    assert_eq!(summary.skipped_special_files, 1);
    assert_eq!(summary.skipped_filesystem_entries, 1);
    assert!(summary.partial_scan);
    assert!(report.issues().iter().all(|issue| {
        issue
            .vault_path
            .as_ref()
            .map_or(true, |vault_path| !vault_path.as_str().starts_with('/'))
    }));
}

#[test]
fn stale_and_expired_echoes_and_degraded_runtime_are_reported() {
    let mut snapshot = snapshot(vec![entry(
        Some("stale.md"),
        WorktreeReconciliationKind::Clean,
        Some(hash(1)),
        Some(hash(1)),
        WorktreeEchoStatus::Stale,
        None,
    )]);
    snapshot.reconciliation_summary.echo_expired = 2;
    snapshot.runtime_status = runtime_status(WorktreeRuntimeWatcherState::Failed(
        WorktreeWatcherFailure::Poll,
    ));

    let report = WorktreeDoctor::diagnose(&snapshot);

    assert_eq!(report.health(), WorktreeDoctorHealth::Degraded);
    assert_eq!(report.summary().stale_echoes, 1);
    assert_eq!(report.summary().expired_echoes, 2);
    assert!(report.summary().runtime_degraded);
}

#[test]
fn repair_plan_never_carries_execution_authority() {
    let mut doctor_snapshot = snapshot(vec![
        entry(
            Some("missing.md"),
            WorktreeReconciliationKind::Missing,
            Some(hash(1)),
            None,
            WorktreeEchoStatus::NotChecked,
            None,
        ),
        entry(
            Some("dirty.md"),
            WorktreeReconciliationKind::Dirty,
            Some(hash(1)),
            Some(hash(2)),
            WorktreeEchoStatus::Stale,
            None,
        ),
        entry(
            Some("reserved"),
            WorktreeReconciliationKind::Skipped,
            None,
            None,
            WorktreeEchoStatus::NotChecked,
            Some(WorktreeScanSkipReason::ReservedPath),
        ),
    ]);
    doctor_snapshot.reconciliation_summary.echo_expired = 1;
    let report = WorktreeDoctor::diagnose(&doctor_snapshot);

    let plan = WorktreeRepairPlanner::plan(&report);

    assert_eq!(plan.confirmation_required(), 3);
    assert!(plan.requires_confirmation());
    assert!(plan.actions().iter().any(|action| {
        action.kind == WorktreeRepairActionKind::RestoreMissingFromCore
            && action.risk == WorktreeRepairRisk::None
    }));
    assert!(plan.actions().iter().any(|action| {
        action.issue_kind == WorktreeDoctorIssueKind::ExpiredEcho
            && action.kind == WorktreeRepairActionKind::Rescan
            && action.risk == WorktreeRepairRisk::None
            && action.vault_path.is_none()
    }));
    assert_eq!(
        plan.actions()
            .iter()
            .filter(|action| action.risk.requires_confirmation())
            .count(),
        plan.confirmation_required()
    );
}

#[test]
fn doctor_runner_uses_injected_fact_source() {
    struct Source(Option<WorktreeDoctorSnapshot>);

    impl WorktreeDoctorFactSource for Source {
        type Error = &'static str;

        fn load_snapshot(&mut self) -> Result<WorktreeDoctorSnapshot, Self::Error> {
            self.0.take().ok_or("snapshot already loaded")
        }
    }

    let mut source = Source(Some(snapshot(Vec::new())));
    let report = WorktreeDoctorRunner::run(&mut source).unwrap();

    assert_eq!(report.health(), WorktreeDoctorHealth::Healthy);
}
