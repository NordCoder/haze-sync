use chrono::{DateTime, Utc};
use haze_sync_common::{AdapterId, ContentHash, OperationId, RevisionId, VaultPath};
use haze_sync_core::delete_guard::{
    DeleteGuard, DeleteGuardDecision, DeleteGuardInput, DeleteRunScope,
};
use haze_sync_core::doctor::{
    db_connectivity_check, missing_blob_detection_check, DbConnectivityCheckInput, DoctorCheckId,
    DoctorCheckResult, DoctorCheckStatus, DoctorNotRunReason, DoctorReport,
    MissingBlobDetectionInput,
};
use haze_sync_core::idempotency::{
    IdempotencyKey, IdempotencyReplayOutcome, IdempotencyScope, IdempotencyService,
    RequestFingerprint, StoredIdempotencyRecord, StoredIdempotencyResponse,
};
use haze_sync_core::operation_log::{
    classify_cursor_update, CursorUpdateOutcome, OperationSequence,
};
use haze_sync_core::revision_service::{
    compute_content_hash, AppendOperationRequest, ContentStore, InsertRevisionRequest,
    OperationLog as RevisionOperationLog, OperationLogEntry as RevisionOperationLogEntry,
    RevisionRepository, RevisionService, RevisionServiceError, StoredContent, StoredRevision,
    UpsertFileRequest, UpsertOutcome,
};
use haze_sync_core::tombstone_service::{
    RestoreEligibility, Tombstone, TombstoneCreationInput, TombstoneRetention, TombstoneService,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const FIXTURE_JSON: &str = include_str!("../fixtures/compatibility/v1/core-compatibility.json");
const CURRENT_BYTES: &[u8] = b"current fixture content\n";
const INCOMING_BYTES: &[u8] = b"incoming fixture content\n";

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct CompatibilityFixtures {
    fixture_set: String,
    version: u32,
    examples: CompatibilityExamples,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct CompatibilityExamples {
    accepted_write: UpsertOutcome,
    same_content: UpsertOutcome,
    conflict_saved: UpsertOutcome,
    hash_mismatch: UpsertOutcome,
    tombstone: Tombstone,
    delete_guard_block: DeleteGuardDecision,
    idempotency_replay: IdempotencyReplayOutcome,
    idempotency_conflict: IdempotencyReplayOutcome,
    operation_log_cursor_outcomes: Vec<CursorOutcomeFixture>,
    doctor_summary_offline: DoctorReport,
    doctor_summary_partial: DoctorReport,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct CursorOutcomeFixture {
    current_seq: i64,
    requested_seq: i64,
    outcome: CursorUpdateOutcome,
}

#[derive(Debug)]
struct FixtureRepository {
    current_revision: Option<StoredRevision>,
}

impl RevisionRepository for FixtureRepository {
    fn current_revision(
        &mut self,
        _path: &VaultPath,
    ) -> Result<Option<StoredRevision>, RevisionServiceError> {
        Ok(self.current_revision.clone())
    }

    fn insert_revision(
        &mut self,
        request: InsertRevisionRequest,
    ) -> Result<StoredRevision, RevisionServiceError> {
        Ok(StoredRevision {
            revision_id: revision_id("rev_01JACCEPTED"),
            path: request.path,
            parent_revision_id: request.parent_revision_id,
            content_hash: request.content_hash,
            size_bytes: request.size_bytes,
            created_by: request.adapter_id,
        })
    }
}

#[derive(Debug)]
struct FixtureContentStore;

impl ContentStore for FixtureContentStore {
    fn put_content(
        &mut self,
        expected_hash: ContentHash,
        bytes: &[u8],
    ) -> Result<StoredContent, RevisionServiceError> {
        Ok(StoredContent {
            hash: expected_hash,
            size_bytes: u64::try_from(bytes.len())
                .map_err(|_error| RevisionServiceError::content_store("fixture_size"))?,
        })
    }
}

#[derive(Debug)]
struct FixtureOperationLog;

impl RevisionOperationLog for FixtureOperationLog {
    fn append_operation(
        &mut self,
        _request: AppendOperationRequest,
    ) -> Result<RevisionOperationLogEntry, RevisionServiceError> {
        Ok(RevisionOperationLogEntry {
            operation_id: operation_id("op_01JACCEPTED"),
            seq: 42,
        })
    }
}

#[test]
fn fixture_catalog_is_canonical_and_secret_free() {
    let fixtures = fixtures();
    let mut serialized =
        serde_json::to_string_pretty(&fixtures).expect("compatibility fixtures should serialize");
    serialized.push('\n');

    assert_eq!(serialized, FIXTURE_JSON);
    assert_eq!(fixtures.fixture_set, "haze-sync-core-compatibility");
    assert_eq!(fixtures.version, 1);

    let lowercase = FIXTURE_JSON.to_ascii_lowercase();
    for forbidden in [
        "authorization",
        "bearer ",
        "idempotency-key",
        "postgres://",
        "oauth",
        "token_hash",
        "\"content\":",
        "\"bytes\":",
        "/srv/",
        "c:\\\\",
    ] {
        assert!(
            !lowercase.contains(forbidden),
            "fixture catalog contains forbidden marker {forbidden:?}"
        );
    }
}

#[test]
fn revision_outcome_fixtures_match_live_core_semantics() {
    let fixtures = fixtures();
    let examples = fixtures.examples;

    let accepted = run_upsert(
        Some(revision_id("rev_01JCURRENT")),
        compute_content_hash(INCOMING_BYTES),
        INCOMING_BYTES,
    );
    let same_content = run_upsert(None, compute_content_hash(CURRENT_BYTES), CURRENT_BYTES);
    let conflict_saved = run_upsert(
        Some(revision_id("rev_01JSTALE")),
        compute_content_hash(INCOMING_BYTES),
        INCOMING_BYTES,
    );
    let hash_mismatch = run_upsert(
        Some(revision_id("rev_01JCURRENT")),
        compute_content_hash(CURRENT_BYTES),
        INCOMING_BYTES,
    );

    assert_public_json_eq(&examples.accepted_write, &accepted);
    assert_public_json_eq(&examples.same_content, &same_content);
    assert_public_json_eq(&examples.conflict_saved, &conflict_saved);
    assert_public_json_eq(&examples.hash_mismatch, &hash_mismatch);

    assert_eq!(
        examples.accepted_write.public_status(),
        "accepted_new_revision"
    );
    assert_eq!(examples.same_content.public_status(), "same_content");
    assert_eq!(examples.conflict_saved.public_status(), "conflict_saved");
    assert_eq!(
        examples.hash_mismatch.public_status(),
        "rejected_hash_mismatch"
    );

    let preserved = examples
        .conflict_saved
        .conflict_saved()
        .expect("conflict fixture must preserve incoming metadata");
    assert!(preserved.incoming_content.content.is_empty());
    assert_eq!(preserved.incoming_content.size_bytes, 25);
}

#[test]
fn tombstone_and_delete_guard_fixtures_match_live_core_semantics() {
    let fixtures = fixtures();
    let examples = fixtures.examples;

    let tombstone = TombstoneService::new()
        .create_tombstone(TombstoneCreationInput::new(
            haze_sync_core::tombstone_service::TombstoneId::parse("tmb_01JDELETED").unwrap(),
            vault_path("Notes/old.md"),
            revision_id("rev_01JDELETED"),
            None,
            adapter_id("gdrive-adapter"),
            TombstoneRetention::new(utc("2026-08-01T00:00:00Z"), Some(30)),
            Some(utc("2026-07-02T00:00:00Z")),
        ))
        .expect("fixture tombstone should be valid");
    assert_eq!(examples.tombstone, tombstone);
    assert_eq!(
        examples.tombstone.classify_restore_eligibility().unwrap(),
        RestoreEligibility::Eligible {
            deleted_revision_id: revision_id("rev_01JDELETED"),
            current_revision_id: revision_id("rev_01JDELETED"),
        }
    );

    let scope = DeleteRunScope::new(adapter_id("gdrive-adapter"), "run_01JDELETE").unwrap();
    let input = DeleteGuardInput::without_manual_unlock(scope, 25, 100);
    assert_eq!(
        examples.delete_guard_block,
        DeleteGuard::default().evaluate(&input)
    );
}

#[test]
fn idempotency_fixtures_match_fingerprint_and_replay_semantics() {
    let fixtures = fixtures();
    let examples = fixtures.examples;

    let stored_fingerprint = RequestFingerprint::from_safe_json_metadata(&json!({
        "content_sha256": compute_content_hash(INCOMING_BYTES),
        "method": "PUT",
        "path": "Notes/plan.md"
    }));
    let incoming_fingerprint = RequestFingerprint::from_safe_json_metadata(&json!({
        "content_sha256": compute_content_hash(CURRENT_BYTES),
        "method": "PUT",
        "path": "Notes/plan.md"
    }));

    let mut headers = BTreeMap::new();
    headers.insert("Content-Type".to_owned(), "application/json".to_owned());
    headers.insert("X-Revision-Id".to_owned(), "rev_01JACCEPTED".to_owned());
    let response = StoredIdempotencyResponse::new(
        200,
        json!({
            "revision_id": "rev_01JACCEPTED",
            "status": "accepted"
        }),
        headers,
    )
    .unwrap();
    let record = StoredIdempotencyRecord::new(
        IdempotencyScope::adapter(adapter_id("gdrive-adapter")),
        IdempotencyKey::parse("synthetic-fixture-key").unwrap(),
        stored_fingerprint,
        response,
    );

    assert_eq!(
        examples.idempotency_replay,
        IdempotencyService::evaluate(Some(&record), stored_fingerprint)
    );
    assert_eq!(
        examples.idempotency_conflict,
        IdempotencyService::evaluate(Some(&record), incoming_fingerprint)
    );
    assert!(!FIXTURE_JSON.contains("synthetic-fixture-key"));
}

#[test]
fn cursor_and_doctor_fixtures_match_live_core_semantics() {
    let fixtures = fixtures();
    let examples = fixtures.examples;

    assert_eq!(examples.operation_log_cursor_outcomes.len(), 3);
    for case in &examples.operation_log_cursor_outcomes {
        let current = OperationSequence::new(case.current_seq).unwrap();
        let requested = OperationSequence::new(case.requested_seq).unwrap();
        assert_eq!(case.outcome, classify_cursor_update(current, requested));
    }

    let offline = DoctorReport::from_results(vec![
        db_connectivity_check(DbConnectivityCheckInput::offline(true)),
        missing_blob_detection_check(MissingBlobDetectionInput::new(2, Vec::new(), 2)),
    ]);
    assert_eq!(examples.doctor_summary_offline, offline);
    assert_eq!(
        examples.doctor_summary_offline.summary().status,
        DoctorCheckStatus::Skipped
    );

    let partial = DoctorReport::from_results(vec![
        DoctorCheckResult::not_run(
            DoctorCheckId::GdriveMapping,
            DoctorNotRunReason::ToolingUnavailable,
        ),
        DoctorCheckResult::placeholder(DoctorCheckId::WorktreeDrift),
    ]);
    assert_eq!(examples.doctor_summary_partial, partial);
    assert_eq!(
        examples.doctor_summary_partial.summary().status,
        DoctorCheckStatus::Placeholder
    );
}

fn fixtures() -> CompatibilityFixtures {
    serde_json::from_str(FIXTURE_JSON).expect("compatibility fixture catalog should deserialize")
}

fn run_upsert(
    base_revision_id: Option<RevisionId>,
    expected_hash: ContentHash,
    content: &[u8],
) -> UpsertOutcome {
    let mut service = RevisionService::new(
        FixtureRepository {
            current_revision: Some(current_revision()),
        },
        FixtureContentStore,
        FixtureOperationLog,
    );
    service
        .upsert_file(UpsertFileRequest::new(
            vault_path("Notes/plan.md"),
            adapter_id("gdrive-adapter"),
            base_revision_id,
            expected_hash,
            content.to_vec(),
        ))
        .expect("fixture upsert should succeed")
}

fn current_revision() -> StoredRevision {
    StoredRevision {
        revision_id: revision_id("rev_01JCURRENT"),
        path: vault_path("Notes/plan.md"),
        parent_revision_id: None,
        content_hash: compute_content_hash(CURRENT_BYTES),
        size_bytes: u64::try_from(CURRENT_BYTES.len()).unwrap(),
        created_by: adapter_id("worktree-adapter"),
    }
}

fn assert_public_json_eq(left: &UpsertOutcome, right: &UpsertOutcome) {
    assert_eq!(public_json(left), public_json(right));
}

fn public_json(value: &impl Serialize) -> Value {
    serde_json::to_value(value).expect("public fixture value should serialize")
}

fn adapter_id(value: &str) -> AdapterId {
    AdapterId::parse(value).expect("fixture adapter id should parse")
}

fn revision_id(value: &str) -> RevisionId {
    RevisionId::parse(value).expect("fixture revision id should parse")
}

fn operation_id(value: &str) -> OperationId {
    OperationId::parse(value).expect("fixture operation id should parse")
}

fn vault_path(value: &str) -> VaultPath {
    VaultPath::parse(value).expect("fixture vault path should parse")
}

fn utc(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .expect("fixture timestamp should parse")
        .with_timezone(&Utc)
}
