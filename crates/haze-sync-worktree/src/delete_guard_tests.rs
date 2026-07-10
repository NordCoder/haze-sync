use crate::*;
use haze_sync_common::{ContentHash, RevisionId, VaultPath};
use std::collections::VecDeque;

fn path(input: &str) -> VaultPath {
    VaultPath::parse(input).unwrap()
}

fn revision(input: &str) -> RevisionId {
    RevisionId::parse(input).unwrap()
}

fn hash(byte: u8) -> ContentHash {
    ContentHash::from_bytes([byte; 32])
}

fn snapshot(vault_path: &str) -> WorktreeFileSnapshot {
    WorktreeFileSnapshot {
        vault_path: path(vault_path),
        size: 1,
        modified: None,
        content_hash: hash(1),
        stability: StableFileState::stable(),
    }
}

#[test]
fn scan_derives_known_base_candidates_and_respects_skipped_prefixes() {
    let mut state = WorktreeStateSnapshot::new();
    state.record_present(path("present.md"), revision("rev_present"), hash(1));
    state.record_present(path("deleted.md"), revision("rev_deleted"), hash(2));
    state.record_present(
        path("blocked/child.md"),
        revision("rev_blocked"),
        hash(3),
    );
    state.record_tombstoned(path("already.md"), revision("rev_tombstone"));
    let scan = WorktreeScanResult {
        files: vec![snapshot("present.md")],
        skipped: vec![WorktreeScanSkipped {
            vault_path: Some(path("blocked")),
            reason: WorktreeScanSkipReason::FilesystemError,
        }],
    };

    let delete_scan = WorktreeDeleteScan::from_scan(&state, &scan).unwrap();

    assert!(delete_scan.is_complete());
    assert_eq!(delete_scan.tracked_present_count(), 3);
    assert_eq!(delete_scan.candidates().len(), 1);
    assert_eq!(delete_scan.candidates()[0].vault_path, path("deleted.md"));
    assert_eq!(
        delete_scan.candidates()[0].base_revision,
        WorktreeBaseRevision::Known(revision("rev_deleted"))
    );
}

#[test]
fn unscoped_skip_blocks_negative_delete_conclusions_and_client_calls() {
    let mut state = WorktreeStateSnapshot::new();
    state.record_present(path("tracked.md"), revision("rev_tracked"), hash(1));
    let scan = WorktreeScanResult {
        files: Vec::new(),
        skipped: vec![WorktreeScanSkipped {
            vault_path: None,
            reason: WorktreeScanSkipReason::FilesystemError,
        }],
    };
    let delete_scan = WorktreeDeleteScan::from_scan(&state, &scan).unwrap();
    let policy = WorktreeDeleteGuardPolicy::new(20, 500).unwrap();
    let plan = WorktreeGuardedDeletePlan::evaluate(
        delete_scan,
        policy,
        WorktreeDeleteAuthorization::ManualUnlock,
    );
    let mut client = FakeClient::new([]);

    let error = WorktreeDeleteRunner::submit(&mut state, plan, &mut client).unwrap_err();

    assert!(matches!(
        error,
        WorktreeDeleteRunError::Blocked {
            reason: WorktreeDeleteBlockReason::IncompleteScan,
            ..
        }
    ));
    assert!(client.received.is_empty());
    assert!(matches!(
        state.path_state(&path("tracked.md")),
        Some(WorktreeAppliedPathState::Present(_))
    ));
}

#[test]
fn ratio_and_count_thresholds_require_explicit_manual_unlock() {
    let ratio_candidates = (0..6).map(|index| {
        WorktreeDeleteCandidate::new(
            path(&format!("ratio-{index}.md")),
            WorktreeBaseRevision::Known(revision(&format!("rev_ratio_{index}"))),
        )
    });
    let ratio_scan = WorktreeDeleteScan::from_candidates(ratio_candidates, 100).unwrap();
    let ratio_policy = WorktreeDeleteGuardPolicy::new(20, 500).unwrap();
    let guarded = WorktreeGuardedDeletePlan::evaluate(
        ratio_scan.clone(),
        ratio_policy,
        WorktreeDeleteAuthorization::Guarded,
    );

    assert!(matches!(
        guarded.decision(),
        WorktreeDeleteGuardDecision::Blocked {
            reason: WorktreeDeleteBlockReason::ThresholdExceeded,
            summary: WorktreeDeleteGuardSummary {
                delete_ratio_basis_points: 600,
                ratio_exceeded: true,
                count_exceeded: false,
                ..
            }
        }
    ));

    let unlocked = WorktreeGuardedDeletePlan::evaluate(
        ratio_scan,
        ratio_policy,
        WorktreeDeleteAuthorization::ManualUnlock,
    );
    assert!(matches!(
        unlocked.decision(),
        WorktreeDeleteGuardDecision::Allowed(WorktreeDeleteGuardSummary {
            manual_unlock_used: true,
            ..
        })
    ));

    let count_candidates = (0..21).map(|index| {
        WorktreeDeleteCandidate::new(
            path(&format!("count-{index}.md")),
            WorktreeBaseRevision::Known(revision(&format!("rev_count_{index}"))),
        )
    });
    let count_scan = WorktreeDeleteScan::from_candidates(count_candidates, 100).unwrap();
    let count_policy = WorktreeDeleteGuardPolicy::new(20, 10_000).unwrap();
    let count_plan = WorktreeGuardedDeletePlan::evaluate(
        count_scan,
        count_policy,
        WorktreeDeleteAuthorization::Guarded,
    );
    assert!(matches!(
        count_plan.decision(),
        WorktreeDeleteGuardDecision::Blocked {
            summary: WorktreeDeleteGuardSummary {
                count_exceeded: true,
                ratio_exceeded: false,
                ..
            },
            ..
        }
    ));
}

#[test]
fn runner_preserves_known_and_null_base_semantics_and_updates_only_tombstones() {
    let known = WorktreeDeleteCandidate::new(
        path("known.md"),
        WorktreeBaseRevision::Known(revision("rev_known")),
    );
    let null = WorktreeDeleteCandidate::new(path("null.md"), WorktreeBaseRevision::Null);
    let scan = WorktreeDeleteScan::from_candidates([known, null], 2).unwrap();
    let policy = WorktreeDeleteGuardPolicy::new(2, 10_000).unwrap();
    let plan = WorktreeGuardedDeletePlan::evaluate(
        scan,
        policy,
        WorktreeDeleteAuthorization::Guarded,
    );
    let mut state = WorktreeStateSnapshot::new();
    state.record_present(path("known.md"), revision("rev_known"), hash(1));
    let mut client = FakeClient::new([
        WorktreeImportOutcome::Tombstoned(WorktreeTombstoneState {
            revision_id: revision("rev_known_tombstone"),
        }),
        WorktreeImportOutcome::Rejected(WorktreeImportRejection::InvalidBase),
    ]);

    let report = WorktreeDeleteRunner::submit(&mut state, plan, &mut client).unwrap();

    assert_eq!(report.submissions().len(), 2);
    assert!(report.submissions()[0].local_state_updated);
    assert!(!report.submissions()[1].local_state_updated);
    assert!(matches!(
        &client.received[0],
        WorktreeImportAction::Delete(delete)
            if delete.vault_path == path("known.md")
                && delete.base_revision
                    == WorktreeBaseRevision::Known(revision("rev_known"))
    ));
    assert!(matches!(
        &client.received[1],
        WorktreeImportAction::Delete(delete)
            if delete.vault_path == path("null.md")
                && delete.base_revision == WorktreeBaseRevision::Null
    ));
    assert!(matches!(
        state.path_state(&path("known.md")),
        Some(WorktreeAppliedPathState::Tombstoned(tombstone))
            if tombstone.revision_id == revision("rev_known_tombstone")
    ));
    assert!(state.path_state(&path("null.md")).is_none());
}

#[test]
fn duplicate_candidates_and_invalid_ratio_are_rejected() {
    let candidate = WorktreeDeleteCandidate::new(path("dup.md"), WorktreeBaseRevision::Null);
    assert_eq!(
        WorktreeDeleteScan::from_candidates([candidate.clone(), candidate], 0).unwrap_err(),
        WorktreeDeletePlanError::DuplicateCandidate {
            vault_path: path("dup.md")
        }
    );
    assert_eq!(
        WorktreeDeleteGuardPolicy::new(20, 10_001).unwrap_err(),
        WorktreeDeleteGuardPolicyError::InvalidRatio
    );
}

struct FakeClient {
    outcomes: VecDeque<WorktreeImportOutcome>,
    received: Vec<WorktreeImportAction>,
}

impl FakeClient {
    fn new(outcomes: impl IntoIterator<Item = WorktreeImportOutcome>) -> Self {
        Self {
            outcomes: outcomes.into_iter().collect(),
            received: Vec::new(),
        }
    }
}

impl WorktreeImportClient for FakeClient {
    type Error = &'static str;

    fn submit(
        &mut self,
        action: WorktreeImportAction,
    ) -> Result<WorktreeImportOutcome, Self::Error> {
        self.received.push(action);
        self.outcomes.pop_front().ok_or("missing fake outcome")
    }
}
