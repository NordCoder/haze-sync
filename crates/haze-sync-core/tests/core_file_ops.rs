//! W2-P7 fake-backed integration tests for normal Core file operations.
//!
//! These tests exercise the pure W2-P4 `RevisionService` boundary with local
//! fakes only. They intentionally do not create database pools, run migrations,
//! call providers, wire routes, or claim end-to-end product behavior before
//! W2-F1 provides the concrete service wiring.

use chrono::{TimeZone, Utc};
use haze_sync_common::{AdapterId, ContentHash, OperationId, RevisionId, VaultPath};
use haze_sync_core::operation_log::{
    ChangeFeedEntry, ChangesPage, ChangesQuery, OperationKind as FeedOperationKind,
    OperationSequence, SyncOperation,
};
use haze_sync_core::revision_service::{
    compute_content_hash, AppendOperationRequest, ContentStore, InsertRevisionRequest,
    OperationKind as RevisionOperationKind, OperationLog, OperationLogEntry, RevisionRepository,
    RevisionService, RevisionServiceError, StoredContent, StoredRevision, UpsertFileRequest,
    UpsertOutcome,
};
use std::collections::HashMap;
use std::sync::{Arc, Barrier, Mutex, MutexGuard};
use std::thread;

#[test]
fn upload_then_downloads_same_bytes_and_verifies_hash_with_fake_store() {
    let harness = SharedHarness::default();
    let bytes = b"hello haze sync";
    let expected_hash = compute_content_hash(bytes);

    let outcome = harness
        .upsert(upsert_request("Notes/a.md", bytes, None))
        .expect("fake upsert should succeed");

    let UpsertOutcome::AcceptedNewFile {
        revision,
        operation,
    } = outcome
    else {
        panic!("expected accepted new file outcome");
    };

    assert_eq!(revision.path.as_str(), "Notes/a.md");
    assert_eq!(revision.content_hash, expected_hash);
    assert_eq!(revision.size_bytes, bytes.len() as u64);
    assert_eq!(operation.seq, 1);
    assert_eq!(harness.download(&revision.content_hash), Some(bytes.to_vec()));
}

#[test]
fn uploads_two_revisions_and_tracks_parent_and_current_revision_with_fakes() {
    let harness = SharedHarness::default();

    let first = accepted_new_file_revision(
        harness
            .upsert(upsert_request("Notes/two-revisions.md", b"first", None))
            .expect("first fake upsert should succeed"),
    );
    let second = accepted_new_revision(
        harness
            .upsert(upsert_request(
                "Notes/two-revisions.md",
                b"second",
                Some(first.revision_id.clone()),
            ))
            .expect("second fake upsert should succeed"),
    );

    assert_eq!(second.parent_revision_id, Some(first.revision_id.clone()));
    assert_eq!(harness.current_revision("Notes/two-revisions.md"), Some(second));
    assert_eq!(harness.revision_insert_count(), 2);
    assert_eq!(harness.operation_count(), 2);
}

#[test]
fn changes_feed_includes_upsert_operations_from_fake_log() {
    let harness = SharedHarness::default();

    let first = accepted_new_file_revision(
        harness
            .upsert(upsert_request("Notes/changes.md", b"first", None))
            .expect("first fake upsert should succeed"),
    );
    let second = accepted_new_revision(
        harness
            .upsert(upsert_request(
                "Notes/changes.md",
                b"second",
                Some(first.revision_id.clone()),
            ))
            .expect("second fake upsert should succeed"),
    );

    let page = harness.changes_page_since_zero();

    assert_eq!(page.from_seq, OperationSequence::ZERO);
    assert_eq!(page.to_seq, OperationSequence::new(2).unwrap());
    assert!(!page.has_more);
    assert_eq!(page.changes.len(), 2);
    assert!(page
        .changes
        .iter()
        .all(|change| change.operation.kind == FeedOperationKind::UpsertFile));
    assert_eq!(page.changes[0].operation.revision_id, Some(first.revision_id));
    assert_eq!(page.changes[1].operation.revision_id, Some(second.revision_id));
}

#[test]
fn same_content_duplicate_is_ignored_without_new_revision_or_operation() {
    let harness = SharedHarness::default();

    let first = accepted_new_file_revision(
        harness
            .upsert(upsert_request("Notes/duplicate.md", b"same", None))
            .expect("first fake upsert should succeed"),
    );

    let duplicate = harness
        .upsert(upsert_request("Notes/duplicate.md", b"same", None))
        .expect("duplicate fake upsert should succeed");

    let UpsertOutcome::IgnoredDuplicateSameContent { current_revision } = duplicate else {
        panic!("expected duplicate same-content upload to be ignored");
    };

    assert_eq!(current_revision, first);
    assert_eq!(harness.revision_insert_count(), 1);
    assert_eq!(harness.operation_count(), 1);
    assert_eq!(harness.content_put_count(), 1);
}

#[test]
fn same_path_concurrent_writes_are_serialized_by_fake_harness_without_state_corruption() {
    let harness = SharedHarness::default();
    let barrier = Arc::new(Barrier::new(2));

    let left_harness = harness.clone();
    let left_barrier = Arc::clone(&barrier);
    let left = thread::spawn(move || {
        left_barrier.wait();
        left_harness.upsert(upsert_request("Notes/concurrent.md", b"left", None))
    });

    let right_harness = harness.clone();
    let right_barrier = Arc::clone(&barrier);
    let right = thread::spawn(move || {
        right_barrier.wait();
        right_harness.upsert(upsert_request("Notes/concurrent.md", b"right", None))
    });

    let outcomes = [
        left.join()
            .expect("left worker should not panic")
            .expect("left fake upsert should not error"),
        right
            .join()
            .expect("right worker should not panic")
            .expect("right fake upsert should not error"),
    ];

    assert_eq!(outcomes.iter().filter(|outcome| is_accepted_new_file(outcome)).count(), 1);
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| is_rejected_stale_or_unknown_base(outcome))
            .count(),
        1
    );
    assert_eq!(harness.revision_insert_count(), 1);
    assert_eq!(harness.operation_count(), 1);
    assert_eq!(harness.stored_blob_count(), 1);

    let current = harness
        .current_revision("Notes/concurrent.md")
        .expect("one current revision should exist");
    let current_bytes = harness
        .download(&current.content_hash)
        .expect("current fake blob should exist");
    assert!(current_bytes == b"left" || current_bytes == b"right");
}

#[test]
#[ignore = "requires W2-F1 real service wiring and a safe opt-in test database"]
fn real_db_upload_then_download_same_bytes_and_verify_hash_is_w2f1_spec() {
    skip_real_db_spec_until_w2f1("upload_then_download_same_bytes_and_verify_hash");
}

#[test]
#[ignore = "requires W2-F1 real service wiring and a safe opt-in test database"]
fn real_db_upload_two_revisions_is_w2f1_spec() {
    skip_real_db_spec_until_w2f1("upload_two_revisions");
}

#[test]
#[ignore = "requires W2-F1 real service wiring and a safe opt-in test database"]
fn real_db_changes_feed_includes_upsert_operations_is_w2f1_spec() {
    skip_real_db_spec_until_w2f1("changes_feed_includes_upsert_operations");
}

#[test]
#[ignore = "requires W2-F1 real service wiring and a safe opt-in test database"]
fn real_db_same_content_duplicate_is_ignored_is_w2f1_spec() {
    skip_real_db_spec_until_w2f1("same_content_duplicate_is_ignored");
}

#[test]
#[ignore = "requires W2-F1 real service wiring and a safe opt-in test database"]
fn real_db_concurrent_writes_same_path_do_not_corrupt_db_is_w2f1_spec() {
    skip_real_db_spec_until_w2f1("concurrent_writes_same_path_do_not_corrupt_db");
}

#[derive(Clone, Default)]
struct SharedHarness {
    state: Arc<Mutex<FakeState>>,
    upsert_lock: Arc<Mutex<()>>,
}

impl SharedHarness {
    fn upsert(&self, request: UpsertFileRequest) -> Result<UpsertOutcome, RevisionServiceError> {
        let _guard = self
            .upsert_lock
            .lock()
            .expect("fake per-path lock should not be poisoned");
        let mut service = RevisionService::new(
            FakeRepository::new(Arc::clone(&self.state)),
            FakeContentStore::new(Arc::clone(&self.state)),
            FakeOperationLog::new(Arc::clone(&self.state)),
        );

        service.upsert_file(request)
    }

    fn download(&self, hash: &ContentHash) -> Option<Vec<u8>> {
        self.lock_state().content_by_hash.get(hash).cloned()
    }

    fn current_revision(&self, path: &str) -> Option<StoredRevision> {
        let state = self.lock_state();
        let revision_id = state.current_revision_by_path.get(path)?;
        state.revisions_by_id.get(revision_id.as_str()).cloned()
    }

    fn changes_page_since_zero(&self) -> ChangesPage {
        let state = self.lock_state();
        let changes = state
            .operations
            .iter()
            .map(|logged| {
                assert_eq!(logged.kind, RevisionOperationKind::UpsertFile);
                let revision = state
                    .revisions_by_id
                    .get(logged.revision_id.as_str())
                    .expect("logged revision should exist in fake repository");

                ChangeFeedEntry {
                    operation: SyncOperation {
                        seq: OperationSequence::new(logged.seq).unwrap(),
                        op_id: logged.op_id.clone(),
                        adapter_id: logged.adapter_id.clone(),
                        kind: FeedOperationKind::UpsertFile,
                        path: logged.path.clone(),
                        revision_id: Some(logged.revision_id.clone()),
                        tombstone_id: None,
                        conflict_id: None,
                        created_at: Utc.timestamp_opt(logged.seq, 0).unwrap(),
                    },
                    content_sha256: Some(revision.content_hash),
                    size_bytes: Some(revision.size_bytes),
                }
            })
            .collect();

        ChangesPage::new(
            ChangesQuery::new(0, 100).expect("query should be valid"),
            changes,
            false,
        )
        .expect("fake changes should be strictly ordered")
    }

    fn revision_insert_count(&self) -> usize {
        self.lock_state().revision_insert_calls
    }

    fn operation_count(&self) -> usize {
        self.lock_state().operations.len()
    }

    fn content_put_count(&self) -> usize {
        self.lock_state().content_put_calls
    }

    fn stored_blob_count(&self) -> usize {
        self.lock_state().content_by_hash.len()
    }

    fn lock_state(&self) -> MutexGuard<'_, FakeState> {
        self.state
            .lock()
            .expect("fake shared state should not be poisoned")
    }
}

#[derive(Default)]
struct FakeState {
    current_revision_by_path: HashMap<String, RevisionId>,
    revisions_by_id: HashMap<String, StoredRevision>,
    content_by_hash: HashMap<ContentHash, Vec<u8>>,
    operations: Vec<LoggedOperation>,
    next_revision_number: u64,
    next_operation_seq: i64,
    content_put_calls: usize,
    revision_insert_calls: usize,
}

#[derive(Clone, Debug)]
struct LoggedOperation {
    seq: i64,
    op_id: OperationId,
    adapter_id: AdapterId,
    kind: RevisionOperationKind,
    path: VaultPath,
    revision_id: RevisionId,
}

#[derive(Clone)]
struct FakeRepository {
    state: Arc<Mutex<FakeState>>,
}

impl FakeRepository {
    fn new(state: Arc<Mutex<FakeState>>) -> Self {
        Self { state }
    }
}

impl RevisionRepository for FakeRepository {
    fn current_revision(
        &mut self,
        path: &VaultPath,
    ) -> Result<Option<StoredRevision>, RevisionServiceError> {
        let state = self
            .state
            .lock()
            .expect("fake repository state should not be poisoned");
        let Some(revision_id) = state.current_revision_by_path.get(path.as_str()) else {
            return Ok(None);
        };

        Ok(state.revisions_by_id.get(revision_id.as_str()).cloned())
    }

    fn insert_revision(
        &mut self,
        request: InsertRevisionRequest,
    ) -> Result<StoredRevision, RevisionServiceError> {
        let mut state = self
            .state
            .lock()
            .expect("fake repository state should not be poisoned");
        state.next_revision_number += 1;
        state.revision_insert_calls += 1;

        let revision_id = RevisionId::parse(&format!("rev_TEST{}", state.next_revision_number))
            .expect("generated fake revision id should be valid");
        let revision = StoredRevision {
            revision_id: revision_id.clone(),
            path: request.path.clone(),
            parent_revision_id: request.parent_revision_id,
            content_hash: request.content_hash,
            size_bytes: request.size_bytes,
            created_by: request.adapter_id,
        };

        state
            .current_revision_by_path
            .insert(request.path.as_str().to_owned(), revision_id.clone());
        state
            .revisions_by_id
            .insert(revision_id.as_str().to_owned(), revision.clone());

        Ok(revision)
    }
}

#[derive(Clone)]
struct FakeContentStore {
    state: Arc<Mutex<FakeState>>,
}

impl FakeContentStore {
    fn new(state: Arc<Mutex<FakeState>>) -> Self {
        Self { state }
    }
}

impl ContentStore for FakeContentStore {
    fn put_content(
        &mut self,
        expected_hash: ContentHash,
        bytes: &[u8],
    ) -> Result<StoredContent, RevisionServiceError> {
        let mut state = self
            .state
            .lock()
            .expect("fake content state should not be poisoned");
        state.content_put_calls += 1;
        state
            .content_by_hash
            .entry(expected_hash)
            .or_insert_with(|| bytes.to_vec());

        Ok(StoredContent {
            hash: expected_hash,
            size_bytes: bytes.len() as u64,
        })
    }
}

#[derive(Clone)]
struct FakeOperationLog {
    state: Arc<Mutex<FakeState>>,
}

impl FakeOperationLog {
    fn new(state: Arc<Mutex<FakeState>>) -> Self {
        Self { state }
    }
}

impl OperationLog for FakeOperationLog {
    fn append_operation(
        &mut self,
        request: AppendOperationRequest,
    ) -> Result<OperationLogEntry, RevisionServiceError> {
        let mut state = self
            .state
            .lock()
            .expect("fake operation log state should not be poisoned");
        state.next_operation_seq += 1;
        let seq = state.next_operation_seq;
        let op_id = OperationId::parse(&format!("op_TEST{seq}"))
            .expect("generated fake operation id should be valid");

        state.operations.push(LoggedOperation {
            seq,
            op_id: op_id.clone(),
            adapter_id: request.adapter_id,
            kind: request.kind,
            path: request.path,
            revision_id: request.revision_id,
        });

        Ok(OperationLogEntry {
            operation_id: op_id,
            seq,
        })
    }
}

fn upsert_request(
    path: &str,
    bytes: &[u8],
    base_revision_id: Option<RevisionId>,
) -> UpsertFileRequest {
    let content = bytes.to_vec();
    let expected_hash = compute_content_hash(&content);

    UpsertFileRequest::new(
        VaultPath::parse(path).expect("test path should be valid"),
        AdapterId::parse("test-adapter").expect("test adapter id should be valid"),
        base_revision_id,
        expected_hash,
        content,
    )
}

fn accepted_new_file_revision(outcome: UpsertOutcome) -> StoredRevision {
    match outcome {
        UpsertOutcome::AcceptedNewFile { revision, .. } => revision,
        other => panic!("expected accepted new file outcome, got {other:?}"),
    }
}

fn accepted_new_revision(outcome: UpsertOutcome) -> StoredRevision {
    match outcome {
        UpsertOutcome::AcceptedNewRevision { revision, .. } => revision,
        other => panic!("expected accepted new revision outcome, got {other:?}"),
    }
}

fn is_accepted_new_file(outcome: &UpsertOutcome) -> bool {
    matches!(outcome, UpsertOutcome::AcceptedNewFile { .. })
}

fn is_rejected_stale_or_unknown_base(outcome: &UpsertOutcome) -> bool {
    matches!(outcome, UpsertOutcome::RejectedStaleOrUnknownBase { .. })
}

fn skip_real_db_spec_until_w2f1(scenario: &str) {
    let db_configured = std::env::var_os("HAZE_SYNC_TEST_DATABASE_URL").is_some()
        || std::env::var_os("DATABASE_URL").is_some();
    let w2f1_enabled = std::env::var_os("HAZE_SYNC_ENABLE_W2F1_CORE_FILEOPS_TESTS").is_some();

    assert!(
        !w2f1_enabled,
        "real DB core file ops scenario {scenario} is documented but not executable before W2-F1 wires Core services; db_configured={db_configured}"
    );

    eprintln!(
        "skipping real DB core file ops scenario {scenario}; db_configured={db_configured}; W2-F1 service wiring is not available yet"
    );
}
