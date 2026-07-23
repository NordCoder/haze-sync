#![cfg(feature = "test-support")]

use haze_sync_storage::test_support::{
    connect_required_test_database_from_env, unique_test_id, STORAGE_TEST_MIGRATIONS,
};
use sqlx::Executor;

const INCOMPATIBLE_GUARD_MESSAGE: &str =
    "gdrive durable state structures require explicit operator review";

#[tokio::test]
#[ignore = "requires explicit HAZE_SYNC_TEST_DATABASE_URL for direct STOR-GDA-P1 migration evidence"]
async fn migration_0011_sql_guard_preserves_incompatible_preexisting_state() {
    let context = connect_required_test_database_from_env().await.unwrap();
    let mut transaction = context.pool().begin().await.unwrap();
    let schema = safe_schema_name("stor_gda_direct_guard");

    sqlx::query(&format!("create schema {schema}"))
        .execute(&mut *transaction)
        .await
        .unwrap();
    sqlx::query(&format!("set local search_path to {schema}"))
        .execute(&mut *transaction)
        .await
        .unwrap();

    for migration in &STORAGE_TEST_MIGRATIONS[..10] {
        (&mut *transaction).execute(migration.sql).await.unwrap();
    }
    sqlx::query("create table gdrive_adapter_state (sentinel text primary key)")
        .execute(&mut *transaction)
        .await
        .unwrap();
    sqlx::query("insert into gdrive_adapter_state (sentinel) values ('preserve-me')")
        .execute(&mut *transaction)
        .await
        .unwrap();

    sqlx::query("savepoint stor_gda_migration_guard")
        .execute(&mut *transaction)
        .await
        .unwrap();
    let migration = STORAGE_TEST_MIGRATIONS
        .iter()
        .find(|migration| migration.name == "0011_gdrive_durable_state.sql")
        .unwrap();
    let error = (&mut *transaction)
        .execute(migration.sql)
        .await
        .expect_err("migration 0011 must reject incompatible pre-existing state");
    let guard_fired = error
        .as_database_error()
        .is_some_and(|database_error| database_error.message() == INCOMPATIBLE_GUARD_MESSAGE);
    assert!(
        guard_fired,
        "migration must fail through the explicit structure guard"
    );

    sqlx::query("rollback to savepoint stor_gda_migration_guard")
        .execute(&mut *transaction)
        .await
        .unwrap();

    let rows = sqlx::query_scalar::<_, i64>(
        "select count(*)::bigint from gdrive_adapter_state where sentinel = 'preserve-me'",
    )
    .fetch_one(&mut *transaction)
    .await
    .unwrap();
    assert_eq!(rows, 1);

    let columns = sqlx::query_scalar::<_, String>(
        "select column_name from information_schema.columns \
         where table_schema = current_schema() and table_name = 'gdrive_adapter_state' \
         order by ordinal_position",
    )
    .fetch_all(&mut *transaction)
    .await
    .unwrap();
    assert_eq!(columns, vec!["sentinel".to_owned()]);

    for table_name in ["gdrive_durable_items", "gdrive_operations"] {
        let created = sqlx::query_scalar::<_, bool>(
            "select to_regclass(current_schema() || '.' || $1) is not null",
        )
        .bind(table_name)
        .fetch_one(&mut *transaction)
        .await
        .unwrap();
        assert!(!created);
    }

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
