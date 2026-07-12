#![cfg(feature = "test-support")]

use haze_sync_storage::test_support::{
    connect_required_test_database_from_env, unique_test_id, STORAGE_TEST_MIGRATIONS,
};
use sqlx::Executor;

const LEGACY_GUARD_MESSAGE: &str = "legacy worktree_state rows require explicit operator migration";
const LEGACY_WORKTREE_STATE_COLUMNS: &[&str] = &[
    "path",
    "last_applied_revision_id",
    "last_seen_sha256",
    "last_seen_mtime",
    "dirty",
    "last_scanned_at",
    "last_written_by_adapter",
];

#[tokio::test]
#[ignore = "requires explicit HAZE_SYNC_TEST_DATABASE_URL for direct STOR-P10 migration evidence"]
async fn migration_0010_sql_guard_preserves_nonempty_legacy_state() {
    let context = connect_required_test_database_from_env().await.unwrap();
    let mut transaction = context.pool().begin().await.unwrap();
    let schema = safe_schema_name("stor_p10_direct_guard");

    sqlx::query(&format!("create schema {schema}"))
        .execute(&mut *transaction)
        .await
        .unwrap();
    sqlx::query(&format!("set local search_path to {schema}"))
        .execute(&mut *transaction)
        .await
        .unwrap();

    for migration in &STORAGE_TEST_MIGRATIONS[..9] {
        (&mut *transaction).execute(migration.sql).await.unwrap();
    }
    sqlx::query("insert into worktree_state (path) values ('Notes/legacy.md')")
        .execute(&mut *transaction)
        .await
        .unwrap();

    sqlx::query("savepoint stor_p10_migration_guard")
        .execute(&mut *transaction)
        .await
        .unwrap();
    let error = (&mut *transaction)
        .execute(STORAGE_TEST_MIGRATIONS[9].sql)
        .await
        .expect_err("migration 0010 must reject non-empty legacy Worktree state");
    let guard_fired = error
        .as_database_error()
        .is_some_and(|database_error| database_error.message() == LEGACY_GUARD_MESSAGE);
    assert!(
        guard_fired,
        "migration must fail through the explicit legacy-row guard"
    );

    sqlx::query("rollback to savepoint stor_p10_migration_guard")
        .execute(&mut *transaction)
        .await
        .unwrap();

    let legacy_rows = sqlx::query_scalar::<_, i64>(
        "select count(*)::bigint from worktree_state where path = 'Notes/legacy.md'",
    )
    .fetch_one(&mut *transaction)
    .await
    .unwrap();
    assert_eq!(legacy_rows, 1);

    let columns = sqlx::query_scalar::<_, String>(
        "select column_name from information_schema.columns \
         where table_schema = current_schema() and table_name = 'worktree_state' \
         order by ordinal_position",
    )
    .fetch_all(&mut *transaction)
    .await
    .unwrap();
    let column_names = columns.iter().map(String::as_str).collect::<Vec<_>>();
    assert_eq!(column_names.as_slice(), LEGACY_WORKTREE_STATE_COLUMNS);

    let instances_created = sqlx::query_scalar::<_, bool>(
        "select to_regclass(current_schema() || '.worktree_instances') is not null",
    )
    .fetch_one(&mut *transaction)
    .await
    .unwrap();
    assert!(!instances_created);

    transaction.rollback().await.unwrap();
}

fn safe_schema_name(prefix: &str) -> String {
    unique_test_id(prefix)
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '_'
            }
        })
        .take(63)
        .collect()
}
