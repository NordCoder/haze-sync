use super::*;

#[test]
fn operation_kind_names_match_contract_strings() {
    assert_eq!(OperationKindName::UpsertFile.as_str(), "upsert_file");
    assert_eq!(OperationKindName::DeleteFile.as_str(), "delete_file");
    assert_eq!(OperationKindName::RestoreFile.as_str(), "restore_file");
    assert_eq!(
        OperationKindName::ConflictCreated.as_str(),
        "conflict_created"
    );
    assert_eq!(
        OperationKindName::ConflictResolved.as_str(),
        "conflict_resolved"
    );
    assert_eq!(OperationKindName::BackupCreated.as_str(), "backup_created");
}

#[test]
fn operation_kind_roundtrips_json() {
    let json = serde_json::to_string(&OperationKindName::ConflictCreated).unwrap();
    assert_eq!(json, "\"conflict_created\"");
    assert_eq!(
        serde_json::from_str::<OperationKindName>(&json).unwrap(),
        OperationKindName::ConflictCreated
    );
    assert_eq!(
        OperationKindName::from_str("overwrite_file"),
        Err(RepositoryError::InvalidOperationKind)
    );
}

#[test]
fn persisted_operation_kinds_reject_unknown_values_safely() {
    assert_eq!(
        validated_persisted_operation_kind("delete_file".to_owned()),
        Ok("delete_file".to_owned())
    );
    assert_eq!(
        validated_persisted_operation_kind("overwrite_file".to_owned()),
        Err(RepositoryError::InvalidOperationKind)
    );
}

#[test]
fn append_entry_carries_revision_metadata_without_policy_outcome() {
    let path = VaultPath::parse("Notes/today.md").unwrap();
    let revision_id = RevisionId::parse("rev_01JSTORP5").unwrap();
    let entry = AppendOperationLogEntry {
        op_id: OperationId::parse("op_01JSTORP5").unwrap(),
        adapter_id: AdapterId::parse("worktree-adapter").unwrap(),
        kind: OperationKindName::UpsertFile,
        path: path.clone(),
        revision_id: Some(revision_id.clone()),
        tombstone_id: None,
        conflict_id: None,
    };

    assert_eq!(entry.kind.as_str(), "upsert_file");
    assert_eq!(entry.path, path);
    assert_eq!(entry.revision_id.as_ref(), Some(&revision_id));
    assert!(entry.tombstone_id.is_none());
    assert!(entry.conflict_id.is_none());
}

#[test]
fn conflict_delete_and_restore_entries_preserve_reference_metadata() {
    let path = VaultPath::parse("Notes/today.md").unwrap();
    let adapter_id = AdapterId::parse("worktree-adapter").unwrap();
    let revision_id = RevisionId::parse("rev_01JSTORP6").unwrap();
    let conflict_id = ConflictId::parse("conf_01JSTORP6").unwrap();

    let delete = AppendOperationLogEntry {
        op_id: OperationId::parse("op_01JDELETE").unwrap(),
        adapter_id: adapter_id.clone(),
        kind: OperationKindName::DeleteFile,
        path: path.clone(),
        revision_id: Some(revision_id.clone()),
        tombstone_id: Some("tmb_01JSTORP6".to_owned()),
        conflict_id: None,
    };
    let restore = AppendOperationLogEntry {
        op_id: OperationId::parse("op_01JRESTORE").unwrap(),
        adapter_id: adapter_id.clone(),
        kind: OperationKindName::RestoreFile,
        path: path.clone(),
        revision_id: Some(revision_id.clone()),
        tombstone_id: Some("tmb_01JSTORP6".to_owned()),
        conflict_id: None,
    };
    let conflict = AppendOperationLogEntry {
        op_id: OperationId::parse("op_01JCONFLICT").unwrap(),
        adapter_id,
        kind: OperationKindName::ConflictCreated,
        path,
        revision_id: Some(revision_id),
        tombstone_id: None,
        conflict_id: Some(conflict_id.clone()),
    };

    assert_eq!(delete.tombstone_id.as_deref(), Some("tmb_01JSTORP6"));
    assert_eq!(restore.tombstone_id, delete.tombstone_id);
    assert_eq!(conflict.conflict_id.as_ref(), Some(&conflict_id));
    assert!(conflict.tombstone_id.is_none());
}

#[test]
fn persisted_sizes_reject_negative_values_safely() {
    assert_eq!(validated_persisted_size_bytes(None), Ok(None));
    assert_eq!(validated_persisted_size_bytes(Some(0)), Ok(Some(0)));
    assert_eq!(validated_persisted_size_bytes(Some(42)), Ok(Some(42)));
    assert_eq!(
        validated_persisted_size_bytes(Some(-1)),
        Err(RepositoryError::InvalidSizeBytes)
    );
}

#[test]
fn change_feed_page_uses_one_sentinel_row_for_has_more() {
    let page = change_feed_page_from_rows(10, 2, vec![change(11), change(12), change(13)]);

    assert_eq!(page.from_seq, 10);
    assert_eq!(page.to_seq, 12);
    assert!(page.has_more);
    assert_eq!(page.changes.len(), 2);
    assert_eq!(page.changes[0].seq, 11);
    assert_eq!(page.changes[1].seq, 12);
}

#[test]
fn empty_change_feed_page_keeps_cursor_at_requested_sequence() {
    let page = change_feed_page_from_rows(10, 5, Vec::new());

    assert_eq!(page.from_seq, 10);
    assert_eq!(page.to_seq, 10);
    assert!(!page.has_more);
    assert!(page.changes.is_empty());
}

fn change(seq: i64) -> ChangeFeedRow {
    ChangeFeedRow {
        seq,
        op_id: format!("op_01JSTORP5{seq}"),
        adapter_id: "worktree-adapter".to_owned(),
        kind: OperationKindName::UpsertFile.as_str().to_owned(),
        path: "Notes/today.md".to_owned(),
        revision_id: Some(format!("rev_01JSTORP5{seq}")),
        content_sha256: Some(
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .to_owned(),
        ),
        size_bytes: Some(42),
        tombstone_id: None,
        conflict_id: None,
        created_at: DateTime::<Utc>::from(std::time::UNIX_EPOCH),
    }
}
