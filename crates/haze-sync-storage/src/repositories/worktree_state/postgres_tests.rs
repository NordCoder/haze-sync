use super::*;
use crate::test_support::connect_test_database_from_env;
use chrono::TimeZone;

#[tokio::test]
async fn worktree_state_roundtrips_in_caller_owned_transaction() {
    let Some(context) = connect_test_database_from_env().await.unwrap() else {
        return;
    };
    context.apply_migrations().await.unwrap();

    let path = VaultPath::parse(&context.namespace().vault_path("worktree-state.md")).unwrap();
    let hash = Sha256::parse(&"b".repeat(64)).unwrap();
    let seen_mtime = Utc.timestamp_opt(1_700_001_000, 0).unwrap();
    let scanned_at = Utc.timestamp_opt(1_700_001_100, 0).unwrap();
    let mut transaction = context.pool().begin().await.unwrap();

    let inserted = upsert_worktree_state(
        &mut *transaction,
        WorktreeStateUpsert {
            path: &path,
            last_applied_revision_id: None,
            last_seen_sha256: Some(&hash),
            last_seen_mtime: Some(seen_mtime),
            dirty: true,
            last_scanned_at: Some(scanned_at),
            last_written_by_adapter: false,
        },
    )
    .await
    .unwrap();

    assert_eq!(inserted.path, path.as_str());
    assert_eq!(inserted.last_seen_sha256, Some(hash.to_string()));
    assert!(inserted.dirty);
    assert!(!inserted.last_written_by_adapter);
    assert_eq!(
        get_worktree_state_by_path(&mut *transaction, &path)
            .await
            .unwrap(),
        Some(inserted)
    );

    let updated = upsert_worktree_state(
        &mut *transaction,
        WorktreeStateUpsert {
            path: &path,
            last_applied_revision_id: None,
            last_seen_sha256: Some(&hash),
            last_seen_mtime: Some(seen_mtime),
            dirty: false,
            last_scanned_at: Some(scanned_at),
            last_written_by_adapter: true,
        },
    )
    .await
    .unwrap();

    assert!(!updated.dirty);
    assert!(updated.last_written_by_adapter);
    assert_eq!(updated.last_seen_sha256, Some(hash.to_string()));

    transaction.rollback().await.unwrap();
}
