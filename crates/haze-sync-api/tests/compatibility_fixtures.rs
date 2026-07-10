use std::{collections::BTreeSet, fmt::Debug};

use haze_sync_api::{
    contracts::errors::{ErrorResponse, PublicErrorCode},
    dto::{
        changes::ChangesResponse,
        common::{
            ConflictPolicyDto, ConflictResolutionDto, ConflictResolveStatusDto, ConflictStatusDto,
            OperationKindDto,
        },
        conflicts::{ConflictListQuery, ResolveConflictRequest, ResolveConflictResponse},
        files::{
            DeleteFileResponse, DeleteRejectedReasonDto, FileIgnoredReasonDto,
            FileMetadataResponse, FileRejectedReasonDto, PutFileResponse,
        },
        server::{ServerCapabilityDto, ServerInfoResponse},
    },
    routes::{
        admin::{
            AdapterListResponse, AdapterOperationalSummary, AdapterRuntimeState, CursorPresence,
            DependencyReadinessState, DoctorCheckKind, DoctorStatusResponse,
            OperationalCheckStatus, ServerStatus, StatusSummaryResponse,
        },
        changes::changes_response_from_parts,
        conflicts::{
            parse_conflicts_query, parse_resolve_conflict_request, resolved_conflict_response,
            ConflictListRequestParts, ConflictListRouteResponse, ResolveConflictRequestParts,
        },
    },
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;

const FIXTURE_JSON: &str = include_str!("../fixtures/api-contract-v1.json");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ApiCompatibilityFixture {
    schema_version: u32,
    server_info: Value,
    file_metadata: Value,
    put_file_outcomes: Vec<Value>,
    changes_page: Value,
    conflict_list_query: Value,
    conflict_list: Value,
    conflict_resolutions: Vec<ConflictResolutionFixture>,
    delete_file_outcomes: Vec<Value>,
    public_errors: Vec<Value>,
    admin: AdminFixtures,
    vocabulary: VocabularyFixture,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConflictResolutionFixture {
    path_conflict_id: String,
    request: Value,
    response: Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AdminFixtures {
    status_summary: Value,
    adapter_list: Value,
    doctor_statuses: Vec<Value>,
    adapter_operational_summaries: Vec<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VocabularyFixture {
    server_capabilities: Vec<String>,
    put_statuses: Vec<String>,
    put_ignored_reasons: Vec<String>,
    put_rejected_reasons: Vec<String>,
    operation_kinds: Vec<String>,
    conflict_policies: Vec<String>,
    conflict_statuses: Vec<String>,
    conflict_resolution_actions: Vec<String>,
    conflict_resolve_statuses: Vec<String>,
    delete_statuses: Vec<String>,
    delete_rejected_reasons: Vec<String>,
    public_error_codes: Vec<String>,
    server_statuses: Vec<String>,
    dependency_readiness_states: Vec<String>,
    operational_check_statuses: Vec<String>,
    doctor_check_kinds: Vec<String>,
    cursor_presence_states: Vec<String>,
    adapter_runtime_states: Vec<String>,
}

fn load_fixture() -> ApiCompatibilityFixture {
    serde_json::from_str(FIXTURE_JSON).expect("API V1 compatibility fixture must be valid JSON")
}

fn assert_fixture_roundtrip<T>(value: &Value) -> T
where
    T: DeserializeOwned + Serialize + Debug,
{
    let decoded = serde_json::from_value::<T>(value.clone())
        .expect("fixture value must deserialize into its public API contract type");
    let encoded = serde_json::to_value(&decoded)
        .expect("public API contract type must serialize back to JSON");
    assert_eq!(encoded, *value, "fixture shape must roundtrip exactly");
    decoded
}

fn wire_value<T>(value: &T) -> String
where
    T: Serialize,
{
    serde_json::to_value(value)
        .expect("public vocabulary value must serialize")
        .as_str()
        .expect("public vocabulary value must serialize as a JSON string")
        .to_owned()
}

fn assert_complete_unique_wires<T>(actual: &[String], expected: &[T])
where
    T: Serialize,
{
    let actual_set = actual.iter().cloned().collect::<BTreeSet<_>>();
    let expected_set = expected.iter().map(wire_value).collect::<BTreeSet<_>>();

    assert_eq!(
        actual.len(),
        actual_set.len(),
        "fixture vocabulary values must be unique"
    );
    assert_eq!(actual_set, expected_set);
}

fn assert_complete_unique_strings(actual: &[String], expected: &[&str]) {
    let actual_set = actual.iter().cloned().collect::<BTreeSet<_>>();
    let expected_set = expected
        .iter()
        .map(|value| (*value).to_owned())
        .collect::<BTreeSet<_>>();

    assert_eq!(
        actual.len(),
        actual_set.len(),
        "fixture string values must be unique"
    );
    assert_eq!(actual_set, expected_set);
}

fn status_values(values: &[Value]) -> Vec<String> {
    values
        .iter()
        .map(|value| {
            value["status"]
                .as_str()
                .expect("outcome fixture must contain a string status")
                .to_owned()
        })
        .collect()
}

#[test]
fn fixture_schema_is_v1_strict_and_safe_to_publish() {
    let fixture = load_fixture();
    assert_eq!(fixture.schema_version, 1);

    let lowercase = FIXTURE_JSON.to_ascii_lowercase();
    for forbidden in [
        "bearer ",
        "oauth",
        "access_token",
        "refresh_token",
        "client_secret",
        "secret",
        "token_hash",
        "database_url",
        "postgres://",
        "mysql://",
        "http://",
        "https://",
        "localhost",
        "127.0.0.1",
        "/home/",
        "/users/",
        "/srv/",
        "c:\\",
        "provider_payload",
        "raw_error",
        "stack_trace",
        "backtrace",
        "raw_cursor",
        "cursor_value",
        "page_token",
        "raw_request_body",
        "request_body",
        "body_bytes",
        "file_bytes",
        "content_base64",
        "idempotency_key",
    ] {
        assert!(
            !lowercase.contains(forbidden),
            "fixture contains forbidden environment/secret fragment {forbidden:?}"
        );
    }
}

#[test]
fn fixture_schema_rejects_unknown_root_and_group_fields() {
    let mut root = serde_json::from_str::<Value>(FIXTURE_JSON).unwrap();
    root.as_object_mut()
        .unwrap()
        .insert("unexpected_field".to_owned(), Value::Null);
    assert!(serde_json::from_value::<ApiCompatibilityFixture>(root).is_err());

    let mut admin = serde_json::from_str::<Value>(FIXTURE_JSON).unwrap();
    admin["admin"]
        .as_object_mut()
        .unwrap()
        .insert("unexpected_field".to_owned(), Value::Null);
    assert!(serde_json::from_value::<ApiCompatibilityFixture>(admin).is_err());

    let mut vocabulary = serde_json::from_str::<Value>(FIXTURE_JSON).unwrap();
    vocabulary["vocabulary"]
        .as_object_mut()
        .unwrap()
        .insert("unexpected_field".to_owned(), Value::Null);
    assert!(serde_json::from_value::<ApiCompatibilityFixture>(vocabulary).is_err());

    let mut resolution = serde_json::from_str::<Value>(FIXTURE_JSON).unwrap();
    resolution["conflict_resolutions"][0]
        .as_object_mut()
        .unwrap()
        .insert("unexpected_field".to_owned(), Value::Null);
    assert!(serde_json::from_value::<ApiCompatibilityFixture>(resolution).is_err());
}

#[test]
fn server_info_and_file_metadata_match_current_public_shapes() {
    let fixture = load_fixture();
    let server_info: ServerInfoResponse = assert_fixture_roundtrip(&fixture.server_info);
    let _file_metadata: FileMetadataResponse = assert_fixture_roundtrip(&fixture.file_metadata);

    server_info
        .validate()
        .expect("fixture server-info metadata must satisfy the public contract");
    assert!(server_info.is_protocol_compatible(1));
    assert!(server_info.supports(ServerCapabilityDto::Sha256));
}

#[test]
fn put_and_delete_outcomes_cover_every_public_status_shape() {
    const PUT_STATUSES: &[&str] = &["accepted", "conflict_saved", "ignored", "rejected"];
    const DELETE_STATUSES: &[&str] = &["tombstoned", "not_found", "rejected"];

    let fixture = load_fixture();

    for outcome in &fixture.put_file_outcomes {
        let _: PutFileResponse = assert_fixture_roundtrip(outcome);
    }
    assert_complete_unique_strings(&status_values(&fixture.put_file_outcomes), PUT_STATUSES);
    assert_complete_unique_strings(&fixture.vocabulary.put_statuses, PUT_STATUSES);

    for outcome in &fixture.delete_file_outcomes {
        let _: DeleteFileResponse = assert_fixture_roundtrip(outcome);
    }
    assert_complete_unique_strings(
        &status_values(&fixture.delete_file_outcomes),
        DELETE_STATUSES,
    );
    assert_complete_unique_strings(&fixture.vocabulary.delete_statuses, DELETE_STATUSES);
}

#[test]
fn changes_page_roundtrips_and_satisfies_route_pagination_invariants() {
    let fixture = load_fixture();
    let page: ChangesResponse = assert_fixture_roundtrip(&fixture.changes_page);

    let rebuilt = changes_response_from_parts(
        page.from_seq,
        page.to_seq,
        page.has_more,
        page.changes.clone(),
    )
    .expect("fixture changes page must satisfy route pagination invariants");
    assert_eq!(serde_json::to_value(rebuilt).unwrap(), fixture.changes_page);
}

#[test]
fn conflict_list_and_resolve_fixtures_match_passive_api_contracts() {
    let fixture = load_fixture();
    let query: ConflictListQuery = assert_fixture_roundtrip(&fixture.conflict_list_query);
    let _list: ConflictListRouteResponse = assert_fixture_roundtrip(&fixture.conflict_list);

    let status = query
        .status
        .as_ref()
        .map(wire_value)
        .expect("fixture conflict query must contain the V1 open status");
    let parsed_query = parse_conflicts_query(ConflictListRequestParts {
        status: Some(status.as_str()),
    })
    .expect("fixture conflict query must be accepted by route helpers");
    assert_eq!(
        serde_json::to_value(parsed_query.to_dto()).unwrap(),
        fixture.conflict_list_query
    );

    let mut resolution_actions = Vec::with_capacity(fixture.conflict_resolutions.len());
    for example in &fixture.conflict_resolutions {
        let request: ResolveConflictRequest = assert_fixture_roundtrip(&example.request);
        let response: ResolveConflictResponse = assert_fixture_roundtrip(&example.response);
        let resolution = wire_value(&request.resolution);
        resolution_actions.push(resolution.clone());
        let parsed = parse_resolve_conflict_request(ResolveConflictRequestParts {
            conflict_id: example.path_conflict_id.as_str(),
            resolution: Some(resolution.as_str()),
            extra_fields: &[],
        })
        .expect("fixture resolution must be accepted by passive API helpers");

        assert_eq!(
            serde_json::to_value(parsed.to_dto()).unwrap(),
            example.request
        );
        assert_eq!(
            parsed.conflict_id.as_str(),
            example.path_conflict_id.as_str()
        );
        assert_eq!(
            serde_json::to_value(resolved_conflict_response(
                parsed.conflict_id,
                parsed.resolution,
                response.seq,
            ))
            .unwrap(),
            example.response
        );
    }

    const RESOLUTION_ACTIONS: &[&str] = &[
        "accept_current",
        "accept_conflict",
        "keep_both",
        "mark_resolved",
    ];
    assert_complete_unique_strings(&resolution_actions, RESOLUTION_ACTIONS);
    assert_complete_unique_strings(
        &fixture.vocabulary.conflict_resolution_actions,
        RESOLUTION_ACTIONS,
    );
}

#[test]
fn public_error_examples_roundtrip_without_runtime_details() {
    let fixture = load_fixture();
    for example in &fixture.public_errors {
        let _: ErrorResponse = assert_fixture_roundtrip(example);
    }
}

#[test]
fn admin_status_doctor_and_adapter_summaries_are_consistent() {
    let fixture = load_fixture();
    let status: StatusSummaryResponse = assert_fixture_roundtrip(&fixture.admin.status_summary);
    let adapters: AdapterListResponse = assert_fixture_roundtrip(&fixture.admin.adapter_list);

    assert!(status.pause.is_consistent());
    assert_eq!(adapters.total_count, adapters.adapters.len() as u64);

    assert_eq!(status.adapter_count, Some(adapters.total_count));

    const DOCTOR_CHECKS: &[&str] = &[
        "database",
        "object_store",
        "operation_log",
        "adapter_registry",
    ];
    for value in &fixture.admin.doctor_statuses {
        let doctor: DoctorStatusResponse = assert_fixture_roundtrip(value);
        let check_kinds = doctor
            .checks
            .iter()
            .map(|check| wire_value(&check.check))
            .collect::<Vec<_>>();
        assert_complete_unique_strings(&check_kinds, DOCTOR_CHECKS);

        for check in doctor.checks {
            assert_eq!(check.readiness_state, check.status.readiness_state());
            match check.status {
                OperationalCheckStatus::Passed | OperationalCheckStatus::Failed => {
                    assert!(check.checked_at.is_some());
                }
                OperationalCheckStatus::Skipped
                | OperationalCheckStatus::NotRun
                | OperationalCheckStatus::Placeholder => {
                    assert!(check.checked_at.is_none());
                }
            }
        }
    }

    let operational_summaries = fixture
        .admin
        .adapter_operational_summaries
        .iter()
        .map(assert_fixture_roundtrip::<AdapterOperationalSummary>)
        .collect::<Vec<_>>();
    let listed_adapter_ids = adapters
        .adapters
        .iter()
        .map(|adapter| adapter.adapter_id.as_str().to_owned())
        .collect::<BTreeSet<_>>();
    let operational_adapter_ids = operational_summaries
        .iter()
        .map(|summary| summary.adapter.adapter_id.as_str().to_owned())
        .collect::<BTreeSet<_>>();
    assert_eq!(listed_adapter_ids.len(), adapters.adapters.len());
    assert_eq!(operational_adapter_ids.len(), operational_summaries.len());
    assert_eq!(listed_adapter_ids, operational_adapter_ids);

    for summary in operational_summaries {
        let listed_adapter = adapters
            .adapters
            .iter()
            .find(|adapter| adapter.adapter_id.as_str() == summary.adapter.adapter_id.as_str())
            .expect("every operational summary must reference a listed adapter");
        assert_eq!(listed_adapter, &summary.adapter);
        assert!(summary.runtime.pause.is_consistent());
        match summary.runtime.observation_status {
            OperationalCheckStatus::Passed | OperationalCheckStatus::Failed => {
                assert!(summary.runtime.state.is_some());
            }
            OperationalCheckStatus::Skipped
            | OperationalCheckStatus::NotRun
            | OperationalCheckStatus::Placeholder => {
                assert!(summary.runtime.state.is_none());
            }
        }
    }
}

#[test]
fn vocabulary_fixture_is_complete_for_represented_v1_contracts() {
    let vocabulary = load_fixture().vocabulary;

    assert_complete_unique_wires(
        &vocabulary.server_capabilities,
        &[
            ServerCapabilityDto::Sha256,
            ServerCapabilityDto::OperationLog,
            ServerCapabilityDto::Tombstones,
            ServerCapabilityDto::Conflicts,
            ServerCapabilityDto::ConflictCenter,
            ServerCapabilityDto::BatchChanges,
        ],
    );
    assert_complete_unique_wires(
        &vocabulary.put_ignored_reasons,
        &[FileIgnoredReasonDto::SameContent],
    );
    assert_complete_unique_wires(
        &vocabulary.put_rejected_reasons,
        &[
            FileRejectedReasonDto::HashMismatch,
            FileRejectedReasonDto::StaleBaseRevision,
            FileRejectedReasonDto::IgnoredPath,
            FileRejectedReasonDto::ValidationError,
            FileRejectedReasonDto::IdempotencyConflict,
        ],
    );
    assert_complete_unique_wires(
        &vocabulary.operation_kinds,
        &[
            OperationKindDto::UpsertFile,
            OperationKindDto::DeleteFile,
            OperationKindDto::RestoreFile,
            OperationKindDto::ConflictCreated,
            OperationKindDto::ConflictResolved,
            OperationKindDto::BackupCreated,
        ],
    );
    assert_complete_unique_wires(
        &vocabulary.conflict_policies,
        &[
            ConflictPolicyDto::PreserveBoth,
            ConflictPolicyDto::CurrentWinsWithIncomingBackup,
        ],
    );
    assert_complete_unique_wires(
        &vocabulary.conflict_statuses,
        &[
            ConflictStatusDto::Open,
            ConflictStatusDto::Resolved,
            ConflictStatusDto::Ignored,
        ],
    );
    assert_complete_unique_wires(
        &vocabulary.conflict_resolution_actions,
        &[
            ConflictResolutionDto::AcceptCurrent,
            ConflictResolutionDto::AcceptConflict,
            ConflictResolutionDto::KeepBoth,
            ConflictResolutionDto::MarkResolved,
        ],
    );
    assert_complete_unique_wires(
        &vocabulary.conflict_resolve_statuses,
        &[ConflictResolveStatusDto::Resolved],
    );
    assert_complete_unique_wires(
        &vocabulary.delete_rejected_reasons,
        &[
            DeleteRejectedReasonDto::StaleBaseRevision,
            DeleteRejectedReasonDto::UnsafeDelete,
            DeleteRejectedReasonDto::IdempotencyConflict,
            DeleteRejectedReasonDto::ValidationError,
        ],
    );
    assert_complete_unique_wires(
        &vocabulary.public_error_codes,
        &[
            PublicErrorCode::InvalidRequest,
            PublicErrorCode::InvalidPath,
            PublicErrorCode::ValidationError,
            PublicErrorCode::Unauthorized,
            PublicErrorCode::MissingToken,
            PublicErrorCode::InvalidToken,
            PublicErrorCode::ForbiddenRole,
            PublicErrorCode::NotFound,
            PublicErrorCode::Conflict,
            PublicErrorCode::IdempotencyConflict,
            PublicErrorCode::PayloadTooLarge,
            PublicErrorCode::RateLimited,
            PublicErrorCode::UnsafeDelete,
            PublicErrorCode::IgnoredPath,
            PublicErrorCode::InternalError,
        ],
    );
    assert_complete_unique_wires(
        &vocabulary.server_statuses,
        &[
            ServerStatus::NotReady,
            ServerStatus::Ready,
            ServerStatus::Degraded,
            ServerStatus::Maintenance,
        ],
    );
    assert_complete_unique_wires(
        &vocabulary.dependency_readiness_states,
        &[
            DependencyReadinessState::Unknown,
            DependencyReadinessState::Ready,
            DependencyReadinessState::NotReady,
        ],
    );
    assert_complete_unique_wires(
        &vocabulary.operational_check_statuses,
        &[
            OperationalCheckStatus::Passed,
            OperationalCheckStatus::Failed,
            OperationalCheckStatus::Skipped,
            OperationalCheckStatus::NotRun,
            OperationalCheckStatus::Placeholder,
        ],
    );
    assert_complete_unique_wires(
        &vocabulary.doctor_check_kinds,
        &[
            DoctorCheckKind::Database,
            DoctorCheckKind::ObjectStore,
            DoctorCheckKind::OperationLog,
            DoctorCheckKind::AdapterRegistry,
        ],
    );
    assert_complete_unique_wires(
        &vocabulary.cursor_presence_states,
        &[
            CursorPresence::Unknown,
            CursorPresence::Absent,
            CursorPresence::Present,
        ],
    );
    assert_complete_unique_wires(
        &vocabulary.adapter_runtime_states,
        &[
            AdapterRuntimeState::Unknown,
            AdapterRuntimeState::Disabled,
            AdapterRuntimeState::Idle,
            AdapterRuntimeState::Running,
            AdapterRuntimeState::Degraded,
        ],
    );
}
