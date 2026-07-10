use crate::*;
use haze_sync_common::{ContentHash, RevisionId, VaultPath};
use std::time::{Duration, SystemTime};

fn path(input: &str) -> VaultPath {
    VaultPath::parse(input).unwrap()
}

fn revision(input: &str) -> RevisionId {
    RevisionId::parse(input).unwrap()
}

fn hash(byte: u8) -> ContentHash {
    ContentHash::from_bytes([byte; 32])
}

fn snapshot(
    vault_path: &str,
    content_hash: ContentHash,
    size: u64,
    modified: SystemTime,
) -> WorktreeFileSnapshot {
    WorktreeFileSnapshot {
        vault_path: path(vault_path),
        size,
        modified: Some(modified),
        content_hash,
        stability: StableFileState::stable(),
    }
}

fn marker(vault_path: &str, revision_id: &str, content_hash: ContentHash) -> WorktreeEchoMarker {
    WorktreeEchoMarker {
        vault_path: path(vault_path),
        revision_id: revision(revision_id),
        content_hash,
    }
}

fn entry<'a>(
    report: &'a WorktreeReconciliationReport,
    vault_path: &str,
) -> &'a WorktreeReconciliationEntry {
    report
        .entries()
        .iter()
        .find(|entry| {
            entry
                .vault_path
                .as_ref()
                .is_some_and(|path| path.as_str() == vault_path)
        })
        .unwrap()
}

#[test]
fn classifies_all_drift_kinds_and_updates_only_clean_observations() {
    let base_time = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
    let current_time = base_time + Duration::from_secs(30);
    let mut applied = WorktreeStateSnapshot::new();
    applied.record_present(path("clean.md"), revision("rev_clean"), hash(1));
    applied.record_present(path("dirty.md"), revision("rev_dirty"), hash(2));
    applied.record_present(path("missing.md"), revision("rev_missing"), hash(3));
    applied.record_tombstoned(path("extra.md"), revision("rev_extra_tombstone"));
    applied.record_present(
        path("_haze_conflicts/open/plan.md"),
        revision("rev_conflict"),
        hash(5),
    );
    applied.record_present(
        path("blocked/child.md"),
        revision("rev_blocked"),
        hash(6),
    );

    let mut state = WorktreeReconciliationState::new(applied);
    state.record_observation(
        path("clean.md"),
        WorktreeObservedFileState {
            revision_id: revision("rev_clean"),
            content_hash: hash(1),
            size: 10,
            modified: Some(base_time),
        },
    );

    let scan = WorktreeScanResult {
        files: vec![
            snapshot("clean.md", hash(1), 10, current_time),
            snapshot("dirty.md", hash(9), 11, current_time),
            snapshot("extra.md", hash(4), 12, current_time),
            snapshot(
                "_haze_conflicts/open/plan.md",
                hash(5),
                13,
                current_time,
            ),
        ],
        skipped: vec![WorktreeScanSkipped {
            vault_path: Some(path("blocked")),
            reason: WorktreeScanSkipReason::FilesystemError,
        }],
    };

    let policy = WorktreeEchoGuardPolicy::new(Duration::from_secs(3600), 16).unwrap();
    let mut guard = WorktreeEchoGuard::new(policy);
    guard
        .record(
            marker("clean.md", "rev_clean", hash(1)),
            base_time,
            base_time,
        )
        .unwrap();
    guard
        .record(
            marker("dirty.md", "rev_dirty", hash(2)),
            base_time,
            base_time,
        )
        .unwrap();
    guard
        .record(
            marker("expired.md", "rev_expired", hash(8)),
            SystemTime::UNIX_EPOCH,
            SystemTime::UNIX_EPOCH,
        )
        .unwrap();

    let report = WorktreeReconciler::reconcile(&mut state, &scan, &mut guard, current_time)
        .unwrap();
    let summary = report.summary();

    assert_eq!(summary.clean, 1);
    assert_eq!(summary.dirty, 1);
    assert_eq!(summary.missing, 1);
    assert_eq!(summary.extra, 1);
    assert_eq!(summary.conflict_materialized, 1);
    assert_eq!(summary.skipped, 1);
    assert_eq!(summary.echo_suppressed, 1);
    assert_eq!(summary.echo_stale, 1);
    assert_eq!(summary.echo_expired, 1);
    assert_eq!(summary.total_entries(), 6);

    assert_eq!(
        entry(&report, "clean.md").kind,
        WorktreeReconciliationKind::Clean
    );
    assert_eq!(
        entry(&report, "clean.md").echo_status,
        WorktreeEchoStatus::Suppressed
    );
    assert_eq!(
        entry(&report, "clean.md").modification_facts_match,
        Some(false)
    );
    assert_eq!(
        entry(&report, "dirty.md").kind,
        WorktreeReconciliationKind::Dirty
    );
    assert_eq!(
        entry(&report, "dirty.md").echo_status,
        WorktreeEchoStatus::Stale
    );
    assert_eq!(
        entry(&report, "missing.md").kind,
        WorktreeReconciliationKind::Missing
    );
    assert_eq!(
        entry(&report, "extra.md").kind,
        WorktreeReconciliationKind::Extra
    );
    assert_eq!(
        entry(&report, "_haze_conflicts/open/plan.md").kind,
        WorktreeReconciliationKind::ConflictMaterialized
    );
    assert!(report
        .entries()
        .iter()
        .all(|entry| entry.vault_path.as_ref().is_none_or(|path| path.as_str() != "blocked/child.md")));

    assert_eq!(report.transitions().len(), 2);
    assert_eq!(
        state.observation(&path("clean.md")).unwrap().modified,
        Some(current_time)
    );
    assert!(state.observation(&path("dirty.md")).is_none());
    assert!(state.observation(&path("missing.md")).is_none());
}

#[test]
fn duplicate_stable_scan_path_is_rejected_before_reconciliation() {
    let observed = SystemTime::UNIX_EPOCH + Duration::from_secs(10);
    let scan = WorktreeScanResult {
        files: vec![
            snapshot("dup.md", hash(1), 1, observed),
            snapshot("dup.md", hash(2), 2, observed),
        ],
        skipped: Vec::new(),
    };
    let policy = WorktreeEchoGuardPolicy::new(Duration::from_secs(60), 4).unwrap();
    let mut guard = WorktreeEchoGuard::new(policy);
    let mut state = WorktreeReconciliationState::default();

    assert_eq!(
        WorktreeReconciler::reconcile(&mut state, &scan, &mut guard, observed).unwrap_err(),
        WorktreeReconciliationError::DuplicateScanPath {
            vault_path: path("dup.md")
        }
    );
}

#[test]
fn persisted_runner_saves_only_real_observation_transitions() {
    let observed = SystemTime::UNIX_EPOCH + Duration::from_secs(20);
    let mut applied = WorktreeStateSnapshot::new();
    applied.record_present(path("a.md"), revision("rev_a"), hash(1));
    let mut store = FakeStore {
        state: WorktreeReconciliationState::new(applied),
        saves: 0,
    };
    let scan = WorktreeScanResult {
        files: vec![snapshot("a.md", hash(1), 4, observed)],
        skipped: Vec::new(),
    };
    let policy = WorktreeEchoGuardPolicy::new(Duration::from_secs(60), 4).unwrap();
    let mut guard = WorktreeEchoGuard::new(policy);

    let first = WorktreeReconciliationRunner::run(&mut store, &scan, &mut guard, observed).unwrap();
    assert!(first.state_changed());
    assert_eq!(store.saves, 1);

    let second = WorktreeReconciliationRunner::run(&mut store, &scan, &mut guard, observed).unwrap();
    assert!(!second.state_changed());
    assert_eq!(store.saves, 1);
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FakeStore {
    state: WorktreeReconciliationState,
    saves: usize,
}

impl WorktreeReconciliationStateStore for FakeStore {
    type Error = &'static str;

    fn load_state(&mut self) -> Result<WorktreeReconciliationState, Self::Error> {
        Ok(self.state.clone())
    }

    fn save_state(&mut self, state: &WorktreeReconciliationState) -> Result<(), Self::Error> {
        self.state = state.clone();
        self.saves += 1;
        Ok(())
    }
}
