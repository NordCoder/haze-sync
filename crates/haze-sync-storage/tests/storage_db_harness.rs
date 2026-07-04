#![cfg(feature = "test-support")]

use chrono::{Duration, Utc};
use haze_sync_common::{
    AdapterId, ConflictId, ContentHash, OperationId, RevisionId, Sha256, VaultPath,
};
use haze_sync_storage::{
    repositories::{
        adapter_cursors::{
            AdapterCursorRepository, AdapterCursorUpdate, AdapterCursorUpdateOutcome,
        },
        conflicts::{ConflictRepository, ConflictStatusName},
        content_blobs::{create_or_get_content_blob, get_content_blob_by_hash, NewContentBlob},
        idempotency::{
            check_or_store_idempotency_record, IdempotencyRecordInput, IdempotencyRepositoryOutcome,
        },
        objects::{
            create_or_find_sync_object_by_path, get_sync_object_by_path,
            set_current_revision_by_object_id, set_sync_object_deleted_at_by_path, NewSyncObject,
            SyncObjectKind,
        },
        operation_log::{AppendOperationLogEntry, OperationKindName, OperationLogRepository},
        revisions::{
            find_file_revision_by_path_and_content, get_current_revision_by_object_id,
            get_current_revision_by_path, get_file_revision_by_id, insert_file_revision,
            list_file_revisions_by_object_id, list_file_revisions_by_path, NewFileRevision,
        },
        tombstones::{NewTombstone, TombstoneRepository},
    },
    test_support::connect_test_database_from_env,
};
use serde_json::json;
use sqlx::PgPool;

fn adapter_id() -> AdapterId {
    AdapterId::parse("obsidian-plugin").expect("fixture adapter id should parse")
}

fn content_hash(hex: char) -> ContentHash {
    ContentHash::parse(&hex.to_string().repeat(64)).expect("fixture hash should parse")
}

struct ConflictFixture {
    original_path: VaultPath,
    current_revision: RevisionId,
}

async fn prepare_test_pool() -> Option<PgPool> {
    let context = connect_test_database_from_env()
        .await
        .expect("test database lookup should stay safe")?;

    context
        .apply_migrations()
        .await
        .expect("migrations should apply");
    context
        .clean_storage_tables()
        .await
        .expect("tables should clean");

    Some(context.pool().clone())
}

async fn seed_adapter(pool: &PgPool, adapter_id: &AdapterId, role: &str) {
    sqlx::query(
        "insert into sync_adapters (adapter_id, display_name, role, token_hash, enabled) \
         values ($1, $2, $3, $4, true)",
    )
    .bind(adapter_id.as_str())
    .bind(adapter_id.as_str())
    .bind(role)
    .bind(format!("sha256:{}", adapter_id.as_str()))
    .execute(pool)
    .await
    .expect("adapter should seed");
}

async fn seed_content_blob(
    pool: &PgPool,
    sha256: ContentHash,
    size_bytes: u64,
    object_store_path: &str,
) {
    create_or_get_content_blob(
        pool,
        &NewContentBlob {
            sha256,
            size_bytes,
            object_store_path,
        },
    )
    .await
    .expect("content blob should seed");
}

async fn seed_sync_object(
    pool: &PgPool,
    object_id: &str,
    path: &VaultPath,
    adapter_id: &AdapterId,
) -> haze_sync_storage::models::SyncObjectRow {
    create_or_find_sync_object_by_path(
        pool,
        NewSyncObject {
            object_id,
            path,
            kind: SyncObjectKind::File,
            updated_by: adapter_id,
        },
    )
    .await
    .expect("sync object should seed")
}

async fn seed_revision(
    pool: &PgPool,
    revision_id: &RevisionId,
    object_id: &str,
    path: &VaultPath,
    content_hash: ContentHash,
    size_bytes: u64,
    adapter_id: &AdapterId,
) {
    insert_file_revision(
        pool,
        NewFileRevision {
            revision_id,
            object_id,
            path,
            parent_revision_id: None,
            content_hash,
            size_bytes,
            created_by: adapter_id,
        },
    )
    .await
    .expect("file revision should seed");
}

async fn seed_conflict_fixture(
    pool: &PgPool,
    adapter_id: &AdapterId,
    conflict_id: &ConflictId,
) -> ConflictFixture {
    let original_path = VaultPath::parse("Notes/a.md").expect("path should parse");
    let conflict_path = VaultPath::parse(
        "_haze_conflicts/open/Notes/a.conflict.obsidian-plugin.20260704T000000Z.md",
    )
    .expect("conflict path should parse");
    let current_hash = content_hash('a');
    let incoming_hash = content_hash('b');

    seed_content_blob(pool, current_hash, 3, "blobs/aa").await;
    seed_content_blob(pool, incoming_hash, 8, "blobs/bb").await;

    let current_object = seed_sync_object(pool, "obj_current", &original_path, adapter_id).await;
    let conflict_object = seed_sync_object(pool, "obj_conflict", &conflict_path, adapter_id).await;
    let current_revision = RevisionId::parse("rev_current").expect("revision should parse");
    let incoming_revision = RevisionId::parse("rev_incoming").expect("revision should parse");

    seed_revision(
        pool,
        &current_revision,
        current_object.object_id.as_str(),
        &original_path,
        current_hash,
        3,
        adapter_id,
    )
    .await;
    seed_revision(
        pool,
        &incoming_revision,
        conflict_object.object_id.as_str(),
        &conflict_path,
        incoming_hash,
        8,
        adapter_id,
    )
    .await;

    set_current_revision_by_object_id(
        pool,
        current_object.object_id.as_str(),
        Some(&current_revision),
        adapter_id,
    )
    .await
    .expect("current head should persist");
    set_current_revision_by_object_id(
        pool,
        conflict_object.object_id.as_str(),
        Some(&incoming_revision),
        adapter_id,
    )
    .await
    .expect("conflict head should persist");

    sqlx::query(
        "insert into conflicts \
         (conflict_id, original_path, base_revision_id, current_revision_id, incoming_revision_id, \
          incoming_adapter_id, policy_applied, materialized_path, status) \
         values ($1, $2, null, $3, $4, $5, 'current_wins_with_incoming_backup', $6, 'open')",
    )
    .bind(conflict_id.as_str())
    .bind(original_path.as_str())
    .bind(current_revision.as_str())
    .bind(incoming_revision.as_str())
    .bind(adapter_id.as_str())
    .bind(conflict_path.as_str())
    .execute(pool)
    .await
    .expect("conflict should seed");

    ConflictFixture {
        original_path,
        current_revision,
    }
}

#[tokio::test]
async fn objects_revisions_and_content_blobs_roundtrip_against_real_postgres_when_available() {
    let Some(pool) = prepare_test_pool().await else {
        return;
    };

    let adapter_id = adapter_id();
    seed_adapter(&pool, &adapter_id, "obsidian_plugin").await;

    let path = VaultPath::parse("Notes/a.md").expect("path should parse");
    let first_hash = content_hash('1');
    let second_hash = content_hash('2');

    let first_blob = create_or_get_content_blob(
        &pool,
        &NewContentBlob {
            sha256: first_hash,
            size_bytes: 3,
            object_store_path: "blobs/11",
        },
    )
    .await
    .expect("blob should insert");
    let same_blob = create_or_get_content_blob(
        &pool,
        &NewContentBlob {
            sha256: first_hash,
            size_bytes: 999,
            object_store_path: "blobs/override-attempt",
        },
    )
    .await
    .expect("existing blob should return");
    assert_eq!(same_blob.sha256, first_blob.sha256);
    assert_eq!(same_blob.size_bytes, 3);
    assert_eq!(same_blob.object_store_path, "blobs/11");

    let loaded_blob = get_content_blob_by_hash(&pool, first_hash)
        .await
        .expect("blob should load")
        .expect("blob should exist");
    assert_eq!(loaded_blob.sha256, first_hash.to_prefixed_string());

    create_or_get_content_blob(
        &pool,
        &NewContentBlob {
            sha256: second_hash,
            size_bytes: 5,
            object_store_path: "blobs/22",
        },
    )
    .await
    .expect("second blob should insert");

    let object = create_or_find_sync_object_by_path(
        &pool,
        NewSyncObject {
            object_id: "obj_notes_a",
            path: &path,
            kind: SyncObjectKind::File,
            updated_by: &adapter_id,
        },
    )
    .await
    .expect("object should insert");
    let same_object = create_or_find_sync_object_by_path(
        &pool,
        NewSyncObject {
            object_id: "obj_other_attempt",
            path: &path,
            kind: SyncObjectKind::DirectoryPlaceholder,
            updated_by: &adapter_id,
        },
    )
    .await
    .expect("existing object should return");
    assert_eq!(same_object.object_id, object.object_id);
    assert_eq!(same_object.kind, "file");

    let rev1 = RevisionId::parse("rev_01JPHASE8A").expect("revision should parse");
    let rev2 = RevisionId::parse("rev_01JPHASE8B").expect("revision should parse");
    insert_file_revision(
        &pool,
        NewFileRevision {
            revision_id: &rev1,
            object_id: object.object_id.as_str(),
            path: &path,
            parent_revision_id: None,
            content_hash: first_hash,
            size_bytes: 3,
            created_by: &adapter_id,
        },
    )
    .await
    .expect("first revision should insert");
    insert_file_revision(
        &pool,
        NewFileRevision {
            revision_id: &rev2,
            object_id: object.object_id.as_str(),
            path: &path,
            parent_revision_id: Some(&rev1),
            content_hash: second_hash,
            size_bytes: 5,
            created_by: &adapter_id,
        },
    )
    .await
    .expect("second revision should insert");

    set_current_revision_by_object_id(&pool, object.object_id.as_str(), Some(&rev2), &adapter_id)
        .await
        .expect("current revision should set");

    let loaded_object = get_sync_object_by_path(&pool, &path)
        .await
        .expect("object should load")
        .expect("object should exist");
    assert_eq!(
        loaded_object.current_revision_id.as_deref(),
        Some(rev2.as_str())
    );

    let current_by_path = get_current_revision_by_path(&pool, &path)
        .await
        .expect("current revision should load")
        .expect("current revision should exist");
    assert_eq!(current_by_path.revision_id, rev2.as_str());

    let current_by_object = get_current_revision_by_object_id(&pool, object.object_id.as_str())
        .await
        .expect("current object revision should load")
        .expect("current object revision should exist");
    assert_eq!(current_by_object.revision_id, rev2.as_str());

    let fetched_rev1 = get_file_revision_by_id(&pool, &rev1)
        .await
        .expect("revision should load")
        .expect("revision should exist");
    assert_eq!(fetched_rev1.content_sha256, first_hash.to_prefixed_string());

    let revisions_by_object =
        list_file_revisions_by_object_id(&pool, object.object_id.as_str(), 10)
            .await
            .expect("revisions should list");
    assert_eq!(revisions_by_object.len(), 2);
    assert_eq!(revisions_by_object[0].revision_id, rev2.as_str());
    assert_eq!(revisions_by_object[1].revision_id, rev1.as_str());

    let revisions_by_path = list_file_revisions_by_path(&pool, &path, 10)
        .await
        .expect("path revisions should list");
    assert_eq!(revisions_by_path.len(), 2);

    let duplicate_by_content = find_file_revision_by_path_and_content(&pool, &path, first_hash)
        .await
        .expect("duplicate lookup should succeed")
        .expect("duplicate revision should exist");
    assert_eq!(duplicate_by_content.revision_id, rev1.as_str());
}

#[tokio::test]
async fn operation_log_changes_tombstones_and_conflicts_roundtrip_when_real_postgres_is_available()
{
    let Some(pool) = prepare_test_pool().await else {
        return;
    };

    let adapter_id = adapter_id();
    seed_adapter(&pool, &adapter_id, "obsidian_plugin").await;

    let conflict_id = ConflictId::parse("conf_01JPHASE8").expect("conflict id should parse");
    let conflict_fixture = seed_conflict_fixture(&pool, &adapter_id, &conflict_id).await;
    let original_path = conflict_fixture.original_path;
    let current_revision = conflict_fixture.current_revision;

    let operation_repo = OperationLogRepository::new();
    let conflict_op = operation_repo
        .append(
            &pool,
            &AppendOperationLogEntry {
                op_id: OperationId::parse("op_01JPHASE8A").expect("operation id should parse"),
                adapter_id: adapter_id.clone(),
                kind: OperationKindName::ConflictCreated,
                path: original_path.clone(),
                revision_id: Some(current_revision.clone()),
                tombstone_id: None,
                conflict_id: Some(conflict_id.clone()),
            },
        )
        .await
        .expect("conflict operation should append");

    let tombstone_repo = TombstoneRepository::new();
    let tombstone = tombstone_repo
        .insert(
            &pool,
            NewTombstone {
                tombstone_id: "tmb_01JPHASE8",
                path: &original_path,
                deleted_revision_id: Some(&current_revision),
                deleted_by: &adapter_id,
                retention_until: Utc::now() + Duration::days(30),
            },
        )
        .await
        .expect("tombstone should insert");
    let loaded_tombstone = tombstone_repo
        .get_by_id(&pool, "tmb_01JPHASE8")
        .await
        .expect("tombstone should load")
        .expect("tombstone should exist");
    assert_eq!(loaded_tombstone.tombstone_id, tombstone.tombstone_id);

    let delete_op = operation_repo
        .append(
            &pool,
            &AppendOperationLogEntry {
                op_id: OperationId::parse("op_01JPHASE8B").expect("operation id should parse"),
                adapter_id: adapter_id.clone(),
                kind: OperationKindName::DeleteFile,
                path: original_path.clone(),
                revision_id: Some(current_revision.clone()),
                tombstone_id: Some(tombstone.tombstone_id.clone()),
                conflict_id: None,
            },
        )
        .await
        .expect("delete operation should append");

    let conflict_repo = ConflictRepository::new();
    let open_conflicts = conflict_repo
        .list_by_status(&pool, ConflictStatusName::Open)
        .await
        .expect("open conflicts should list");
    assert_eq!(open_conflicts.len(), 1);

    let resolved = conflict_repo
        .mark_open_resolved(&pool, &conflict_id, &adapter_id)
        .await
        .expect("conflict resolution should persist")
        .expect("open conflict should resolve");
    assert_eq!(resolved.status, "resolved");
    assert_eq!(resolved.resolved_by.as_deref(), Some(adapter_id.as_str()));

    let no_second_resolution = conflict_repo
        .mark_open_resolved(&pool, &conflict_id, &adapter_id)
        .await
        .expect("second resolution should stay safe");
    assert!(no_second_resolution.is_none());

    let operation_by_seq = operation_repo
        .get_by_sequence(&pool, conflict_op.seq)
        .await
        .expect("operation should load")
        .expect("operation should exist");
    assert_eq!(operation_by_seq.kind, "conflict_created");

    let operation_by_id = operation_repo
        .get_by_operation_id(
            &pool,
            &OperationId::parse(delete_op.op_id.as_str()).expect("stored op id should parse"),
        )
        .await
        .expect("operation id lookup should load")
        .expect("operation should exist");
    assert_eq!(
        operation_by_id.tombstone_id.as_deref(),
        Some("tmb_01JPHASE8")
    );

    let page = operation_repo
        .changes_since(&pool, 0, 1)
        .await
        .expect("change feed should load");
    assert_eq!(page.from_seq, 0);
    assert_eq!(page.to_seq, conflict_op.seq);
    assert!(page.has_more);
    assert_eq!(page.changes.len(), 1);
    assert_eq!(
        page.changes[0].revision_id.as_deref(),
        Some(current_revision.as_str())
    );
    assert_eq!(
        page.changes[0].content_sha256.as_deref(),
        Some(content_hash('a').to_prefixed_string().as_str())
    );

    let all_ops = operation_repo
        .list_since(&pool, 0, 10)
        .await
        .expect("all operations should list");
    assert_eq!(all_ops.len(), 2);

    let active_tombstones = tombstone_repo
        .list_active(&pool, 10)
        .await
        .expect("active tombstones should list");
    assert_eq!(active_tombstones.len(), 1);
    let tombstones_by_path = tombstone_repo
        .list_by_path(&pool, &original_path, 10)
        .await
        .expect("path tombstones should list");
    assert_eq!(tombstones_by_path.len(), 1);
}

#[tokio::test]
async fn idempotency_repository_distinguishes_new_replay_and_conflict_when_real_postgres_is_available(
) {
    let Some(pool) = prepare_test_pool().await else {
        return;
    };

    let adapter_id = adapter_id();
    seed_adapter(&pool, &adapter_id, "obsidian_plugin").await;

    let request_hash_a = Sha256::parse(&"a".repeat(64)).expect("hash should parse");
    let request_hash_b = Sha256::parse(&"b".repeat(64)).expect("hash should parse");

    let input_a = IdempotencyRecordInput::new(
        adapter_id.clone(),
        "idem-key-1",
        request_hash_a,
        json!({ "status": "accepted", "seq": 1 }),
    )
    .expect("idempotency input should build");
    let input_b = IdempotencyRecordInput::new(
        adapter_id.clone(),
        "idem-key-1",
        request_hash_b,
        json!({ "status": "accepted", "seq": 2 }),
    )
    .expect("idempotency input should build");

    let mut conn = pool.acquire().await.expect("connection should acquire");
    let first = check_or_store_idempotency_record(&mut conn, &input_a)
        .await
        .expect("new idempotency record should store");
    match first {
        IdempotencyRepositoryOutcome::NewRequest { record } => {
            assert_eq!(record.idempotency_key, "idem-key-1");
        }
        other => panic!("expected new request outcome, got {other:?}"),
    }

    let replay = check_or_store_idempotency_record(&mut conn, &input_a)
        .await
        .expect("same request should replay");
    match replay {
        IdempotencyRepositoryOutcome::ReplaySameRequest { record } => {
            assert_eq!(record.request_hash, request_hash_a.to_string());
        }
        other => panic!("expected replay outcome, got {other:?}"),
    }

    let mismatch = check_or_store_idempotency_record(&mut conn, &input_b)
        .await
        .expect("different request should conflict");
    match mismatch {
        IdempotencyRepositoryOutcome::ConflictDifferentRequest { record } => {
            assert_eq!(record.request_hash, request_hash_a.to_string());
        }
        other => panic!("expected conflict outcome, got {other:?}"),
    }
}

#[tokio::test]
async fn adapter_cursor_repository_enforces_monotonic_updates_when_real_postgres_is_available() {
    let Some(pool) = prepare_test_pool().await else {
        return;
    };

    let adapter_id = adapter_id();
    seed_adapter(&pool, &adapter_id, "obsidian_plugin").await;

    let repository = AdapterCursorRepository::new();
    let initial = repository
        .initialize_if_missing(&pool, &adapter_id)
        .await
        .expect("cursor should initialize");
    assert_eq!(initial.last_core_seq, 0);
    assert_eq!(initial.external_cursor_json, json!({}));

    let accepted = repository
        .update_monotonic(
            &pool,
            &AdapterCursorUpdate {
                adapter_id: adapter_id.clone(),
                last_core_seq: 7,
                external_cursor_json: Some(json!({ "page_token": "cursor-7" })),
                mark_success: true,
            },
        )
        .await
        .expect("cursor should update");
    let updated = match accepted {
        AdapterCursorUpdateOutcome::Updated(row) => row,
        other => panic!("expected updated outcome, got {other:?}"),
    };
    assert_eq!(updated.last_core_seq, 7);
    assert_eq!(
        updated.external_cursor_json,
        json!({ "page_token": "cursor-7" })
    );
    assert!(updated.last_success_at.is_some());

    let rejected = repository
        .update_monotonic(
            &pool,
            &AdapterCursorUpdate {
                adapter_id: adapter_id.clone(),
                last_core_seq: 3,
                external_cursor_json: Some(json!({ "page_token": "cursor-3" })),
                mark_success: false,
            },
        )
        .await
        .expect("regression should return safe outcome");
    match rejected {
        AdapterCursorUpdateOutcome::RejectedRegression {
            current,
            requested_seq,
        } => {
            assert_eq!(requested_seq, 3);
            assert_eq!(current.last_core_seq, 7);
            assert_eq!(
                current.external_cursor_json,
                json!({ "page_token": "cursor-7" })
            );
        }
        other => panic!("expected rejected regression, got {other:?}"),
    }

    let persisted = repository
        .get_by_adapter_id(&pool, &adapter_id)
        .await
        .expect("cursor should load")
        .expect("cursor should exist");
    assert_eq!(persisted.last_core_seq, 7);
}

#[tokio::test]
async fn object_delete_marker_updates_are_persisted_without_hard_delete_when_real_postgres_is_available(
) {
    let Some(pool) = prepare_test_pool().await else {
        return;
    };

    let adapter_id = adapter_id();
    seed_adapter(&pool, &adapter_id, "obsidian_plugin").await;

    let path = VaultPath::parse("Notes/deleted.md").expect("path should parse");
    create_or_find_sync_object_by_path(
        &pool,
        NewSyncObject {
            object_id: "obj_deleted",
            path: &path,
            kind: SyncObjectKind::File,
            updated_by: &adapter_id,
        },
    )
    .await
    .expect("object should insert");

    let deleted_at = Utc::now();
    let updated = set_sync_object_deleted_at_by_path(&pool, &path, Some(deleted_at), &adapter_id)
        .await
        .expect("delete marker should persist")
        .expect("object should exist");
    assert!(updated.deleted_at.is_some());

    let restored = set_sync_object_deleted_at_by_path(&pool, &path, None, &adapter_id)
        .await
        .expect("delete marker should clear")
        .expect("object should exist");
    assert_eq!(restored.deleted_at, None);

    let row_count: i64 = sqlx::query_scalar("select count(*) from sync_objects where path = $1")
        .bind(path.as_str())
        .fetch_one(&pool)
        .await
        .expect("object count should load");
    assert_eq!(row_count, 1);
}
