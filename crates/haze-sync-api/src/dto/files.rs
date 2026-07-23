//! DTOs for GET, PUT, and DELETE /v1/files/{path}.
//!
//! File content itself is raw bytes and is intentionally not represented here.
//! These DTOs cover only JSON metadata, response bodies, and byte download
//! metadata headers for later route layers.

use serde::{Deserialize, Serialize};

use crate::dto::common::ConflictPolicyDto;
use crate::dto::primitives::{
    AdapterIdDto, ConflictIdDto, ContentSha256Dto, RevisionIdDto, TimestampDto, TombstoneIdDto,
    VaultPathDto,
};

/// Query parameters for GET /v1/files/{path} metadata selection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileQuery {
    /// Optional revision to download; current revision is selected when absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision_id: Option<RevisionIdDto>,
}

/// JSON metadata shape for the current or selected file revision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileMetadataResponse {
    /// Vault path of the file.
    pub path: VaultPathDto,
    /// Selected revision identifier.
    pub revision_id: RevisionIdDto,
    /// SHA-256 content hash for the selected revision.
    pub content_sha256: ContentSha256Dto,
    /// Size of the selected byte payload.
    pub size_bytes: u64,
    /// Adapter that created the selected revision.
    pub updated_by: AdapterIdDto,
    /// Timestamp when the selected revision was created.
    pub updated_at: TimestampDto,
}

/// Metadata usually represented by GET /v1/files/{path} download headers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileDownloadMetadata {
    /// Header-equivalent X-Revision-Id value.
    pub revision_id: RevisionIdDto,
    /// Header-equivalent X-Content-SHA256 value.
    pub content_sha256: ContentSha256Dto,
    /// Header-equivalent X-Size-Bytes value.
    pub size_bytes: u64,
    /// Download content type, normally application/octet-stream.
    pub content_type: String,
}

/// Metadata contract for PUT /v1/files/{path}; bytes are sent separately.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PutFileRequestMetadata {
    /// Vault path from the route parameter after normalization.
    pub path: VaultPathDto,
    /// Base revision from X-Base-Revision-Id, or null when explicitly unknown.
    pub base_revision_id: Option<RevisionIdDto>,
    /// Expected upload hash from X-Content-SHA256.
    pub content_sha256: ContentSha256Dto,
    /// Optional upload size when the route layer can determine it before apply.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
}

/// JSON response body for PUT /v1/files/{path}.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PutFileResponse {
    /// New file revision was accepted and operation log entry was appended.
    Accepted {
        /// Path accepted by Core.
        path: VaultPathDto,
        /// Newly created revision identifier.
        revision_id: RevisionIdDto,
        /// Operation log sequence for the accepted write.
        seq: i64,
    },
    /// Incoming content was preserved as a conflict rather than overwriting.
    ConflictSaved {
        /// Original path submitted by the adapter.
        path: VaultPathDto,
        /// Created conflict record identifier.
        conflict_id: ConflictIdDto,
        /// Materialized conflict copy path.
        materialized_path: VaultPathDto,
        /// Conflict policy applied by Core.
        policy_applied: ConflictPolicyDto,
        /// Operation log sequence for the conflict event.
        seq: i64,
    },
    /// Incoming write was a safe no-op, such as same content.
    Ignored {
        /// Public no-op reason.
        reason: FileIgnoredReasonDto,
        /// Path submitted by the adapter.
        path: VaultPathDto,
    },
    /// Incoming write was rejected by a public validation or policy rule.
    Rejected {
        /// Public rejection reason.
        reason: FileRejectedReasonDto,
        /// Path submitted by the adapter.
        path: VaultPathDto,
    },
}

/// Public no-op reasons for PUT /v1/files/{path}.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileIgnoredReasonDto {
    /// Incoming bytes match current content.
    SameContent,
}

/// Public rejection reasons for PUT /v1/files/{path}.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileRejectedReasonDto {
    /// Uploaded bytes did not match X-Content-SHA256.
    HashMismatch,
    /// Write could not prove a safe current base revision.
    StaleBaseRevision,
    /// Path is intentionally ignored by sync policy.
    IgnoredPath,
    /// Request failed validation before Core apply semantics.
    ValidationError,
    /// Request conflicted with idempotency contract.
    IdempotencyConflict,
}

/// Metadata contract for DELETE /v1/files/{path}.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeleteFileRequestMetadata {
    /// Vault path from the route parameter after normalization.
    pub path: VaultPathDto,
    /// Base revision from X-Base-Revision-Id, or null when explicitly unknown.
    pub base_revision_id: Option<RevisionIdDto>,
}

/// JSON response body for DELETE /v1/files/{path}.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum DeleteFileResponse {
    /// Delete created a tombstone and preserved revision history.
    Tombstoned {
        /// Tombstoned path.
        path: VaultPathDto,
        /// Created tombstone identifier.
        tombstone_id: TombstoneIdDto,
        /// Operation log sequence for the delete event.
        seq: i64,
        /// Timestamp after which physical cleanup may be considered.
        retention_until: TimestampDto,
    },
    /// File was not found as a current file.
    NotFound {
        /// Path submitted by the adapter.
        path: VaultPathDto,
    },
    /// Delete was rejected by stale-base or mass-delete safety rules.
    Rejected {
        /// Public rejection reason.
        reason: DeleteRejectedReasonDto,
        /// Path submitted by the adapter.
        path: VaultPathDto,
    },
}

/// Public rejection reasons for DELETE /v1/files/{path}.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeleteRejectedReasonDto {
    /// Delete could not prove a safe current base revision.
    StaleBaseRevision,
    /// Delete guard rejected a potentially dangerous delete.
    UnsafeDelete,
    /// Request conflicted with idempotency contract.
    IdempotencyConflict,
    /// Request failed validation before delete semantics.
    ValidationError,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash() -> ContentSha256Dto {
        ContentSha256Dto::from(format!("sha256:{}", "b".repeat(64)))
    }

    #[test]
    fn file_metadata_roundtrips_json() {
        let metadata = FileMetadataResponse {
            path: VaultPathDto::from("Projects/Haze/plan.md"),
            revision_id: RevisionIdDto::from("rev_124"),
            content_sha256: hash(),
            size_bytes: 1_842,
            updated_by: AdapterIdDto::from("iphone-anna"),
            updated_at: TimestampDto::from("2026-07-01T22:00:00Z"),
        };

        let json = serde_json::to_string(&metadata).unwrap();
        assert!(json.contains("content_sha256"));

        let decoded: FileMetadataResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, metadata);
    }

    #[test]
    fn put_file_responses_match_status_contract() {
        let accepted = PutFileResponse::Accepted {
            path: VaultPathDto::from("Projects/Haze/plan.md"),
            revision_id: RevisionIdDto::from("rev_124"),
            seq: 12_381,
        };
        let conflict = PutFileResponse::ConflictSaved {
            path: VaultPathDto::from("Projects/Haze/plan.md"),
            conflict_id: ConflictIdDto::from("conf_01J"),
            materialized_path: VaultPathDto::from(
                "_haze_conflicts/open/Projects/Haze/plan.conflict.iphone-anna.md",
            ),
            policy_applied: ConflictPolicyDto::PreserveBoth,
            seq: 12_382,
        };
        let ignored = PutFileResponse::Ignored {
            reason: FileIgnoredReasonDto::SameContent,
            path: VaultPathDto::from("Projects/Haze/plan.md"),
        };
        let hash_mismatch = PutFileResponse::Rejected {
            reason: FileRejectedReasonDto::HashMismatch,
            path: VaultPathDto::from("Projects/Haze/plan.md"),
        };
        let stale = PutFileResponse::Rejected {
            reason: FileRejectedReasonDto::StaleBaseRevision,
            path: VaultPathDto::from("Projects/Haze/plan.md"),
        };

        assert!(serde_json::to_string(&accepted)
            .unwrap()
            .contains("\"accepted\""));
        assert!(serde_json::to_string(&conflict)
            .unwrap()
            .contains("conflict_saved"));
        assert!(serde_json::to_string(&ignored)
            .unwrap()
            .contains("same_content"));
        assert!(serde_json::to_string(&hash_mismatch)
            .unwrap()
            .contains("hash_mismatch"));
        assert!(serde_json::to_string(&stale)
            .unwrap()
            .contains("stale_base_revision"));
    }

    #[test]
    fn delete_file_response_roundtrips_tombstone_json() {
        let response = DeleteFileResponse::Tombstoned {
            path: VaultPathDto::from("Projects/Haze/old.md"),
            tombstone_id: TombstoneIdDto::from("tmb_01J"),
            seq: 12_382,
            retention_until: TimestampDto::from("2026-08-01T00:00:00Z"),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("tombstoned"));
        assert!(json.contains("retention_until"));

        let decoded: DeleteFileResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, response);
    }
}
