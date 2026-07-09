//! Import planning boundary between local Worktree facts and Core/API submission.
//!
//! This module does not decide conflict policy and does not write to Storage.
//! It converts safe local scan facts into explicit import requests carrying a
//! known base revision or an explicit null base, then applies local state updates
//! only from Core/API outcomes that confirm the authoritative result.

use crate::WorktreeFileSnapshot;
use haze_sync_common::{ConflictId, ContentHash, RevisionId, VaultPath};
use std::collections::BTreeMap;

/// Last-applied Worktree knowledge for vault paths previously materialized from Core.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorktreeStateSnapshot {
    paths: BTreeMap<VaultPath, WorktreeAppliedPathState>,
}

impl WorktreeStateSnapshot {
    /// Create an empty Worktree state snapshot.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that a file path was last applied from an authoritative Core revision.
    pub fn record_present(
        &mut self,
        vault_path: VaultPath,
        revision_id: RevisionId,
        content_hash: ContentHash,
    ) {
        self.paths.insert(
            vault_path,
            WorktreeAppliedPathState::Present(WorktreeAppliedFileState {
                revision_id,
                content_hash,
            }),
        );
    }

    /// Record that a path is known to be tombstoned by an authoritative Core revision.
    pub fn record_tombstoned(&mut self, vault_path: VaultPath, revision_id: RevisionId) {
        self.paths.insert(
            vault_path,
            WorktreeAppliedPathState::Tombstoned(WorktreeTombstoneState { revision_id }),
        );
    }

    /// Return the known state for a vault path, if any.
    #[must_use]
    pub fn path_state(&self, vault_path: &VaultPath) -> Option<&WorktreeAppliedPathState> {
        self.paths.get(vault_path)
    }

    /// Iterate over known path states in deterministic vault-path order.
    pub fn iter(&self) -> impl Iterator<Item = (&VaultPath, &WorktreeAppliedPathState)> {
        self.paths.iter()
    }

    /// Number of known path states.
    #[must_use]
    pub fn len(&self) -> usize {
        self.paths.len()
    }

    /// Returns true when no path state is known.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }
}

/// Last-applied state for one vault path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorktreeAppliedPathState {
    /// A local file was last materialized from this Core revision and content hash.
    Present(WorktreeAppliedFileState),
    /// Core has accepted a tombstone for this path.
    Tombstoned(WorktreeTombstoneState),
}

impl WorktreeAppliedPathState {
    /// Authoritative Core revision currently known for this path state.
    #[must_use]
    pub fn revision_id(&self) -> &RevisionId {
        match self {
            Self::Present(file) => &file.revision_id,
            Self::Tombstoned(tombstone) => &tombstone.revision_id,
        }
    }
}

/// Last-applied present-file state for one vault path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeAppliedFileState {
    /// Authoritative Core revision that produced the local file.
    pub revision_id: RevisionId,
    /// Content hash that was materialized locally from Core.
    pub content_hash: ContentHash,
}

/// Last-applied tombstone state for one vault path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeTombstoneState {
    /// Authoritative Core tombstone revision.
    pub revision_id: RevisionId,
}

/// Stable local file bytes ready to be considered for import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeImportFile {
    /// Stable scanner fact for the local file.
    pub snapshot: WorktreeFileSnapshot,
    /// Bytes read for the stable local file import.
    pub bytes: Vec<u8>,
}

impl WorktreeImportFile {
    /// Build an import file only from a stable scanner fact.
    pub fn new(
        snapshot: WorktreeFileSnapshot,
        bytes: Vec<u8>,
    ) -> Result<Self, WorktreeImportPlanError> {
        if !snapshot.stability.is_stable() {
            return Err(WorktreeImportPlanError::UnstableLocalFile {
                vault_path: snapshot.vault_path,
            });
        }

        Ok(Self { snapshot, bytes })
    }

    fn vault_path(&self) -> &VaultPath {
        &self.snapshot.vault_path
    }
}

/// Planner failure with safe vault-relative context only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorktreeImportPlanError {
    /// Import planning received a local file that was not stable.
    UnstableLocalFile { vault_path: VaultPath },
    /// Import planning received more than one local fact for the same vault path.
    DuplicateLocalFile { vault_path: VaultPath },
}

/// Explicit base for a Core/API import request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorktreeBaseRevision {
    /// The file is based on a known Core revision.
    Known(RevisionId),
    /// The local file has no known Core base.
    Null,
}

/// A planned local change request to submit through the abstract Core/API boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorktreeImportAction {
    /// Create or update file content.
    Put(WorktreePutImport),
    /// Submit a local delete/tombstone candidate.
    Delete(WorktreeDeleteImport),
}

impl WorktreeImportAction {
    fn put(
        vault_path: VaultPath,
        base_revision: WorktreeBaseRevision,
        content_hash: ContentHash,
        bytes: Vec<u8>,
    ) -> Self {
        Self::Put(WorktreePutImport {
            vault_path,
            base_revision,
            content_hash,
            bytes,
        })
    }

    fn delete(vault_path: VaultPath, base_revision: WorktreeBaseRevision) -> Self {
        Self::Delete(WorktreeDeleteImport {
            vault_path,
            base_revision,
        })
    }

    /// Vault-relative path affected by this request.
    #[must_use]
    pub fn vault_path(&self) -> &VaultPath {
        match self {
            Self::Put(request) => &request.vault_path,
            Self::Delete(request) => &request.vault_path,
        }
    }

    /// Explicit base revision carried by this request.
    #[must_use]
    pub fn base_revision(&self) -> &WorktreeBaseRevision {
        match self {
            Self::Put(request) => &request.base_revision,
            Self::Delete(request) => &request.base_revision,
        }
    }
}

/// Planned file create/update import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreePutImport {
    /// Vault-relative path to write through Core/API.
    pub vault_path: VaultPath,
    /// Known base revision or explicit null base.
    pub base_revision: WorktreeBaseRevision,
    /// Hash of the stable local bytes.
    pub content_hash: ContentHash,
    /// Stable local bytes to submit.
    pub bytes: Vec<u8>,
}

/// Planned local delete import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeDeleteImport {
    /// Vault-relative path to tombstone through Core/API.
    pub vault_path: VaultPath,
    /// Known base revision for the file being deleted.
    pub base_revision: WorktreeBaseRevision,
}

/// Local files that matched the last applied Core state and need no request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeUnchangedFile {
    /// Vault-relative path that is unchanged.
    pub vault_path: VaultPath,
    /// Authoritative revision already represented locally.
    pub revision_id: RevisionId,
    /// Hash already represented locally.
    pub content_hash: ContentHash,
}

/// Deterministic import plan built from stable local facts and last-applied state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorktreeImportPlan {
    actions: Vec<WorktreeImportAction>,
    unchanged: Vec<WorktreeUnchangedFile>,
}

impl WorktreeImportPlan {
    /// Planned Core/API requests in deterministic vault-path order.
    #[must_use]
    pub fn actions(&self) -> &[WorktreeImportAction] {
        &self.actions
    }

    /// Local files that required no request because content matched known Core state.
    #[must_use]
    pub fn unchanged(&self) -> &[WorktreeUnchangedFile] {
        &self.unchanged
    }

    /// Returns true when there are no requests to submit.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }
}

/// Builds import plans without making Core/API calls.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorktreeImportPlanner;

impl WorktreeImportPlanner {
    /// Build an import plan from the last-applied state and stable local files.
    pub fn plan(
        state: &WorktreeStateSnapshot,
        local_files: impl IntoIterator<Item = WorktreeImportFile>,
    ) -> Result<WorktreeImportPlan, WorktreeImportPlanError> {
        let local_files = stable_files_by_path(local_files)?;
        let mut actions = Vec::new();
        let mut unchanged = Vec::new();

        for (vault_path, local_file) in &local_files {
            match state.path_state(vault_path) {
                None => actions.push(put_action(local_file, WorktreeBaseRevision::Null)),
                Some(WorktreeAppliedPathState::Tombstoned(tombstone)) => {
                    actions.push(put_action(
                        local_file,
                        WorktreeBaseRevision::Known(tombstone.revision_id.clone()),
                    ));
                }
                Some(WorktreeAppliedPathState::Present(applied)) => {
                    if applied.content_hash == local_file.snapshot.content_hash {
                        unchanged.push(WorktreeUnchangedFile {
                            vault_path: vault_path.clone(),
                            revision_id: applied.revision_id.clone(),
                            content_hash: applied.content_hash,
                        });
                    } else {
                        actions.push(put_action(
                            local_file,
                            WorktreeBaseRevision::Known(applied.revision_id.clone()),
                        ));
                    }
                }
            }
        }

        for (vault_path, path_state) in state.iter() {
            if local_files.contains_key(vault_path) {
                continue;
            }
            if let WorktreeAppliedPathState::Present(applied) = path_state {
                actions.push(WorktreeImportAction::delete(
                    vault_path.clone(),
                    WorktreeBaseRevision::Known(applied.revision_id.clone()),
                ));
            }
        }

        actions.sort_by(|left, right| left.vault_path().as_str().cmp(right.vault_path().as_str()));

        Ok(WorktreeImportPlan { actions, unchanged })
    }
}

fn stable_files_by_path(
    local_files: impl IntoIterator<Item = WorktreeImportFile>,
) -> Result<BTreeMap<VaultPath, WorktreeImportFile>, WorktreeImportPlanError> {
    let mut by_path = BTreeMap::new();
    for local_file in local_files {
        if !local_file.snapshot.stability.is_stable() {
            return Err(WorktreeImportPlanError::UnstableLocalFile {
                vault_path: local_file.snapshot.vault_path,
            });
        }

        let vault_path = local_file.vault_path().clone();
        if by_path.insert(vault_path.clone(), local_file).is_some() {
            return Err(WorktreeImportPlanError::DuplicateLocalFile { vault_path });
        }
    }
    Ok(by_path)
}

fn put_action(
    local_file: &WorktreeImportFile,
    base_revision: WorktreeBaseRevision,
) -> WorktreeImportAction {
    WorktreeImportAction::put(
        local_file.snapshot.vault_path.clone(),
        base_revision,
        local_file.snapshot.content_hash,
        local_file.bytes.clone(),
    )
}

/// Abstract Core/API boundary for submitting Worktree import requests.
pub trait WorktreeImportClient {
    /// Transport or client-level error.
    type Error;

    /// Submit one planned import request through Core/API.
    fn submit(
        &mut self,
        action: WorktreeImportAction,
    ) -> Result<WorktreeImportOutcome, Self::Error>;
}

/// Core/API result for one import request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorktreeImportOutcome {
    /// Core accepted new content and returned its authoritative revision.
    Accepted(WorktreeAcceptedImport),
    /// Core already had the same content and returned its authoritative revision.
    SameContent(WorktreeAcceptedImport),
    /// Core saved a conflict instead of accepting the local change.
    ConflictSaved { conflict_id: Option<ConflictId> },
    /// Core rejected the request without changing authoritative state.
    Rejected(WorktreeImportRejection),
    /// Core accepted a tombstone for a local delete.
    Tombstoned(WorktreeTombstoneState),
    /// Core could not find the requested base/path and made no accepted state update.
    NotFound,
}

/// Accepted present-file outcome data from Core/API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeAcceptedImport {
    /// Authoritative revision after the accepted import.
    pub revision_id: RevisionId,
    /// Authoritative content hash after the accepted import.
    pub content_hash: ContentHash,
}

/// Safe rejection reason categories from the abstract Core/API boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorktreeImportRejection {
    /// The request carried an invalid or stale base revision.
    InvalidBase,
    /// The request was structurally unsafe or invalid.
    UnsafeRequest,
    /// Core requires conflict handling before accepting this request.
    ConflictPolicyRequired,
    /// The request was rejected for a reason not modeled by Worktree yet.
    Other,
}

/// Submission summary for one request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeImportSubmission {
    /// Submitted request.
    pub action: WorktreeImportAction,
    /// Core/API outcome.
    pub outcome: WorktreeImportOutcome,
    /// True when local last-applied state was updated from an accepted outcome.
    pub local_state_updated: bool,
}

/// Submission report for a complete plan.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorktreeImportSubmissionReport {
    submissions: Vec<WorktreeImportSubmission>,
}

impl WorktreeImportSubmissionReport {
    /// Submitted request outcomes in request order.
    #[must_use]
    pub fn submissions(&self) -> &[WorktreeImportSubmission] {
        &self.submissions
    }

    /// Returns true when no request was submitted.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.submissions.is_empty()
    }
}

/// Submits import plans and updates Worktree state only from accepted Core/API outcomes.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorktreeImportRunner;

impl WorktreeImportRunner {
    /// Submit all actions in a plan through the abstract client boundary.
    pub fn submit_plan<C>(
        state: &mut WorktreeStateSnapshot,
        plan: WorktreeImportPlan,
        client: &mut C,
    ) -> Result<WorktreeImportSubmissionReport, C::Error>
    where
        C: WorktreeImportClient,
    {
        let mut submissions = Vec::with_capacity(plan.actions.len());
        for action in plan.actions {
            let outcome = client.submit(action.clone())?;
            let local_state_updated = update_state_from_outcome(state, &action, &outcome);
            submissions.push(WorktreeImportSubmission {
                action,
                outcome,
                local_state_updated,
            });
        }
        Ok(WorktreeImportSubmissionReport { submissions })
    }
}

fn update_state_from_outcome(
    state: &mut WorktreeStateSnapshot,
    action: &WorktreeImportAction,
    outcome: &WorktreeImportOutcome,
) -> bool {
    match (action, outcome) {
        (WorktreeImportAction::Put(request), WorktreeImportOutcome::Accepted(accepted))
        | (WorktreeImportAction::Put(request), WorktreeImportOutcome::SameContent(accepted)) => {
            state.record_present(
                request.vault_path.clone(),
                accepted.revision_id.clone(),
                accepted.content_hash,
            );
            true
        }
        (WorktreeImportAction::Delete(request), WorktreeImportOutcome::Tombstoned(tombstone)) => {
            state.record_tombstoned(request.vault_path.clone(), tombstone.revision_id.clone());
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StableFileState;
    use std::collections::VecDeque;

    fn path(input: &str) -> VaultPath {
        VaultPath::parse(input).unwrap()
    }

    fn revision(input: &str) -> RevisionId {
        RevisionId::parse(input).unwrap()
    }

    fn conflict(input: &str) -> ConflictId {
        ConflictId::parse(input).unwrap()
    }

    fn hash(byte: u8) -> ContentHash {
        ContentHash::from_bytes([byte; 32])
    }

    fn stable_file(
        vault_path: &str,
        content_hash: ContentHash,
        bytes: &[u8],
    ) -> WorktreeImportFile {
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

    fn unstable_snapshot(vault_path: &str) -> WorktreeFileSnapshot {
        WorktreeFileSnapshot {
            vault_path: path(vault_path),
            size: 3,
            modified: None,
            content_hash: hash(1),
            stability: StableFileState::unstable(),
        }
    }

    #[test]
    fn planner_detects_new_modified_deleted_and_unchanged_files() {
        let mut state = WorktreeStateSnapshot::new();
        state.record_present(path("same.md"), revision("rev_same"), hash(1));
        state.record_present(path("modified.md"), revision("rev_modified"), hash(2));
        state.record_present(path("deleted.md"), revision("rev_deleted"), hash(3));
        state.record_tombstoned(path("already-deleted.md"), revision("rev_tombstone"));

        let plan = WorktreeImportPlanner::plan(
            &state,
            [
                stable_file("new.md", hash(4), b"new"),
                stable_file("modified.md", hash(5), b"modified"),
                stable_file("same.md", hash(1), b"same"),
            ],
        )
        .unwrap();

        assert_eq!(plan.unchanged().len(), 1);
        assert_eq!(plan.unchanged()[0].vault_path.as_str(), "same.md");
        assert_eq!(plan.actions().len(), 3);

        assert!(matches!(
            &plan.actions()[0],
            WorktreeImportAction::Delete(delete)
                if delete.vault_path.as_str() == "deleted.md"
                    && delete.base_revision == WorktreeBaseRevision::Known(revision("rev_deleted"))
        ));
        assert!(matches!(
            &plan.actions()[1],
            WorktreeImportAction::Put(put)
                if put.vault_path.as_str() == "modified.md"
                    && put.base_revision == WorktreeBaseRevision::Known(revision("rev_modified"))
                    && put.content_hash == hash(5)
                    && put.bytes.as_slice() == b"modified"
        ));
        assert!(matches!(
            &plan.actions()[2],
            WorktreeImportAction::Put(put)
                if put.vault_path.as_str() == "new.md"
                    && put.base_revision == WorktreeBaseRevision::Null
                    && put.content_hash == hash(4)
                    && put.bytes.as_slice() == b"new"
        ));
    }

    #[test]
    fn planner_uses_tombstone_revision_as_known_base_when_file_reappears() {
        let mut state = WorktreeStateSnapshot::new();
        state.record_tombstoned(path("restored.md"), revision("rev_deleted"));

        let plan =
            WorktreeImportPlanner::plan(&state, [stable_file("restored.md", hash(7), b"restored")])
                .unwrap();

        assert_eq!(plan.actions().len(), 1);
        assert!(matches!(
            &plan.actions()[0],
            WorktreeImportAction::Put(put)
                if put.vault_path.as_str() == "restored.md"
                    && put.base_revision == WorktreeBaseRevision::Known(revision("rev_deleted"))
        ));
    }

    #[test]
    fn planner_rejects_unstable_and_duplicate_local_facts() {
        let unstable =
            WorktreeImportFile::new(unstable_snapshot("unstable.md"), b"bad".to_vec()).unwrap_err();
        assert_eq!(
            unstable,
            WorktreeImportPlanError::UnstableLocalFile {
                vault_path: path("unstable.md")
            }
        );

        let duplicate = WorktreeImportPlanner::plan(
            &WorktreeStateSnapshot::new(),
            [
                stable_file("dup.md", hash(1), b"one"),
                stable_file("dup.md", hash(2), b"two"),
            ],
        )
        .unwrap_err();
        assert_eq!(
            duplicate,
            WorktreeImportPlanError::DuplicateLocalFile {
                vault_path: path("dup.md")
            }
        );
    }

    #[test]
    fn runner_updates_state_only_for_accepted_outcomes() {
        let mut state = WorktreeStateSnapshot::new();
        state.record_present(path("deleted.md"), revision("rev_deleted_old"), hash(1));
        state.record_present(path("conflict.md"), revision("rev_conflict_old"), hash(2));

        let plan = WorktreeImportPlanner::plan(
            &state,
            [
                stable_file("accepted.md", hash(3), b"accepted"),
                stable_file("conflict.md", hash(4), b"conflict"),
            ],
        )
        .unwrap();
        let mut client = FakeClient::new([
            WorktreeImportOutcome::Accepted(WorktreeAcceptedImport {
                revision_id: revision("rev_accepted_new"),
                content_hash: hash(3),
            }),
            WorktreeImportOutcome::ConflictSaved {
                conflict_id: Some(conflict("conf_1")),
            },
            WorktreeImportOutcome::Tombstoned(WorktreeTombstoneState {
                revision_id: revision("rev_deleted_new"),
            }),
        ]);

        let report = WorktreeImportRunner::submit_plan(&mut state, plan, &mut client).unwrap();

        assert_eq!(report.submissions().len(), 3);
        assert_eq!(client.received.len(), 3);
        assert!(report.submissions()[0].local_state_updated);
        assert!(!report.submissions()[1].local_state_updated);
        assert!(report.submissions()[2].local_state_updated);

        assert!(matches!(
            state.path_state(&path("accepted.md")),
            Some(WorktreeAppliedPathState::Present(applied))
                if applied.revision_id == revision("rev_accepted_new")
                    && applied.content_hash == hash(3)
        ));
        assert!(matches!(
            state.path_state(&path("conflict.md")),
            Some(WorktreeAppliedPathState::Present(applied))
                if applied.revision_id == revision("rev_conflict_old")
                    && applied.content_hash == hash(2)
        ));
        assert!(matches!(
            state.path_state(&path("deleted.md")),
            Some(WorktreeAppliedPathState::Tombstoned(tombstone))
                if tombstone.revision_id == revision("rev_deleted_new")
        ));
    }

    #[test]
    fn rejected_same_content_and_not_found_outcomes_are_classified_safely() {
        let mut state = WorktreeStateSnapshot::new();
        let plan = WorktreeImportPlan {
            actions: vec![
                WorktreeImportAction::put(
                    path("same.md"),
                    WorktreeBaseRevision::Null,
                    hash(1),
                    b"same".to_vec(),
                ),
                WorktreeImportAction::put(
                    path("rejected.md"),
                    WorktreeBaseRevision::Null,
                    hash(2),
                    b"rejected".to_vec(),
                ),
                WorktreeImportAction::delete(
                    path("missing.md"),
                    WorktreeBaseRevision::Known(revision("rev_missing")),
                ),
            ],
            unchanged: Vec::new(),
        };
        let mut client = FakeClient::new([
            WorktreeImportOutcome::SameContent(WorktreeAcceptedImport {
                revision_id: revision("rev_same"),
                content_hash: hash(1),
            }),
            WorktreeImportOutcome::Rejected(WorktreeImportRejection::InvalidBase),
            WorktreeImportOutcome::NotFound,
        ]);

        let report = WorktreeImportRunner::submit_plan(&mut state, plan, &mut client).unwrap();

        assert!(report.submissions()[0].local_state_updated);
        assert!(!report.submissions()[1].local_state_updated);
        assert!(!report.submissions()[2].local_state_updated);
        assert!(state.path_state(&path("same.md")).is_some());
        assert!(state.path_state(&path("rejected.md")).is_none());
        assert!(state.path_state(&path("missing.md")).is_none());
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
}
