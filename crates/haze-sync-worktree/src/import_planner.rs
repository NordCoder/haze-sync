//! Shared import contracts between local Worktree facts and Core/API submission.
//!
//! Planning local file puts lives in `file_import`; guarded local deletes live in
//! `delete_guard`. This module contains only the shared state, request, outcome,
//! and abstract-client contracts used by those two explicit boundaries.

use crate::WorktreeFileSnapshot;
use haze_sync_common::{ConflictId, ContentHash, RevisionId, VaultPath};
use std::collections::BTreeMap;

/// Last-applied Worktree knowledge for vault paths materialized from Core.
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

    /// Record a present file accepted from an authoritative Core revision.
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

    /// Record an authoritative Core tombstone for a path.
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

    /// Iterate over known states in deterministic vault-path order.
    pub fn iter(&self) -> impl Iterator<Item = (&VaultPath, &WorktreeAppliedPathState)> {
        self.paths.iter()
    }

    /// Number of known path states.
    #[must_use]
    pub fn len(&self) -> usize {
        self.paths.len()
    }

    /// True when no path state is known.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }
}

/// Last-applied state for one vault path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorktreeAppliedPathState {
    /// A local file represents this authoritative Core revision and content hash.
    Present(WorktreeAppliedFileState),
    /// Core accepted a tombstone for this path.
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
    /// Authoritative revision that produced the local file.
    pub revision_id: RevisionId,
    /// Content hash materialized locally from Core.
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
}

/// Put-plan failure with safe vault-relative context only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorktreeImportPlanError {
    /// Planning received a local file that was not stable.
    UnstableLocalFile { vault_path: VaultPath },
    /// Planning received more than one local fact for the same vault path.
    DuplicateLocalFile { vault_path: VaultPath },
}

/// Explicit base for a Core/API import request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorktreeBaseRevision {
    /// The local fact is based on a known Core revision.
    Known(RevisionId),
    /// The local fact has no known Core base.
    Null,
}

/// Local fact submitted through the abstract Core/API boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorktreeImportAction {
    /// Create or update file content.
    Put(WorktreePutImport),
    /// Submit a guarded local delete/tombstone candidate.
    Delete(WorktreeDeleteImport),
}

impl WorktreeImportAction {
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

/// Guarded local delete import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeDeleteImport {
    /// Vault-relative path to tombstone through Core/API.
    pub vault_path: VaultPath,
    /// Known base revision or explicit null base.
    pub base_revision: WorktreeBaseRevision,
}

/// Local file that already matches last-applied Core state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeUnchangedFile {
    /// Vault-relative path that is unchanged.
    pub vault_path: VaultPath,
    /// Authoritative revision already represented locally.
    pub revision_id: RevisionId,
    /// Hash already represented locally.
    pub content_hash: ContentHash,
}

/// Abstract Core/API boundary for Worktree import facts.
pub trait WorktreeImportClient {
    /// Transport or client-level error.
    type Error;

    /// Submit one normalized Worktree fact through Core/API.
    fn submit(
        &mut self,
        action: WorktreeImportAction,
    ) -> Result<WorktreeImportOutcome, Self::Error>;
}

/// Core/API result for one import fact.
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
    /// Core accepted a tombstone for a guarded local delete.
    Tombstoned(WorktreeTombstoneState),
    /// Core could not find the requested base/path and made no accepted update.
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

/// Safe rejection categories from the abstract Core/API boundary.
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
