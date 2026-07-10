//! Explicit Worktree drift classification and persisted-state abstraction.

use crate::echo_guard::{WorktreeEchoDecision, WorktreeEchoGuard, WorktreeEchoGuardError};
use crate::import_planner::{WorktreeAppliedPathState, WorktreeStateSnapshot};
use crate::scanner::{
    WorktreeFileSnapshot, WorktreeScanResult, WorktreeScanSkipReason, WorktreeScanSkipped,
};
use haze_sync_common::{ContentHash, RevisionId, VaultPath};
use std::collections::BTreeMap;
use std::fmt;
use std::time::SystemTime;

const CONFLICT_PATH_PREFIX: &str = "_haze_conflicts";

/// Last clean filesystem observation bound to one authoritative revision/hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeObservedFileState {
    /// Authoritative revision represented when this observation was recorded.
    pub revision_id: RevisionId,
    /// Authoritative hash represented when this observation was recorded.
    pub content_hash: ContentHash,
    /// Stable file size observed locally.
    pub size: u64,
    /// Stable local modification timestamp when available.
    pub modified: Option<SystemTime>,
}

/// Persistable Worktree state composed without direct Storage/DB ownership.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorktreeReconciliationState {
    applied: WorktreeStateSnapshot,
    observations: BTreeMap<VaultPath, WorktreeObservedFileState>,
}

impl WorktreeReconciliationState {
    /// Create reconciliation state around authoritative last-applied path state.
    #[must_use]
    pub fn new(applied: WorktreeStateSnapshot) -> Self {
        Self {
            applied,
            observations: BTreeMap::new(),
        }
    }

    /// Authoritative last-applied state used for drift comparison.
    #[must_use]
    pub fn applied_state(&self) -> &WorktreeStateSnapshot {
        &self.applied
    }

    /// Mutable authoritative state boundary for accepted Core/API outcomes.
    #[must_use]
    pub fn applied_state_mut(&mut self) -> &mut WorktreeStateSnapshot {
        &mut self.applied
    }

    /// Replace the last-applied snapshot after an accepted integration update.
    pub fn replace_applied_state(&mut self, applied: WorktreeStateSnapshot) {
        self.applied = applied;
    }

    /// Return the last clean local observation for one path.
    #[must_use]
    pub fn observation(&self, vault_path: &VaultPath) -> Option<&WorktreeObservedFileState> {
        self.observations.get(vault_path)
    }

    /// Iterate over recorded observations in deterministic path order.
    pub fn observations(&self) -> impl Iterator<Item = (&VaultPath, &WorktreeObservedFileState)> {
        self.observations.iter()
    }

    /// Record a clean observation explicitly, for state restoration or tests.
    pub fn record_observation(
        &mut self,
        vault_path: VaultPath,
        observation: WorktreeObservedFileState,
    ) {
        self.observations.insert(vault_path, observation);
    }

    fn update_clean_observation(
        &mut self,
        vault_path: &VaultPath,
        observation: WorktreeObservedFileState,
    ) -> Option<WorktreeReconciliationStateTransition> {
        let previous = self.observations.get(vault_path).cloned();
        if previous.as_ref() == Some(&observation) {
            return None;
        }
        self.observations
            .insert(vault_path.clone(), observation.clone());
        Some(WorktreeReconciliationStateTransition {
            vault_path: vault_path.clone(),
            previous,
            current: observation,
        })
    }
}

/// Abstraction through which Server/Storage integration may persist Worktree state.
pub trait WorktreeReconciliationStateStore {
    /// Integration-owned persistence error.
    type Error;

    /// Load the current Worktree reconciliation state.
    fn load_state(&mut self) -> Result<WorktreeReconciliationState, Self::Error>;

    /// Save observation-only state transitions after reconciliation.
    fn save_state(&mut self, state: &WorktreeReconciliationState) -> Result<(), Self::Error>;
}

/// One safe observation-state transition produced by a clean filesystem match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeReconciliationStateTransition {
    /// Vault-relative path whose observation changed.
    pub vault_path: VaultPath,
    /// Previous clean observation, if any.
    pub previous: Option<WorktreeObservedFileState>,
    /// Current clean observation.
    pub current: WorktreeObservedFileState,
}

/// Required Worktree drift classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WorktreeReconciliationKind {
    /// Filesystem bytes match last-applied authoritative content.
    Clean,
    /// A tracked local file differs from last-applied authoritative content.
    Dirty,
    /// A tracked present file is absent from the scan without a covering skip.
    Missing,
    /// A local file has no present authoritative path state.
    Extra,
    /// A local path belongs to Core conflict materialization space.
    ConflictMaterialized,
    /// Scanner could not safely produce a stable file fact.
    Skipped,
}

impl WorktreeReconciliationKind {
    /// Stable machine-readable class code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::Dirty => "dirty",
            Self::Missing => "missing",
            Self::Extra => "extra",
            Self::ConflictMaterialized => "conflict_materialized",
            Self::Skipped => "skipped",
        }
    }
}

/// Echo-marker disposition attached to a reconciliation entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorktreeEchoStatus {
    /// Echo state was not checked because no stable file fact existed.
    NotChecked,
    /// No marker existed for this path.
    NoMarker,
    /// An exact, bounded adapter-write marker suppressed one scanner echo.
    Suppressed,
    /// A non-matching marker was consumed as stale.
    Stale,
}

/// Safe path-relative reconciliation fact for one file or skipped entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeReconciliationEntry {
    /// Vault-relative path when the scanner could represent one safely.
    pub vault_path: Option<VaultPath>,
    /// Drift classification.
    pub kind: WorktreeReconciliationKind,
    /// Last-applied authoritative revision when known.
    pub expected_revision: Option<RevisionId>,
    /// Last-applied authoritative content hash when known.
    pub expected_content_hash: Option<ContentHash>,
    /// Stable local content hash when observed.
    pub observed_content_hash: Option<ContentHash>,
    /// Last clean observed byte length bound to the expected revision/hash.
    pub expected_size: Option<u64>,
    /// Stable local byte length when observed.
    pub observed_size: Option<u64>,
    /// Last clean local modification timestamp bound to the expected revision/hash.
    pub expected_modified: Option<SystemTime>,
    /// Stable local modification timestamp when observed.
    pub observed_modified: Option<SystemTime>,
    /// True/false when prior modification facts exist; unknown otherwise.
    pub modification_facts_match: Option<bool>,
    /// Echo marker disposition for the path.
    pub echo_status: WorktreeEchoStatus,
    /// Scanner skip reason only for `Skipped` entries.
    pub skip_reason: Option<WorktreeScanSkipReason>,
}

/// Count-only summary safe for future doctor/status mapping.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct WorktreeReconciliationSummary {
    /// Clean tracked files.
    pub clean: usize,
    /// Dirty tracked files.
    pub dirty: usize,
    /// Missing tracked files.
    pub missing: usize,
    /// Extra or tombstone-reappeared files.
    pub extra: usize,
    /// Files in conflict materialization space.
    pub conflict_materialized: usize,
    /// Scanner-skipped entries.
    pub skipped: usize,
    /// Exact adapter echoes consumed during this pass.
    pub echo_suppressed: usize,
    /// Stale markers consumed during this pass.
    pub echo_stale: usize,
    /// Markers expired before classification.
    pub echo_expired: usize,
}

impl WorktreeReconciliationSummary {
    /// Total emitted reconciliation entries.
    #[must_use]
    pub const fn total_entries(self) -> usize {
        self.clean
            + self.dirty
            + self.missing
            + self.extra
            + self.conflict_materialized
            + self.skipped
    }
}

/// Deterministic reconciliation output plus observation-state transitions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorktreeReconciliationReport {
    entries: Vec<WorktreeReconciliationEntry>,
    transitions: Vec<WorktreeReconciliationStateTransition>,
    summary: WorktreeReconciliationSummary,
}

impl WorktreeReconciliationReport {
    /// Reconciliation facts in deterministic path/class order.
    #[must_use]
    pub fn entries(&self) -> &[WorktreeReconciliationEntry] {
        &self.entries
    }

    /// Clean observation transitions applied to reconciliation state.
    #[must_use]
    pub fn transitions(&self) -> &[WorktreeReconciliationStateTransition] {
        &self.transitions
    }

    /// Safe count-only summary.
    #[must_use]
    pub const fn summary(&self) -> WorktreeReconciliationSummary {
        self.summary
    }

    /// True when the persisted observation snapshot changed.
    #[must_use]
    pub fn state_changed(&self) -> bool {
        !self.transitions.is_empty()
    }
}

/// Safe reconciliation failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorktreeReconciliationError {
    /// Scan input contained duplicate stable facts for one path.
    DuplicateScanPath { vault_path: VaultPath },
    /// Echo marker lifecycle failed safely.
    EchoGuard(WorktreeEchoGuardError),
}

impl WorktreeReconciliationError {
    /// Stable machine-readable error code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::DuplicateScanPath { .. } => "duplicate_reconciliation_scan_path",
            Self::EchoGuard(error) => error.code(),
        }
    }

    /// Stable path-redacted error message.
    #[must_use]
    pub const fn message(&self) -> &'static str {
        match self {
            Self::DuplicateScanPath { .. } => {
                "reconciliation scan contained duplicate stable path facts"
            }
            Self::EchoGuard(error) => error.message(),
        }
    }
}

impl fmt::Display for WorktreeReconciliationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message())
    }
}

impl std::error::Error for WorktreeReconciliationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::EchoGuard(error) => Some(error),
            Self::DuplicateScanPath { .. } => None,
        }
    }
}

impl From<WorktreeEchoGuardError> for WorktreeReconciliationError {
    fn from(error: WorktreeEchoGuardError) -> Self {
        Self::EchoGuard(error)
    }
}

/// Load/reconcile/save failure without assigning persistence ownership to Worktree.
#[derive(Debug, PartialEq, Eq)]
pub enum WorktreeReconciliationRunError<E> {
    /// Integration state could not be loaded.
    Load(E),
    /// Filesystem and echo facts could not be reconciled.
    Reconcile(WorktreeReconciliationError),
    /// Observation-state transitions could not be persisted.
    Save(E),
}

/// Explicit Worktree drift reconciler.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorktreeReconciler;

impl WorktreeReconciler {
    /// Compare stable scanner facts against last-applied and last-observed state.
    pub fn reconcile(
        state: &mut WorktreeReconciliationState,
        scan: &WorktreeScanResult,
        echo_guard: &mut WorktreeEchoGuard,
        now: SystemTime,
    ) -> Result<WorktreeReconciliationReport, WorktreeReconciliationError> {
        let local_files = stable_files_by_path(&scan.files)?;
        let skipped_prefixes = skipped_prefixes(&scan.skipped);
        let expired = echo_guard.expire(now)?;
        let applied_paths: Vec<(VaultPath, WorktreeAppliedPathState)> = state
            .applied_state()
            .iter()
            .map(|(path, applied)| (path.clone(), applied.clone()))
            .collect();

        let mut entries = Vec::new();
        let mut transitions = Vec::new();

        for (vault_path, snapshot) in &local_files {
            let applied = state.applied_state().path_state(vault_path).cloned();
            let expected_observation = expected_observation(state, vault_path, applied.as_ref());
            let echo_status = consume_echo(
                echo_guard,
                vault_path,
                applied.as_ref(),
                Some(snapshot),
                now,
            )?;
            let kind = classify_present_path(vault_path, applied.as_ref(), snapshot);

            if matches!(applied, Some(WorktreeAppliedPathState::Present(ref file)) if file.content_hash == snapshot.content_hash)
            {
                let observation = WorktreeObservedFileState {
                    revision_id: applied
                        .as_ref()
                        .expect("matching present state must exist")
                        .revision_id()
                        .clone(),
                    content_hash: snapshot.content_hash,
                    size: snapshot.size,
                    modified: snapshot.modified,
                };
                if let Some(transition) = state.update_clean_observation(vault_path, observation) {
                    transitions.push(transition);
                }
            }

            entries.push(entry_for_snapshot(
                vault_path,
                kind,
                applied.as_ref(),
                expected_observation.as_ref(),
                snapshot,
                echo_status,
            ));
        }

        for (vault_path, applied) in &applied_paths {
            if local_files.contains_key(vault_path)
                || covered_by_skipped_prefix(vault_path, &skipped_prefixes)
            {
                continue;
            }
            let WorktreeAppliedPathState::Present(file) = applied else {
                continue;
            };
            let echo_status = consume_echo(echo_guard, vault_path, Some(applied), None, now)?;
            let expected_observation = state.observation(vault_path).filter(|observation| {
                observation.revision_id == file.revision_id
                    && observation.content_hash == file.content_hash
            });
            entries.push(WorktreeReconciliationEntry {
                vault_path: Some(vault_path.clone()),
                kind: WorktreeReconciliationKind::Missing,
                expected_revision: Some(file.revision_id.clone()),
                expected_content_hash: Some(file.content_hash),
                observed_content_hash: None,
                expected_size: expected_observation.map(|observation| observation.size),
                observed_size: None,
                expected_modified: expected_observation
                    .and_then(|observation| observation.modified),
                observed_modified: None,
                modification_facts_match: None,
                echo_status,
                skip_reason: None,
            });
        }

        entries.extend(scan.skipped.iter().map(entry_for_skipped));
        entries.sort_by(|left, right| {
            left.vault_path
                .as_ref()
                .map(VaultPath::as_str)
                .cmp(&right.vault_path.as_ref().map(VaultPath::as_str))
                .then_with(|| left.kind.cmp(&right.kind))
                .then_with(|| left.skip_reason.cmp(&right.skip_reason))
        });

        let summary = summarize(&entries, expired);
        Ok(WorktreeReconciliationReport {
            entries,
            transitions,
            summary,
        })
    }
}

/// Persisted-state runner that depends only on an integration-provided store trait.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorktreeReconciliationRunner;

impl WorktreeReconciliationRunner {
    /// Load state, reconcile, and save only when clean observation facts changed.
    pub fn run<S>(
        store: &mut S,
        scan: &WorktreeScanResult,
        echo_guard: &mut WorktreeEchoGuard,
        now: SystemTime,
    ) -> Result<WorktreeReconciliationReport, WorktreeReconciliationRunError<S::Error>>
    where
        S: WorktreeReconciliationStateStore,
    {
        let mut state = store
            .load_state()
            .map_err(WorktreeReconciliationRunError::Load)?;
        let report = WorktreeReconciler::reconcile(&mut state, scan, echo_guard, now)
            .map_err(WorktreeReconciliationRunError::Reconcile)?;
        if report.state_changed() {
            store
                .save_state(&state)
                .map_err(WorktreeReconciliationRunError::Save)?;
        }
        Ok(report)
    }
}

fn stable_files_by_path(
    files: &[WorktreeFileSnapshot],
) -> Result<BTreeMap<VaultPath, &WorktreeFileSnapshot>, WorktreeReconciliationError> {
    let mut by_path = BTreeMap::new();
    for file in files {
        if by_path.insert(file.vault_path.clone(), file).is_some() {
            return Err(WorktreeReconciliationError::DuplicateScanPath {
                vault_path: file.vault_path.clone(),
            });
        }
    }
    Ok(by_path)
}

fn skipped_prefixes(skipped: &[WorktreeScanSkipped]) -> Vec<&VaultPath> {
    skipped
        .iter()
        .filter_map(|entry| entry.vault_path.as_ref())
        .collect()
}

fn covered_by_skipped_prefix(vault_path: &VaultPath, skipped_prefixes: &[&VaultPath]) -> bool {
    skipped_prefixes.iter().any(|prefix| {
        vault_path == *prefix
            || vault_path
                .as_str()
                .strip_prefix(prefix.as_str())
                .is_some_and(|suffix| suffix.starts_with('/'))
    })
}

fn classify_present_path(
    vault_path: &VaultPath,
    applied: Option<&WorktreeAppliedPathState>,
    snapshot: &WorktreeFileSnapshot,
) -> WorktreeReconciliationKind {
    if is_conflict_materialized_path(vault_path) {
        return WorktreeReconciliationKind::ConflictMaterialized;
    }

    match applied {
        Some(WorktreeAppliedPathState::Present(file))
            if file.content_hash == snapshot.content_hash =>
        {
            WorktreeReconciliationKind::Clean
        }
        Some(WorktreeAppliedPathState::Present(_)) => WorktreeReconciliationKind::Dirty,
        Some(WorktreeAppliedPathState::Tombstoned(_)) | None => WorktreeReconciliationKind::Extra,
    }
}

fn is_conflict_materialized_path(vault_path: &VaultPath) -> bool {
    vault_path
        .segments()
        .next()
        .is_some_and(|segment| segment == CONFLICT_PATH_PREFIX)
}

fn expected_observation(
    state: &WorktreeReconciliationState,
    vault_path: &VaultPath,
    applied: Option<&WorktreeAppliedPathState>,
) -> Option<WorktreeObservedFileState> {
    let WorktreeAppliedPathState::Present(file) = applied? else {
        return None;
    };
    state
        .observation(vault_path)
        .filter(|observation| {
            observation.revision_id == file.revision_id
                && observation.content_hash == file.content_hash
        })
        .cloned()
}

fn consume_echo(
    guard: &mut WorktreeEchoGuard,
    vault_path: &VaultPath,
    applied: Option<&WorktreeAppliedPathState>,
    snapshot: Option<&WorktreeFileSnapshot>,
    now: SystemTime,
) -> Result<WorktreeEchoStatus, WorktreeReconciliationError> {
    let (revision, applied_hash) = match applied {
        Some(WorktreeAppliedPathState::Present(file)) => {
            (Some(&file.revision_id), Some(file.content_hash))
        }
        Some(WorktreeAppliedPathState::Tombstoned(tombstone)) => {
            (Some(&tombstone.revision_id), None)
        }
        None => (None, None),
    };
    let observed_hash = snapshot.map(|snapshot| snapshot.content_hash);
    let decision =
        guard.consume_for_observation(vault_path, revision, applied_hash, observed_hash, now)?;
    Ok(match decision {
        WorktreeEchoDecision::NoMarker => WorktreeEchoStatus::NoMarker,
        WorktreeEchoDecision::Suppressed(_) => WorktreeEchoStatus::Suppressed,
        WorktreeEchoDecision::Stale(_) => WorktreeEchoStatus::Stale,
    })
}

fn entry_for_snapshot(
    vault_path: &VaultPath,
    kind: WorktreeReconciliationKind,
    applied: Option<&WorktreeAppliedPathState>,
    expected_observation: Option<&WorktreeObservedFileState>,
    snapshot: &WorktreeFileSnapshot,
    echo_status: WorktreeEchoStatus,
) -> WorktreeReconciliationEntry {
    let expected_revision = applied.map(WorktreeAppliedPathState::revision_id).cloned();
    let expected_content_hash = match applied {
        Some(WorktreeAppliedPathState::Present(file)) => Some(file.content_hash),
        Some(WorktreeAppliedPathState::Tombstoned(_)) | None => None,
    };
    let modification_facts_match = expected_observation.map(|observation| {
        observation.size == snapshot.size && observation.modified == snapshot.modified
    });

    WorktreeReconciliationEntry {
        vault_path: Some(vault_path.clone()),
        kind,
        expected_revision,
        expected_content_hash,
        observed_content_hash: Some(snapshot.content_hash),
        expected_size: expected_observation.map(|observation| observation.size),
        observed_size: Some(snapshot.size),
        expected_modified: expected_observation.and_then(|observation| observation.modified),
        observed_modified: snapshot.modified,
        modification_facts_match,
        echo_status,
        skip_reason: None,
    }
}

fn entry_for_skipped(skipped: &WorktreeScanSkipped) -> WorktreeReconciliationEntry {
    WorktreeReconciliationEntry {
        vault_path: skipped.vault_path.clone(),
        kind: WorktreeReconciliationKind::Skipped,
        expected_revision: None,
        expected_content_hash: None,
        observed_content_hash: None,
        expected_size: None,
        observed_size: None,
        expected_modified: None,
        observed_modified: None,
        modification_facts_match: None,
        echo_status: WorktreeEchoStatus::NotChecked,
        skip_reason: Some(skipped.reason),
    }
}

fn summarize(
    entries: &[WorktreeReconciliationEntry],
    echo_expired: usize,
) -> WorktreeReconciliationSummary {
    let mut summary = WorktreeReconciliationSummary {
        echo_expired,
        ..WorktreeReconciliationSummary::default()
    };
    for entry in entries {
        match entry.kind {
            WorktreeReconciliationKind::Clean => summary.clean += 1,
            WorktreeReconciliationKind::Dirty => summary.dirty += 1,
            WorktreeReconciliationKind::Missing => summary.missing += 1,
            WorktreeReconciliationKind::Extra => summary.extra += 1,
            WorktreeReconciliationKind::ConflictMaterialized => {
                summary.conflict_materialized += 1;
            }
            WorktreeReconciliationKind::Skipped => summary.skipped += 1,
        }
        match entry.echo_status {
            WorktreeEchoStatus::Suppressed => summary.echo_suppressed += 1,
            WorktreeEchoStatus::Stale => summary.echo_stale += 1,
            WorktreeEchoStatus::NotChecked | WorktreeEchoStatus::NoMarker => {}
        }
    }
    summary
}
