//! Public DTO serialization audit tests.
//!
//! These tests lock down stable JSON vocabulary and verify that representative
//! public DTO payloads do not contain secret-like or raw runtime values.

use std::collections::BTreeMap;
use std::fmt::Debug;

use haze_sync_common::{AdapterId, AdapterMode, AdapterRole};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};

use crate::contracts::errors::{ErrorResponse, PublicError, PublicErrorCode, SafeErrorDetails};
use crate::dto::changes::{ChangeEntryDto, ChangesQuery, ChangesResponse};
use crate::dto::common::{
    CommonResponseStatus, ConflictPolicyDto, ConflictResolutionDto, ConflictResolveStatusDto,
    ConflictStatusDto, OperationKindDto,
};
use crate::dto::conflicts::{
    ConflictDetailSummaryDto, ConflictListQuery, ConflictListResponse, ConflictSummaryDto,
    ResolveConflictRequest, ResolveConflictResponse,
};
use crate::dto::files::{
    DeleteFileRequestMetadata, DeleteFileResponse, DeleteRejectedReasonDto, FileDownloadMetadata,
    FileIgnoredReasonDto, FileMetadataResponse, FileQuery, FileRejectedReasonDto,
    PutFileRequestMetadata, PutFileResponse,
};
use crate::dto::primitives::{
    AdapterIdDto, ConflictIdDto, ContentSha256Dto, RevisionIdDto, TimestampDto, TombstoneIdDto,
    VaultPathDto,
};
use crate::dto::server::{ServerCapabilityDto, ServerInfoResponse};
use crate::routes::admin::{
    AdapterCursorSummary, AdapterListRequest, AdapterListResponse, AdapterSummary,
    DependencyReadinessState, PauseStatusSummary, ServerStatus, StatusSummaryRequest,
    StatusSummaryResponse,
};

#[test]
fn shared_enum_vocabulary_is_snake_case_and_stable() {
    assert_json_string(&CommonResponseStatus::Accepted, "accepted");
    assert_json_string(&CommonResponseStatus::SameContent, "same_content");
    assert_json_string(&CommonResponseStatus::ConflictSaved, "conflict_saved");
    assert_json_string(&CommonResponseStatus::Tombstoned, "tombstoned");
    assert_json_string(&CommonResponseStatus::TombstoneCreated, "tombstone_created");
    assert_json_string(&CommonResponseStatus::NotFound, "not_found");
    assert_json_string(&CommonResponseStatus::ValidationError, "validation_error");
    assert_json_string(&CommonResponseStatus::Unauthorized, "unauthorized");
    assert_json_string(&CommonResponseStatus::Conflict, "conflict");
    assert_json_string(
        &CommonResponseStatus::IdempotencyConflict,
        "idempotency_conflict",
    );
    assert_json_string(&CommonResponseStatus::Rejected, "rejected");
    assert_json_string(&CommonResponseStatus::Ignored, "ignored");
    assert_json_string(&CommonResponseStatus::Resolved, "resolved");

    assert_json_string(&OperationKindDto::UpsertFile, "upsert_file");
    assert_json_string(&OperationKindDto::DeleteFile, "delete_file");
    assert_json_string(&OperationKindDto::RestoreFile, "restore_file");
    assert_json_string(&OperationKindDto::ConflictCreated, "conflict_created");
    assert_json_string(&OperationKindDto::ConflictResolved, "conflict_resolved");
    assert_json_string(&OperationKindDto::BackupCreated, "backup_created");

    assert_json_string(&ConflictPolicyDto::PreserveBoth, "preserve_both");
    assert_json_string(
        &ConflictPolicyDto::CurrentWinsWithIncomingBackup,
        "current_wins_with_incoming_backup",
    );

    assert_json_string(&ConflictStatusDto::Open, "open");
    assert_json_string(&ConflictStatusDto::Resolved, "resolved");
    assert_json_string(&ConflictStatusDto::Ignored, "ignored");

    assert_json_string(&ConflictResolutionDto::AcceptCurrent, "accept_current");
    assert_json_string(&ConflictResolutionDto::AcceptConflict, "accept_conflict");
    assert_json_string(&ConflictResolutionDto::KeepBoth, "keep_both");
    assert_json_string(&ConflictResolutionDto::MarkResolved, "mark_resolved");

    assert_json_string(&ConflictResolveStatusDto::Resolved, "resolved");
}

#[test]
fn server_info_response_roundtrips_stable_public_json() {
    let response = ServerInfoResponse {
        server_id: "haze-sync-vps-1".to_owned(),
        protocol_version: 1,
        max_upload_bytes: 52_428_800,
        capabilities: vec![
            ServerCapabilityDto::Sha256,
            ServerCapabilityDto::OperationLog,
            ServerCapabilityDto::Tombstones,
            ServerCapabilityDto::Conflicts,
            ServerCapabilityDto::ConflictCenter,
            ServerCapabilityDto::BatchChanges,
        ],
    };

    assert_eq!(
        serde_json::to_value(&response).unwrap(),
        json!({
            "server_id": "haze-sync-vps-1",
            "protocol_version": 1,
            "max_upload_bytes": 52_428_800,
            "capabilities": [
                "sha256",
                "operation_log",
                "tombstones",
                "conflicts",
                "conflict_center",
                "batch_changes"
            ]
        })
    );
    assert_roundtrip(&response);
    assert_safe_public_json(&response);
}

#[test]
fn changes_query_and_response_roundtrip_without_raw_cursor_values() {
    let query = ChangesQuery {
        since: 41,
        limit: 100,
    };
    let response = ChangesResponse {
        from_seq: 41,
        to_seq: 44,
        has_more: true,
        changes: vec![
            ChangeEntryDto {
                seq: 42,
                kind: OperationKindDto::UpsertFile,
                path: VaultPathDto::from("Projects/Haze/plan.md"),
                revision_id: Some(RevisionIdDto::from("rev_042")),
                content_sha256: Some(content_hash("a")),
                size_bytes: Some(1_842),
                tombstone_id: None,
                conflict_id: None,
                updated_by: AdapterIdDto::from("obsidian-plugin"),
                updated_at: TimestampDto::from("2026-07-02T10:00:00Z"),
            },
            ChangeEntryDto {
                seq: 43,
                kind: OperationKindDto::DeleteFile,
                path: VaultPathDto::from("Projects/Haze/old.md"),
                revision_id: None,
                content_sha256: None,
                size_bytes: None,
                tombstone_id: Some(TombstoneIdDto::from("tmb_043")),
                conflict_id: None,
                updated_by: AdapterIdDto::from("worktree-adapter"),
                updated_at: TimestampDto::from("2026-07-02T10:01:00Z"),
            },
            ChangeEntryDto {
                seq: 44,
                kind: OperationKindDto::ConflictCreated,
                path: VaultPathDto::from("Projects/Haze/plan.md"),
                revision_id: None,
                content_sha256: None,
                size_bytes: None,
                tombstone_id: None,
                conflict_id: Some(ConflictIdDto::from("conf_044")),
                updated_by: AdapterIdDto::from("gdrive-adapter"),
                updated_at: TimestampDto::from("2026-07-02T10:02:00Z"),
            },
        ],
    };

    assert_eq!(
        serde_json::to_value(&query).unwrap(),
        json!({"since": 41, "limit": 100})
    );
    assert_eq!(
        serde_json::to_value(&response).unwrap(),
        json!({
            "from_seq": 41,
            "to_seq": 44,
            "has_more": true,
            "changes": [
                {
                    "seq": 42,
                    "kind": "upsert_file",
                    "path": "Projects/Haze/plan.md",
                    "revision_id": "rev_042",
                    "content_sha256": content_hash("a"),
                    "size_bytes": 1_842,
                    "updated_by": "obsidian-plugin",
                    "updated_at": "2026-07-02T10:00:00Z"
                },
                {
                    "seq": 43,
                    "kind": "delete_file",
                    "path": "Projects/Haze/old.md",
                    "tombstone_id": "tmb_043",
                    "updated_by": "worktree-adapter",
                    "updated_at": "2026-07-02T10:01:00Z"
                },
                {
                    "seq": 44,
                    "kind": "conflict_created",
                    "path": "Projects/Haze/plan.md",
                    "conflict_id": "conf_044",
                    "updated_by": "gdrive-adapter",
                    "updated_at": "2026-07-02T10:02:00Z"
                }
            ]
        })
    );
    assert_roundtrip(&query);
    assert_roundtrip(&response);
    assert_safe_public_json(&response);
}

#[test]
fn file_metadata_and_request_metadata_do_not_embed_file_bytes() {
    let file_query = FileQuery { revision_id: None };
    let selected_query = FileQuery {
        revision_id: Some(RevisionIdDto::from("rev_042")),
    };
    let metadata = FileMetadataResponse {
        path: VaultPathDto::from("Projects/Haze/plan.md"),
        revision_id: RevisionIdDto::from("rev_042"),
        content_sha256: content_hash("b"),
        size_bytes: 1_842,
        updated_by: AdapterIdDto::from("obsidian-plugin"),
        updated_at: TimestampDto::from("2026-07-02T10:00:00Z"),
    };
    let download_metadata = FileDownloadMetadata {
        revision_id: RevisionIdDto::from("rev_042"),
        content_sha256: content_hash("b"),
        size_bytes: 1_842,
        content_type: "application/octet-stream".to_owned(),
    };
    let put_metadata = PutFileRequestMetadata {
        path: VaultPathDto::from("Projects/Haze/plan.md"),
        base_revision_id: None,
        content_sha256: content_hash("b"),
        size_bytes: Some(1_842),
    };
    let delete_metadata = DeleteFileRequestMetadata {
        path: VaultPathDto::from("Projects/Haze/plan.md"),
        base_revision_id: Some(RevisionIdDto::from("rev_042")),
    };

    assert_eq!(serde_json::to_value(&file_query).unwrap(), json!({}));
    assert_eq!(
        serde_json::to_value(&selected_query).unwrap(),
        json!({"revision_id": "rev_042"})
    );
    assert_eq!(
        serde_json::to_value(&metadata).unwrap(),
        json!({
            "path": "Projects/Haze/plan.md",
            "revision_id": "rev_042",
            "content_sha256": content_hash("b"),
            "size_bytes": 1_842,
            "updated_by": "obsidian-plugin",
            "updated_at": "2026-07-02T10:00:00Z"
        })
    );
    assert_eq!(
        serde_json::to_value(&put_metadata).unwrap(),
        json!({
            "path": "Projects/Haze/plan.md",
            "base_revision_id": null,
            "content_sha256": content_hash("b"),
            "size_bytes": 1_842
        })
    );

    for value in [
        serde_json::to_value(&metadata).unwrap(),
        serde_json::to_value(&download_metadata).unwrap(),
        serde_json::to_value(&put_metadata).unwrap(),
        serde_json::to_value(&delete_metadata).unwrap(),
    ] {
        assert_safe_json_value(&value);
        assert!(!value.to_string().contains("file_content"));
        assert!(!value.to_string().contains("content_base64"));
    }

    assert_roundtrip(&file_query);
    assert_roundtrip(&selected_query);
    assert_roundtrip(&metadata);
    assert_roundtrip(&download_metadata);
    assert_roundtrip(&put_metadata);
    assert_roundtrip(&delete_metadata);
}

#[test]
fn put_file_outcomes_roundtrip_all_public_status_variants() {
    let responses = vec![
        PutFileResponse::Accepted {
            path: VaultPathDto::from("Projects/Haze/plan.md"),
            revision_id: RevisionIdDto::from("rev_043"),
            seq: 43,
        },
        PutFileResponse::ConflictSaved {
            path: VaultPathDto::from("Projects/Haze/plan.md"),
            conflict_id: ConflictIdDto::from("conf_044"),
            materialized_path: VaultPathDto::from(
                "_haze_conflicts/open/Projects/Haze/plan.conflict.gdrive.md",
            ),
            policy_applied: ConflictPolicyDto::PreserveBoth,
            seq: 44,
        },
        PutFileResponse::Ignored {
            reason: FileIgnoredReasonDto::SameContent,
            path: VaultPathDto::from("Projects/Haze/plan.md"),
        },
        PutFileResponse::Rejected {
            reason: FileRejectedReasonDto::IdempotencyConflict,
            path: VaultPathDto::from("Projects/Haze/plan.md"),
        },
    ];

    assert_status_values(
        &responses,
        &["accepted", "conflict_saved", "ignored", "rejected"],
    );
    for response in responses {
        assert_roundtrip(&response);
        assert_safe_public_json(&response);
    }
}

#[test]
fn delete_file_outcomes_roundtrip_tombstone_guard_vocabulary() {
    let responses = vec![
        DeleteFileResponse::Tombstoned {
            path: VaultPathDto::from("Projects/Haze/old.md"),
            tombstone_id: TombstoneIdDto::from("tmb_044"),
            seq: 44,
            retention_until: TimestampDto::from("2026-08-02T10:00:00Z"),
        },
        DeleteFileResponse::NotFound {
            path: VaultPathDto::from("Projects/Haze/missing.md"),
        },
        DeleteFileResponse::Rejected {
            reason: DeleteRejectedReasonDto::UnsafeDelete,
            path: VaultPathDto::from("Projects/Haze/unsafe.md"),
        },
    ];

    assert_status_values(&responses, &["tombstoned", "not_found", "rejected"]);
    assert_json_string(
        &DeleteRejectedReasonDto::StaleBaseRevision,
        "stale_base_revision",
    );
    assert_json_string(&DeleteRejectedReasonDto::UnsafeDelete, "unsafe_delete");
    assert_json_string(
        &DeleteRejectedReasonDto::IdempotencyConflict,
        "idempotency_conflict",
    );
    assert_json_string(
        &DeleteRejectedReasonDto::ValidationError,
        "validation_error",
    );

    for response in responses {
        assert_roundtrip(&response);
        assert_safe_public_json(&response);
    }
}

#[test]
fn conflict_list_detail_and_resolve_dtos_roundtrip_safe_json() {
    let list_query = ConflictListQuery {
        status: Some(ConflictStatusDto::Open),
    };
    let summary = ConflictSummaryDto {
        conflict_id: ConflictIdDto::from("conf_044"),
        path: VaultPathDto::from("Projects/Haze/plan.md"),
        current_revision_id: RevisionIdDto::from("rev_042"),
        incoming_revision_id: RevisionIdDto::from("rev_043"),
        incoming_adapter_id: AdapterIdDto::from("gdrive-adapter"),
        policy_applied: ConflictPolicyDto::CurrentWinsWithIncomingBackup,
        materialized_path: VaultPathDto::from(
            "_haze_conflicts/open/Projects/Haze/plan.conflict.gdrive.md",
        ),
        created_at: TimestampDto::from("2026-07-02T10:03:00Z"),
        status: ConflictStatusDto::Open,
    };
    let list_response = ConflictListResponse {
        conflicts: vec![summary.clone()],
    };
    let detail = ConflictDetailSummaryDto {
        conflict_id: summary.conflict_id.clone(),
        original_path: summary.path.clone(),
        base_revision_id: None,
        current_revision_id: summary.current_revision_id.clone(),
        incoming_revision_id: summary.incoming_revision_id.clone(),
        incoming_adapter_id: summary.incoming_adapter_id.clone(),
        policy_applied: summary.policy_applied.clone(),
        materialized_path: summary.materialized_path.clone(),
        status: summary.status.clone(),
        created_at: summary.created_at.clone(),
        resolved_at: None,
        resolved_by: None,
    };
    let resolve_request = ResolveConflictRequest {
        resolution: ConflictResolutionDto::KeepBoth,
    };
    let resolve_response = ResolveConflictResponse {
        status: ConflictResolveStatusDto::Resolved,
        conflict_id: ConflictIdDto::from("conf_044"),
        resolution: ConflictResolutionDto::KeepBoth,
        seq: 45,
    };

    assert_eq!(
        serde_json::to_value(&list_query).unwrap(),
        json!({"status": "open"})
    );
    assert_eq!(
        serde_json::to_value(&detail).unwrap()["base_revision_id"],
        Value::Null
    );
    assert_eq!(
        serde_json::to_value(&resolve_request).unwrap(),
        json!({"resolution": "keep_both"})
    );
    assert_eq!(
        serde_json::to_value(&resolve_response).unwrap(),
        json!({
            "status": "resolved",
            "conflict_id": "conf_044",
            "resolution": "keep_both",
            "seq": 45
        })
    );

    for value in [
        serde_json::to_value(&list_query).unwrap(),
        serde_json::to_value(&list_response).unwrap(),
        serde_json::to_value(&detail).unwrap(),
        serde_json::to_value(&resolve_request).unwrap(),
        serde_json::to_value(&resolve_response).unwrap(),
    ] {
        assert_safe_json_value(&value);
    }

    assert_roundtrip(&list_query);
    assert_roundtrip(&list_response);
    assert_roundtrip(&detail);
    assert_roundtrip(&resolve_request);
    assert_roundtrip(&resolve_response);
}

#[test]
fn public_error_json_uses_safe_codes_and_sanitized_details_only() {
    let mut details = BTreeMap::new();
    details.insert(
        "X-Base-Revision-Id".to_owned(),
        vec!["must be a revision id or literal null".to_owned()],
    );
    details.insert(
        "Idempotency-Key".to_owned(),
        vec!["is required for write routes".to_owned()],
    );

    let response = ErrorResponse {
        error: PublicError::new(PublicErrorCode::ValidationError, "validation failed")
            .with_request_id("req_044")
            .with_details(SafeErrorDetails::Map(details)),
    };

    assert_json_string(&PublicErrorCode::InvalidRequest, "invalid_request");
    assert_json_string(&PublicErrorCode::InvalidPath, "invalid_path");
    assert_json_string(&PublicErrorCode::ValidationError, "validation_error");
    assert_json_string(&PublicErrorCode::Unauthorized, "unauthorized");
    assert_json_string(&PublicErrorCode::MissingToken, "missing_token");
    assert_json_string(&PublicErrorCode::InvalidToken, "invalid_token");
    assert_json_string(&PublicErrorCode::ForbiddenRole, "forbidden_role");
    assert_json_string(&PublicErrorCode::NotFound, "not_found");
    assert_json_string(&PublicErrorCode::Conflict, "conflict");
    assert_json_string(
        &PublicErrorCode::IdempotencyConflict,
        "idempotency_conflict",
    );
    assert_json_string(&PublicErrorCode::PayloadTooLarge, "payload_too_large");
    assert_json_string(&PublicErrorCode::RateLimited, "rate_limited");
    assert_json_string(&PublicErrorCode::UnsafeDelete, "unsafe_delete");
    assert_json_string(&PublicErrorCode::IgnoredPath, "ignored_path");
    assert_json_string(&PublicErrorCode::InternalError, "internal_error");

    assert_eq!(
        serde_json::to_value(&response).unwrap()["error"]["code"],
        json!("validation_error")
    );
    assert_roundtrip(&response);
    assert_safe_public_json(&response);
}

#[test]
fn admin_status_contract_json_is_summary_only_and_cursor_safe() {
    let status_request = StatusSummaryRequest::default();
    let adapter_request = AdapterListRequest::default();
    let status_response = StatusSummaryResponse::from_safe_parts(
        ServerStatus::Degraded,
        DependencyReadinessState::Ready,
        DependencyReadinessState::NotReady,
        Some(44),
        Some(1),
        PauseStatusSummary {
            supported: true,
            active: Some(false),
        },
    );
    let adapter = AdapterSummary::new(
        AdapterId::parse("gdrive-adapter").unwrap(),
        Some(AdapterRole::GdriveAdapter),
        true,
    )
    .with_mode(AdapterMode::ImportOnly)
    .with_last_seen_at(TimestampDto::from("2026-07-02T10:04:00Z"))
    .with_cursor(AdapterCursorSummary::new(
        Some(44),
        Some(TimestampDto::from("2026-07-02T10:05:00Z")),
        true,
    ));
    let adapter_response = AdapterListResponse::new(vec![adapter]);

    assert_eq!(
        serde_json::to_value(&status_request).unwrap(),
        json!({"include_placeholders": true})
    );
    assert_eq!(
        serde_json::to_value(&adapter_request).unwrap(),
        json!({"include_disabled": true})
    );
    assert_eq!(
        serde_json::to_value(&status_response).unwrap(),
        json!({
            "server_status": "degraded",
            "db_readiness_state": "ready",
            "object_store_readiness_state": "not_ready",
            "last_operation_sequence": 44,
            "adapter_count": 1,
            "pause": {"supported": true, "active": false}
        })
    );
    assert_eq!(
        serde_json::to_value(&adapter_response).unwrap(),
        json!({
            "total_count": 1,
            "adapters": [
                {
                    "adapter_id": "gdrive-adapter",
                    "role": "gdrive_adapter",
                    "mode": "import_only",
                    "enabled": true,
                    "last_seen_at": "2026-07-02T10:04:00Z",
                    "cursor": {
                        "last_core_seq": 44,
                        "last_success_at": "2026-07-02T10:05:00Z",
                        "has_external_cursor": true
                    }
                }
            ]
        })
    );

    assert_roundtrip(&status_request);
    assert_roundtrip(&adapter_request);
    assert_roundtrip(&status_response);
    assert_roundtrip(&adapter_response);
    assert_safe_public_json(&status_response);
    assert_safe_public_json(&adapter_response);
}

fn content_hash(hex_digit: &str) -> ContentSha256Dto {
    ContentSha256Dto::from(format!("sha256:{}", hex_digit.repeat(64)))
}

fn assert_json_string<T>(value: &T, expected: &str)
where
    T: Serialize,
{
    assert_eq!(serde_json::to_value(value).unwrap(), json!(expected));
}

fn assert_roundtrip<T>(value: &T)
where
    T: Serialize + DeserializeOwned + PartialEq + Debug,
{
    let json = serde_json::to_string(value).unwrap();
    let decoded: T = serde_json::from_str(&json).unwrap();
    assert_eq!(&decoded, value);
}

fn assert_status_values<T>(responses: &[T], expected_statuses: &[&str])
where
    T: Serialize,
{
    let statuses = responses
        .iter()
        .map(|response| serde_json::to_value(response).unwrap()["status"].clone())
        .collect::<Vec<_>>();
    let expected = expected_statuses
        .iter()
        .map(|status| json!(status))
        .collect::<Vec<_>>();

    assert_eq!(statuses, expected);
}

fn assert_safe_public_json<T>(value: &T)
where
    T: Serialize,
{
    assert_safe_json_value(&serde_json::to_value(value).unwrap());
}

fn assert_safe_json_value(value: &Value) {
    let json = value.to_string().to_lowercase();
    for forbidden in [
        "bearer ",
        "oauth",
        "access_token",
        "refresh_token",
        "token_hash",
        "secret",
        "database_url",
        "postgres://",
        "mysql://",
        "/home/",
        "/users/",
        "/srv/",
        "c:\\",
        "stack_trace",
        "backtrace",
        "provider_payload",
        "raw_request_body",
        "request_body",
        "body_bytes",
        "file_bytes",
        "content_bytes",
        "content_base64",
        "raw_cursor",
        "cursor_value",
        "page_token",
        "idempotency_key_value",
    ] {
        assert!(
            !json.contains(forbidden),
            "public DTO JSON leaked forbidden marker {forbidden}: {json}"
        );
    }
}
