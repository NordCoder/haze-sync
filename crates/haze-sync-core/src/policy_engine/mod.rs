//! Pure conflict policy engine for safe Core conflict preservation.
//!
//! The policy engine decides how to preserve both sides of a file conflict and
//! prepares storage/API-neutral record shapes. It does not store revisions,
//! write files, append operation-log entries, call adapters, or resolve
//! conflicts.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::conflict_service::{
    generate_conflict_path, validate_original_path, ConflictPathRequest, ConflictPolicy,
    ConflictPolicyError, ConflictRecordInput, ConflictRecordStatus, CurrentRevision,
    IncomingBackupPlan, IncomingConflictCandidate,
};

/// Request accepted by the pure conflict policy engine.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConflictPolicyRequest {
    /// Policy Core should apply for this conflict.
    pub policy: ConflictPolicy,
    /// Current winning Core revision.
    pub current_revision: CurrentRevision,
    /// Incoming content that must not overwrite the current revision silently.
    pub incoming_candidate: IncomingConflictCandidate,
    /// UTC decision timestamp used for record and path generation.
    pub detected_at: DateTime<Utc>,
}

impl ConflictPolicyRequest {
    /// Build and validate a conflict policy request from safe Core primitives.
    pub fn new(
        policy: ConflictPolicy,
        current_revision: CurrentRevision,
        incoming_candidate: IncomingConflictCandidate,
        detected_at: DateTime<Utc>,
    ) -> Result<Self, ConflictPolicyError> {
        validate_conflict_inputs(&current_revision, &incoming_candidate)?;
        Ok(Self {
            policy,
            current_revision,
            incoming_candidate,
            detected_at,
        })
    }
}

/// Preserve-both outcome. Current remains authoritative and incoming is planned as a conflict copy.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PreserveBothOutcome {
    /// Current winning Core revision.
    pub current_revision: CurrentRevision,
    /// Planned materialization for incoming content.
    pub incoming_backup: IncomingBackupPlan,
    /// Conflict record input for later storage/API fan-in.
    pub conflict_record: ConflictRecordInput,
}

/// Current-wins outcome. Current remains authoritative and incoming is planned as a backup copy.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CurrentWinsWithIncomingBackupOutcome {
    /// Current winning Core revision.
    pub current_revision: CurrentRevision,
    /// Planned materialization for incoming content.
    pub incoming_backup: IncomingBackupPlan,
    /// Conflict record input for later storage/API fan-in.
    pub conflict_record: ConflictRecordInput,
}

/// Safe conflict policy outcome variants.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "policy_applied", content = "outcome")]
pub enum ConflictPolicyOutcome {
    /// Default V1 policy: preserve current and incoming content.
    PreserveBoth(PreserveBothOutcome),
    /// Keep current as winner while preserving incoming content as backup.
    CurrentWinsWithIncomingBackup(CurrentWinsWithIncomingBackupOutcome),
}

impl ConflictPolicyOutcome {
    /// Current winning revision after policy application.
    #[must_use]
    pub fn current_revision(&self) -> &CurrentRevision {
        match self {
            Self::PreserveBoth(outcome) => &outcome.current_revision,
            Self::CurrentWinsWithIncomingBackup(outcome) => &outcome.current_revision,
        }
    }

    /// Planned incoming-content backup/conflict copy.
    #[must_use]
    pub fn incoming_backup(&self) -> &IncomingBackupPlan {
        match self {
            Self::PreserveBoth(outcome) => &outcome.incoming_backup,
            Self::CurrentWinsWithIncomingBackup(outcome) => &outcome.incoming_backup,
        }
    }

    /// Pure conflict record input for later storage/API fan-in.
    #[must_use]
    pub fn conflict_record(&self) -> &ConflictRecordInput {
        match self {
            Self::PreserveBoth(outcome) => &outcome.conflict_record,
            Self::CurrentWinsWithIncomingBackup(outcome) => &outcome.conflict_record,
        }
    }
}

/// Apply a V1 Core conflict policy without mutating storage or runtime state.
pub fn apply_conflict_policy(
    request: ConflictPolicyRequest,
) -> Result<ConflictPolicyOutcome, ConflictPolicyError> {
    validate_conflict_inputs(&request.current_revision, &request.incoming_candidate)?;

    let ConflictPolicyRequest {
        policy,
        current_revision,
        incoming_candidate,
        detected_at,
    } = request;

    let conflict_path_request = ConflictPathRequest::new(
        incoming_candidate.original_path.clone(),
        incoming_candidate.adapter_id.clone(),
        detected_at,
    )?;
    let conflict_path = generate_conflict_path(&conflict_path_request)?;

    let incoming_backup = IncomingBackupPlan {
        original_path: incoming_candidate.original_path.clone(),
        conflict_path: conflict_path.clone(),
        adapter_id: incoming_candidate.adapter_id.clone(),
        content_hash: incoming_candidate.content_hash,
        size_bytes: incoming_candidate.size_bytes,
    };
    let conflict_record = ConflictRecordInput {
        original_path: incoming_candidate.original_path,
        conflict_path,
        base_revision_id: incoming_candidate.base_revision_id,
        current_revision_id: current_revision.revision_id.clone(),
        incoming_content_hash: incoming_candidate.content_hash,
        incoming_size_bytes: incoming_candidate.size_bytes,
        adapter_id: incoming_candidate.adapter_id,
        policy_applied: policy,
        status: ConflictRecordStatus::Open,
        created_at: detected_at,
    };

    match policy {
        ConflictPolicy::PreserveBoth => Ok(ConflictPolicyOutcome::PreserveBoth(
            PreserveBothOutcome {
                current_revision,
                incoming_backup,
                conflict_record,
            },
        )),
        ConflictPolicy::CurrentWinsWithIncomingBackup => {
            Ok(ConflictPolicyOutcome::CurrentWinsWithIncomingBackup(
                CurrentWinsWithIncomingBackupOutcome {
                    current_revision,
                    incoming_backup,
                    conflict_record,
                },
            ))
        }
    }
}

fn validate_conflict_inputs(
    current_revision: &CurrentRevision,
    incoming_candidate: &IncomingConflictCandidate,
) -> Result<(), ConflictPolicyError> {
    validate_original_path(&current_revision.path)?;
    validate_original_path(&incoming_candidate.original_path)?;
    if current_revision.path != incoming_candidate.original_path {
        return Err(ConflictPolicyError::PathMismatch);
    }
    Ok(())
}
