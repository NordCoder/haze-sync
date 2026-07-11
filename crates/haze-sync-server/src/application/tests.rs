use super::*;
use haze_sync_common::{AdapterId, RevisionId, VaultPath};
use haze_sync_core::revision_service::compute_content_hash;
use haze_sync_storage::{
    repositories::{
        conflicts::{ConflictRepository, ConflictStatusName},
        operation_log::OperationLogRepository,
        revisions::get_current_revision_by_path,
        tombstones::TombstoneRepository,
    },
    LocalObjectStore,
};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

struct TestObjectStore {
    root: PathBuf,
    store: LocalObjectStore,
}

impl TestObjectStore {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be valid")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "haze-sync-server-application-{}-{nonce}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        Self {
            store: LocalObjectStore::new(root.clone()),
            root,
        }
    }
}

impl Drop for TestObjectStore {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

async fn seed_adapter(pool: &sqlx::PgPool, adapter_id: &AdapterId) {
    sqlx::query(
        "insert into sync_adapters (adapter_id, display_name, role, token_hash, enabled) \
         values ($1, $2, 'worktree_adapter', $3, true)",
    )
    .bind(adapter_id.as_str())
    .bind(adapter_id.as_str())
    .bind("sha256:test-token-hash")
    .execute(pool)
    .await
    .expect("adapter should seed");
}

fn file_command(
    actor: &ApplicationActor,
    path: &VaultPath,
    base: Option<&RevisionId>,
    bytes: &[u8],
    key: &str,
) -> ApplyFileCommand {
    let content_hash = compute_content_hash(bytes);
    ApplyFileCommand {
        actor: actor.clone(),
        path: path.clone(),
        base_revision_id: base.cloned(),
        content_hash,
        bytes: bytes.to_vec(),
        idempotency: ApplicationIdempotency::new(
            key,
            file_request_fingerprint(actor, path, base, content_hash, bytes),
        ),
    }
}

fn delete_command(
    actor: &ApplicationActor,
    path: &VaultPath,
    base: Option<&RevisionId>,
    requested_delete_count: u32,
    key: &str,
) -> ApplyDeleteCommand {
    ApplyDeleteCommand {
        actor: actor.clone(),
        path: path.clone(),
        base_revision_id: base.cloned(),
        requested_delete_count,
        idempotency: ApplicationIdempotency::new(
            key,
            delete_request_fingerprint(actor, path, base, requested_delete_count),
        ),
    }
}

#[tokio::test]
async fn application_services_preserve_atomic_file_delete_and_read_semantics() {
    let database = super::test_db::acquire_required().await;
    let pool = database.pool().clone();
    let adapter_id = AdapterId::parse("worktree").unwrap();
    seed_adapter(&pool, &adapter_id).await;
    let actor = ApplicationActor::new(adapter_id);
    let object_store = TestObjectStore::new();
    let services = ServerApplicationServices::new(pool.clone(), Some(object_store.store.clone()));
    let path = VaultPath::parse("Notes/application.md").unwrap();

    let accepted = services
        .apply_file(file_command(&actor, &path, None, b"first", "put-1"))
        .await
        .expect("new file should be accepted");
    let (revision_id, content_hash, accepted_seq) = match accepted {
        ApplyFileOutcome::Accepted {
            revision_id,
            content_hash: Some(content_hash),
            seq,
            ..
        } => (revision_id, content_hash, seq),
        other => panic!("unexpected accepted outcome: {other:?}"),
    };

    let replay = services
        .apply_file(file_command(&actor, &path, None, b"first", "put-1"))
        .await
        .expect("same command should replay");
    assert!(matches!(
        replay,
        ApplyFileOutcome::Accepted {
            revision_id: ref replay_revision,
            content_hash: Some(replay_hash),
            seq,
            ..
        } if replay_revision == &revision_id && replay_hash == content_hash && seq == accepted_seq
    ));

    let operations_before_mismatch = OperationLogRepository::new()
        .list_since(&pool, 0, 100)
        .await
        .expect("operations should load")
        .len();
    let mismatch = services
        .apply_file(file_command(&actor, &path, None, b"different", "put-1"))
        .await;
    assert_eq!(mismatch, Err(ApplicationError::IdempotencyMismatch));
    assert_eq!(
        OperationLogRepository::new()
            .list_since(&pool, 0, 100)
            .await
            .expect("operations should load")
            .len(),
        operations_before_mismatch
    );

    let same_content = services
        .apply_file(file_command(
            &actor,
            &path,
            Some(&revision_id),
            b"first",
            "put-same-content",
        ))
        .await
        .expect("same content should be a no-op");
    assert!(matches!(
        same_content,
        ApplyFileOutcome::SameContent {
            revision_id: Some(ref current),
            content_hash: Some(hash),
            ..
        } if current == &revision_id && hash == content_hash
    ));

    let conflict = services
        .apply_file(file_command(
            &actor,
            &path,
            Some(&RevisionId::parse("rev_stale").unwrap()),
            b"incoming",
            "put-conflict",
        ))
        .await
        .expect("stale write should preserve a conflict");
    assert!(matches!(conflict, ApplyFileOutcome::ConflictSaved { .. }));
    assert_eq!(
        ConflictRepository::new()
            .list_by_status(&pool, ConflictStatusName::Open)
            .await
            .expect("conflicts should load")
            .len(),
        1
    );

    let content = services
        .revision_content(RevisionContentQuery {
            path: path.clone(),
            revision_id: None,
        })
        .await
        .expect("current content should load");
    assert_eq!(content.revision_id, revision_id);
    assert_eq!(content.content_hash, content_hash);
    assert_eq!(content.into_bytes(), b"first".to_vec());

    let changes = services
        .authoritative_changes(AuthoritativeChangesQuery::new(0, 1).unwrap())
        .await
        .expect("bounded changes should load");
    assert_eq!(changes.from, 0);
    assert_eq!(changes.changes.len(), 1);
    assert!(changes.to >= 1);
    assert!(changes.has_more);

    let stale_delete = services
        .apply_delete(delete_command(
            &actor,
            &path,
            Some(&RevisionId::parse("rev_stale").unwrap()),
            1,
            "delete-stale",
        ))
        .await
        .expect("stale delete should return a typed rejection");
    assert!(matches!(stale_delete, ApplyDeleteOutcome::StaleBase { .. }));
    assert!(TombstoneRepository::new()
        .list_active(&pool, 100)
        .await
        .expect("tombstones should load")
        .is_empty());

    let guarded_delete = services
        .apply_delete(delete_command(
            &actor,
            &path,
            Some(&revision_id),
            10_000,
            "delete-guarded",
        ))
        .await
        .expect("unsafe delete count should return a typed rejection");
    assert!(matches!(
        guarded_delete,
        ApplyDeleteOutcome::GuardRejected { .. }
    ));

    let deleted = services
        .apply_delete(delete_command(
            &actor,
            &path,
            Some(&revision_id),
            1,
            "delete-accepted",
        ))
        .await
        .expect("current-base delete should tombstone");
    let deleted_seq = match &deleted {
        ApplyDeleteOutcome::Tombstoned { seq, .. } => *seq,
        other => panic!("unexpected delete outcome: {other:?}"),
    };
    let delete_replay = services
        .apply_delete(delete_command(
            &actor,
            &path,
            Some(&revision_id),
            1,
            "delete-accepted",
        ))
        .await
        .expect("same delete should replay");
    assert!(matches!(
        delete_replay,
        ApplyDeleteOutcome::Tombstoned { seq, .. } if seq == deleted_seq
    ));
    assert!(get_current_revision_by_path(&pool, &path)
        .await
        .expect("current revision lookup should succeed")
        .is_none());

    let missing = services
        .revision_content(RevisionContentQuery {
            path,
            revision_id: None,
        })
        .await;
    assert_eq!(missing, Err(ApplicationError::NotFound));
}

#[test]
fn application_errors_and_commands_are_secret_safe() {
    let actor = ApplicationActor::new(AdapterId::parse("worktree").unwrap());
    let path = VaultPath::parse("Notes/private.md").unwrap();
    let command = file_command(&actor, &path, None, b"request-body-secret", "secret-key");
    let rendered = format!(
        "{command:?} {:?} {}",
        ApplicationError::Internal,
        ApplicationError::Internal
    );
    assert!(!rendered.contains("request-body-secret"));
    assert!(!rendered.contains("secret-key"));
    assert!(!rendered.contains("postgres://"));
}
