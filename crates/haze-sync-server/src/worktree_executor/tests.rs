use super::*;
use crate::application::{
    delete_request_fingerprint, file_request_fingerprint, test_db, ApplyDeleteCommand,
    ApplyDeleteOutcome, ApplyFileCommand, ApplyFileOutcome, ApplicationActor,
    ApplicationIdempotency, ServerApplicationServices,
};
use haze_sync_common::{AdapterId, RevisionId, Sha256, VaultPath};
use haze_sync_core::revision_service::compute_content_hash;
use haze_sync_storage::{
    repositories::{
        adapter_cursors::AdapterCursorRepository,
        operation_log::OperationLogRepository,
        revisions::get_current_revision_by_path,
        tombstones::TombstoneRepository,
        worktree_state::{
            bind_or_verify_worktree_instance, load_path_state, WorktreeInstanceBinding,
            WorktreeStateKind,
        },
    },
    LocalObjectStore,
};
use haze_sync_worktree::{WorktreeRuntimeCycleBudget, WorktreeRuntimeCycleCause};
use sqlx::PgPool;
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

struct TempTree {
    root: PathBuf,
}

impl TempTree {
    fn new(name: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be valid")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "haze-sync-server-executor-{name}-{}-{nonce}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("temp root should create");
        Self { root }
    }

    fn write(&self, path: &str, bytes: &[u8]) {
        let destination = self.root.join(path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).expect("parent should create");
        }
        fs::write(destination, bytes).expect("test file should write");
    }

    fn read(&self, path: &str) -> Vec<u8> {
        fs::read(self.root.join(path)).expect("test file should read")
    }

    fn remove(&self, path: &str) {
        fs::remove_file(self.root.join(path)).expect("test file should remove");
    }
}

impl Drop for TempTree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

struct Harness {
    _database: test_db::TestDatabaseLease,
    pool: PgPool,
    worktree: TempTree,
    _objects: TempTree,
    services: ServerApplicationServices,
    worktree_id: AdapterId,
    remote_id: AdapterId,
    fingerprint: Sha256,
}

impl Harness {
    async fn new(name: &str) -> Self {
        let database = test_db::acquire_required().await;
        let pool = database.pool().clone();
        let worktree = TempTree::new(&format!("{name}-worktree"));
        let objects = TempTree::new(&format!("{name}-objects"));
        let store = LocalObjectStore::new(objects.root.clone());
        let services = ServerApplicationServices::new(pool.clone(), Some(store));
        let worktree_id = AdapterId::parse("worktree").unwrap();
        let remote_id = AdapterId::parse("remote-test").unwrap();
        seed_adapter(&pool, &worktree_id).await;
        seed_adapter(&pool, &remote_id).await;
        let fingerprint = Sha256::from_bytes([7_u8; 32]);
        bind_or_verify_worktree_instance(
            &pool,
            &WorktreeInstanceBinding::new(worktree_id.clone(), fingerprint),
        )
        .await
        .expect("binding should seed");
        Self {
            _database: database,
            pool,
            worktree,
            _objects: objects,
            services,
            worktree_id,
            remote_id,
            fingerprint,
        }
    }

    fn executor(&self) -> ServerWorktreeCycleExecutor {
        ServerWorktreeCycleExecutor::new(
            self.services.clone(),
            self.pool.clone(),
            self.worktree_id.clone(),
            self.worktree.root.clone(),
            self.fingerprint,
            test_policy(),
        )
        .expect("executor should build")
    }

    async fn seed_remote_file(&self, path: &VaultPath, bytes: &[u8], key: &str) -> RevisionId {
        let actor = ApplicationActor::new(self.remote_id.clone());
        let content_hash = compute_content_hash(bytes);
        let outcome = self
            .services
            .apply_file(ApplyFileCommand {
                actor: actor.clone(),
                path: path.clone(),
                base_revision_id: None,
                content_hash,
                bytes: bytes.to_vec(),
                idempotency: ApplicationIdempotency::new(
                    key,
                    file_request_fingerprint(&actor, path, None, content_hash, bytes),
                ),
            })
            .await
            .expect("remote file should seed");
        match outcome {
            ApplyFileOutcome::Accepted { revision_id, .. } => revision_id,
            other => panic!("unexpected seed outcome: {other:?}"),
        }
    }

    async fn update_remote_file(
        &self,
        path: &VaultPath,
        base_revision_id: &RevisionId,
        bytes: &[u8],
        key: &str,
    ) -> ApplyFileOutcome {
        let actor = ApplicationActor::new(self.remote_id.clone());
        let content_hash = compute_content_hash(bytes);
        self.services
            .apply_file(ApplyFileCommand {
                actor: actor.clone(),
                path: path.clone(),
                base_revision_id: Some(base_revision_id.clone()),
                content_hash,
                bytes: bytes.to_vec(),
                idempotency: ApplicationIdempotency::new(
                    key,
                    file_request_fingerprint(
                        &actor,
                        path,
                        Some(base_revision_id),
                        content_hash,
                        bytes,
                    ),
                ),
            })
            .await
            .expect("remote update should complete")
    }

    async fn delete_remote_file(
        &self,
        path: &VaultPath,
        base_revision_id: &RevisionId,
        key: &str,
    ) -> ApplyDeleteOutcome {
        let actor = ApplicationActor::new(self.remote_id.clone());
        self.services
            .apply_delete(ApplyDeleteCommand {
                actor: actor.clone(),
                path: path.clone(),
                base_revision_id: Some(base_revision_id.clone()),
                requested_delete_count: 1,
                idempotency: ApplicationIdempotency::new(
                    key,
                    delete_request_fingerprint(&actor, path, Some(base_revision_id), 1),
                ),
            })
            .await
            .expect("remote delete should complete")
    }
}

fn test_policy() -> ServerWorktreeExecutorPolicy {
    ServerWorktreeExecutorPolicy::new(
        10,
        100,
        10_000,
        Duration::from_secs(60),
        100,
        Duration::from_secs(60),
    )
    .unwrap()
}

fn request(
    mode: WorktreeMode,
    full_scan_required: bool,
    import_enabled: bool,
    export_enabled: bool,
    imports: usize,
    deletes: usize,
    exports: usize,
) -> WorktreeRuntimeCycleRequest {
    WorktreeRuntimeCycleRequest {
        cause: WorktreeRuntimeCycleCause::Startup,
        mode,
        full_scan_required,
        import_enabled,
        export_enabled,
        budget: WorktreeRuntimeCycleBudget {
            max_import_actions: imports,
            max_delete_candidates: deletes,
            max_export_actions: exports,
        },
        coalesced_watcher_hints: 0,
    }
}

async fn seed_adapter(pool: &PgPool, adapter_id: &AdapterId) {
    sqlx::query(
        "insert into sync_adapters (adapter_id, display_name, role, token_hash, enabled) \
         values ($1, $2, 'worktree_adapter', $3, true) on conflict (adapter_id) do nothing",
    )
    .bind(adapter_id.as_str())
    .bind(adapter_id.as_str())
    .bind("sha256:test-token-hash")
    .execute(pool)
    .await
    .expect("adapter should seed");
}

async fn operation_count(pool: &PgPool) -> usize {
    OperationLogRepository::new()
        .list_since(pool, 0, 100)
        .await
        .expect("operations should load")
        .len()
}

#[tokio::test]
async fn bounded_full_scan_imports_through_application_services() {
    let harness = Harness::new("import").await;
    harness.worktree.write("Notes/new.md", b"local");
    let mut executor = harness.executor();
    let summary = executor
        .run_cycle(
            request(WorktreeMode::ImportOnly, true, true, false, 10, 10, 10),
            WorktreeCancellationToken::new(),
        )
        .await
        .expect("cycle should succeed");

    assert!(summary.full_scan_completed);
    assert_eq!(summary.scanned_files, 1);
    assert_eq!(summary.planned_imports, 1);
    assert_eq!(summary.submitted_imports, 1);
    let path = VaultPath::parse("Notes/new.md").unwrap();
    assert!(get_current_revision_by_path(&harness.pool, &path)
        .await
        .unwrap()
        .is_some());
    assert_eq!(
        load_path_state(&harness.pool, &harness.worktree_id, &path)
            .await
            .unwrap()
            .unwrap()
            .state_kind,
        WorktreeStateKind::Present.as_str()
    );
}

#[tokio::test]
async fn mutation_before_state_interruption_replays_without_duplicate_operation() {
    let harness = Harness::new("import-replay").await;
    harness.worktree.write("Notes/replay.md", b"local");
    let path = VaultPath::parse("Notes/replay.md").unwrap();
    let mut executor = harness.executor();
    executor.fail_after_import_submission_once();

    assert_eq!(
        executor
            .run_cycle(
                request(WorktreeMode::ImportOnly, true, true, false, 10, 10, 10),
                WorktreeCancellationToken::new(),
            )
            .await,
        Err(WorktreeRuntimeCycleFailure::Submit)
    );
    assert_eq!(operation_count(&harness.pool).await, 1);
    assert!(load_path_state(&harness.pool, &harness.worktree_id, &path)
        .await
        .unwrap()
        .is_none());

    executor
        .run_cycle(
            request(WorktreeMode::ImportOnly, true, true, false, 10, 10, 10),
            WorktreeCancellationToken::new(),
        )
        .await
        .expect("replay should converge");
    assert_eq!(operation_count(&harness.pool).await, 1);
    assert!(load_path_state(&harness.pool, &harness.worktree_id, &path)
        .await
        .unwrap()
        .is_some());
}

#[tokio::test]
async fn local_delete_uses_guarded_application_service_and_stays_bounded() {
    let harness = Harness::new("delete").await;
    harness.worktree.write("Notes/delete.md", b"local");
    let path = VaultPath::parse("Notes/delete.md").unwrap();
    let mut executor = harness.executor();
    executor
        .run_cycle(
            request(WorktreeMode::ImportOnly, true, true, false, 10, 10, 10),
            WorktreeCancellationToken::new(),
        )
        .await
        .unwrap();
    harness.worktree.remove("Notes/delete.md");

    let summary = executor
        .run_cycle(
            request(WorktreeMode::ImportOnly, true, true, false, 10, 1, 10),
            WorktreeCancellationToken::new(),
        )
        .await
        .expect("guarded delete should succeed");
    assert_eq!(summary.planned_deletes, 1);
    assert_eq!(summary.submitted_imports, 0);
    assert_eq!(
        load_path_state(&harness.pool, &harness.worktree_id, &path)
            .await
            .unwrap()
            .unwrap()
            .state_kind,
        WorktreeStateKind::Tombstoned.as_str()
    );
    assert_eq!(
        TombstoneRepository::new()
            .list_active(&harness.pool, 10)
            .await
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn ordered_export_materializes_and_advances_exact_cursor() {
    let harness = Harness::new("export").await;
    let first = VaultPath::parse("Notes/a.md").unwrap();
    let second = VaultPath::parse("Notes/b.md").unwrap();
    harness.seed_remote_file(&first, b"a", "remote-a").await;
    harness.seed_remote_file(&second, b"b", "remote-b").await;
    let mut executor = harness.executor();

    let summary = executor
        .run_cycle(
            request(WorktreeMode::ExportOnly, false, false, true, 10, 10, 2),
            WorktreeCancellationToken::new(),
        )
        .await
        .expect("export should succeed");
    assert_eq!(summary.applied_exports, 2);
    assert_eq!(harness.worktree.read("Notes/a.md"), b"a");
    assert_eq!(harness.worktree.read("Notes/b.md"), b"b");
    assert_eq!(
        AdapterCursorRepository::new()
            .get_by_adapter_id(&harness.pool, &harness.worktree_id)
            .await
            .unwrap()
            .unwrap()
            .last_core_seq,
        2
    );
}

#[tokio::test]
async fn conflict_created_export_uses_revision_materialized_path() {
    let harness = Harness::new("export-conflict").await;
    let original = VaultPath::parse("Notes/conflict.md").unwrap();
    let first_revision = harness
        .seed_remote_file(&original, b"first", "conflict-first")
        .await;
    let second = harness
        .update_remote_file(&original, &first_revision, b"second", "conflict-second")
        .await;
    let second_revision = match second {
        ApplyFileOutcome::Accepted { revision_id, .. } => revision_id,
        other => panic!("unexpected second revision outcome: {other:?}"),
    };
    let conflict = harness
        .update_remote_file(&original, &first_revision, b"incoming", "conflict-stale")
        .await;
    let materialized_path = match conflict {
        ApplyFileOutcome::ConflictSaved {
            materialized_path, ..
        } => materialized_path,
        other => panic!("unexpected conflict outcome: {other:?}"),
    };
    let mut executor = harness.executor();

    let summary = executor
        .run_cycle(
            request(WorktreeMode::ExportOnly, false, false, true, 10, 10, 3),
            WorktreeCancellationToken::new(),
        )
        .await
        .expect("conflict export should use stored revision path");

    assert_eq!(summary.applied_exports, 3);
    assert_eq!(harness.worktree.read(original.as_str()), b"second");
    assert_eq!(harness.worktree.read(materialized_path.as_str()), b"incoming");
    assert_eq!(
        get_current_revision_by_path(&harness.pool, &original)
            .await
            .unwrap()
            .unwrap()
            .revision_id,
        second_revision.as_str()
    );
    assert_eq!(
        AdapterCursorRepository::new()
            .get_by_adapter_id(&harness.pool, &harness.worktree_id)
            .await
            .unwrap()
            .unwrap()
            .last_core_seq,
        3
    );
}

#[tokio::test]
async fn tombstone_export_moves_content_to_retained_trash_and_checkpoints() {
    let harness = Harness::new("export-tombstone").await;
    let path = VaultPath::parse("Notes/deleted.md").unwrap();
    let revision_id = harness
        .seed_remote_file(&path, b"delete-me", "delete-seed")
        .await;
    assert!(matches!(
        harness
            .delete_remote_file(&path, &revision_id, "delete-remote")
            .await,
        ApplyDeleteOutcome::Tombstoned { .. }
    ));
    let mut executor = harness.executor();

    let summary = executor
        .run_cycle(
            request(WorktreeMode::ExportOnly, false, false, true, 10, 10, 2),
            WorktreeCancellationToken::new(),
        )
        .await
        .expect("tombstone export should succeed");

    assert_eq!(summary.applied_exports, 2);
    assert!(!harness.worktree.root.join(path.as_str()).exists());
    assert!(harness
        .worktree
        .root
        .join("_haze_runtime/trash/records")
        .exists());
    assert_eq!(
        load_path_state(&harness.pool, &harness.worktree_id, &path)
            .await
            .unwrap()
            .unwrap()
            .state_kind,
        WorktreeStateKind::Tombstoned.as_str()
    );
    assert_eq!(
        AdapterCursorRepository::new()
            .get_by_adapter_id(&harness.pool, &harness.worktree_id)
            .await
            .unwrap()
            .unwrap()
            .last_core_seq,
        2
    );
}

#[tokio::test]
async fn dirty_export_failure_leaves_failed_sequence_unadvanced() {
    let harness = Harness::new("export-dirty").await;
    let path = VaultPath::parse("Notes/dirty.md").unwrap();
    harness.seed_remote_file(&path, b"core", "remote-dirty").await;
    harness.worktree.write("Notes/dirty.md", b"local-dirty");
    let mut executor = harness.executor();

    assert_eq!(
        executor
            .run_cycle(
                request(WorktreeMode::ExportOnly, false, false, true, 10, 10, 10),
                WorktreeCancellationToken::new(),
            )
            .await,
        Err(WorktreeRuntimeCycleFailure::Export)
    );
    assert_eq!(harness.worktree.read("Notes/dirty.md"), b"local-dirty");
    assert_eq!(
        AdapterCursorRepository::new()
            .get_by_adapter_id(&harness.pool, &harness.worktree_id)
            .await
            .unwrap()
            .unwrap()
            .last_core_seq,
        0
    );
}

#[tokio::test]
async fn materialization_before_checkpoint_replay_converges() {
    let harness = Harness::new("export-replay").await;
    let path = VaultPath::parse("Notes/replay-export.md").unwrap();
    harness.seed_remote_file(&path, b"core", "remote-replay").await;
    let mut executor = harness.executor();
    executor.fail_after_export_filesystem_once();

    assert_eq!(
        executor
            .run_cycle(
                request(WorktreeMode::ExportOnly, false, false, true, 10, 10, 10),
                WorktreeCancellationToken::new(),
            )
            .await,
        Err(WorktreeRuntimeCycleFailure::Export)
    );
    assert_eq!(harness.worktree.read("Notes/replay-export.md"), b"core");
    assert_eq!(
        AdapterCursorRepository::new()
            .get_by_adapter_id(&harness.pool, &harness.worktree_id)
            .await
            .unwrap()
            .unwrap()
            .last_core_seq,
        0
    );

    executor
        .run_cycle(
            request(WorktreeMode::ExportOnly, false, false, true, 10, 10, 10),
            WorktreeCancellationToken::new(),
        )
        .await
        .expect("replay should checkpoint");
    assert_eq!(
        AdapterCursorRepository::new()
            .get_by_adapter_id(&harness.pool, &harness.worktree_id)
            .await
            .unwrap()
            .unwrap()
            .last_core_seq,
        1
    );
    assert!(load_path_state(&harness.pool, &harness.worktree_id, &path)
        .await
        .unwrap()
        .is_some());
}

#[tokio::test]
async fn cancellation_before_and_during_actions_stops_later_work() {
    let harness = Harness::new("cancel").await;
    harness.worktree.write("Notes/a.md", b"a");
    harness.worktree.write("Notes/b.md", b"b");
    let mut executor = harness.executor();
    let cancelled = WorktreeCancellationToken::new();
    cancelled.cancel();
    assert_eq!(
        executor
            .run_cycle(
                request(WorktreeMode::ImportOnly, true, true, false, 10, 10, 10),
                cancelled,
            )
            .await,
        Err(WorktreeRuntimeCycleFailure::Cancelled)
    );
    assert_eq!(operation_count(&harness.pool).await, 0);

    executor.cancel_after_import_commit_once();
    assert_eq!(
        executor
            .run_cycle(
                request(WorktreeMode::ImportOnly, true, true, false, 10, 10, 10),
                WorktreeCancellationToken::new(),
            )
            .await,
        Err(WorktreeRuntimeCycleFailure::Cancelled)
    );
    assert_eq!(operation_count(&harness.pool).await, 1);
}

#[tokio::test]
async fn import_budget_limits_completed_actions() {
    let harness = Harness::new("budget").await;
    harness.worktree.write("Notes/a.md", b"a");
    harness.worktree.write("Notes/b.md", b"b");
    let mut executor = harness.executor();

    let summary = executor
        .run_cycle(
            request(WorktreeMode::ImportOnly, true, true, false, 1, 10, 10),
            WorktreeCancellationToken::new(),
        )
        .await
        .expect("bounded cycle should succeed");
    assert_eq!(summary.planned_imports, 1);
    assert_eq!(summary.submitted_imports, 1);
    assert_eq!(operation_count(&harness.pool).await, 1);
}

#[tokio::test]
async fn dry_run_reports_bounded_plan_without_durable_changes() {
    let harness = Harness::new("dry-run").await;
    harness.worktree.write("Notes/dry.md", b"local");
    let path = VaultPath::parse("Notes/dry.md").unwrap();
    let mut executor = harness.executor();

    let summary = executor
        .run_cycle(
            request(WorktreeMode::DryRun, true, false, false, 1, 1, 1),
            WorktreeCancellationToken::new(),
        )
        .await
        .expect("dry-run planning should succeed");

    assert!(summary.full_scan_completed);
    assert_eq!(summary.planned_imports, 1);
    assert_eq!(summary.submitted_imports, 0);
    assert_eq!(summary.applied_exports, 0);
    assert_eq!(operation_count(&harness.pool).await, 0);
    assert!(load_path_state(&harness.pool, &harness.worktree_id, &path)
        .await
        .unwrap()
        .is_none());
    assert!(AdapterCursorRepository::new()
        .get_by_adapter_id(&harness.pool, &harness.worktree_id)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn required_scan_and_mode_permissions_fail_closed() {
    let harness = Harness::new("permissions").await;
    let mut executor = harness.executor();
    for invalid in [
        request(WorktreeMode::ImportOnly, false, true, false, 1, 1, 1),
        request(WorktreeMode::ImportOnly, true, true, true, 1, 1, 1),
        request(WorktreeMode::DryRun, true, true, false, 1, 1, 1),
    ] {
        assert_eq!(
            executor
                .run_cycle(invalid, WorktreeCancellationToken::new())
                .await,
            Err(WorktreeRuntimeCycleFailure::Plan)
        );
    }
    assert_eq!(operation_count(&harness.pool).await, 0);
}

#[tokio::test]
async fn missing_binding_fails_before_filesystem_or_authoritative_work() {
    let database = test_db::acquire_required().await;
    let pool = database.pool().clone();
    let worktree = TempTree::new("missing-binding");
    let objects = TempTree::new("missing-binding-objects");
    let worktree_id = AdapterId::parse("worktree").unwrap();
    seed_adapter(&pool, &worktree_id).await;
    let services = ServerApplicationServices::new(
        pool.clone(),
        Some(LocalObjectStore::new(objects.root.clone())),
    );
    let mut executor = ServerWorktreeCycleExecutor::new(
        services,
        pool.clone(),
        worktree_id,
        worktree.root.clone(),
        Sha256::from_bytes([9_u8; 32]),
        test_policy(),
    )
    .unwrap();

    assert_eq!(
        executor
            .run_cycle(
                request(WorktreeMode::ImportOnly, true, true, false, 1, 1, 1),
                WorktreeCancellationToken::new(),
            )
            .await,
        Err(WorktreeRuntimeCycleFailure::Plan)
    );
    assert_eq!(operation_count(&pool).await, 0);
}

#[test]
fn debug_and_errors_are_root_database_and_fingerprint_safe() {
    let pool = PgPool::connect_lazy("postgres://user:secret@db.invalid/private").unwrap();
    let root = Path::new("/srv/private/token-like-worktree-root").to_path_buf();
    let services = ServerApplicationServices::new(pool.clone(), None);
    let executor = ServerWorktreeCycleExecutor::new(
        services,
        pool,
        AdapterId::parse("worktree").unwrap(),
        root.clone(),
        Sha256::from_bytes([0xab; 32]),
        test_policy(),
    )
    .unwrap();
    let rendered = format!(
        "{executor:?} {} {}",
        ServerWorktreeExecutorConfigError::InvalidWorktreeRoot,
        ServerWorktreeExecutorConfigError::InvalidPolicy
    );
    assert!(!rendered.contains(root.to_string_lossy().as_ref()));
    assert!(!rendered.contains("/srv/private"));
    assert!(!rendered.contains("postgres://"));
    assert!(!rendered.contains("secret"));
    assert!(!rendered.contains(&"ab".repeat(32)));
}
