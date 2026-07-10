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

fn stable_file(vault_path: &str, content_hash: ContentHash, bytes: &[u8]) -> WorktreeImportFile {
    WorktreeImportFile::new(
        WorktreeFileSnapshot {
            vault_path: path(vault_path),
            size: bytes.len() as u64,
            modified: None,
            content_hash,
            stability: StableFileState::stable(),
        },
        bytes.to_vec(),
    )
    .unwrap()
}

#[test]
fn public_file_import_plan_never_infers_delete_actions() {
    let mut state = WorktreeStateSnapshot::new();
    state.record_present(path("missing.md"), revision("rev_missing"), hash(1));
    state.record_present(path("same.md"), revision("rev_same"), hash(2));

    let plan =
        WorktreeImportPlanner::plan(&state, [stable_file("same.md", hash(2), b"same")]).unwrap();

    assert!(plan.actions().is_empty());
    assert_eq!(plan.unchanged().len(), 1);
    assert!(plan
        .actions()
        .iter()
        .all(|action| matches!(action, WorktreeImportAction::Put(_))));
}

#[test]
fn file_imports_preserve_known_null_and_tombstone_base_semantics() {
    let mut state = WorktreeStateSnapshot::new();
    state.record_present(path("modified.md"), revision("rev_modified"), hash(1));
    state.record_tombstoned(path("restored.md"), revision("rev_deleted"));

    let plan = WorktreeImportPlanner::plan(
        &state,
        [
            stable_file("modified.md", hash(2), b"modified"),
            stable_file("new.md", hash(3), b"new"),
            stable_file("restored.md", hash(4), b"restored"),
        ],
    )
    .unwrap();

    assert_eq!(plan.actions().len(), 3);
    assert!(matches!(
        &plan.actions()[0],
        WorktreeImportAction::Put(put)
            if put.vault_path == path("modified.md")
                && put.base_revision
                    == WorktreeBaseRevision::Known(revision("rev_modified"))
    ));
    assert!(matches!(
        &plan.actions()[1],
        WorktreeImportAction::Put(put)
            if put.vault_path == path("new.md")
                && put.base_revision == WorktreeBaseRevision::Null
    ));
    assert!(matches!(
        &plan.actions()[2],
        WorktreeImportAction::Put(put)
            if put.vault_path == path("restored.md")
                && put.base_revision
                    == WorktreeBaseRevision::Known(revision("rev_deleted"))
    ));
}

#[test]
fn file_import_runner_updates_state_only_for_accepted_put_outcomes() {
    let mut state = WorktreeStateSnapshot::new();
    let plan = WorktreeImportPlanner::plan(
        &state,
        [
            stable_file("accepted.md", hash(1), b"accepted"),
            stable_file("rejected.md", hash(2), b"rejected"),
        ],
    )
    .unwrap();
    let mut client = FakeClient::new([
        WorktreeImportOutcome::Accepted(WorktreeAcceptedImport {
            revision_id: revision("rev_accepted"),
            content_hash: hash(1),
        }),
        WorktreeImportOutcome::Rejected(WorktreeImportRejection::InvalidBase),
    ]);

    let report = WorktreeImportRunner::submit_plan(&mut state, plan, &mut client).unwrap();

    assert_eq!(report.submissions().len(), 2);
    assert!(report.submissions()[0].local_state_updated);
    assert!(!report.submissions()[1].local_state_updated);
    assert!(matches!(
        state.path_state(&path("accepted.md")),
        Some(WorktreeAppliedPathState::Present(applied))
            if applied.revision_id == revision("rev_accepted")
                && applied.content_hash == hash(1)
    ));
    assert!(state.path_state(&path("rejected.md")).is_none());
    assert!(client
        .received
        .iter()
        .all(|action| matches!(action, WorktreeImportAction::Put(_))));
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
