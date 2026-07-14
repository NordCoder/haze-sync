use std::collections::BTreeSet;

use haze_sync_api::{
    auth::{AdapterPrincipal, AdapterRole},
    dto::worktree::{
        WorktreeConfiguredMode, WorktreeHostLifecycle, WorktreeManualAvailability,
        WorktreeReadiness, WorktreeReadinessReason, WorktreeStatusResponse,
        WorktreeStatusSafeParts, WorktreeSyncOnceRequest, WorktreeSyncOnceResponse,
        WorktreeSyncOnceSubmissionStatus,
    },
    routes::worktree::{
        authorize_worktree_sync_once_request, worktree_sync_once_response, WorktreeRouteError,
    },
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;

const FIXTURE_JSON: &str = include_str!("../fixtures/worktree-contract-v1.json");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorktreeCompatibilityFixture {
    schema_version: u32,
    status: Value,
    sync_once_request: Value,
    sync_once_outcomes: Vec<Value>,
    vocabulary: WorktreeVocabularyFixture,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorktreeVocabularyFixture {
    configured_modes: Vec<String>,
    host_lifecycles: Vec<String>,
    readiness_categories: Vec<String>,
    readiness_reasons: Vec<String>,
    manual_availability: Vec<String>,
    sync_once_submission_statuses: Vec<String>,
}

fn load_fixture() -> WorktreeCompatibilityFixture {
    serde_json::from_str(FIXTURE_JSON).expect("Worktree compatibility fixture must be valid JSON")
}

fn assert_roundtrip<T>(value: &Value) -> T
where
    T: DeserializeOwned + Serialize,
{
    let decoded = serde_json::from_value::<T>(value.clone())
        .expect("fixture value must deserialize into its public contract type");
    let encoded = serde_json::to_value(&decoded).expect("public contract must serialize");
    assert_eq!(encoded, *value, "fixture shape must roundtrip exactly");
    decoded
}

fn wire_value<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .expect("vocabulary value must serialize")
        .as_str()
        .expect("vocabulary value must be a JSON string")
        .to_owned()
}

fn assert_complete_unique_wires<T: Serialize>(actual: &[String], expected: &[T]) {
    let actual_set = actual.iter().cloned().collect::<BTreeSet<_>>();
    let expected_set = expected.iter().map(wire_value).collect::<BTreeSet<_>>();
    assert_eq!(
        actual.len(),
        actual_set.len(),
        "fixture values must be unique"
    );
    assert_eq!(actual_set, expected_set);
}

#[test]
fn fixture_is_strict_and_contains_no_private_runtime_material() {
    let fixture = load_fixture();
    assert_eq!(fixture.schema_version, 1);

    let lowercase = FIXTURE_JSON.to_ascii_lowercase();
    for forbidden in [
        "bearer ",
        "oauth",
        "token_hash",
        "database_url",
        "postgres://",
        "provider_payload",
        "request_payload",
        "raw_error",
        "root_fingerprint",
        "raw_cursor",
        "cursor_value",
        "idempotency",
        "runtime_ticket",
        "cycle_ticket",
        "generation_id",
        "mode_override",
        "force",
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
    assert!(serde_json::from_value::<WorktreeCompatibilityFixture>(root).is_err());
}

#[test]
fn status_fixture_roundtrips_and_preserves_server_supplied_semantics() {
    let fixture = load_fixture();
    let status: WorktreeStatusResponse = assert_roundtrip(&fixture.status);

    assert_eq!(status.configured_mode, WorktreeConfiguredMode::DryRun);
    assert_eq!(status.host_lifecycle, WorktreeHostLifecycle::Running);
    assert_eq!(status.readiness, WorktreeReadiness::Ready);
    assert_eq!(status.readiness_reason, WorktreeReadinessReason::Running);
    assert_eq!(status.manual_availability, WorktreeManualAvailability::Busy);
    assert!(status.is_ready());

    let disabled = WorktreeStatusResponse::from_safe_parts(WorktreeStatusSafeParts {
        configured_mode: WorktreeConfiguredMode::Disabled,
        host_lifecycle: WorktreeHostLifecycle::Disabled,
        readiness: WorktreeReadiness::Ready,
        readiness_reason: WorktreeReadinessReason::DisabledInert,
        cycles_completed: 0,
        cycles_failed: 0,
        cycle_in_progress: false,
        pending_watcher_hints: 0,
        manual_availability: WorktreeManualAvailability::Unavailable,
    });
    assert!(disabled.is_ready());

    let failed = WorktreeStatusResponse::from_safe_parts(WorktreeStatusSafeParts {
        configured_mode: WorktreeConfiguredMode::DryRun,
        host_lifecycle: WorktreeHostLifecycle::Failed,
        readiness: WorktreeReadiness::NotReady,
        readiness_reason: WorktreeReadinessReason::Failed,
        cycles_completed: u64::MAX,
        cycles_failed: u64::MAX,
        cycle_in_progress: false,
        pending_watcher_hints: u64::MAX,
        manual_availability: WorktreeManualAvailability::Failed,
    });
    assert!(!failed.is_ready());

    let busy_with_different_counts =
        WorktreeStatusResponse::from_safe_parts(WorktreeStatusSafeParts {
            cycles_completed: 0,
            cycles_failed: u64::MAX,
            pending_watcher_hints: u64::MAX,
            ..WorktreeStatusSafeParts {
                configured_mode: WorktreeConfiguredMode::DryRun,
                host_lifecycle: WorktreeHostLifecycle::Running,
                readiness: WorktreeReadiness::Ready,
                readiness_reason: WorktreeReadinessReason::Running,
                cycles_completed: 17,
                cycles_failed: 2,
                cycle_in_progress: true,
                pending_watcher_hints: 23,
                manual_availability: WorktreeManualAvailability::Busy,
            }
        });
    assert!(busy_with_different_counts.is_ready());
}

#[test]
fn sync_once_fixture_is_bodyless_submission_only_and_complete() {
    let fixture = load_fixture();
    let request: WorktreeSyncOnceRequest = assert_roundtrip(&fixture.sync_once_request);
    assert_eq!(request, WorktreeSyncOnceRequest::default());

    let mut statuses = Vec::new();
    for value in &fixture.sync_once_outcomes {
        let response: WorktreeSyncOnceResponse = assert_roundtrip(value);
        statuses.push(wire_value(&response.status));
        assert_eq!(
            worktree_sync_once_response(response.status),
            response,
            "pure response builder must preserve the sanitized outcome"
        );
    }

    assert_complete_unique_wires(
        &statuses,
        &[
            WorktreeSyncOnceSubmissionStatus::Accepted,
            WorktreeSyncOnceSubmissionStatus::Busy,
            WorktreeSyncOnceSubmissionStatus::NotStarted,
            WorktreeSyncOnceSubmissionStatus::Cancelling,
            WorktreeSyncOnceSubmissionStatus::Shutdown,
            WorktreeSyncOnceSubmissionStatus::Unavailable,
            WorktreeSyncOnceSubmissionStatus::Failed,
        ],
    );

    let accepted = worktree_sync_once_response(WorktreeSyncOnceSubmissionStatus::Accepted);
    assert!(accepted.was_accepted());
    assert!(!worktree_sync_once_response(WorktreeSyncOnceSubmissionStatus::Busy).was_accepted());
    assert!(serde_json::from_str::<WorktreeSyncOnceRequest>("{\"force\":true}").is_err());
}

#[test]
fn sync_once_authorization_accepts_only_verified_admin_principals() {
    let admin = AdapterPrincipal::new("admin-fixture", AdapterRole::Admin).unwrap();
    assert!(
        authorize_worktree_sync_once_request(WorktreeSyncOnceRequest::default(), Some(&admin))
            .is_ok()
    );

    let adapter = AdapterPrincipal::new("worktree-fixture", AdapterRole::WorktreeAdapter).unwrap();
    assert_eq!(
        authorize_worktree_sync_once_request(WorktreeSyncOnceRequest::default(), Some(&adapter)),
        Err(WorktreeRouteError::ForbiddenRole)
    );
    assert_eq!(
        authorize_worktree_sync_once_request(WorktreeSyncOnceRequest::default(), None),
        Err(WorktreeRouteError::MissingAdapterPrincipal)
    );
}

#[test]
fn vocabulary_fixture_is_complete_for_the_worktree_contract() {
    let vocabulary = load_fixture().vocabulary;

    assert_complete_unique_wires(
        &vocabulary.configured_modes,
        &[
            WorktreeConfiguredMode::Disabled,
            WorktreeConfiguredMode::ReadOnly,
            WorktreeConfiguredMode::ImportOnly,
            WorktreeConfiguredMode::ExportOnly,
            WorktreeConfiguredMode::Bidirectional,
            WorktreeConfiguredMode::DryRun,
        ],
    );
    assert_complete_unique_wires(
        &vocabulary.host_lifecycles,
        &[
            WorktreeHostLifecycle::Disabled,
            WorktreeHostLifecycle::Starting,
            WorktreeHostLifecycle::Running,
            WorktreeHostLifecycle::Cancelling,
            WorktreeHostLifecycle::Shutdown,
            WorktreeHostLifecycle::Failed,
        ],
    );
    assert_complete_unique_wires(
        &vocabulary.readiness_categories,
        &[WorktreeReadiness::Ready, WorktreeReadiness::NotReady],
    );
    assert_complete_unique_wires(
        &vocabulary.readiness_reasons,
        &[
            WorktreeReadinessReason::DisabledInert,
            WorktreeReadinessReason::Running,
            WorktreeReadinessReason::Starting,
            WorktreeReadinessReason::Cancelling,
            WorktreeReadinessReason::Shutdown,
            WorktreeReadinessReason::Failed,
        ],
    );
    assert_complete_unique_wires(
        &vocabulary.manual_availability,
        &[
            WorktreeManualAvailability::Available,
            WorktreeManualAvailability::Busy,
            WorktreeManualAvailability::NotStarted,
            WorktreeManualAvailability::Cancelling,
            WorktreeManualAvailability::Shutdown,
            WorktreeManualAvailability::Unavailable,
            WorktreeManualAvailability::Failed,
        ],
    );
    assert_complete_unique_wires(
        &vocabulary.sync_once_submission_statuses,
        &[
            WorktreeSyncOnceSubmissionStatus::Accepted,
            WorktreeSyncOnceSubmissionStatus::Busy,
            WorktreeSyncOnceSubmissionStatus::NotStarted,
            WorktreeSyncOnceSubmissionStatus::Cancelling,
            WorktreeSyncOnceSubmissionStatus::Shutdown,
            WorktreeSyncOnceSubmissionStatus::Unavailable,
            WorktreeSyncOnceSubmissionStatus::Failed,
        ],
    );
}
