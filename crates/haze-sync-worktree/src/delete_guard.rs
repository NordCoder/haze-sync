//! Guarded local-delete planning and abstract Core/API submission.
//!
//! Scanner absence is only a local fact. This module emits explicit delete
//! candidates only when absence is safely scoped, applies bounded mass-delete
//! thresholds, and submits approved candidates through the abstract Core/API
//! client. Core remains the tombstone and conflict-policy authority.

use crate::{
    WorktreeAppliedPathState, WorktreeBaseRevision, WorktreeDeleteImport, WorktreeImportAction,
    WorktreeImportClient, WorktreeImportOutcome, WorktreeScanResult, WorktreeScanSkipReason,
    WorktreeStateSnapshot,
};
use haze_sync_common::VaultPath;
use std::collections::BTreeSet;
use std::fmt;

const RATIO_SCALE_BASIS_POINTS: u128 = 10_000;

/// One local absence fact ready for guarded Core/API submission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeDeleteCandidate {
    /// Vault-relative path observed as absent.
    pub vault_path: VaultPath,
    /// Known Core base revision or explicit null base.
    pub base_revision: WorktreeBaseRevision,
}

impl WorktreeDeleteCandidate {
    /// Construct a delete candidate from an accepted local observation boundary.
    #[must_use]
    pub fn new(vault_path: VaultPath, base_revision: WorktreeBaseRevision) -> Self {
        Self {
            vault_path,
            base_revision,
        }
    }

    fn into_action(self) -> WorktreeImportAction {
        WorktreeImportAction::Delete(WorktreeDeleteImport {
            vault_path: self.vault_path,
            base_revision: self.base_revision,
        })
    }
}

/// Delete candidates derived from one scan or another accepted local-fact source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeDeleteScan {
    candidates: Vec<WorktreeDeleteCandidate>,
    tracked_present_count: usize,
    complete: bool,
}

impl WorktreeDeleteScan {
    /// Derive known-base delete candidates from a scanner result.
    ///
    /// An unscoped filesystem error makes negative absence conclusions unsafe.
    /// Expected unscoped reserved-runtime skips do not hide valid user paths and
    /// therefore do not block delete planning. Any path-scoped skipped entry
    /// suppresses candidates at or below that path.
    pub fn from_scan(
        state: &WorktreeStateSnapshot,
        scan: &WorktreeScanResult,
    ) -> Result<Self, WorktreeDeletePlanError> {
        let local_paths = local_scan_paths(scan)?;
        let tracked_present_count = state
            .iter()
            .filter(|(_, path_state)| matches!(path_state, WorktreeAppliedPathState::Present(_)))
            .count();
        let complete = !scan.skipped.iter().any(|entry| {
            entry.vault_path.is_none() && entry.reason == WorktreeScanSkipReason::FilesystemError
        });

        if !complete {
            return Ok(Self {
                candidates: Vec::new(),
                tracked_present_count,
                complete: false,
            });
        }

        let skipped_prefixes: Vec<&VaultPath> = scan
            .skipped
            .iter()
            .filter_map(|entry| entry.vault_path.as_ref())
            .collect();
        let mut candidates = Vec::new();
        for (vault_path, path_state) in state.iter() {
            let WorktreeAppliedPathState::Present(applied) = path_state else {
                continue;
            };
            if local_paths.contains(vault_path)
                || covered_by_skipped_prefix(vault_path, &skipped_prefixes)
            {
                continue;
            }
            candidates.push(WorktreeDeleteCandidate::new(
                vault_path.clone(),
                WorktreeBaseRevision::Known(applied.revision_id.clone()),
            ));
        }

        Ok(Self {
            candidates,
            tracked_present_count,
            complete,
        })
    }

    /// Construct a complete candidate set from another accepted observation source.
    ///
    /// This supports explicit null-base semantics without assigning delete policy
    /// to Worktree. Duplicate paths are rejected.
    pub fn from_candidates(
        candidates: impl IntoIterator<Item = WorktreeDeleteCandidate>,
        tracked_present_count: usize,
    ) -> Result<Self, WorktreeDeletePlanError> {
        let mut candidates: Vec<WorktreeDeleteCandidate> = candidates.into_iter().collect();
        let mut seen = BTreeSet::new();
        for candidate in &candidates {
            if !seen.insert(candidate.vault_path.clone()) {
                return Err(WorktreeDeletePlanError::DuplicateCandidate {
                    vault_path: candidate.vault_path.clone(),
                });
            }
        }
        candidates.sort_by(|left, right| left.vault_path.as_str().cmp(right.vault_path.as_str()));
        Ok(Self {
            candidates,
            tracked_present_count,
            complete: true,
        })
    }

    /// Candidates in deterministic vault-path order.
    #[must_use]
    pub fn candidates(&self) -> &[WorktreeDeleteCandidate] {
        &self.candidates
    }

    /// Number of last-applied present paths used as the ratio denominator.
    #[must_use]
    pub const fn tracked_present_count(&self) -> usize {
        self.tracked_present_count
    }

    /// Whether scanner output was complete enough for absence conclusions.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.complete
    }
}

/// Safe candidate-planning failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorktreeDeletePlanError {
    /// Scanner result contained the same stable path more than once.
    DuplicateScanPath { vault_path: VaultPath },
    /// Accepted candidate source contained the same delete path more than once.
    DuplicateCandidate { vault_path: VaultPath },
}

impl WorktreeDeletePlanError {
    /// Stable machine-readable code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::DuplicateScanPath { .. } => "duplicate_delete_scan_path",
            Self::DuplicateCandidate { .. } => "duplicate_delete_candidate",
        }
    }

    /// Stable path-redacted message.
    #[must_use]
    pub const fn message(&self) -> &'static str {
        match self {
            Self::DuplicateScanPath { .. } => {
                "delete planning received duplicate stable scan paths"
            }
            Self::DuplicateCandidate { .. } => "delete planning received duplicate candidate paths",
        }
    }
}

impl fmt::Display for WorktreeDeletePlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message())
    }
}

impl std::error::Error for WorktreeDeletePlanError {}

/// Bounded mass-delete thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorktreeDeleteGuardPolicy {
    max_deletes_per_run: usize,
    max_delete_ratio_basis_points: u16,
}

impl WorktreeDeleteGuardPolicy {
    /// Construct a policy. `500` basis points represents five percent.
    pub fn new(
        max_deletes_per_run: usize,
        max_delete_ratio_basis_points: u16,
    ) -> Result<Self, WorktreeDeleteGuardPolicyError> {
        if max_delete_ratio_basis_points > RATIO_SCALE_BASIS_POINTS as u16 {
            return Err(WorktreeDeleteGuardPolicyError::InvalidRatio);
        }
        Ok(Self {
            max_deletes_per_run,
            max_delete_ratio_basis_points,
        })
    }

    /// Maximum delete count accepted without an explicit manual unlock.
    #[must_use]
    pub const fn max_deletes_per_run(self) -> usize {
        self.max_deletes_per_run
    }

    /// Maximum delete ratio in basis points accepted without manual unlock.
    #[must_use]
    pub const fn max_delete_ratio_basis_points(self) -> u16 {
        self.max_delete_ratio_basis_points
    }
}

/// Invalid delete-guard configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorktreeDeleteGuardPolicyError {
    /// Ratio exceeded 100 percent.
    InvalidRatio,
}

impl fmt::Display for WorktreeDeleteGuardPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("delete guard ratio must be between zero and 10000 basis points")
    }
}

impl std::error::Error for WorktreeDeleteGuardPolicyError {}

/// Authorization used when evaluating delete thresholds.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum WorktreeDeleteAuthorization {
    /// Apply normal configured thresholds.
    #[default]
    Guarded,
    /// Explicit operator/Core-approved unlock for this candidate batch.
    ManualUnlock,
}

/// Why delete propagation was blocked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorktreeDeleteBlockReason {
    /// Scanner output contained an unscoped filesystem error.
    IncompleteScan,
    /// One or both configured mass-delete thresholds were exceeded.
    ThresholdExceeded,
}

impl WorktreeDeleteBlockReason {
    /// Stable machine-readable reason code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::IncompleteScan => "incomplete_delete_scan",
            Self::ThresholdExceeded => "unsafe_mass_delete_detected",
        }
    }
}

/// Count-only guard facts safe for status/doctor mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorktreeDeleteGuardSummary {
    /// Number of delete candidates in the batch.
    pub delete_count: usize,
    /// Number of last-applied present paths used as denominator.
    pub tracked_present_count: usize,
    /// Conservative, ceiling-rounded candidate ratio in basis points.
    pub delete_ratio_basis_points: u32,
    /// Whether the count threshold was exceeded.
    pub count_exceeded: bool,
    /// Whether the ratio threshold was exceeded.
    pub ratio_exceeded: bool,
    /// Whether explicit manual unlock authorized an over-threshold batch.
    pub manual_unlock_used: bool,
}

/// Guard outcome attached to a delete plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorktreeDeleteGuardDecision {
    /// Batch may be submitted through Core/API.
    Allowed(WorktreeDeleteGuardSummary),
    /// Batch must not be submitted.
    Blocked {
        /// Safe block category.
        reason: WorktreeDeleteBlockReason,
        /// Count-only guard facts.
        summary: WorktreeDeleteGuardSummary,
    },
}

impl WorktreeDeleteGuardDecision {
    /// True only when Core/API submission is allowed.
    #[must_use]
    pub const fn is_allowed(self) -> bool {
        matches!(self, Self::Allowed(_))
    }

    /// Count-only facts for either outcome.
    #[must_use]
    pub const fn summary(self) -> WorktreeDeleteGuardSummary {
        match self {
            Self::Allowed(summary) | Self::Blocked { summary, .. } => summary,
        }
    }
}

/// Candidate batch plus its immutable guard decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeGuardedDeletePlan {
    candidates: Vec<WorktreeDeleteCandidate>,
    decision: WorktreeDeleteGuardDecision,
}

impl WorktreeGuardedDeletePlan {
    /// Evaluate one candidate batch against configured thresholds.
    #[must_use]
    pub fn evaluate(
        scan: WorktreeDeleteScan,
        policy: WorktreeDeleteGuardPolicy,
        authorization: WorktreeDeleteAuthorization,
    ) -> Self {
        let delete_count = scan.candidates.len();
        let ratio = delete_ratio_basis_points(delete_count, scan.tracked_present_count);
        let count_exceeded = delete_count > policy.max_deletes_per_run;
        let ratio_exceeded = ratio > u32::from(policy.max_delete_ratio_basis_points);
        let manual_unlock_used = authorization == WorktreeDeleteAuthorization::ManualUnlock
            && (count_exceeded || ratio_exceeded);
        let summary = WorktreeDeleteGuardSummary {
            delete_count,
            tracked_present_count: scan.tracked_present_count,
            delete_ratio_basis_points: ratio,
            count_exceeded,
            ratio_exceeded,
            manual_unlock_used,
        };
        let decision = if !scan.complete {
            WorktreeDeleteGuardDecision::Blocked {
                reason: WorktreeDeleteBlockReason::IncompleteScan,
                summary,
            }
        } else if (count_exceeded || ratio_exceeded)
            && authorization != WorktreeDeleteAuthorization::ManualUnlock
        {
            WorktreeDeleteGuardDecision::Blocked {
                reason: WorktreeDeleteBlockReason::ThresholdExceeded,
                summary,
            }
        } else {
            WorktreeDeleteGuardDecision::Allowed(summary)
        };
        Self {
            candidates: scan.candidates,
            decision,
        }
    }

    /// Candidates in deterministic order.
    #[must_use]
    pub fn candidates(&self) -> &[WorktreeDeleteCandidate] {
        &self.candidates
    }

    /// Immutable decision made before any client calls.
    #[must_use]
    pub const fn decision(&self) -> WorktreeDeleteGuardDecision {
        self.decision
    }
}

/// One submitted delete candidate and authoritative Core/API outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeDeleteSubmission {
    /// Submitted local absence fact.
    pub candidate: WorktreeDeleteCandidate,
    /// Authoritative Core/API result.
    pub outcome: WorktreeImportOutcome,
    /// True only when an accepted tombstone advanced local last-applied state.
    pub local_state_updated: bool,
}

/// Guarded delete-submission report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeDeleteSubmissionReport {
    guard: WorktreeDeleteGuardSummary,
    submissions: Vec<WorktreeDeleteSubmission>,
}

impl WorktreeDeleteSubmissionReport {
    /// Guard facts applied before submission.
    #[must_use]
    pub const fn guard(&self) -> WorktreeDeleteGuardSummary {
        self.guard
    }

    /// Submitted outcomes in deterministic candidate order.
    #[must_use]
    pub fn submissions(&self) -> &[WorktreeDeleteSubmission] {
        &self.submissions
    }
}

/// Failure from guarded delete submission.
#[derive(Debug, PartialEq, Eq)]
pub enum WorktreeDeleteRunError<E> {
    /// Guard prevented all client calls.
    Blocked {
        /// Safe block category.
        reason: WorktreeDeleteBlockReason,
        /// Count-only guard facts.
        summary: WorktreeDeleteGuardSummary,
    },
    /// Abstract Core/API client failed.
    Client(E),
}

/// Submits only pre-authorized delete candidates through the abstract client.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorktreeDeleteRunner;

impl WorktreeDeleteRunner {
    /// Submit an approved plan and update state only from accepted tombstones.
    pub fn submit<C>(
        state: &mut WorktreeStateSnapshot,
        plan: WorktreeGuardedDeletePlan,
        client: &mut C,
    ) -> Result<WorktreeDeleteSubmissionReport, WorktreeDeleteRunError<C::Error>>
    where
        C: WorktreeImportClient,
    {
        let guard = match plan.decision {
            WorktreeDeleteGuardDecision::Allowed(summary) => summary,
            WorktreeDeleteGuardDecision::Blocked { reason, summary } => {
                return Err(WorktreeDeleteRunError::Blocked { reason, summary });
            }
        };

        let mut submissions = Vec::with_capacity(plan.candidates.len());
        for candidate in plan.candidates {
            let outcome = client
                .submit(candidate.clone().into_action())
                .map_err(WorktreeDeleteRunError::Client)?;
            let local_state_updated = match &outcome {
                WorktreeImportOutcome::Tombstoned(tombstone) => {
                    state.record_tombstoned(
                        candidate.vault_path.clone(),
                        tombstone.revision_id.clone(),
                    );
                    true
                }
                _ => false,
            };
            submissions.push(WorktreeDeleteSubmission {
                candidate,
                outcome,
                local_state_updated,
            });
        }

        Ok(WorktreeDeleteSubmissionReport { guard, submissions })
    }
}

fn local_scan_paths(
    scan: &WorktreeScanResult,
) -> Result<BTreeSet<VaultPath>, WorktreeDeletePlanError> {
    let mut paths = BTreeSet::new();
    for file in &scan.files {
        if !paths.insert(file.vault_path.clone()) {
            return Err(WorktreeDeletePlanError::DuplicateScanPath {
                vault_path: file.vault_path.clone(),
            });
        }
    }
    Ok(paths)
}

fn covered_by_skipped_prefix(vault_path: &VaultPath, prefixes: &[&VaultPath]) -> bool {
    prefixes.iter().any(|prefix| {
        vault_path == *prefix
            || vault_path
                .as_str()
                .strip_prefix(prefix.as_str())
                .is_some_and(|suffix| suffix.starts_with('/'))
    })
}

fn delete_ratio_basis_points(delete_count: usize, tracked_present_count: usize) -> u32 {
    if delete_count == 0 {
        return 0;
    }
    if tracked_present_count == 0 {
        return RATIO_SCALE_BASIS_POINTS as u32;
    }
    let numerator = (delete_count as u128) * RATIO_SCALE_BASIS_POINTS;
    let denominator = tracked_present_count as u128;
    let ceiling = numerator.div_ceil(denominator);
    ceiling.min(u128::from(u32::MAX)) as u32
}
