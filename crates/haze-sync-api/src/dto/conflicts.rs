//! DTOs for GET /v1/conflicts and POST /v1/conflicts/{conflict_id}/resolve.
//!
//! This module defines Conflict Center JSON shapes only. It does not implement
//! conflict policy decisions, file replacement, backup creation, or route wiring.

use serde::{Deserialize, Serialize};

use crate::dto::common::{
    ConflictPolicyDto, ConflictResolutionDto, ConflictResolveStatusDto, ConflictStatusDto,
};
use crate::dto::primitives::{
    AdapterIdDto, ConflictIdDto, RevisionIdDto, TimestampDto, VaultPathDto,
};

/// Query parameters for GET /v1/conflicts?status=open.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictListQuery {
    /// Optional conflict status filter. The Conflict Center normally requests open.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ConflictStatusDto>,
}

/// Response body for GET /v1/conflicts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictListResponse {
    /// Public conflict summaries visible to clients.
    pub conflicts: Vec<ConflictSummaryDto>,
}

/// Summary shape for conflict list entries.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictSummaryDto {
    /// Public conflict identifier.
    pub conflict_id: ConflictIdDto,
    /// Original path that encountered a conflicting incoming change.
    pub path: VaultPathDto,
    /// Current revision at the original path.
    pub current_revision_id: RevisionIdDto,
    /// Incoming preserved revision.
    pub incoming_revision_id: RevisionIdDto,
    /// Adapter that submitted the incoming change.
    pub incoming_adapter_id: AdapterIdDto,
    /// Policy Core applied to preserve the conflict safely.
    pub policy_applied: ConflictPolicyDto,
    /// Materialized conflict copy path.
    pub materialized_path: VaultPathDto,
    /// Conflict creation timestamp.
    pub created_at: TimestampDto,
    /// Current conflict lifecycle status.
    pub status: ConflictStatusDto,
}

/// More complete public summary shape for a single conflict.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictDetailSummaryDto {
    /// Public conflict identifier.
    pub conflict_id: ConflictIdDto,
    /// Original path that encountered a conflicting incoming change.
    pub original_path: VaultPathDto,
    /// Base revision the incoming adapter claimed, or null when unknown.
    pub base_revision_id: Option<RevisionIdDto>,
    /// Current revision at the original path.
    pub current_revision_id: RevisionIdDto,
    /// Incoming preserved revision.
    pub incoming_revision_id: RevisionIdDto,
    /// Adapter that submitted the incoming change.
    pub incoming_adapter_id: AdapterIdDto,
    /// Policy Core applied to preserve the conflict safely.
    pub policy_applied: ConflictPolicyDto,
    /// Materialized conflict copy path.
    pub materialized_path: VaultPathDto,
    /// Current conflict lifecycle status.
    pub status: ConflictStatusDto,
    /// Conflict creation timestamp.
    pub created_at: TimestampDto,
    /// Conflict resolution timestamp when resolved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_at: Option<TimestampDto>,
    /// Adapter or admin that resolved the conflict when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_by: Option<AdapterIdDto>,
}

/// Request body for POST /v1/conflicts/{conflict_id}/resolve.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolveConflictRequest {
    /// Requested safe resolution action.
    pub resolution: ConflictResolutionDto,
}

/// Response body for POST /v1/conflicts/{conflict_id}/resolve.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolveConflictResponse {
    /// Resolution status, normally resolved.
    pub status: ConflictResolveStatusDto,
    /// Conflict identifier resolved by the action.
    pub conflict_id: ConflictIdDto,
    /// Action that was applied.
    pub resolution: ConflictResolutionDto,
    /// Operation log sequence for the resolution event.
    pub seq: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conflict_list_roundtrips_contract_json() {
        let response = ConflictListResponse {
            conflicts: vec![ConflictSummaryDto {
                conflict_id: ConflictIdDto::from("conf_01J"),
                path: VaultPathDto::from("Projects/Haze/plan.md"),
                current_revision_id: RevisionIdDto::from("rev_124"),
                incoming_revision_id: RevisionIdDto::from("rev_125"),
                incoming_adapter_id: AdapterIdDto::from("worktree-adapter"),
                policy_applied: ConflictPolicyDto::PreserveBoth,
                materialized_path: VaultPathDto::from(
                    "_haze_conflicts/open/Projects/Haze/plan.conflict.worktree.md",
                ),
                created_at: TimestampDto::from("2026-07-01T22:30:00Z"),
                status: ConflictStatusDto::Open,
            }],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("preserve_both"));
        assert!(json.contains("\"open\""));

        let decoded: ConflictListResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, response);
    }

    #[test]
    fn resolve_conflict_request_and_response_roundtrip_json() {
        let request = ResolveConflictRequest {
            resolution: ConflictResolutionDto::AcceptCurrent,
        };
        let response = ResolveConflictResponse {
            status: ConflictResolveStatusDto::Resolved,
            conflict_id: ConflictIdDto::from("conf_01J"),
            resolution: ConflictResolutionDto::AcceptCurrent,
            seq: 12_390,
        };

        let request_json = serde_json::to_string(&request).unwrap();
        let response_json = serde_json::to_string(&response).unwrap();

        assert!(request_json.contains("accept_current"));
        assert!(response_json.contains("resolved"));
        assert_eq!(
            serde_json::from_str::<ResolveConflictRequest>(&request_json).unwrap(),
            request
        );
        assert_eq!(
            serde_json::from_str::<ResolveConflictResponse>(&response_json).unwrap(),
            response
        );
    }
}
