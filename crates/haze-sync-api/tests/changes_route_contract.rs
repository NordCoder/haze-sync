use haze_sync_api::{
    contracts::errors::PublicErrorCode,
    dto::{
        changes::ChangeEntryDto,
        common::OperationKindDto,
        primitives::{AdapterIdDto, ContentSha256Dto, RevisionIdDto, TimestampDto, VaultPathDto},
    },
    routes::changes::{
        changes_response_from_parts, parse_changes_query, ChangesRouteErrorKind,
        DEFAULT_CHANGES_LIMIT, HTTP_STATUS_BAD_REQUEST, MAX_CHANGES_LIMIT,
    },
};

fn sample_change(seq: i64) -> ChangeEntryDto {
    ChangeEntryDto {
        seq,
        kind: OperationKindDto::UpsertFile,
        path: VaultPathDto::from("Projects/Haze/plan.md"),
        revision_id: Some(RevisionIdDto::from("rev_01JTEST")),
        content_sha256: Some(ContentSha256Dto::from(format!("sha256:{}", "a".repeat(64)))),
        size_bytes: Some(1_842),
        tombstone_id: None,
        conflict_id: None,
        updated_by: AdapterIdDto::from("gdrive-adapter"),
        updated_at: TimestampDto::from("2026-07-01T22:00:00Z"),
    }
}

#[test]
fn default_query_parses() {
    let request = parse_changes_query(None, None).unwrap();

    assert_eq!(request.since_value(), 0);
    assert_eq!(request.limit_value(), DEFAULT_CHANGES_LIMIT);
}

#[test]
fn since_and_limit_parse_correctly() {
    let request = parse_changes_query(Some("12345"), Some("250")).unwrap();

    assert_eq!(request.since_value(), 12_345);
    assert_eq!(request.limit_value(), 250);
    assert_eq!(request.limit_value_with_sentinel(), 251);
    assert_eq!(request.to_dto().since, 12_345);
    assert_eq!(request.to_dto().limit, 250);
}

#[test]
fn negative_since_is_rejected_as_invalid_request() {
    let error = parse_changes_query(Some("-1"), None).unwrap_err();

    assert_eq!(error.kind(), ChangesRouteErrorKind::InvalidSince);
    assert_eq!(error.status_code(), HTTP_STATUS_BAD_REQUEST);
    assert_eq!(
        error.error_response().error.code,
        PublicErrorCode::InvalidRequest
    );
}

#[test]
fn zero_limit_is_rejected_as_invalid_request() {
    let error = parse_changes_query(None, Some("0")).unwrap_err();

    assert_eq!(error.kind(), ChangesRouteErrorKind::InvalidLimit);
    assert_eq!(error.status_code(), HTTP_STATUS_BAD_REQUEST);
    assert_eq!(
        error.error_response().error.code,
        PublicErrorCode::InvalidRequest
    );
}

#[test]
fn too_large_limit_is_rejected_as_invalid_request() {
    let too_large = (MAX_CHANGES_LIMIT + 1).to_string();
    let error = parse_changes_query(None, Some(&too_large)).unwrap_err();

    assert_eq!(error.kind(), ChangesRouteErrorKind::InvalidLimit);
    assert_eq!(error.status_code(), HTTP_STATUS_BAD_REQUEST);
    assert_eq!(
        error.error_response().error.code,
        PublicErrorCode::InvalidRequest
    );
}

#[test]
fn response_serialization_matches_stable_safe_shape() {
    let response =
        changes_response_from_parts(12_345, 12_380, false, vec![sample_change(12_380)]).unwrap();
    let hash = format!("sha256:{}", "a".repeat(64));
    let expected = format!(
        "{{\"from_seq\":12345,\"to_seq\":12380,\"has_more\":false,\"changes\":[{{\"seq\":12380,\"kind\":\"upsert_file\",\"path\":\"Projects/Haze/plan.md\",\"revision_id\":\"rev_01JTEST\",\"content_sha256\":\"{hash}\",\"size_bytes\":1842,\"updated_by\":\"gdrive-adapter\",\"updated_at\":\"2026-07-01T22:00:00Z\"}}]}}"
    );

    assert_eq!(serde_json::to_string(&response).unwrap(), expected);
}

#[test]
fn pagination_metadata_is_preserved() {
    let response =
        changes_response_from_parts(10, 12, true, vec![sample_change(11), sample_change(12)])
            .unwrap();

    assert_eq!(response.from_seq, 10);
    assert_eq!(response.to_seq, 12);
    assert!(response.has_more);
    assert_eq!(response.changes.len(), 2);
}
