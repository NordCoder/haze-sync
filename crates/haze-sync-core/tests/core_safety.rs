use std::collections::BTreeMap;

use chrono::{DateTime, TimeZone, Utc};
use haze_sync_common::{AdapterId, ContentHash, OperationId, RevisionId, VaultPath};
use haze_sync_core::{
    conflict_saved_planner::{
        plan_upsert_conflict_saved, require_upsert_conflict_saved_plan,
        ConflictSavedPlanningError,
    },
    conflict_service::{ConflictPolicy, ConflictPolicyError, ConflictRecordStatus},
    idempotency::{
        IdempotencyKey, IdempotencyReplayOutcome, IdempotencyScope, IdempotencyService,
        RequestFingerprint, StoredIdempotencyRecord, StoredIdempotencyResponse,
    },
    revision_service::{
        compute_content_hash, AppendOperationRequest, ConflictPolicyHint, ConflictSavedOutcome,
        ContentStore, IncomingConflictContent, InsertRevisionRequest, OperationLog,
        OperationLogEntry, RevisionRepository, RevisionService, RevisionServiceError,
        StoredContent, StoredRevision, UpsertFileRequest, UpsertOutcome,
    },
};
use serde_json::json;

#[derive(Debug)]
struct FakeRevisionRepository {
    current: Option<StoredRevision>,
    inserted: Vec<InsertRevisionRequest>,
}

impl FakeRevisionRepository {
    fn with_current(current: StoredRevision) -> Self {
        Self {
            current: Some(current),
            inserted: Vec::new(),
        }
    }
}

impl RevisionRepository for FakeRevisionRepository {
    fn current_revision(
        &mut self,
        _path: &VaultPath,
    ) -> Result<Option<StoredRevision>, RevisionServiceError> {
        Ok(self.current.clone())
    }

    fn insert_revision(
        &mut self,
        request: InsertRevisionRequest,
    ) -> Result<StoredRevision, RevisionServiceError> {
        let next_index = self.inserted.len() + 1;
        let revision = StoredRevision {
            revision_id: revision_id(&format!("rev_inserted_{next_index}")),
            path: request.path.clone(),
            parent_revision_id: request.parent_revision_id.clone(),
            content_hash: request.content_hash,
            size_bytes: request.size_bytes,
            created_by: request.adapter_id.clone(),
        };
        self.inserted.push(request);
        self.current = Some(revision.clone());
        Ok(revision)
    }
}

#[derive(Debug, Default)]
struct FakeContentStore {
    writes: Vec<(ContentHash, Vec<u8>)>,
}

impl ContentStore for FakeContentStore {
    fn put_content(
        &mut self,
        expected_hash: ContentHash,
        bytes: &[u8],
    ) -> Result<StoredContent, RevisionServiceError> {
        self.writes.push((expected_hash, bytes.to_vec()));
        Ok(StoredContent {
            hash: expected_hash,
            size_bytes: bytes.len() as u64,
        })
    }
}

#[derive(Debug, Default)]
struct FakeOperationLog {
    entries: Vec<AppendOperationRequest>,
}

impl OperationLog for FakeOperationLog {
    fn append_operation(
        &mut self,
        request: AppendOperationRequest,
    ) -> Result<OperationLogEntry, RevisionServiceError> {
        let next_seq = i64::try_from(self.entries.len() + 1)
            .expect("fixture operation log length should fit in i64");
        self.entries.push(request);
        Ok(OperationLogEntry {
            operation_id: OperationId::parse(&format!("op_{next_seq}"))
                .expect("fixture operation id should parse"),
            seq: next_seq,
        })
    }
}

fn adapter_id() -> AdapterId {
    AdapterId::parse("iphone-anna").expect("fixture adapter id should parse")
}

fn revision_id(input: &str) -> RevisionId {
    RevisionId::parse(input).expect("fixture revision id should parse")
}

fn path(input: &str) -> VaultPath {
    VaultPath::parse(input).expect("fixture vault path should parse")
}

fn fixed_timestamp() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 7, 3, 10, 11, 12)
        .single()
        .expect("fixture timestamp should be valid")
}

fn current_revision_at(path_input: &str, bytes: &[u8]) -> StoredRevision {
    StoredRevision {
        revision_id: revision_id("rev_current"),
        path: path(path_input),
        parent_revision_id: None,
        content_hash: compute_content_hash(bytes),
        size_bytes: bytes.len() as u64,
        created_by: adapter_id(),
    }
}

fn current_revision(bytes: &[u8]) -> StoredRevision {
    current_revision_at("Projects/Haze/plan.md", bytes)
}

fn upsert_request_at(
    path_input: &str,
    base_revision_id: Option<RevisionId>,
    bytes: &[u8],
) -> UpsertFileRequest {
    UpsertFileRequest::new(
        path(path_input),
        adapter_id(),
        base_revision_id,
        compute_content_hash(bytes),
        bytes.to_vec(),
    )
}

fn upsert_request(base_revision_id: Option<RevisionId>, bytes: &[u8]) -> UpsertFileRequest {
    upsert_request_at("Projects/Haze/plan.md", base_revision_id, bytes)
}

fn expected_conflict_saved(
    current_revision: &StoredRevision,
    provided_base_revision_id: Option<RevisionId>,
    bytes: &[u8],
) -> Box<ConflictSavedOutcome> {
    Box::new(ConflictSavedOutcome {
        current_revision: current_revision.clone(),
        provided_base_revision_id,
        incoming_content: IncomingConflictContent {
            path: current_revision.path.clone(),
            adapter_id: adapter_id(),
            content_hash: compute_content_hash(bytes),
            size_bytes: bytes.len() as u64,
            content: bytes.to_vec(),
        },
        policy_hint: ConflictPolicyHint::PreserveBoth,
    })
}

fn delete_response() -> StoredIdempotencyResponse {
    StoredIdempotencyResponse::json(
        200,
        json!({
            "status": "tombstoned",
            "tombstone_id": "tmb_01JSAFE",
            "path": "Projects/Haze/old.md"
        }),
    )
    .expect("fixture response should be safe")
}

fn delete_fingerprint(path: &str, base_revision_id: Option<&str>) -> RequestFingerprint {
    RequestFingerprint::from_safe_json_metadata(&json!({
        "method": "DELETE",
        "path": path,
        "base_revision_id": base_revision_id,
        "content_sha256": null
    }))
}

fn conflict_saved_outcome(
    current: StoredRevision,
    base_revision_id: Option<RevisionId>,
    incoming: &[u8],
) -> UpsertOutcome {
    let mut service = RevisionService::new(
        FakeRevisionRepository::with_current(current),
        FakeContentStore::default(),
        FakeOperationLog::default(),
    );

    service
        .upsert_file(upsert_request(base_revision_id, incoming))
        .expect("fake-backed service should not fail")
}

#[test]
fn stale_base_different_content_rejects_without_silent_overwrite() {
    let current = current_revision(b"current content");
    let mut service = RevisionService::new(
        FakeRevisionRepository::with_current(current.clone()),
        FakeContentStore::default(),
        FakeOperationLog::default(),
    );

    let outcome = service
        .upsert_file(upsert_request(
            Some(revision_id("rev_stale")),
            b"incoming content",
        ))
        .expect("fake-backed service should not fail");

    assert_eq!(
        outcome,
        UpsertOutcome::RejectedStaleOrUnknownBase {
            current_revision: Some(current.clone()),
            provided_base_revision_id: Some(revision_id("rev_stale")),
            conflict_saved: Some(expected_conflict_saved(
                &current,
                Some(revision_id("rev_stale")),
                b"incoming content",
            )),
        }
    );

    let (repository, content_store, operation_log) = service.into_inner();
    assert_eq!(repository.current, Some(current));
    assert!(repository.inserted.is_empty());
    assert!(content_store.writes.is_empty());
    assert!(operation_log.entries.is_empty());
}

#[test]
fn null_base_existing_different_content_rejects_without_silent_overwrite() {
    let current = current_revision(b"current content");
    let mut service = RevisionService::new(
        FakeRevisionRepository::with_current(current.clone()),
        FakeContentStore::default(),
        FakeOperationLog::default(),
    );

    let outcome = service
        .upsert_file(upsert_request(None, b"incoming content"))
        .expect("fake-backed service should not fail");

    assert_eq!(
        outcome,
        UpsertOutcome::RejectedStaleOrUnknownBase {
            current_revision: Some(current.clone()),
            provided_base_revision_id: None,
            conflict_saved: Some(expected_conflict_saved(&current, None, b"incoming content")),
        }
    );

    let (repository, content_store, operation_log) = service.into_inner();
    assert_eq!(repository.current, Some(current));
    assert!(repository.inserted.is_empty());
    assert!(content_store.writes.is_empty());
    assert!(operation_log.entries.is_empty());
}

#[test]
fn stale_base_different_content_produces_conflict_saved_planning_surface() {
    let current = current_revision(b"current content");
    let outcome = conflict_saved_outcome(
        current.clone(),
        Some(revision_id("rev_stale")),
        b"incoming content",
    );

    let plan = require_upsert_conflict_saved_plan(&outcome, fixed_timestamp())
        .expect("stale base should produce conflict_saved plan");

    assert_eq!(outcome.public_status(), "conflict_saved");
    assert_eq!(plan.original_path, path("Projects/Haze/plan.md"));
    assert_eq!(
        plan.materialized_path.as_str(),
        "_haze_conflicts/open/Projects/Haze/plan.conflict.iphone-anna.2026-07-03-101112.md"
    );
    assert_eq!(plan.provided_base_revision_id, Some(revision_id("rev_stale")));
    assert_eq!(plan.current_revision_id, current.revision_id);
    assert_eq!(plan.current_content_hash, current.content_hash);
    assert_eq!(plan.current_size_bytes, current.size_bytes);
    assert_eq!(plan.incoming_adapter_id, adapter_id());
    assert_eq!(plan.incoming_content_hash, compute_content_hash(b"incoming content"));
    assert_eq!(plan.incoming_size_bytes, b"incoming content".len() as u64);
    assert_eq!(plan.policy_applied, ConflictPolicy::PreserveBoth);
    assert_eq!(plan.status, ConflictRecordStatus::Open);
    assert_eq!(plan.created_at, fixed_timestamp());
    assert_eq!(plan.conflict_record.conflict_path, plan.materialized_path);
}

#[test]
fn unknown_base_different_content_produces_conflict_saved_planning_surface() {
    let current = current_revision(b"current content");
    let outcome = conflict_saved_outcome(
        current.clone(),
        Some(revision_id("rev_unknown")),
        b"incoming content",
    );

    let plan = require_upsert_conflict_saved_plan(&outcome, fixed_timestamp())
        .expect("unknown base should produce conflict_saved plan");

    assert_eq!(outcome.public_status(), "conflict_saved");
    assert_eq!(plan.original_path, path("Projects/Haze/plan.md"));
    assert_eq!(plan.provided_base_revision_id, Some(revision_id("rev_unknown")));
    assert_eq!(plan.current_revision_id, current.revision_id);
    assert_eq!(plan.incoming_content_hash, compute_content_hash(b"incoming content"));
    assert_eq!(plan.policy_applied, ConflictPolicy::PreserveBoth);
    assert_eq!(plan.status, ConflictRecordStatus::Open);
}

#[test]
fn null_base_existing_different_content_produces_conflict_saved_planning_surface() {
    let current = current_revision(b"current content");
    let outcome = conflict_saved_outcome(current.clone(), None, b"incoming content");

    let plan = require_upsert_conflict_saved_plan(&outcome, fixed_timestamp())
        .expect("null base over existing different content should produce conflict_saved plan");

    assert_eq!(outcome.public_status(), "conflict_saved");
    assert_eq!(plan.original_path, path("Projects/Haze/plan.md"));
    assert_eq!(plan.provided_base_revision_id, None);
    assert_eq!(plan.current_revision_id, current.revision_id);
    assert_eq!(plan.incoming_content_hash, compute_content_hash(b"incoming content"));
    assert_eq!(plan.policy_applied, ConflictPolicy::PreserveBoth);
    assert_eq!(plan.status, ConflictRecordStatus::Open);
}

#[test]
fn same_content_stale_unknown_and_null_base_remain_ignored() {
    let cases = [
        Some(revision_id("rev_stale")),
        Some(revision_id("rev_unknown")),
        None,
    ];

    for base_revision_id in cases {
        let current = current_revision(b"same content");
        let mut service = RevisionService::new(
            FakeRevisionRepository::with_current(current.clone()),
            FakeContentStore::default(),
            FakeOperationLog::default(),
        );

        let outcome = service
            .upsert_file(upsert_request(base_revision_id, b"same content"))
            .expect("fake-backed service should not fail");

        assert!(matches!(
            outcome,
            UpsertOutcome::IgnoredDuplicateSameContent { .. }
        ));
        assert_eq!(outcome.public_status(), "same_content");
        assert_eq!(
            plan_upsert_conflict_saved(&outcome, fixed_timestamp())
                .expect("ignored same-content outcome should be plannable as none"),
            None
        );

        let (repository, content_store, operation_log) = service.into_inner();
        assert_eq!(repository.current, Some(current));
        assert!(repository.inserted.is_empty());
        assert!(content_store.writes.is_empty());
        assert!(operation_log.entries.is_empty());
    }
}

#[test]
fn conflict_materialized_path_stays_under_open_conflicts_dir() {
    let current = current_revision(b"current content");
    let outcome = conflict_saved_outcome(
        current,
        Some(revision_id("rev_stale")),
        b"incoming content",
    );

    let plan = require_upsert_conflict_saved_plan(&outcome, fixed_timestamp())
        .expect("stale conflict should produce materialization path");

    assert!(
        plan.materialized_path
            .as_str()
            .starts_with("_haze_conflicts/open/"),
        "materialized conflict path should stay below open conflict root"
    );
    assert_eq!(
        plan.incoming_backup.conflict_path,
        path("_haze_conflicts/open/Projects/Haze/plan.conflict.iphone-anna.2026-07-03-101112.md")
    );
}

#[test]
fn recursive_haze_conflicts_source_path_is_rejected() {
    let conflict_path = "_haze_conflicts/open/Projects/Haze/plan.conflict.iphone-anna.2026-07-03-101112.md";
    let current = current_revision_at(conflict_path, b"current conflict copy");
    let mut service = RevisionService::new(
        FakeRevisionRepository::with_current(current),
        FakeContentStore::default(),
        FakeOperationLog::default(),
    );

    let outcome = service
        .upsert_file(upsert_request_at(
            conflict_path,
            Some(revision_id("rev_stale")),
            b"incoming conflict copy",
        ))
        .expect("fake-backed service should not fail");

    assert_eq!(outcome.public_status(), "conflict_saved");
    assert_eq!(
        require_upsert_conflict_saved_plan(&outcome, fixed_timestamp()).unwrap_err(),
        ConflictSavedPlanningError::ConflictPolicy {
            reason: ConflictPolicyError::RecursiveConflictPath,
        }
    );
}

#[test]
fn conflict_saved_plan_serializes_without_raw_incoming_bytes() {
    let current = current_revision(b"current content");
    let outcome = conflict_saved_outcome(
        current,
        Some(revision_id("rev_stale")),
        b"do not leak raw incoming bytes",
    );
    let plan = require_upsert_conflict_saved_plan(&outcome, fixed_timestamp())
        .expect("stale conflict should produce plan");

    let serialized = serde_json::to_string(&plan).expect("plan should serialize");

    assert!(serialized.contains("_haze_conflicts/open"));
    assert!(serialized.contains("preserve_both"));
    assert!(!serialized.contains("do not leak raw incoming bytes"));
}

#[test]
fn idempotent_delete_same_key_same_request_replays_saved_response() {
    let fingerprint = delete_fingerprint("Projects/Haze/old.md", Some("rev_current"));
    let record = StoredIdempotencyRecord::new(
        IdempotencyScope::adapter(adapter_id()),
        IdempotencyKey::parse("iphone-anna:delete-001")
            .expect("fixture idempotency key should parse"),
        fingerprint,
        delete_response(),
    );

    let outcome = IdempotencyService::evaluate(Some(&record), fingerprint);

    assert_eq!(
        outcome,
        IdempotencyReplayOutcome::ReplaySameRequest {
            response: delete_response(),
        }
    );
}

#[test]
fn idempotent_delete_same_key_different_request_is_rejected() {
    let stored_fingerprint = delete_fingerprint("Projects/Haze/old.md", Some("rev_current"));
    let incoming_fingerprint = delete_fingerprint("Projects/Haze/other.md", Some("rev_current"));
    let record = StoredIdempotencyRecord::new(
        IdempotencyScope::adapter(adapter_id()),
        IdempotencyKey::parse("iphone-anna:delete-001")
            .expect("fixture idempotency key should parse"),
        stored_fingerprint,
        delete_response(),
    );

    let outcome = IdempotencyService::evaluate(Some(&record), incoming_fingerprint);

    assert_eq!(
        outcome,
        IdempotencyReplayOutcome::ConflictDifferentRequest {
            stored_request_fingerprint: stored_fingerprint,
            incoming_request_fingerprint: incoming_fingerprint,
        }
    );
}

#[test]
fn stored_delete_idempotency_response_is_public_json_only() {
    let response = delete_response();
    let serialized = serde_json::to_string(&response).expect("response should serialize");
    let empty_headers: BTreeMap<String, String> = BTreeMap::new();
    let authorization_header = ["Author", "ization"].concat();
    let database_url_key = ["DATABASE", "_URL"].concat();

    assert!(serialized.contains("tombstoned"));
    assert!(!serialized.contains(&authorization_header));
    assert!(!serialized.contains(&database_url_key));
    assert!(!serialized.contains("/srv/"));
    assert_eq!(response.headers(), &empty_headers);
}

fn pending_sibling_behavior(phase: &str, behavior: &str) -> ! {
    panic!(
        "{phase} must replace this ignored W3-P8 spec placeholder with executable coverage for {behavior}"
    );
}

#[test]
#[ignore = "requires W3-P3 tombstone service implementation"]
fn delete_creates_tombstone_and_retains_revision_history() {
    pending_sibling_behavior("W3-P3", "delete creates tombstone");
}

#[test]
#[ignore = "requires W3-P3 delete guard implementation"]
fn mass_delete_is_blocked_by_count() {
    pending_sibling_behavior("W3-P3", "mass delete blocked by count");
}

#[test]
#[ignore = "requires W3-P3 delete guard implementation"]
fn mass_delete_is_blocked_by_ratio() {
    pending_sibling_behavior("W3-P3", "mass delete blocked by ratio");
}

#[test]
#[ignore = "requires W3-P3 manual unlock implementation"]
fn manual_unlock_allows_explicitly_authorized_delete_batch() {
    pending_sibling_behavior("W3-P3", "manual unlock behavior");
}

#[test]
#[ignore = "requires W3-P3 tombstone service implementation"]
fn delete_never_hard_deletes_content_in_core() {
    pending_sibling_behavior("W3-P3", "delete never hard-deletes content");
}
