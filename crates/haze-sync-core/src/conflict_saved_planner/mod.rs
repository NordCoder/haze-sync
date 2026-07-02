//! Pure fan-in planner for revision-service conflict_saved outcomes.
//!
//! This module connects the W3 base-revision safety outcome to the W3 conflict
//! policy/path primitives. It returns only storage/API-neutral metadata and never
//! writes a database row, object-store blob, operation-log entry, adapter state,
//! provider state, or runtime side effect.

use chrono::{DateTime, Utc};
use haze_sync_common::{AdapterId, ContentHash, RevisionId, VaultPath};
use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};

use crate::{
    conflict_service::{
        ConflictPolicy, ConflictPolicyError, ConflictRecordInput, ConflictRecordStatus,
        CurrentRevision, IncomingBackupPlan, IncomingConflictCandidate,
    },
    policy_engine::{apply_conflict_policy, ConflictPolicyOutcome, ConflictPolicyRequest},
    revision_service::{ConflictPolicyHint, ConflictSavedOutcome, StoredRevision, UpsertOutcome},
};

/// Safe, side-effect-free conflict preservation plan for a conflict_saved Core outcome.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConflictSavedPreservationPlan {
    /// Original vault path submitted by the adapter.
    pub original_path: VaultPath,
    /// Planned conflict-copy path under the open conflicts area.
    pub materialized_path: VaultPath,
    /// Base revision provided by the caller, or null when the caller explicitly had no base.
    pub provided_base_revision_id: Option<RevisionId>,
    /// Current revision that remains authoritative.
    pub current_revision_id: RevisionId,
    /// Current content hash retained as the winning file content.
    pub current_content_hash: ContentHash,
    /// Current content size retained as the winning file content.
    pub current_size_bytes: u64,
    /// Adapter that supplied the incoming conflicting bytes.
    pub incoming_adapter_id: AdapterId,
    /// Hash of incoming bytes that must be preserved without silent overwrite.
    pub incoming_content_hash: ContentHash,
    /// Size of incoming bytes that must be preserved without silent overwrite.
    pub incoming_size_bytes: u64,
    /// Conflict policy selected by Core for later materialization.
    pub policy_applied: ConflictPolicy,
    /// Initial status for the planned conflict record.
    pub status: ConflictRecordStatus,
    /// UTC timestamp used for path generation and record planning.
    pub created_at: DateTime<Utc>,
    /// Metadata-only backup/materialization plan for incoming content.
    pub incoming_backup: IncomingBackupPlan,
    /// Metadata-only conflict record input for later storage fan-in.
    pub conflict_record: ConflictRecordInput,
}

impl ConflictSavedPreservationPlan {
    #[must_use]
    fn from_policy_outcome(outcome: ConflictPolicyOutcome) -> Self {
        Self::from_parts(
            outcome.current_revision().clone(),
            outcome.incoming_backup().clone(),
            outcome.conflict_record().clone(),
        )
    }

    #[must_use]
    fn from_parts(
        current_revision: CurrentRevision,
        incoming_backup: IncomingBackupPlan,
        conflict_record: ConflictRecordInput,
    ) -> Self {
        Self {
            original_path: conflict_record.original_path.clone(),
            materialized_path: conflict_record.conflict_path.clone(),
            provided_base_revision_id: conflict_record.base_revision_id.clone(),
            current_revision_id: conflict_record.current_revision_id.clone(),
            current_content_hash: current_revision.content_hash,
            current_size_bytes: current_revision.size_bytes,
            incoming_adapter_id: conflict_record.adapter_id.clone(),
            incoming_content_hash: conflict_record.incoming_content_hash,
            incoming_size_bytes: conflict_record.incoming_size_bytes,
            policy_applied: conflict_record.policy_applied,
            status: conflict_record.status,
            created_at: conflict_record.created_at,
            incoming_backup,
            conflict_record,
        }
    }
}

/// Safe planner errors. These variants intentionally omit raw bytes, storage paths, runtime data,
/// provider payloads, stack traces, and credentials.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "error")]
pub enum ConflictSavedPlanningError {
    /// Caller requested a mandatory plan from an upsert outcome that did not save a conflict.
    MissingConflictSavedOutcome,
    /// W3 conflict-policy/path validation rejected the preservation plan.
    ConflictPolicy { reason: ConflictPolicyError },
}

impl ConflictSavedPlanningError {
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::MissingConflictSavedOutcome => "missing_conflict_saved_outcome",
            Self::ConflictPolicy { reason } => reason.code(),
        }
    }

    #[must_use]
    pub const fn message(&self) -> &'static str {
        match self {
            Self::MissingConflictSavedOutcome => "upsert outcome did not contain conflict_saved data",
            Self::ConflictPolicy { reason } => reason.message(),
        }
    }
}

impl fmt::Display for ConflictSavedPlanningError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message())
    }
}

impl Error for ConflictSavedPlanningError {}

impl From<ConflictPolicyError> for ConflictSavedPlanningError {
    fn from(reason: ConflictPolicyError) -> Self {
        Self::ConflictPolicy { reason }
    }
}

/// Build a metadata-only conflict preservation plan from W3-P2 conflict_saved data.
pub fn plan_conflict_saved_outcome(
    conflict_saved: &ConflictSavedOutcome,
    created_at: DateTime<Utc>,
) -> Result<ConflictSavedPreservationPlan, ConflictSavedPlanningError> {
    let current_revision = current_revision_from_stored(&conflict_saved.current_revision);
    let incoming_candidate = IncomingConflictCandidate::new(
        conflict_saved.incoming_content.path.clone(),
        conflict_saved.provided_base_revision_id.clone(),
        conflict_saved.incoming_content.adapter_id.clone(),
        conflict_saved.incoming_content.content_hash,
        conflict_saved.incoming_content.size_bytes,
    );
    let request = ConflictPolicyRequest::new(
        policy_from_hint(conflict_saved.policy_hint),
        current_revision,
        incoming_candidate,
        created_at,
    )?;
    let outcome = apply_conflict_policy(request)?;

    Ok(ConflictSavedPreservationPlan::from_policy_outcome(outcome))
}

/// Return a metadata-only conflict preservation plan when an upsert outcome saved a conflict.
pub fn plan_upsert_conflict_saved(
    outcome: &UpsertOutcome,
    created_at: DateTime<Utc>,
) -> Result<Option<ConflictSavedPreservationPlan>, ConflictSavedPlanningError> {
    outcome
        .conflict_saved()
        .map(|conflict_saved| plan_conflict_saved_outcome(conflict_saved, created_at))
        .transpose()
}

/// Build a metadata-only plan and fail if the upsert outcome is not conflict_saved.
pub fn require_upsert_conflict_saved_plan(
    outcome: &UpsertOutcome,
    created_at: DateTime<Utc>,
) -> Result<ConflictSavedPreservationPlan, ConflictSavedPlanningError> {
    plan_upsert_conflict_saved(outcome, created_at)?
        .ok_or(ConflictSavedPlanningError::MissingConflictSavedOutcome)
}

fn current_revision_from_stored(revision: &StoredRevision) -> CurrentRevision {
    CurrentRevision::new(
        revision.revision_id.clone(),
        revision.path.clone(),
        revision.content_hash,
        revision.size_bytes,
    )
}

#[must_use]
const fn policy_from_hint(policy_hint: ConflictPolicyHint) -> ConflictPolicy {
    match policy_hint {
        ConflictPolicyHint::PreserveBoth => ConflictPolicy::PreserveBoth,
    }
}
