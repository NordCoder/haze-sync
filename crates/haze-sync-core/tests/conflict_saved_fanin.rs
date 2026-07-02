use chrono::{DateTime, TimeZone, Utc};
use haze_sync_common::{AdapterId, ContentHash, OperationId, RevisionId, VaultPath};
use haze_sync_core::{
    conflict_saved_planner::{
        plan_upsert_conflict_saved, require_upsert_conflict_saved_plan,
        ConflictSavedPlanningError,
    },
    conflict_service::{ConflictPolicy, ConflictPolicyError, ConflictRecordStatus},
    revision_service::{
        compute_content_hash, AppendOperationRequest, ContentStore, InsertRevisionRequest,
        OperationLog, OperationLogEntry, RevisionRepository, RevisionService, RevisionServiceError,
        StoredContent, StoredRevision, UpsertFileRequest, UpsertOutcome,
    },
};

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
        let revision = StoredRevision {
            revision_id: revision_id("rev_inserted"),
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
        self.entries.push(request);
        Ok(OperationLogEntry {
            operation_id: OperationId::parse("op_inserted")
                .expect("fixture operation id should parse"),
            seq: 1,
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

fn run_upsert(
    path_input: &str,
    current_bytes: &[u8],
    base_revision_id: Option<RevisionId>,
    incoming_bytes: &[u8],
) -> (
    UpsertOutcome,
    FakeRevisionRepository,
    FakeContentStore,
    FakeOperationLog,
) {
    let current = current_revision_at(path_input, current_bytes);
    let mut service = RevisionService::new(
        FakeRevisionRepository::with_current(current),
        FakeContentStore::default(),
        FakeOperationLog::default(),
    );
    let outcome = service
        .upsert_file(upsert_request_at(path_input, base_revision_id, incoming_bytes))
        .expect("fake-backed service should not fail");
    let (repository, content_store, operation_log) = service.into_inner();

    (outcome, repository, content_store, operation_log)
}

fn assert_conflict_saved_plan(
    outcome: &UpsertOutcome,
    current: &StoredRevision,
    provided_base_revision_id: Option<RevisionId>,
    incoming_bytes: &[u8],
) {
    let plan = require_upsert_conflict_saved_plan(outcome, fixed_timestamp())
        .expect("conflict_saved outcome should produce a preservation plan");

    assert_eq!(outcome.public_status(), "conflict_saved");
    assert_eq!(plan.original_path, current.path.clone());
    assert_eq!(plan.provided_base_revision_id, provided_base_revision_id);
    assert_eq!(plan.current_revision_id, current.revision_id.clone());
    assert_eq!(plan.current_content_hash, current.content_hash);
    assert_eq!(plan.current_size_bytes, current.size_bytes);
    assert_eq!(plan.incoming_adapter_id, adapter_id());
    assert_eq!(plan.incoming_content_hash, compute_content_hash(incoming_bytes));
    assert_eq!(plan.incoming_size_bytes, incoming_bytes.len() as u64);
    assert_eq!(plan.policy_applied, ConflictPolicy::PreserveBoth);
    assert_eq!(plan.status, ConflictRecordStatus::Open);
    assert_eq!(plan.created_at, fixed_timestamp());
    assert_eq!(plan.conflict_record.conflict_path, plan.materialized_path);
}

#[test]
fn stale_base_different_content_produces_conflict_saved_planning_surface() {
    let current = current_revision_at("Projects/Haze/plan.md", b"current content");
    let (outcome, repository, content_store, operation_log) = run_upsert(
        "Projects/Haze/plan.md",
        b"current content",
        Some(revision_id("rev_stale")),
        b"incoming content",
    );

    assert_conflict_saved_plan(
        &outcome,
        &current,
        Some(revision_id("rev_stale")),
        b"incoming content",
    );
    assert_eq!(repository.current, Some(current));
    assert!(repository.inserted.is_empty());
    assert!(content_store.writes.is_empty());
    assert!(operation_log.entries.is_empty());
}

#[test]
fn unknown_base_different_content_produces_conflict_saved_planning_surface() {
    let current = current_revision_at("Projects/Haze/plan.md", b"current content");
    let (outcome, _, _, _) = run_upsert(
        "Projects/Haze/plan.md",
        b"current content",
        Some(revision_id("rev_unknown")),
        b"incoming content",
    );

    assert_conflict_saved_plan(
        &outcome,
        &current,
        Some(revision_id("rev_unknown")),
        b"incoming content",
    );
}

#[test]
fn null_base_existing_different_content_produces_conflict_saved_planning_surface() {
    let current = current_revision_at("Projects/Haze/plan.md", b"current content");
    let (outcome, _, _, _) = run_upsert(
        "Projects/Haze/plan.md",
        b"current content",
        None,
        b"incoming content",
    );

    assert_conflict_saved_plan(&outcome, &current, None, b"incoming content");
}

#[test]
fn same_content_stale_unknown_and_null_base_remain_ignored() {
    let cases = [
        Some(revision_id("rev_stale")),
        Some(revision_id("rev_unknown")),
        None,
    ];

    for base_revision_id in cases {
        let (outcome, repository, content_store, operation_log) = run_upsert(
            "Projects/Haze/plan.md",
            b"same content",
            base_revision_id,
            b"same content",
        );

        assert!(matches!(
            &outcome,
            UpsertOutcome::IgnoredDuplicateSameContent { .. }
        ));
        assert_eq!(outcome.public_status(), "same_content");
        assert_eq!(
            plan_upsert_conflict_saved(&outcome, fixed_timestamp())
                .expect("ignored same-content outcome should be plannable as none"),
            None
        );
        assert!(repository.inserted.is_empty());
        assert!(content_store.writes.is_empty());
        assert!(operation_log.entries.is_empty());
    }
}

#[test]
fn conflict_materialized_path_stays_under_open_conflicts_dir() {
    let (outcome, _, _, _) = run_upsert(
        "Projects/Haze/plan.md",
        b"current content",
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
        plan.materialized_path.as_str(),
        "_haze_conflicts/open/Projects/Haze/plan.conflict.iphone-anna.2026-07-03-101112.md"
    );
}

#[test]
fn recursive_haze_conflicts_source_path_is_rejected() {
    let conflict_path = "_haze_conflicts/open/Projects/Haze/plan.conflict.iphone-anna.2026-07-03-101112.md";
    let (outcome, _, _, _) = run_upsert(
        conflict_path,
        b"current conflict copy",
        Some(revision_id("rev_stale")),
        b"incoming conflict copy",
    );

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
    let (outcome, _, _, _) = run_upsert(
        "Projects/Haze/plan.md",
        b"current content",
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
