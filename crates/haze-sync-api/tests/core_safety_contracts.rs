use std::collections::BTreeMap;

use haze_sync_api::{
    contracts::{
        errors::{ErrorResponse, PublicError, PublicErrorCode, SafeErrorDetails},
        headers::{IDEMPOTENCY_KEY_HEADER, X_BASE_REVISION_ID_HEADER},
    },
    dto::{
        changes::ChangeEntryDto,
        common::{
            ConflictPolicyDto, ConflictResolutionDto, ConflictResolveStatusDto, ConflictStatusDto,
            OperationKindDto,
        },
        files::{FileIgnoredReasonDto, PutFileResponse},
        primitives::{
            AdapterIdDto, ConflictIdDto, ContentSha256Dto, RevisionIdDto, TimestampDto,
            TombstoneIdDto, VaultPathDto,
        },
    },
    routes::{
        changes::{changes_response_from_parts, parse_changes_query},
        conflicts::{
            conflict_list_response_from_parts, parse_conflicts_query,
            parse_resolve_conflict_request, resolved_conflict_response, ConflictListRequestParts,
            ConflictRouteSummaryParts, ResolveConflictRequestParts,
        },
        delete::{
            parse_delete_file_request, stale_base_delete_response, tombstoned_delete_response,
            unsafe_delete_response, DeleteFileRouteRequestParts, DeleteRouteError,
            HTTP_STATUS_BAD_REQUEST,
        },
        files::{
            accepted_upload_response, ignored_same_content_response, parse_get_file_request,
            parse_put_file_request, FileDownloadRouteHeaders, GetFileRouteRequestParts,
            PutFileRouteRequestParts, APPLICATION_OCTET_STREAM,
        },
    },
};
use haze_sync_common::{AdapterId, ConflictId, ContentHash, RevisionId, VaultPath};
use haze_sync_core::revision_service::compute_content_hash;
use serde_json::json;

fn path(input: &str) -> VaultPath {
    VaultPath::parse(input).expect("fixture path should parse")
}

fn revision_id(input: &str) -> RevisionId {
    RevisionId::parse(input).expect("fixture revision id should parse")
}

fn conflict_id(input: &str) -> ConflictId {
    ConflictId::parse(input).expect("fixture conflict id should parse")
}

fn adapter_id(input: &str) -> AdapterId {
    AdapterId::parse(input).expect("fixture adapter id should parse")
}

fn content_hash(bytes: &[u8]) -> ContentHash {
    compute_content_hash(bytes)
}

#[test]
fn conflict_delete_and_idempotency_errors_are_safe_public_json() {
    let mut details = BTreeMap::new();
    details.insert(
        "path".to_owned(),
        vec!["operation requires conflict center review".to_owned()],
    );

    let responses = [
        ErrorResponse {
            error: PublicError::new(PublicErrorCode::Conflict, "write preserved as conflict")
                .with_request_id("req_conflict_01J")
                .with_details(SafeErrorDetails::Map(details)),
        },
        ErrorResponse {
            error: PublicError::new(
                PublicErrorCode::UnsafeDelete,
                "delete rejected by safety rules",
            )
            .with_request_id("req_delete_01J"),
        },
        ErrorResponse {
            error: PublicError::new(
                PublicErrorCode::IdempotencyConflict,
                "idempotency key was reused for a different request",
            )
            .with_details(SafeErrorDetails::List(vec![
                "submit a new idempotency key for a different write".to_owned(),
            ])),
        },
    ];

    for response in responses {
        let serialized = serde_json::to_string(&response).expect("public error should serialize");
        assert!(serialized.contains("error"));
        assert!(!serialized.contains("DATABASE_URL"));
        assert!(!serialized.contains("Bearer"));
        assert!(!serialized.contains("token"));
        assert!(!serialized.contains("/srv/"));
        assert!(!serialized.contains("stack"));
        assert!(!serialized.contains("provider_payload"));
        assert!(!serialized.contains("request_body"));
    }
}

#[test]
fn conflict_resolution_action_vocabulary_is_json_serializable_spec() {
    let actions = json!([
        "accept_current",
        "accept_conflict",
        "keep_both",
        "mark_resolved"
    ]);

    assert_eq!(actions[0], "accept_current");
    assert_eq!(actions[1], "accept_conflict");
    assert_eq!(actions[2], "keep_both");
    assert_eq!(actions[3], "mark_resolved");
}

#[test]
fn get_conflicts_open_maps_to_safe_public_dto() {
    let request = parse_conflicts_query(ConflictListRequestParts {
        status: Some("open"),
    })
    .expect("open status should parse");
    assert_eq!(request.status, Some(ConflictStatusDto::Open));

    let response = conflict_list_response_from_parts(vec![ConflictRouteSummaryParts {
        conflict_id: conflict_id("conf_01JW3"),
        original_path: path("Projects/Haze/plan.md"),
        conflict_path: path(
            "_haze_conflicts/open/Projects/Haze/plan.conflict.iphone-anna.2026-07-01-120000.md",
        ),
        current_revision_id: revision_id("rev_current"),
        conflict_revision_id: Some(revision_id("rev_conflict")),
        incoming_revision_id: Some(revision_id("rev_conflict")),
        source_adapter_id: adapter_id("iphone-anna"),
        policy_applied: ConflictPolicyDto::PreserveBoth,
        status: ConflictStatusDto::Open,
        created_at: Some(TimestampDto::from("2026-07-01T12:00:00Z")),
        updated_at: None,
    }]);

    let serialized = serde_json::to_string(&response).expect("conflict DTO should serialize");
    assert!(serialized.contains("_haze_conflicts/open/"));
    assert!(serialized.contains("preserve_both"));
    assert!(!serialized.contains("DATABASE_URL"));
    assert!(!serialized.contains("Bearer"));
    assert!(!serialized.contains("token_hash"));
    assert!(!serialized.contains("/srv/"));
}

#[test]
fn conflict_resolution_actions_are_accepted_or_safely_deferred() {
    for (raw, expected) in [
        ("accept_current", ConflictResolutionDto::AcceptCurrent),
        ("accept_conflict", ConflictResolutionDto::AcceptConflict),
        ("keep_both", ConflictResolutionDto::KeepBoth),
        ("mark_resolved", ConflictResolutionDto::MarkResolved),
    ] {
        let request = parse_resolve_conflict_request(ResolveConflictRequestParts {
            conflict_id: "conf_01JW3",
            resolution: Some(raw),
            extra_fields: &[],
        })
        .expect("resolution action should parse safely");
        assert_eq!(request.conflict_id, conflict_id("conf_01JW3"));
        assert_eq!(request.resolution, expected.clone());

        let response = resolved_conflict_response(conflict_id("conf_01JW3"), expected, 27);
        assert_eq!(response.status, ConflictResolveStatusDto::Resolved);
        assert_eq!(response.seq, 27);
        let serialized = serde_json::to_string(&response).expect("response should serialize");
        assert!(serialized.contains(raw));
        assert!(!serialized.contains("Bearer"));
        assert!(!serialized.contains("DATABASE_URL"));
        assert!(!serialized.contains("request_body"));
    }
}

#[test]
fn delete_route_requires_idempotency_key() {
    let error = parse_delete_file_request(DeleteFileRouteRequestParts {
        route_path: "Projects/Haze/old.md",
        idempotency_key: None,
        base_revision_id: Some("rev_current"),
        requested_delete_count: None,
    })
    .expect_err("DELETE must require Idempotency-Key");

    assert_eq!(error.http_status_code(), HTTP_STATUS_BAD_REQUEST);
    assert_eq!(error.public_code(), PublicErrorCode::InvalidRequest);
    assert!(matches!(
        error,
        DeleteRouteError::MissingRequiredHeader { header } if header == IDEMPOTENCY_KEY_HEADER
    ));
}

#[test]
fn delete_route_public_errors_are_safe_for_stale_and_mass_delete_rejections() {
    let stale = stale_base_delete_response(path("Projects/Haze/old.md"));
    let unsafe_delete = unsafe_delete_response(path("Projects/Haze/old.md"));
    let tombstoned = tombstoned_delete_response(
        path("Projects/Haze/old.md"),
        "tmb_01JW3DELETE",
        42,
        "2026-08-01T00:00:00Z",
    );

    assert!(serde_json::to_string(&stale).unwrap().contains("stale_base_revision"));
    assert!(serde_json::to_string(&unsafe_delete).unwrap().contains("unsafe_delete"));
    assert!(serde_json::to_string(&tombstoned).unwrap().contains("tombstoned"));

    for error in [
        DeleteRouteError::DeleteGuardBlocked,
        DeleteRouteError::IdempotencyMismatch,
        DeleteRouteError::StaleBaseConflict,
    ] {
        let serialized = serde_json::to_string(&error.to_error_response())
            .expect("delete error should serialize");
        assert!(!serialized.contains("route_test_token"));
        assert!(!serialized.contains("postgres://"));
        assert!(!serialized.contains("/srv/"));
        assert!(!serialized.contains("stack"));
        assert!(!serialized.contains("request_body"));
    }
}

#[test]
fn w2_put_get_file_route_contracts_remain_intact() {
    let body = b"w2 file body".to_vec();
    let request = parse_put_file_request(PutFileRouteRequestParts {
        route_path: "Projects/Haze/plan.md",
        idempotency_key: Some("iphone-anna:put-001"),
        content_sha256: Some(&content_hash(&body).to_string()),
        base_revision_id: Some("null"),
        body: body.clone(),
        max_upload_bytes: Some(1024),
    })
    .expect("W2 PUT accepted path should still parse");

    assert_eq!(request.path().as_str(), "Projects/Haze/plan.md");
    assert_eq!(request.base_revision_id(), None);
    assert_eq!(request.body(), body.as_slice());
    assert_eq!(request.to_metadata_dto().size_bytes, Some(12));
    assert!(!format!("{request:?}").contains("put-001"));
    assert!(!format!("{request:?}").contains("w2 file body"));

    let accepted = accepted_upload_response(path("Projects/Haze/plan.md"), revision_id("rev_w2"), 11);
    assert!(matches!(accepted, PutFileResponse::Accepted { .. }));

    let ignored = ignored_same_content_response(path("Projects/Haze/plan.md"));
    assert_eq!(
        ignored,
        PutFileResponse::Ignored {
            reason: FileIgnoredReasonDto::SameContent,
            path: VaultPathDto::from("Projects/Haze/plan.md"),
        }
    );

    let get = parse_get_file_request(GetFileRouteRequestParts {
        route_path: "Projects/Haze/plan.md",
        revision_id: Some("rev_w2"),
    })
    .expect("W2 GET file route should still parse");
    assert_eq!(get.path().as_str(), "Projects/Haze/plan.md");
    assert_eq!(get.revision_id().map(RevisionId::as_str), Some("rev_w2"));

    let headers = FileDownloadRouteHeaders::new(revision_id("rev_w2"), content_hash(&body), 12);
    let header_map = headers.to_header_map();
    assert_eq!(headers.content_type(), APPLICATION_OCTET_STREAM);
    assert_eq!(header_map["Content-Type"], APPLICATION_OCTET_STREAM);
}

#[test]
fn w2_changes_route_semantics_remain_intact_and_include_w3_entries() {
    let request = parse_changes_query(Some("10"), Some("100"))
        .expect("W2 changes query should still parse");
    assert_eq!(request.since_value(), 10);
    assert_eq!(request.limit_value(), 100);

    let response = changes_response_from_parts(
        10,
        13,
        false,
        vec![
            ChangeEntryDto {
                seq: 11,
                kind: OperationKindDto::UpsertFile,
                path: VaultPathDto::from("Projects/Haze/plan.md"),
                revision_id: Some(RevisionIdDto::from("rev_w2")),
                content_sha256: Some(ContentSha256Dto::from(content_hash(b"w2 file body"))),
                size_bytes: Some(12),
                tombstone_id: None,
                conflict_id: None,
                updated_by: AdapterIdDto::from("iphone-anna"),
                updated_at: TimestampDto::from("2026-07-01T12:00:00Z"),
            },
            ChangeEntryDto {
                seq: 12,
                kind: OperationKindDto::DeleteFile,
                path: VaultPathDto::from("Projects/Haze/old.md"),
                revision_id: Some(RevisionIdDto::from("rev_deleted")),
                content_sha256: None,
                size_bytes: None,
                tombstone_id: Some(TombstoneIdDto::from("tmb_01JW3DELETE")),
                conflict_id: None,
                updated_by: AdapterIdDto::from("iphone-anna"),
                updated_at: TimestampDto::from("2026-07-01T12:01:00Z"),
            },
            ChangeEntryDto {
                seq: 13,
                kind: OperationKindDto::ConflictResolved,
                path: VaultPathDto::from("Projects/Haze/plan.md"),
                revision_id: None,
                content_sha256: None,
                size_bytes: None,
                tombstone_id: None,
                conflict_id: Some(ConflictIdDto::from("conf_01JW3")),
                updated_by: AdapterIdDto::from("iphone-anna"),
                updated_at: TimestampDto::from("2026-07-01T12:02:00Z"),
            },
        ],
    )
    .expect("changes response should preserve monotonic W2/W3 entries");

    assert_eq!(response.from_seq, 10);
    assert_eq!(response.to_seq, 13);
    assert_eq!(response.changes[0].kind, OperationKindDto::UpsertFile);
    assert_eq!(response.changes[1].kind, OperationKindDto::DeleteFile);
    assert_eq!(response.changes[2].kind, OperationKindDto::ConflictResolved);
    let serialized = serde_json::to_string(&response).expect("changes response should serialize");
    assert!(serialized.contains("upsert_file"));
    assert!(serialized.contains("delete_file"));
    assert!(serialized.contains("conflict_resolved"));
    assert!(!serialized.contains("postgres://"));
    assert!(!serialized.contains("/srv/"));
}

#[test]
fn w2_idempotency_error_mapping_remains_stable() {
    let error = DeleteRouteError::IdempotencyMismatch;
    let response = error.to_error_response();

    assert_eq!(error.public_code(), PublicErrorCode::IdempotencyConflict);
    assert_eq!(error.http_status_code(), 409);
    let serialized = serde_json::to_string(&response).expect("idempotency error should serialize");
    assert!(serialized.contains("idempotency_conflict"));
    assert!(!serialized.contains(IDEMPOTENCY_KEY_HEADER));
    assert!(!serialized.contains(X_BASE_REVISION_ID_HEADER));
    assert!(!serialized.contains("route_test_token"));
}
