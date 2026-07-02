//! Pure W3 safety fan-in planning helpers.
//!
//! These helpers connect the already-merged W3 stale-base conflict outcome with
//! the conflict policy/path primitives without touching storage, routes,
//! adapters, providers, or local filesystems. Server/storage fan-in code can use
//! this deterministic plan inside a transaction when those runtime surfaces are
//! safely wired.

use chrono::{DateTime, Utc};
use haze_sync_common::VaultPath;
use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};

use crate::{
    conflict_service::{
        generate_conflict_path, ConflictPathRequest, ConflictPolicy, ConflictPolicyError,
        ConflictRecordInput, ConflictRecordStatus, IncomingBackupPlan,
    },
    revision_service::{ConflictPolicyHint, ConflictSavedOutcome},
};

/// Deterministic pure plan for materializing a W3 conflict_saved outcome.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConflictSavedFanInPlan {
    /// Original vault path submitted by the adapter.
    pub original_path: VaultPath,
    /// Safe materialized conflict-copy path under `_haze_conflicts/open/**`.
    pub conflict_path: VaultPath,
    /// Storage-ready conflict record input. Callers must still assign a
    /// ConflictId through the storage repository surface.
    pub conflict_record: ConflictRecordInput,
    /// Object-store backup plan for the incoming content. This plan intentionally
    /// contains only hash and size metadata, never raw file bytes.
    pub incoming_backup: IncomingBackupPlan,
}

impl ConflictSavedFanInPlan {
    /// Return the safe status string used by public PUT responses.
    #[must_use]
    pub const fn public_status(&self) -> &'static str {
        "conflict_saved"
    }
}

/// Build a deterministic preservation plan for Core conflict_saved output.
pub fn plan_conflict_saved_fan_in(
    outcome: &ConflictSavedOutcome,
    timestamp: DateTime<Utc>,
) -> Result<ConflictSavedFanInPlan, ConflictFanInError> {
    let policy_applied = match outcome.policy_hint {
        ConflictPolicyHint::PreserveBoth => ConflictPolicy::PreserveBoth,
    };

    let conflict_path = generate_conflict_path(&ConflictPathRequest::new(
        outcome.incoming_content.path.clone(),
        outcome.incoming_content.adapter_id.clone(),
        timestamp,
    )?)?;

    let conflict_record = ConflictRecordInput {
        original_path: outcome.incoming_content.path.clone(),
        conflict_path: conflict_path.clone(),
        base_revision_id: outcome.provided_base_revision_id.clone(),
        current_revision_id: outcome.current_revision.revision_id.clone(),
        incoming_content_hash: outcome.incoming_content.content_hash,
        incoming_size_bytes: outcome.incoming_content.size_bytes,
        adapter_id: outcome.incoming_content.adapter_id.clone(),
        policy_applied,
        status: ConflictRecordStatus::Open,
        created_at: timestamp,
    };

    let incoming_backup = IncomingBackupPlan {
        original_path: outcome.incoming_content.path.clone(),
        conflict_path: conflict_path.clone(),
        adapter_id: outcome.incoming_content.adapter_id.clone(),
        content_hash: outcome.incoming_content.content_hash,
        size_bytes: outcome.incoming_content.size_bytes,
    };

    Ok(ConflictSavedFanInPlan {
        original_path: outcome.incoming_content.path.clone(),
        conflict_path,
        conflict_record,
        incoming_backup,
    })
}

/// Safe W3 fan-in planning errors.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConflictFanInError {
    /// Conflict path generation rejected the input, for example recursive
    /// `_haze_conflicts/**` materialization.
    ConflictPolicy(ConflictPolicyError),
}

impl ConflictFanInError {
    /// Stable machine-readable error code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::ConflictPolicy(error) => error.code(),
        }
    }
}

impl fmt::Display for ConflictFanInError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ConflictPolicy(_) => "conflict fan-in planning failed",
        })
    }
}

impl Error for ConflictFanInError {}

impl From<ConflictPolicyError> for ConflictFanInError {
    fn from(error: ConflictPolicyError) -> Self {
        Self::ConflictPolicy(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::revision_service::{
        compute_content_hash, IncomingConflictContent, StoredRevision,
    };
    use chrono::DateTime;
    use haze_sync_common::{AdapterId, RevisionId};

    fn timestamp() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-07-01T22:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn outcome(path: &str) -> ConflictSavedOutcome {
        let path = VaultPath::parse(path).unwrap();
        let adapter_id = AdapterId::parse("iphone-anna").unwrap();
        let old = b"old";
        let incoming = b"incoming";
        ConflictSavedOutcome {
            current_revision: StoredRevision {
                revision_id: RevisionId::parse("rev_current").unwrap(),
                path: path.clone(),
                parent_revision_id: None,
                content_hash: compute_content_hash(old),
                size_bytes: old.len() as u64,
                created_by: adapter_id.clone(),
            },
            provided_base_revision_id: Some(RevisionId::parse("rev_old").unwrap()),
            incoming_content: IncomingConflictContent {
                path,
                adapter_id,
                content_hash: compute_content_hash(incoming),
                size_bytes: incoming.len() as u64,
                content: incoming.to_vec(),
            },
            policy_hint: ConflictPolicyHint::PreserveBoth,
        }
    }

    #[test]
    fn plans_conflict_saved_preserve_both_without_raw_bytes() {
        let plan = plan_conflict_saved_fan_in(
            &outcome("Projects/Haze/plan.md"),
            timestamp(),
        )
        .unwrap();

        assert_eq!(plan.public_status(), "conflict_saved");
        assert_eq!(plan.original_path.as_str(), "Projects/Haze/plan.md");
        assert_eq!(
            plan.conflict_path.as_str(),
            "_haze_conflicts/open/Projects/Haze/plan.conflict.iphone-anna.2026-07-01-220000.md"
        );
        assert_eq!(plan.conflict_record.status, ConflictRecordStatus::Open);
        assert_eq!(plan.conflict_record.policy_applied, ConflictPolicy::PreserveBoth);
        assert_eq!(plan.conflict_record.current_revision_id.as_str(), "rev_current");
        assert_eq!(
            plan.conflict_record.base_revision_id.as_ref().unwrap().as_str(),
            "rev_old"
        );
        assert_eq!(
            plan.incoming_backup.content_hash,
            plan.conflict_record.incoming_content_hash
        );
    }

    #[test]
    fn rejects_recursive_conflict_source_paths() {
        let error = plan_conflict_saved_fan_in(
            &outcome("_haze_conflicts/open/Notes/a.md"),
            timestamp(),
        )
        .unwrap_err();

        assert_eq!(error.code(), "recursive_conflict_path");
    }
}
