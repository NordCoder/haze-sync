//! Shared public enums used by multiple API DTOs.
//!
//! These enums define JSON contract vocabulary only. They do not perform Core
//! conflict, idempotency, delete, or authorization decisions.

use serde::{Deserialize, Serialize};

/// Common response status vocabulary across Core API write and error surfaces.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommonResponseStatus {
    /// Write was accepted and created a new Core revision or operation.
    Accepted,
    /// Incoming content matched current content and can be treated as a no-op.
    SameContent,
    /// Incoming content was preserved as a conflict instead of overwriting.
    ConflictSaved,
    /// Delete created a tombstone according to the contract wording.
    Tombstoned,
    /// Alternative tombstone status name available for future fan-in mapping.
    TombstoneCreated,
    /// Requested public resource was not found.
    NotFound,
    /// Request data failed validation.
    ValidationError,
    /// Request was not authorized.
    Unauthorized,
    /// Request conflicted with current state.
    Conflict,
    /// Idempotency key was reused with a different request.
    IdempotencyConflict,
    /// Request was rejected by public policy.
    Rejected,
    /// Request was ignored as a safe no-op or ignored-path operation.
    Ignored,
    /// Conflict resolution completed.
    Resolved,
}

/// Operation log kind values returned by GET /v1/changes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationKindDto {
    /// File was created or updated.
    UpsertFile,
    /// File was tombstoned.
    DeleteFile,
    /// Tombstoned file was restored.
    RestoreFile,
    /// Conflict record was created.
    ConflictCreated,
    /// Conflict record was resolved.
    ConflictResolved,
    /// Backup copy was created by a conflict policy.
    BackupCreated,
}

/// Conflict policy names exposed by API responses.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictPolicyDto {
    /// Preserve both current and incoming content.
    PreserveBoth,
    /// Keep current content while storing incoming content as a backup copy.
    CurrentWinsWithIncomingBackup,
}

/// Conflict status filter and response values.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictStatusDto {
    /// Conflict is unresolved and visible in the Conflict Center.
    Open,
    /// Conflict has been resolved by an adapter or admin action.
    Resolved,
    /// Conflict was intentionally ignored.
    Ignored,
}

/// Supported POST /v1/conflicts/{conflict_id}/resolve actions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolutionDto {
    /// Keep the current main file unchanged and resolve the conflict record.
    AcceptCurrent,
    /// Replace the main file with the conflict content through a new revision.
    AcceptConflict,
    /// Keep both current and materialized conflict copies, then resolve.
    KeepBoth,
    /// Mark the record resolved after a manual merge or external action.
    MarkResolved,
}

/// Conflict resolution response status values.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolveStatusDto {
    /// Conflict was marked resolved.
    Resolved,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enums_use_snake_case_json() {
        assert_eq!(
            serde_json::to_string(&OperationKindDto::ConflictCreated).unwrap(),
            "\"conflict_created\""
        );
        assert_eq!(
            serde_json::to_string(&ConflictResolutionDto::AcceptCurrent).unwrap(),
            "\"accept_current\""
        );
    }
}
