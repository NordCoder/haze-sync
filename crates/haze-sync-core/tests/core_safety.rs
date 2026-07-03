use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use haze_sync_common::{AdapterId, ConflictId, ContentHash, OperationId, RevisionId, VaultPath};
use haze_sync_core::{
    conflict_saved_planner::require_upsert_conflict_saved_plan,
    conflict_service::{
        generate_conflict_path, ConflictPathRequest, ConflictPolicy, ConflictPolicyError,
        ConflictRecordStatus,
    },
    delete_guard::{
        DeleteGuard, DeleteGuardBlockReason, DeleteGuardDecision, DeleteGuardInput,
        DeleteGuardPolicy, DeleteRatioLimit, DeleteRunScope, ManualDeleteUnlock,
    },
    idempotency::{
        IdempotencyKey, IdempotencyReplayOutcome, IdempotencyScope, IdempotencyService,
        RequestFingerprint, StoredIdempotencyRecord, StoredIdempotencyResponse,
    },
    operation_log::{
        ChangeFeedEntry, ChangesPage, ChangesQuery, OperationKind as FeedOperationKind,
        OperationSequence, SyncOperation, TombstoneId as OperationLogTombstoneId,
    },
    revision_service::{
        compute_content_hash, AppendOperationRequest, ConflictPolicyHint, ConflictSavedOutcome,
        ContentStore, IncomingConflictContent, InsertRevisionRequest, OperationKind, OperationLog,
        OperationLogEntry, RevisionRepository, RevisionService, RevisionServiceError,
        StoredContent, StoredRevision, UpsertFileRequest, UpsertOutcome,
    },
    tombstone_service::{
        TombstoneCreationInput, TombstoneId, TombstoneRetention, TombstoneService,
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

fn operation_id(input: &str) -> OperationId {
    OperationId::parse(input).expect("fixture operation id should parse")
}

fn conflict_id(input: &str) -> ConflictId {
    ConflictId::parse(input).expect("fixture conflict id should parse")
}

fn path(input: &str) -> VaultPath {
    VaultPath::parse(input).expect("fixture vault path should parse")
}

fn timestamp(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .expect("fixture timestamp should parse")
        .with_timezone(&Utc)
}

fn current_revision(bytes: &[u8]) -> StoredRevision {
    StoredRevision {
        revision_id: revision_id("rev_current"),
        path: path("Projects/Haze/plan.md"),
        parent_revision_id: None,
        content_hash: compute_content_hash(bytes),
        size_bytes: bytes.len() as u64,
        created_by: adapter_id(),
    }
}

fn upsert_request(base_revision_id: Option<RevisionId>, bytes: &[u8]) -> UpsertFileRequest {
    UpsertFileRequest::new(
        path("Projects/Haze/plan.md"),
        adapter_id(),
        base_revision_id,
        compute_content_hash(bytes),
        bytes.to_vec(),
    )
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
            path: path("Projects/Haze/plan.md"),
            adapter_id: adapter_id(),
            content_hash: compute_content_hash(bytes),
            size_bytes: bytes.len() as u64,
            content: bytes.to_vec(),
        },
        policy_hint: ConflictPolicyHint::PreserveBoth,
    })
}

fn run_upsert_against_current(base_revision_id: Option<RevisionId>, bytes: &[u8]) -> UpsertOutcome {
    let current = current_revision(b"current content");
    let mut service = RevisionService::new(
        FakeRevisionRepository::with_current(current),
        FakeContentStore::default(),
        FakeOperationLog::default(),
    );

    service
        .upsert_file(upsert_request(base_revision_id, bytes))
        .expect("fake-backed service should not fail")
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

#[test]
fn stale_base_different_content_creates_conflict_saved() {
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

    assert_eq!(outcome.public_status(), "conflict_saved");
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

    let plan = require_upsert_conflict_saved_plan(&outcome, timestamp("2026-07-01T12:00:00Z"))
        .expect("conflict_saved should produce preservation plan");
    assert_eq!(plan.original_path.as_str(), "Projects/Haze/plan.md");
    assert_eq!(
        plan.provided_base_revision_id,
        Some(revision_id("rev_stale"))
    );
    assert_eq!(plan.policy_applied, ConflictPolicy::PreserveBoth);
    assert_eq!(plan.status, ConflictRecordStatus::Open);
    assert!(plan
        .materialized_path
        .as_str()
        .starts_with("_haze_conflicts/open/Projects/Haze/plan.conflict.iphone-anna."));

    let (repository, content_store, operation_log) = service.into_inner();
    assert_eq!(repository.current, Some(current));
    assert!(repository.inserted.is_empty());
    assert!(content_store.writes.is_empty());
    assert!(operation_log.entries.is_empty());
}

#[test]
fn unknown_base_different_content_creates_conflict_saved() {
    let outcome =
        run_upsert_against_current(Some(revision_id("rev_unknown_remote")), b"remote edit");

    let conflict_saved = outcome
        .conflict_saved()
        .expect("unknown base different content should save conflict");
    assert_eq!(outcome.public_status(), "conflict_saved");
    assert_eq!(
        conflict_saved.provided_base_revision_id,
        Some(revision_id("rev_unknown_remote"))
    );
    assert_eq!(
        conflict_saved.incoming_content.content_hash,
        compute_content_hash(b"remote edit")
    );
    assert_eq!(conflict_saved.policy_hint, ConflictPolicyHint::PreserveBoth);
}

#[test]
fn null_base_existing_different_content_creates_conflict_saved() {
    let current = current_revision(b"current content");
    let mut service = RevisionService::new(
        FakeRevisionRepository::with_current(current.clone()),
        FakeContentStore::default(),
        FakeOperationLog::default(),
    );

    let outcome = service
        .upsert_file(upsert_request(None, b"incoming content"))
        .expect("fake-backed service should not fail");

    assert_eq!(outcome.public_status(), "conflict_saved");
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
fn same_content_stale_unknown_and_null_base_are_ignored() {
    for base_revision_id in [
        Some(revision_id("rev_stale")),
        Some(revision_id("rev_unknown_remote")),
        None,
    ] {
        let current = current_revision(b"current content");
        let mut service = RevisionService::new(
            FakeRevisionRepository::with_current(current.clone()),
            FakeContentStore::default(),
            FakeOperationLog::default(),
        );

        let outcome = service
            .upsert_file(upsert_request(base_revision_id, b"current content"))
            .expect("fake-backed service should not fail");

        assert_eq!(outcome.public_status(), "same_content");
        assert_eq!(
            outcome,
            UpsertOutcome::IgnoredDuplicateSameContent {
                current_revision: current.clone()
            }
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
    let request = ConflictPathRequest::parse(
        "Projects/Haze/plan.md",
        "iphone-anna",
        timestamp("2026-07-01T12:00:00Z"),
    )
    .expect("conflict path request should parse");

    let conflict_path = generate_conflict_path(&request).expect("path generation should succeed");

    assert_eq!(
        conflict_path.as_str(),
        "_haze_conflicts/open/Projects/Haze/plan.conflict.iphone-anna.2026-07-01-120000.md"
    );
    assert!(conflict_path.as_str().starts_with("_haze_conflicts/open/"));
    assert!(!conflict_path
        .as_str()
        .contains("/_haze_conflicts/open/_haze_conflicts/"));
}

#[test]
fn recursive_haze_conflicts_source_path_is_rejected() {
    let err = ConflictPathRequest::parse(
        "_haze_conflicts/open/Projects/Haze/plan.md",
        "iphone-anna",
        timestamp("2026-07-01T12:00:00Z"),
    )
    .expect_err("recursive conflict source paths must be rejected");

    assert_eq!(err, ConflictPolicyError::RecursiveConflictPath);
    assert_eq!(err.code(), "recursive_conflict_path");
}

#[test]
fn delete_creates_tombstone_and_retains_revision_history() {
    let created_at = timestamp("2026-07-01T00:00:00Z");
    let retention_until = timestamp("2026-08-01T00:00:00Z");
    let input = TombstoneCreationInput::new(
        TombstoneId::parse("tmb_01JW3DELETE").expect("fixture tombstone id should parse"),
        path("Projects/Haze/old.md"),
        revision_id("rev_deleted"),
        Some(revision_id("rev_current")),
        adapter_id(),
        TombstoneRetention::new(retention_until, Some(30)),
        Some(created_at),
    );

    let tombstone = TombstoneService::new()
        .create_tombstone(input)
        .expect("tombstone metadata should be created");

    assert_eq!(tombstone.path.as_str(), "Projects/Haze/old.md");
    assert_eq!(tombstone.deleted_revision_id().as_str(), "rev_deleted");
    assert_eq!(tombstone.current_revision_id().as_str(), "rev_current");
    assert_eq!(tombstone.retention.retention_until, retention_until);
    assert_eq!(tombstone.restore.restored_at, None);
}

#[test]
fn mass_delete_is_blocked_by_count() {
    let guard = DeleteGuard::new(DeleteGuardPolicy::new(
        2,
        DeleteRatioLimit::percent(100).expect("ratio should parse"),
        false,
    ));
    let input = DeleteGuardInput::without_manual_unlock(delete_scope("scan-count"), 3, 100);

    assert_eq!(
        guard.evaluate(&input),
        DeleteGuardDecision::BlockedTooManyDeletes {
            proposed_delete_count: 3,
            max_deletes_per_run: 2,
        }
    );
}

#[test]
fn mass_delete_is_blocked_by_ratio() {
    let guard = DeleteGuard::new(DeleteGuardPolicy::new(
        20,
        DeleteRatioLimit::percent(5).expect("ratio should parse"),
        false,
    ));
    let input = DeleteGuardInput::without_manual_unlock(delete_scope("scan-ratio"), 6, 100);

    assert_eq!(
        guard.evaluate(&input),
        DeleteGuardDecision::BlockedDeleteRatio {
            proposed_delete_count: 6,
            total_files_before_run: 100,
            max_delete_ratio_per_run: DeleteRatioLimit::percent(5).expect("ratio should parse"),
        }
    );
}

#[test]
fn manual_unlock_allows_explicitly_authorized_delete_batch() {
    let guard = DeleteGuard::new(DeleteGuardPolicy::new(
        2,
        DeleteRatioLimit::percent(5).expect("ratio should parse"),
        true,
    ));
    let scope = delete_scope("scan-unlocked");
    let locked_input = DeleteGuardInput::without_manual_unlock(scope.clone(), 6, 100);

    assert_eq!(
        guard.evaluate(&locked_input),
        DeleteGuardDecision::BlockedRequiresManualUnlock {
            proposed_delete_count: 6,
            total_files_before_run: 100,
            reason: DeleteGuardBlockReason::TooManyDeletes,
        }
    );

    let unlocked_input = locked_input.with_manual_unlock(ManualDeleteUnlock::scoped_for_all(scope));
    assert_eq!(
        guard.evaluate(&unlocked_input),
        DeleteGuardDecision::Allowed
    );
}

#[test]
fn delete_never_hard_deletes_content_in_core() {
    let tombstone = TombstoneService::new()
        .create_tombstone(TombstoneCreationInput::new(
            TombstoneId::parse("tmb_01JNOHARDDELETE").expect("fixture tombstone id should parse"),
            path("Projects/Haze/old.md"),
            revision_id("rev_deleted"),
            Some(revision_id("rev_current")),
            adapter_id(),
            TombstoneRetention::new(timestamp("2026-08-01T00:00:00Z"), Some(30)),
            Some(timestamp("2026-07-01T00:00:00Z")),
        ))
        .expect("tombstone metadata should be created");

    let serialized = serde_json::to_string(&tombstone).expect("tombstone should serialize");
    assert!(serialized.contains("tmb_01JNOHARDDELETE"));
    assert!(serialized.contains("retention_until"));
    assert!(!serialized.contains("hard_delete"));
    assert!(!serialized.contains("file_bytes"));
    assert!(!serialized.contains("content"));
}

#[test]
fn changes_feed_includes_w3_delete_and_conflict_entries() {
    let query = ChangesQuery::new(100, 10).expect("changes query should parse");
    let page = ChangesPage::new(
        query,
        vec![
            change_entry(
                101,
                FeedOperationKind::DeleteFile,
                Some(revision_id("rev_deleted")),
                Some(OperationLogTombstoneId::parse("tmb_01JW3DELETE").unwrap()),
                None,
            ),
            change_entry(
                102,
                FeedOperationKind::ConflictResolved,
                None,
                None,
                Some(conflict_id("conf_01JW3")),
            ),
        ],
        false,
    )
    .expect("W3 changes page should validate");

    assert_eq!(page.from_seq.value(), 100);
    assert_eq!(page.to_seq.value(), 102);
    assert_eq!(
        page.changes[0].operation.kind,
        FeedOperationKind::DeleteFile
    );
    assert_eq!(
        page.changes[1].operation.kind,
        FeedOperationKind::ConflictResolved
    );
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

#[test]
fn w2_put_current_base_still_accepts_new_revision_and_logs_upsert() {
    let current = current_revision(b"current content");
    let mut service = RevisionService::new(
        FakeRevisionRepository::with_current(current.clone()),
        FakeContentStore::default(),
        FakeOperationLog::default(),
    );

    let outcome = service
        .upsert_file(upsert_request(
            Some(current.revision_id.clone()),
            b"next content",
        ))
        .expect("fake-backed service should not fail");

    let UpsertOutcome::AcceptedNewRevision {
        revision,
        operation,
    } = outcome
    else {
        panic!("current-base write should remain accepted");
    };

    assert_eq!(revision.parent_revision_id, Some(current.revision_id));
    assert_eq!(operation.seq, 1);
    let (_repository, content_store, operation_log) = service.into_inner();
    assert_eq!(content_store.writes.len(), 1);
    assert_eq!(operation_log.entries[0].kind, OperationKind::UpsertFile);
}

fn delete_scope(run_id: &str) -> DeleteRunScope {
    DeleteRunScope::new(adapter_id(), run_id).expect("fixture delete scope should parse")
}

fn change_entry(
    seq: i64,
    kind: FeedOperationKind,
    revision_id: Option<RevisionId>,
    tombstone_id: Option<OperationLogTombstoneId>,
    conflict_id: Option<ConflictId>,
) -> ChangeFeedEntry {
    ChangeFeedEntry {
        operation: SyncOperation {
            seq: OperationSequence::new(seq).expect("fixture sequence should parse"),
            op_id: operation_id(&format!("op_{seq}")),
            adapter_id: adapter_id(),
            kind,
            path: path("Projects/Haze/plan.md"),
            revision_id,
            tombstone_id,
            conflict_id,
            created_at: timestamp("2026-07-01T12:00:00Z"),
        },
        content_sha256: None,
        size_bytes: None,
    }
}
