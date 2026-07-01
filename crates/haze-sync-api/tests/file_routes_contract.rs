use haze_sync_api::{
    contracts::{
        errors::PublicErrorCode,
        headers::{IDEMPOTENCY_KEY_HEADER, X_BASE_REVISION_ID_HEADER},
    },
    dto::files::FileRejectedReasonDto,
    routes::files::{
        accepted_upload_response, ignored_same_content_response, parse_get_file_request,
        parse_put_file_request, rejected_upload_response, FileDownloadRouteHeaders,
        FileRouteError, GetFileRouteRequestParts, PutFileRouteRequestParts,
        APPLICATION_OCTET_STREAM, CONTENT_TYPE_HEADER, X_REVISION_ID_HEADER, X_SIZE_BYTES_HEADER,
    },
};
use haze_sync_common::{ContentHash, RevisionId, VaultPath};

fn prefixed_hash(ch: char) -> String {
    let hex = ch.to_string().repeat(64);
    format!("sha256:{hex}")
}

fn valid_put_parts<'a>(base_revision_id: Option<&'a str>) -> PutFileRouteRequestParts<'a> {
    PutFileRouteRequestParts {
        route_path: "./Notes//plan.md",
        idempotency_key: Some("iphone-anna:op-001"),
        content_sha256: Some(
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ),
        base_revision_id,
        body: b"hello".to_vec(),
        max_upload_bytes: Some(1024),
    }
}

#[test]
fn valid_put_headers_parse() {
    let request = parse_put_file_request(valid_put_parts(Some("rev_01JBASE")))
        .expect("valid PUT contract should parse");

    assert_eq!(request.path().as_str(), "Notes/plan.md");
    assert_eq!(request.idempotency_key().as_str(), "iphone-anna:op-001");
    assert_eq!(
        request
            .base_revision_id()
            .expect("base revision should be present")
            .as_str(),
        "rev_01JBASE"
    );
    assert_eq!(request.content_sha256().to_string(), prefixed_hash('a'));
    assert_eq!(request.body(), b"hello");
    assert_eq!(request.size_bytes(), 5);

    let metadata = request.to_metadata_dto();
    assert_eq!(metadata.path.as_str(), "Notes/plan.md");
    assert_eq!(
        metadata
            .base_revision_id
            .expect("metadata should preserve base revision")
            .as_str(),
        "rev_01JBASE"
    );
    assert_eq!(metadata.content_sha256.as_str(), prefixed_hash('a'));
    assert_eq!(metadata.size_bytes, Some(5));
}

#[test]
fn missing_idempotency_key_rejected() {
    let mut parts = valid_put_parts(Some("rev_01JBASE"));
    parts.idempotency_key = None;

    let error = parse_put_file_request(parts).expect_err("missing idempotency key should reject");

    assert_eq!(
        error,
        FileRouteError::MissingRequiredHeader {
            header: IDEMPOTENCY_KEY_HEADER,
        }
    );
    assert_eq!(error.http_status_code(), 400);
    assert_eq!(error.public_code(), PublicErrorCode::InvalidRequest);
}

#[test]
fn invalid_content_sha256_rejected() {
    let mut parts = valid_put_parts(Some("rev_01JBASE"));
    parts.content_sha256 = Some("not-a-sha256");

    let error = parse_put_file_request(parts).expect_err("invalid hash should reject");

    assert_eq!(error, FileRouteError::InvalidContentSha256);
    assert_eq!(error.http_status_code(), 400);
    assert_eq!(error.public_code(), PublicErrorCode::ValidationError);
}

#[test]
fn explicit_null_base_revision_parses_as_none() {
    let request = parse_put_file_request(valid_put_parts(Some("null")))
        .expect("explicit null base should parse");

    assert_eq!(request.base_revision_id(), None);
    assert_eq!(request.to_metadata_dto().base_revision_id, None);
}

#[test]
fn invalid_base_revision_rejected() {
    let error = parse_put_file_request(valid_put_parts(Some("not_rev_01J")))
        .expect_err("non-revision base should reject");

    assert_eq!(error, FileRouteError::InvalidBaseRevision);
    assert_eq!(error.http_status_code(), 400);
    assert_eq!(error.public_code(), PublicErrorCode::ValidationError);

    let missing = valid_put_parts(None);
    assert_eq!(
        parse_put_file_request(missing).expect_err("missing base header should reject"),
        FileRouteError::MissingRequiredHeader {
            header: X_BASE_REVISION_ID_HEADER,
        }
    );
}

#[test]
fn get_optional_revision_id_parses() {
    let current = parse_get_file_request(GetFileRouteRequestParts {
        route_path: "Notes%2Fplan.md",
        revision_id: None,
    })
    .expect("current revision query should parse");
    assert_eq!(current.path().as_str(), "Notes/plan.md");
    assert_eq!(current.revision_id(), None);
    assert_eq!(current.to_query_dto().revision_id, None);

    let selected = parse_get_file_request(GetFileRouteRequestParts {
        route_path: "Notes/plan.md",
        revision_id: Some("rev_01JSELECTED"),
    })
    .expect("selected revision query should parse");
    assert_eq!(
        selected
            .revision_id()
            .expect("revision should be selected")
            .as_str(),
        "rev_01JSELECTED"
    );
    assert_eq!(
        selected
            .to_query_dto()
            .revision_id
            .expect("DTO should include revision")
            .as_str(),
        "rev_01JSELECTED"
    );
}

#[test]
fn response_headers_are_deterministic_and_safe() {
    let revision_id = RevisionId::parse("rev_01JDOWN").unwrap();
    let content_sha256 = ContentHash::parse(&prefixed_hash('b')).unwrap();
    let headers = FileDownloadRouteHeaders::new(revision_id, content_sha256, 1842);

    let map = headers.to_header_map();
    let keys: Vec<&str> = map.keys().copied().collect();

    assert_eq!(
        keys,
        vec![
            CONTENT_TYPE_HEADER,
            "X-Content-SHA256",
            X_REVISION_ID_HEADER,
            X_SIZE_BYTES_HEADER,
        ]
    );
    assert_eq!(map[CONTENT_TYPE_HEADER], APPLICATION_OCTET_STREAM);
    assert_eq!(map["X-Content-SHA256"], prefixed_hash('b'));
    assert_eq!(map[X_REVISION_ID_HEADER], "rev_01JDOWN");
    assert_eq!(map[X_SIZE_BYTES_HEADER], "1842");

    let metadata = headers.to_metadata_dto();
    assert_eq!(metadata.content_type, APPLICATION_OCTET_STREAM);
    assert_eq!(metadata.revision_id.as_str(), "rev_01JDOWN");
    assert_eq!(metadata.content_sha256.as_str(), prefixed_hash('b'));
    assert_eq!(metadata.size_bytes, 1842);
}

#[test]
fn response_dto_mappers_preserve_contract_shapes() {
    let accepted = accepted_upload_response(
        VaultPath::parse("Notes/plan.md").unwrap(),
        RevisionId::parse("rev_01JACCEPTED").unwrap(),
        42,
    );
    let ignored = ignored_same_content_response(VaultPath::parse("Notes/plan.md").unwrap());
    let rejected = rejected_upload_response(
        VaultPath::parse("Notes/plan.md").unwrap(),
        FileRejectedReasonDto::ValidationError,
    );

    assert!(serde_json::to_string(&accepted)
        .unwrap()
        .contains("accepted"));
    assert!(serde_json::to_string(&ignored)
        .unwrap()
        .contains("same_content"));
    assert!(serde_json::to_string(&rejected)
        .unwrap()
        .contains("validation_error"));
}

#[test]
fn public_errors_are_sanitized() {
    let response = FileRouteError::InvalidContentSha256.to_error_response();
    let json = serde_json::to_string(&response).expect("error response should serialize");

    assert!(json.contains("validation_error"));
    assert!(json.contains("X-Content-SHA256"));
    assert!(!json.contains("Bearer"));
    assert!(!json.contains("/srv/"));
    assert!(!json.contains("stack"));
}
