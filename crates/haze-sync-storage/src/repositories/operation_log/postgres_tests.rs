use super::*;
use crate::locks::{lock_vault_path, path_lock_key};
use crate::repositories::conflicts::{ConflictRepository, ConflictStatusName, NewConflict};
use crate::repositories::content_blobs::{
    create_or_get_content_blob, get_content_blob_by_hash, NewContentBlob,
};
use crate::repositories::objects::{
    create_or_find_sync_object_by_path, get_sync_object_by_path, set_current_revision_by_path,
    NewSyncObject, SyncObjectKind,
};
use crate::repositories::revisions::{
    get_current_revision_by_path, get_file_revision_by_id, insert_file_revision, NewFileRevision,
};
use crate::repositories::tombstones::{NewTombstone, TombstoneRepository};
use crate::test_support::connect_test_database_from_env;
use chrono::Duration;
use haze_sync_common::ContentHash;

#[tokio::test]
async fn file_conflict_and_delete_flows_roundtrip_under_caller_owned_transaction() {
    let Some(context) = connect_test_database_from_env().await.unwrap() else {
        return;
    };
    context.apply_migrations().await.unwrap();
    context.clean_storage_tables().await.unwrap();

    let adapter_id = AdapterId::parse("worktree-adapter").unwrap();
    let path = VaultPath::parse("Notes/today.md").unwrap();
    let content_hash = ContentHash::parse(&"a".repeat(64)).unwrap();
    let revision_id = RevisionId::parse("rev_01JSTORP5").unwrap();
    let incoming_revision_id = RevisionId::parse("rev_01JSTORP6").unwrap();
    let op_id = OperationId::parse("op_01JSTORP5").unwrap();
    let conflict_id = ConflictId::parse("conf_01JSTORP6").unwrap();
    let materialized_path = VaultPath::parse("Notes/today.conflict.md").unwrap();
    let tombstone_id = "tmb_01JSTORP6";
    let operation_log = OperationLogRepository::new();
    let conflict_repository = ConflictRepository::new();
    let tombstone_repository = TombstoneRepository::new();

    let mut tx = context.pool().begin().await.unwrap();
    sqlx::query(
        "insert into sync_adapters (adapter_id, display_name, role, token_hash) \
         values ($1, $2, $3, $4)",
    )
    .bind(adapter_id.as_str())
    .bind("Worktree Adapter")
    .bind("worktree")
    .bind("sha256:test-token-hash")
    .execute(&mut *tx)
    .await
    .unwrap();

    let lock_key = lock_vault_path(&mut *tx, &path).await.unwrap();
    assert_eq!(lock_key, path_lock_key(&path));

    let blob = create_or_get_content_blob(
        &mut *tx,
        &NewContentBlob {
            sha256: content_hash,
            size_bytes: 42,
            object_store_path:
                "sha256/aa/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        },
    )
    .await
    .unwrap();
    let duplicate_blob = create_or_get_content_blob(
        &mut *tx,
        &NewContentBlob {
            sha256: content_hash,
            size_bytes: 999,
            object_store_path: "sha256/aa/ignored-on-duplicate",
        },
    )
    .await
    .unwrap();
    assert_eq!(duplicate_blob, blob);

    let object = create_or_find_sync_object_by_path(
        &mut *tx,
        NewSyncObject {
            object_id: "obj_01JSTORP5",
            path: &path,
            kind: SyncObjectKind::File,
            updated_by: &adapter_id,
        },
    )
    .await
    .unwrap();
    assert_eq!(object.path, path.as_str());
    assert!(object.current_revision_id.is_none());

    let revision = insert_file_revision(
        &mut *tx,
        NewFileRevision {
            revision_id: &revision_id,
            object_id: "obj_01JSTORP5",
            path: &path,
            parent_revision_id: None,
            content_hash,
            size_bytes: 42,
            created_by: &adapter_id,
        },
    )
    .await
    .unwrap();
    let incoming_revision = insert_file_revision(
        &mut *tx,
        NewFileRevision {
            revision_id: &incoming_revision_id,
            object_id: "obj_01JSTORP5",
            path: &path,
            parent_revision_id: Some(&revision_id),
            content_hash,
            size_bytes: 42,
            created_by: &adapter_id,
        },
    )
    .await
    .unwrap();
    assert_eq!(revision.revision_id, revision_id.as_str());
    assert_eq!(incoming_revision.revision_id, incoming_revision_id.as_str());
    assert_eq!(revision.content_sha256, content_hash.to_prefixed_string());

    let updated_object =
        set_current_revision_by_path(&mut *tx, &path, Some(&revision_id), &adapter_id)
            .await
            .unwrap()
            .unwrap();
    assert_eq!(
        updated_object.current_revision_id.as_deref(),
        Some(revision_id.as_str())
    );

    let operation = operation_log
        .append(
            &mut *tx,
            &AppendOperationLogEntry {
                op_id: op_id.clone(),
                adapter_id: adapter_id.clone(),
                kind: OperationKindName::UpsertFile,
                path: path.clone(),
                revision_id: Some(revision_id.clone()),
                tombstone_id: None,
                conflict_id: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(operation.seq, 1);
    assert_eq!(operation.kind, OperationKindName::UpsertFile.as_str());

    let conflict = conflict_repository
        .insert(
            &mut *tx,
            &NewConflict {
                conflict_id: &conflict_id,
                original_path: &path,
                base_revision_id: Some(&revision_id),
                current_revision_id: &revision_id,
                incoming_revision_id: &incoming_revision_id,
                incoming_adapter_id: &adapter_id,
                policy_applied: "preserve_both",
                materialized_path: &materialized_path,
                status: ConflictStatusName::Open,
            },
        )
        .await
        .unwrap();
    assert_eq!(conflict.status, ConflictStatusName::Open.as_str());
    assert_eq!(
        conflict_repository
            .list_by_status_limited(&mut *tx, ConflictStatusName::Open, 10)
            .await
            .unwrap(),
        vec![conflict.clone()]
    );
    assert_eq!(
        conflict_repository
            .get_by_id(&mut *tx, &conflict_id)
            .await
            .unwrap(),
        Some(conflict.clone())
    );

    let tombstone = tombstone_repository
        .insert(
            &mut *tx,
            NewTombstone {
                tombstone_id,
                path: &path,
                deleted_revision_id: Some(&revision_id),
                deleted_by: &adapter_id,
                retention_until: Utc::now() + Duration::days(30),
            },
        )
        .await
        .unwrap();
    assert!(tombstone.restored_at.is_none());
    assert_eq!(
        tombstone_repository
            .get_by_id(&mut *tx, tombstone_id)
            .await
            .unwrap(),
        Some(tombstone.clone())
    );
    assert_eq!(
        tombstone_repository
            .list_active(&mut *tx, 10)
            .await
            .unwrap(),
        vec![tombstone.clone()]
    );

    let conflict_created = operation_log
        .append(
            &mut *tx,
            &AppendOperationLogEntry {
                op_id: OperationId::parse("op_01JCONFLICTCREATE").unwrap(),
                adapter_id: adapter_id.clone(),
                kind: OperationKindName::ConflictCreated,
                path: path.clone(),
                revision_id: Some(incoming_revision_id.clone()),
                tombstone_id: None,
                conflict_id: Some(conflict_id.clone()),
            },
        )
        .await
        .unwrap();
    let deleted = operation_log
        .append(
            &mut *tx,
            &AppendOperationLogEntry {
                op_id: OperationId::parse("op_01JDELETE").unwrap(),
                adapter_id: adapter_id.clone(),
                kind: OperationKindName::DeleteFile,
                path: path.clone(),
                revision_id: Some(revision_id.clone()),
                tombstone_id: Some(tombstone_id.to_owned()),
                conflict_id: None,
            },
        )
        .await
        .unwrap();

    let resolved = conflict_repository
        .mark_open_resolved(&mut *tx, &conflict_id, &adapter_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(resolved.status, ConflictStatusName::Resolved.as_str());
    assert!(resolved.resolved_at.is_some());
    assert_eq!(resolved.resolved_by.as_deref(), Some(adapter_id.as_str()));
    assert!(conflict_repository
        .mark_open_ignored(&mut *tx, &conflict_id, &adapter_id)
        .await
        .unwrap()
        .is_none());

    let restored = tombstone_repository
        .mark_restored(&mut *tx, tombstone_id)
        .await
        .unwrap()
        .unwrap();
    assert!(restored.restored_at.is_some());
    assert!(tombstone_repository
        .mark_restored(&mut *tx, tombstone_id)
        .await
        .unwrap()
        .is_none());
    assert!(tombstone_repository
        .list_active(&mut *tx, 10)
        .await
        .unwrap()
        .is_empty());

    let conflict_resolved = operation_log
        .append(
            &mut *tx,
            &AppendOperationLogEntry {
                op_id: OperationId::parse("op_01JCONFLICTRESOLVE").unwrap(),
                adapter_id: adapter_id.clone(),
                kind: OperationKindName::ConflictResolved,
                path: path.clone(),
                revision_id: Some(revision_id.clone()),
                tombstone_id: None,
                conflict_id: Some(conflict_id.clone()),
            },
        )
        .await
        .unwrap();
    let restored_operation = operation_log
        .append(
            &mut *tx,
            &AppendOperationLogEntry {
                op_id: OperationId::parse("op_01JRESTORE").unwrap(),
                adapter_id: adapter_id.clone(),
                kind: OperationKindName::RestoreFile,
                path: path.clone(),
                revision_id: Some(revision_id.clone()),
                tombstone_id: Some(tombstone_id.to_owned()),
                conflict_id: None,
            },
        )
        .await
        .unwrap();

    let loaded_blob = get_content_blob_by_hash(&mut *tx, content_hash)
        .await
        .unwrap()
        .unwrap();
    let loaded_object = get_sync_object_by_path(&mut *tx, &path)
        .await
        .unwrap()
        .unwrap();
    let loaded_revision = get_file_revision_by_id(&mut *tx, &revision_id)
        .await
        .unwrap()
        .unwrap();
    let current_revision = get_current_revision_by_path(&mut *tx, &path)
        .await
        .unwrap()
        .unwrap();
    let loaded_operation = operation_log
        .get_by_operation_id(&mut *tx, &op_id)
        .await
        .unwrap()
        .unwrap();
    let changes = operation_log
        .changes_since(&mut *tx, operation.seq, 10)
        .await
        .unwrap();
    let expected_hash = content_hash.to_prefixed_string();

    assert_eq!(loaded_blob, blob);
    assert_eq!(
        loaded_object.current_revision_id,
        updated_object.current_revision_id
    );
    assert_eq!(loaded_revision, revision);
    assert_eq!(current_revision, revision);
    assert_eq!(loaded_operation.seq, operation.seq);
    assert_eq!(changes.from_seq, operation.seq);
    assert_eq!(changes.to_seq, restored_operation.seq);
    assert!(!changes.has_more);
    assert_eq!(changes.changes.len(), 4);
    assert_eq!(changes.changes[0].seq, conflict_created.seq);
    assert_eq!(
        changes.changes[0].conflict_id.as_deref(),
        Some(conflict_id.as_str())
    );
    assert_eq!(changes.changes[1].seq, deleted.seq);
    assert_eq!(
        changes.changes[1].tombstone_id.as_deref(),
        Some(tombstone_id)
    );
    assert_eq!(changes.changes[2].seq, conflict_resolved.seq);
    assert_eq!(changes.changes[3].seq, restored_operation.seq);
    assert_eq!(
        changes.changes[3].content_sha256.as_deref(),
        Some(expected_hash.as_str())
    );
    assert_eq!(changes.changes[3].size_bytes, Some(42));

    tx.commit().await.unwrap();
    context.clean_storage_tables().await.unwrap();
}
