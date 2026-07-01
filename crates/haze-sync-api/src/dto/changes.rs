//! DTOs for GET /v1/changes.
//!
//! This module defines change feed query and JSON response shapes only. It does
//! not implement cursor updates, operation log storage, or route behavior.

use serde::{Deserialize, Serialize};

use crate::dto::common::OperationKindDto;
use crate::dto::primitives::{
    AdapterIdDto, ConflictIdDto, ContentSha256Dto, RevisionIdDto, TimestampDto, TombstoneIdDto,
    VaultPathDto,
};

/// Query parameters for GET /v1/changes?since={seq}&limit={n}.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangesQuery {
    /// Return operations strictly after this sequence number.
    pub since: i64,
    /// Maximum number of operations to return.
    pub limit: u32,
}

/// Response body for GET /v1/changes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangesResponse {
    /// Sequence supplied by the adapter request.
    pub from_seq: i64,
    /// Highest sequence included in this response, or from_seq when empty.
    pub to_seq: i64,
    /// True when the adapter should request another page.
    pub has_more: bool,
    /// Ordered operation log entries after from_seq.
    pub changes: Vec<ChangeEntryDto>,
}

/// Single operation log entry returned by GET /v1/changes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeEntryDto {
    /// Global append-only sequence number.
    pub seq: i64,
    /// Public operation kind.
    pub kind: OperationKindDto,
    /// Vault path affected by the operation.
    pub path: VaultPathDto,
    /// Revision associated with file-like operations when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision_id: Option<RevisionIdDto>,
    /// Content hash for upsert-like operations when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_sha256: Option<ContentSha256Dto>,
    /// Content size for upsert-like operations when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    /// Tombstone associated with delete operations when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tombstone_id: Option<TombstoneIdDto>,
    /// Conflict associated with conflict operations when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflict_id: Option<ConflictIdDto>,
    /// Adapter that caused the operation.
    pub updated_by: AdapterIdDto,
    /// Timestamp when the operation was recorded.
    pub updated_at: TimestampDto,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changes_response_roundtrips_representative_json() {
        let response = ChangesResponse {
            from_seq: 12_345,
            to_seq: 12_380,
            has_more: false,
            changes: vec![ChangeEntryDto {
                seq: 12_380,
                kind: OperationKindDto::UpsertFile,
                path: VaultPathDto::from("Projects/Haze/plan.md"),
                revision_id: Some(RevisionIdDto::from("rev_01J")),
                content_sha256: Some(ContentSha256Dto::from(format!("sha256:{}", "a".repeat(64)))),
                size_bytes: Some(1_842),
                tombstone_id: None,
                conflict_id: None,
                updated_by: AdapterIdDto::from("gdrive-adapter"),
                updated_at: TimestampDto::from("2026-07-01T22:00:00Z"),
            }],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("upsert_file"));
        assert!(json.contains("content_sha256"));
        assert!(!json.contains("tombstone_id"));

        let decoded: ChangesResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, response);
    }
}
