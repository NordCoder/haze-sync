//! Pure fan-in planner for conflict_saved Core outcomes.
//!
//! This planner bridges W3 stale/unknown-base conflict_saved outcomes with the
//! conflict policy/path primitives. It only produces JSON-safe planning metadata;
//! it never writes content bytes, persists records, appends operation-log rows,
//! calls providers, or touches runtime state.

use chrono::{DateTime, Utc};
use haze_sync_common::ConflictId;
use serde::{Deserialize, Serialize};

use crate::{
    conflict_service::{
        ConflictPolicy, ConflictPolicyError, ConflictRecord, CurrentRevision, IncomingBackupPlan,
        IncomingConflictCandidate,
    },
    policy_engine::{apply_conflict_policy, ConflictPolicyOutcome, ConflictPolicyRequest},
    revision_service::{ConflictPolicyHint, ConflictSavedOutcome, UpsertOutcome},
};

/// Safe, storage-neutral plan for preserving an incoming conflict copy.
///
/// The plan intentionally exposes only vault-relative paths, ids, content
/// hashes, sizes, policy/status metadata, and timestamps. It does not contain
/// raw file bytes and does not imply that any DB/object-store/operation-log
/// mutation has happened.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConflictPreservationPlan {
    pub conflict_id: ConflictId,
    pub current_revision: CurrentRevision,
    pub incoming_backup: IncomingBackupPlan,
    pub conflict_record: ConflictRecord,
}

impl ConflictPreservationPlan {
    #[must_use]
    fn from_policy_outcome(conflict_id: ConflictId, outcome: ConflictPolicyOutcome) -> Self {
        let current_revision = outcome.current_revision().clone();
        let incoming_backup = outcome.incoming_backup().clone();
        let conflict_record = ConflictRecord::from_input(
            conflict_id.clone(),
            outcome.conflict_record().clone(),
        );

        Self {
            conflict_id,
            current_revision,
            incoming_backup,
            conflict_record,
        }
    }
}

/// Convert a W3-P2 policy hint into a W3-P1 concrete conflict policy.
#[must_use]
pub const fn conflict_policy_from_hint(hint: ConflictPolicyHint) -> ConflictPolicy {
    match hint {
        ConflictPolicyHint::PreserveBoth => ConflictPolicy::PreserveBoth,
    }
}

/// Plan preservation of a conflict_saved incoming write without mutating storage.
pub fn plan_conflict_saved(
    conflict_saved: &ConflictSavedOutcome,
    conflict_id: ConflictId,
    detected_at: DateTime<Utc>,
) -> Result<ConflictPreservationPlan, ConflictPolicyError> {
    let policy = conflict_policy_from_hint(conflict_saved.policy_hint);
    let current_revision = CurrentRevision::new(
        conflict_saved.current_revision.revision_id.clone(),
        conflict_saved.current_revision.path.clone(),
        conflict_saved.current_revision.content_hash,
        conflict_saved.current_revision.size_bytes,
    );
    let incoming_candidate = IncomingConflictCandidate::new(
        conflict_saved.incoming_content.path.clone(),
        conflict_saved.provided_base_revision_id.clone(),
        conflict_saved.incoming_content.adapter_id.clone(),
        conflict_saved.incoming_content.content_hash,
        conflict_saved.incoming_content.size_bytes,
    );

    let request =
        ConflictPolicyRequest::new(policy, current_revision, incoming_candidate, detected_at)?;
    let outcome = apply_conflict_policy(request)?;

    Ok(ConflictPreservationPlan::from_policy_outcome(
        conflict_id,
        outcome,
    ))
}

/// Extract and plan a conflict_saved upsert outcome when present.
pub fn plan_upsert_conflict_saved(
    outcome: &UpsertOutcome,
    conflict_id: ConflictId,
    detected_at: DateTime<Utc>,
) -> Result<Option<ConflictPreservationPlan>, ConflictPolicyError> {
    outcome
        .conflict_saved()
        .map(|conflict_saved| plan_conflict_saved(conflict_saved, conflict_id, detected_at))
        .transpose()
}
