use haze_sync_common::{AdapterId, ContentHash, OperationId, RevisionId, VaultPath};
use haze_sync_core::revision_service::{
    compute_content_hash, AppendOperationRequest, ConflictPolicyHint, ContentStore,
    InsertRevisionRequest, OperationLog, OperationLogEntry, RevisionRepository, RevisionService,
    RevisionServiceError, StoredContent, StoredRevision, UpsertFileRequest, UpsertOutcome,
};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Default)]
struct Events(Rc<RefCell<Vec<&'static str>>>);

impl Events {
    fn push(&self, event: &'static str) {
        self.0.borrow_mut().push(event);
    }

    fn snapshot(&self) -> Vec<&'static str> {
        self.0.borrow().clone()
    }
}

#[derive(Default)]
struct FakeRevisionRepository {
    current: Option<StoredRevision>,
    inserted: Vec<InsertRevisionRequest>,
    current_reads: usize,
    next_revision_number: usize,
    events: Events,
}

impl FakeRevisionRepository {
    fn with_current(current: StoredRevision) -> Self {
        Self {
            current: Some(current),
            ..Self::default()
        }
    }

    fn with_events(mut self, events: Events) -> Self {
        self.events = events;
        self
    }
}

impl RevisionRepository for FakeRevisionRepository {
    fn current_revision(
        &mut self,
        _path: &VaultPath,
    ) -> Result<Option<StoredRevision>, RevisionServiceError> {
        self.current_reads += 1;
        Ok(self.current.clone())
    }

    fn insert_revision(
        &mut self,
        request: InsertRevisionRequest,
    ) -> Result<StoredRevision, RevisionServiceError> {
        self.events.push("insert_revision");
        self.inserted.push(request.clone());
        self.next_revision_number += 1;

        let revision = StoredRevision {
            revision_id: RevisionId::parse(&format!("rev_{:04}", self.next_revision_number))
                .unwrap(),
            path: request.path,
            parent_revision_id: request.parent_revision_id,
            content_hash: request.content_hash,
            size_bytes: request.size_bytes,
            created_by: request.adapter_id,
        };
        self.current = Some(revision.clone());
        Ok(revision)
    }
}

#[derive(Default)]
struct FakeContentStore {
    put_calls: Vec<(ContentHash, Vec<u8>)>,
    events: Events,
}

impl FakeContentStore {
    fn with_events(mut self, events: Events) -> Self {
        self.events = events;
        self
    }
}

impl ContentStore for FakeContentStore {
    fn put_content(
        &mut self,
        expected_hash: ContentHash,
        bytes: &[u8],
    ) -> Result<StoredContent, RevisionServiceError> {
        self.events.push("put_content");
        self.put_calls.push((expected_hash, bytes.to_vec()));
        Ok(StoredContent {
            hash: expected_hash,
            size_bytes: bytes.len() as u64,
        })
    }
}

#[derive(Default)]
struct FakeOperationLog {
    appended: Vec<AppendOperationRequest>,
    events: Events,
}

impl FakeOperationLog {
    fn with_events(mut self, events: Events) -> Self {
        self.events = events;
        self
    }
}

impl OperationLog for FakeOperationLog {
    fn append_operation(
        &mut self,
        request: AppendOperationRequest,
    ) -> Result<OperationLogEntry, RevisionServiceError> {
        self.events.push("append_operation");
        self.appended.push(request);
        let seq = i64::try_from(self.appended.len()).unwrap();
        Ok(OperationLogEntry {
            operation_id: OperationId::parse(&format!("op_{seq:04}")).unwrap(),
            seq,
        })
    }
}

fn path() -> VaultPath {
    VaultPath::parse("Notes/a.md").unwrap()
}

fn adapter_id() -> AdapterId {
    AdapterId::parse("iphone-anna").unwrap()
}

fn stored_revision(revision_id: &str, bytes: &[u8]) -> StoredRevision {
    StoredRevision {
        revision_id: RevisionId::parse(revision_id).unwrap(),
        path: path(),
        parent_revision_id: None,
        content_hash: compute_content_hash(bytes),
        size_bytes: bytes.len() as u64,
        created_by: adapter_id(),
    }
}

fn request(base_revision_id: Option<RevisionId>, bytes: &[u8]) -> UpsertFileRequest {
    UpsertFileRequest::new(
        path(),
        adapter_id(),
        base_revision_id,
        compute_content_hash(bytes),
        bytes.to_vec(),
    )
}

fn service(
    repository: FakeRevisionRepository,
    content_store: FakeContentStore,
    operation_log: FakeOperationLog,
) -> RevisionService<FakeRevisionRepository, FakeContentStore, FakeOperationLog> {
    RevisionService::new(repository, content_store, operation_log)
}

fn assert_conflict_saved(
    outcome: &UpsertOutcome,
    current: &StoredRevision,
    expected_base: Option<&RevisionId>,
    incoming_bytes: &[u8],
) {
    assert_eq!(outcome.public_status(), "conflict_saved");

    let UpsertOutcome::RejectedStaleOrUnknownBase {
        current_revision,
        provided_base_revision_id,
        conflict_saved: Some(conflict_saved),
    } = outcome
    else {
        panic!("unexpected outcome: {outcome:?}");
    };

    assert_eq!(current_revision.as_ref(), Some(current));
    assert_eq!(provided_base_revision_id.as_ref(), expected_base);
    assert_eq!(outcome.conflict_saved(), Some(conflict_saved.as_ref()));
    assert_eq!(&conflict_saved.current_revision, current);
    assert_eq!(
        conflict_saved.provided_base_revision_id.as_ref(),
        expected_base
    );
    assert_eq!(conflict_saved.incoming_content.path, path());
    assert_eq!(conflict_saved.incoming_content.adapter_id, adapter_id());
    assert_eq!(
        conflict_saved.incoming_content.content_hash,
        compute_content_hash(incoming_bytes)
    );
    assert_eq!(
        conflict_saved.incoming_content.size_bytes,
        incoming_bytes.len() as u64
    );
    assert_eq!(
        conflict_saved.incoming_content.content.as_slice(),
        incoming_bytes
    );
    assert_eq!(conflict_saved.policy_hint, ConflictPolicyHint::PreserveBoth);
}

#[test]
fn new_file_with_null_base_is_accepted() {
    let events = Events::default();
    let mut service = service(
        FakeRevisionRepository::default().with_events(events.clone()),
        FakeContentStore::default().with_events(events.clone()),
        FakeOperationLog::default().with_events(events.clone()),
    );

    let outcome = service.upsert_file(request(None, b"hello")).unwrap();

    let revision = match outcome {
        UpsertOutcome::AcceptedNewFile {
            revision,
            operation,
        } => {
            assert_eq!(operation.seq, 1);
            revision
        }
        other => panic!("unexpected outcome: {other:?}"),
    };
    assert_eq!(revision.parent_revision_id, None);
    assert_eq!(revision.content_hash, compute_content_hash(b"hello"));
    assert_eq!(revision.size_bytes, 5);

    let (repository, content_store, operation_log) = service.into_inner();
    assert_eq!(repository.inserted.len(), 1);
    assert_eq!(content_store.put_calls.len(), 1);
    assert_eq!(operation_log.appended.len(), 1);
    assert_eq!(
        events.snapshot(),
        ["put_content", "insert_revision", "append_operation"]
    );
}

#[test]
fn current_base_is_accepted_as_new_revision() {
    let current = stored_revision("rev_0001", b"old");
    let mut service = service(
        FakeRevisionRepository::with_current(current.clone()),
        FakeContentStore::default(),
        FakeOperationLog::default(),
    );

    let outcome = service
        .upsert_file(request(Some(current.revision_id.clone()), b"new"))
        .unwrap();

    let revision = match outcome {
        UpsertOutcome::AcceptedNewRevision {
            revision,
            operation,
        } => {
            assert_eq!(operation.seq, 1);
            revision
        }
        other => panic!("unexpected outcome: {other:?}"),
    };
    assert_eq!(revision.parent_revision_id, Some(current.revision_id));
    assert_eq!(revision.content_hash, compute_content_hash(b"new"));

    let (repository, content_store, operation_log) = service.into_inner();
    assert_eq!(repository.inserted.len(), 1);
    assert_eq!(content_store.put_calls.len(), 1);
    assert_eq!(operation_log.appended.len(), 1);
}

#[test]
fn same_content_duplicate_is_ignored_without_persistence() {
    let current = stored_revision("rev_0001", b"same");
    let mut service = service(
        FakeRevisionRepository::with_current(current.clone()),
        FakeContentStore::default(),
        FakeOperationLog::default(),
    );

    let outcome = service.upsert_file(request(None, b"same")).unwrap();

    assert_eq!(
        outcome,
        UpsertOutcome::IgnoredDuplicateSameContent {
            current_revision: current,
        }
    );

    let (repository, content_store, operation_log) = service.into_inner();
    assert!(repository.inserted.is_empty());
    assert!(content_store.put_calls.is_empty());
    assert!(operation_log.appended.is_empty());
}

#[test]
fn hash_mismatch_is_rejected_before_persistence() {
    let mut bad_request = request(None, b"actual");
    bad_request.expected_hash = compute_content_hash(b"expected");

    let mut service = service(
        FakeRevisionRepository::default(),
        FakeContentStore::default(),
        FakeOperationLog::default(),
    );

    let outcome = service.upsert_file(bad_request).unwrap();

    assert_eq!(
        outcome,
        UpsertOutcome::RejectedHashMismatch {
            expected: compute_content_hash(b"expected"),
            actual: compute_content_hash(b"actual"),
        }
    );

    let (repository, content_store, operation_log) = service.into_inner();
    assert_eq!(repository.current_reads, 0);
    assert!(repository.inserted.is_empty());
    assert!(content_store.put_calls.is_empty());
    assert!(operation_log.appended.is_empty());
}

#[test]
fn stale_or_unknown_base_does_not_overwrite_different_current_content() {
    let current = stored_revision("rev_0001", b"current");
    let unknown_base = RevisionId::parse("rev_unknown").unwrap();
    let mut service = service(
        FakeRevisionRepository::with_current(current.clone()),
        FakeContentStore::default(),
        FakeOperationLog::default(),
    );

    let outcome = service
        .upsert_file(request(Some(unknown_base.clone()), b"incoming"))
        .unwrap();

    assert_conflict_saved(&outcome, &current, Some(&unknown_base), b"incoming");

    let (repository, content_store, operation_log) = service.into_inner();
    assert_eq!(repository.current, Some(current));
    assert!(repository.inserted.is_empty());
    assert!(content_store.put_calls.is_empty());
    assert!(operation_log.appended.is_empty());
}

#[test]
fn null_base_for_existing_file_does_not_overwrite_different_content() {
    let current = stored_revision("rev_0001", b"current");
    let mut service = service(
        FakeRevisionRepository::with_current(current.clone()),
        FakeContentStore::default(),
        FakeOperationLog::default(),
    );

    let outcome = service.upsert_file(request(None, b"incoming")).unwrap();

    assert_conflict_saved(&outcome, &current, None, b"incoming");

    let (repository, content_store, operation_log) = service.into_inner();
    assert_eq!(repository.current, Some(current));
    assert!(repository.inserted.is_empty());
    assert!(content_store.put_calls.is_empty());
    assert!(operation_log.appended.is_empty());
}

#[test]
fn non_null_base_for_missing_file_is_rejected_as_unknown_base() {
    let unknown_base = RevisionId::parse("rev_unknown").unwrap();
    let mut service = service(
        FakeRevisionRepository::default(),
        FakeContentStore::default(),
        FakeOperationLog::default(),
    );

    let outcome = service
        .upsert_file(request(Some(unknown_base.clone()), b"incoming"))
        .unwrap();

    assert_eq!(
        outcome,
        UpsertOutcome::RejectedStaleOrUnknownBase {
            current_revision: None,
            provided_base_revision_id: Some(unknown_base),
            conflict_saved: None,
        }
    );
    assert_eq!(outcome.public_status(), "rejected_stale_or_unknown_base");
    assert!(outcome.conflict_saved().is_none());

    let (repository, content_store, operation_log) = service.into_inner();
    assert!(repository.inserted.is_empty());
    assert!(content_store.put_calls.is_empty());
    assert!(operation_log.appended.is_empty());
}
