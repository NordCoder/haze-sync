use std::collections::BTreeSet;

use haze_sync_api::{
    auth::{AdapterPrincipal, AdapterRole},
    dto::gdrive::{
        GDriveCursorAdvanceDto, GDriveEchoStateDto, GDriveOperationKindDto,
        GDrivePrivateCursorStateDto, GDriveRawCursorDto, GDriveStateAdminSummaryResponse,
        GDriveStateCommitRequest, GDriveStateCommitResponse, GDriveStateErrorCode,
        GDriveStateSnapshotResponse,
    },
    routes::gdrive::{
        parse_authenticated_gdrive_state_commit_request,
        parse_authenticated_get_gdrive_state_request, sanitize_snapshot_for_admin,
        validate_private_snapshot, GDriveStateCommitRouteParts, GDriveStateReadAccess,
        GetGDriveStateRouteParts,
    },
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;

const FIXTURE_JSON: &str = include_str!("../fixtures/gdrive-state-contract-v1.json");
const PRIVATE_CURSOR_SENTINEL: &str = "synthetic-private-cursor-fixture";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GDriveCompatibilityFixture {
    schema_version: u32,
    private_snapshot: Value,
    admin_summary: Value,
    commit_request_without_cursor_advance: Value,
    commit_outcomes: Vec<Value>,
    vocabulary: GDriveVocabularyFixture,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GDriveVocabularyFixture {
    echo_states: Vec<String>,
    operation_kinds: Vec<String>,
    commit_statuses: Vec<String>,
    error_codes: Vec<String>,
}

fn load_fixture() -> GDriveCompatibilityFixture {
    serde_json::from_str(FIXTURE_JSON).expect("GDrive compatibility fixture must be valid JSON")
}

fn assert_roundtrip<T>(value: &Value) -> T
where
    T: DeserializeOwned + Serialize,
{
    let decoded = serde_json::from_value::<T>(value.clone())
        .expect("fixture value must deserialize into the public contract");
    let encoded = serde_json::to_value(&decoded).expect("public contract must serialize");
    assert_eq!(encoded, *value, "fixture shape must roundtrip exactly");
    decoded
}

fn wire_string<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .expect("value must serialize")
        .as_str()
        .expect("value must serialize as a JSON string")
        .to_owned()
}

fn assert_complete_unique(actual: &[String], expected: BTreeSet<String>) {
    let actual_set = actual.iter().cloned().collect::<BTreeSet<_>>();
    assert_eq!(
        actual.len(),
        actual_set.len(),
        "fixture values must be unique"
    );
    assert_eq!(actual_set, expected);
}

#[test]
fn fixture_is_strict_and_contains_only_synthetic_private_cursor_material() {
    let fixture = load_fixture();
    assert_eq!(fixture.schema_version, 1);
    assert!(FIXTURE_JSON.contains(PRIVATE_CURSOR_SENTINEL));

    let lowercase = FIXTURE_JSON.to_ascii_lowercase();
    for forbidden in [
        "raw_cursor",
        "drive_cursor\"",
        "bearer ",
        "oauth_token",
        "refresh_token",
        "access_token",
        "token_hash",
        "database_url",
        "postgres://",
        "provider_payload",
        "request_payload",
        "raw_error",
        "stack_trace",
        "/home/",
        "/users/",
        "/srv/",
        "c:\\",
    ] {
        assert!(
            !lowercase.contains(forbidden),
            "fixture contains forbidden fragment {forbidden:?}"
        );
    }

    let mut root = serde_json::from_str::<Value>(FIXTURE_JSON).unwrap();
    root.as_object_mut()
        .unwrap()
        .insert("unexpected".to_owned(), Value::Null);
    assert!(serde_json::from_value::<GDriveCompatibilityFixture>(root).is_err());
}

#[test]
fn snapshot_and_admin_summary_roundtrip_and_sanitize_deterministically() {
    let fixture = load_fixture();
    let snapshot: GDriveStateSnapshotResponse = assert_roundtrip(&fixture.private_snapshot);
    let expected_admin: GDriveStateAdminSummaryResponse = assert_roundtrip(&fixture.admin_summary);

    validate_private_snapshot(&snapshot).unwrap();
    assert_eq!(
        snapshot.cursor,
        GDrivePrivateCursorStateDto::Present {
            generation: 3,
            cursor: GDriveRawCursorDto::parse(PRIVATE_CURSOR_SENTINEL).unwrap(),
        }
    );
    assert_eq!(
        sanitize_snapshot_for_admin(&snapshot).unwrap(),
        expected_admin
    );
    assert_eq!(
        serde_json::to_value(&expected_admin).unwrap()["cursor"],
        serde_json::json!({"generation": 3, "present": true})
    );
    let admin_json = serde_json::to_string(&expected_admin).unwrap();
    assert!(admin_json.contains("\"mapping_count\":1"));
    assert!(admin_json.contains("\"generation\":3"));
    assert!(admin_json.contains("\"present\":true"));
    assert!(!admin_json.contains(PRIVATE_CURSOR_SENTINEL));

    let snapshot_debug = format!("{snapshot:?}");
    assert!(!snapshot_debug.contains(PRIVATE_CURSOR_SENTINEL));
    assert!(!snapshot_debug.contains("drive-file-fixture-01"));
    assert!(!snapshot_debug.contains("drive-version-fixture-07"));
}

#[test]
fn read_route_allows_matching_adapter_and_sanitized_admin_only() {
    let adapter = AdapterPrincipal::new("gdrive-main", AdapterRole::GdriveAdapter).unwrap();
    let adapter_request = parse_authenticated_get_gdrive_state_request(
        GetGDriveStateRouteParts {
            adapter_id: "gdrive-main",
            after_path: Some("Notes/alpha.md"),
            limit: Some(25),
        },
        Some(&adapter),
    )
    .unwrap();
    assert_eq!(
        adapter_request.access(),
        GDriveStateReadAccess::AdapterPrivate
    );

    let admin = AdapterPrincipal::new("admin-main", AdapterRole::Admin).unwrap();
    let admin_request = parse_authenticated_get_gdrive_state_request(
        GetGDriveStateRouteParts {
            adapter_id: "gdrive-main",
            after_path: None,
            limit: None,
        },
        Some(&admin),
    )
    .unwrap();
    assert_eq!(
        admin_request.access(),
        GDriveStateReadAccess::AdminSanitized
    );

    let wrong_adapter = AdapterPrincipal::new("gdrive-other", AdapterRole::GdriveAdapter).unwrap();
    assert!(parse_authenticated_get_gdrive_state_request(
        GetGDriveStateRouteParts {
            adapter_id: "gdrive-main",
            after_path: None,
            limit: None,
        },
        Some(&wrong_adapter),
    )
    .is_err());
    assert!(parse_authenticated_get_gdrive_state_request(
        GetGDriveStateRouteParts {
            adapter_id: "gdrive-main",
            after_path: None,
            limit: None,
        },
        None,
    )
    .is_err());
}

#[test]
fn commit_fixture_is_strict_authenticated_idempotent_metadata() {
    let fixture = load_fixture();
    let request: GDriveStateCommitRequest =
        assert_roundtrip(&fixture.commit_request_without_cursor_advance);
    let principal = AdapterPrincipal::new("gdrive-main", AdapterRole::GdriveAdapter).unwrap();
    let authenticated = parse_authenticated_gdrive_state_commit_request(
        GDriveStateCommitRouteParts {
            adapter_id: "gdrive-main",
            idempotency_key: Some("gdrive-commit-key-fixture-01"),
            body: request,
        },
        Some(&principal),
    )
    .unwrap();

    let debug = format!("{authenticated:?}");
    assert!(!debug.contains("gdrive-commit-key-fixture-01"));
    assert!(!debug.contains("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
    assert!(!debug.contains("drive-file-fixture-01"));

    let mut request_value = fixture.commit_request_without_cursor_advance;
    request_value
        .as_object_mut()
        .unwrap()
        .insert("force".to_owned(), Value::Bool(true));
    assert!(serde_json::from_value::<GDriveStateCommitRequest>(request_value).is_err());
}

#[test]
fn raw_cursor_exists_only_in_authenticated_private_contracts_and_is_redacted() {
    let fixture = load_fixture();
    let snapshot: GDriveStateSnapshotResponse = assert_roundtrip(&fixture.private_snapshot);
    let private_json = serde_json::to_string(&snapshot).unwrap();
    assert!(private_json.contains(PRIVATE_CURSOR_SENTINEL));
    assert!(!format!("{snapshot:?}").contains(PRIVATE_CURSOR_SENTINEL));

    let mut request: GDriveStateCommitRequest =
        assert_roundtrip(&fixture.commit_request_without_cursor_advance);
    request.cursor.advance = Some(GDriveCursorAdvanceDto {
        next_generation: 4,
        cursor: GDriveRawCursorDto::parse("synthetic-private-commit-cursor").unwrap(),
    });

    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains("synthetic-private-commit-cursor"));
    assert!(!format!("{request:?}").contains("synthetic-private-commit-cursor"));

    let principal = AdapterPrincipal::new("gdrive-main", AdapterRole::GdriveAdapter).unwrap();
    parse_authenticated_gdrive_state_commit_request(
        GDriveStateCommitRouteParts {
            adapter_id: "gdrive-main",
            idempotency_key: Some("gdrive-cursor-key-fixture-01"),
            body: request,
        },
        Some(&principal),
    )
    .unwrap();
}

#[test]
fn outcome_and_error_vocabularies_are_complete() {
    let fixture = load_fixture();
    let mut outcome_statuses = Vec::new();
    for value in &fixture.commit_outcomes {
        let response: GDriveStateCommitResponse = assert_roundtrip(value);
        outcome_statuses.push(
            serde_json::to_value(response)
                .unwrap()
                .get("status")
                .and_then(Value::as_str)
                .unwrap()
                .to_owned(),
        );
    }
    assert_complete_unique(
        &outcome_statuses,
        [
            "committed",
            "replayed",
            "stale_state",
            "cursor_regression",
            "cursor_gap",
            "mapping_conflict",
            "idempotency_conflict",
            "validation_failed",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
    );

    assert_complete_unique(
        &fixture.vocabulary.echo_states,
        [
            GDriveEchoStateDto::None,
            GDriveEchoStateDto::Pending,
            GDriveEchoStateDto::Confirmed,
        ]
        .iter()
        .map(wire_string)
        .collect(),
    );
    assert_complete_unique(
        &fixture.vocabulary.operation_kinds,
        [
            GDriveOperationKindDto::Import,
            GDriveOperationKindDto::Export,
            GDriveOperationKindDto::ProviderMutation,
            GDriveOperationKindDto::CursorCheckpoint,
            GDriveOperationKindDto::DeleteCandidate,
        ]
        .iter()
        .map(wire_string)
        .collect(),
    );
    assert_complete_unique(
        &fixture.vocabulary.commit_statuses,
        outcome_statuses.into_iter().collect(),
    );
    assert_complete_unique(
        &fixture.vocabulary.error_codes,
        [
            GDriveStateErrorCode::Unauthorized,
            GDriveStateErrorCode::Forbidden,
            GDriveStateErrorCode::AdapterNotFound,
            GDriveStateErrorCode::StateVersionMismatch,
            GDriveStateErrorCode::InvalidCursorState,
            GDriveStateErrorCode::StaleState,
            GDriveStateErrorCode::CursorRegression,
            GDriveStateErrorCode::CursorGap,
            GDriveStateErrorCode::MappingConflict,
            GDriveStateErrorCode::IdempotencyConflict,
            GDriveStateErrorCode::ValidationError,
            GDriveStateErrorCode::Unavailable,
            GDriveStateErrorCode::Internal,
        ]
        .iter()
        .map(wire_string)
        .collect(),
    );
}
