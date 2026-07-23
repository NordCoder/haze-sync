#[test]
fn embedded_migrations_match_schema_metadata() {
    let actual_names: Vec<_> = STORAGE_TEST_MIGRATIONS
        .iter()
        .map(|migration| migration.name)
        .collect();
    assert_eq!(actual_names.as_slice(), INITIAL_MIGRATIONS);
    assert!(STORAGE_TEST_MIGRATIONS.iter().all(|migration| {
        migration.sql.contains("create table") || migration.sql.contains("alter table")
    }));
}

#[test]
fn schema_probe_covers_current_and_accepted_pre_state_tables() {
    let sql = owned_storage_table_names_sql();
    for table_name in table_names::ALL {
        assert!(sql.contains(table_name));
    }
    for table_name in table_names::PRE_STOR_GDA_P11 {
        assert!(sql.contains(table_name));
    }
    for table_name in table_names::PRE_STOR_P10 {
        assert!(sql.contains(table_name));
    }
    assert!(sql.contains("current_schema()"));
    assert!(sql.contains("table_type = 'BASE TABLE'"));
}

#[test]
fn cleanup_sql_covers_every_current_owned_table_without_drop_statements() {
    let sql = clean_storage_tables_sql().to_ascii_lowercase();
    for table_name in table_names::ALL {
        assert!(sql.contains(table_name));
    }
    assert!(sql.starts_with("truncate table"));
    assert!(!sql.contains("drop database"));
    assert!(!sql.contains("drop schema"));
}

#[test]
fn stor_p10_migration_guards_legacy_rows_before_drop() {
    let migration = STORAGE_TEST_MIGRATIONS
        .iter()
        .find(|migration| migration.name == "0010_worktree_durable_state.sql")
        .unwrap()
        .sql
        .to_ascii_lowercase();
    let guard = migration
        .find("if exists (select 1 from worktree_state limit 1)")
        .unwrap();
    let drop_table = migration.find("drop table worktree_state").unwrap();
    assert!(guard < drop_table);
    assert!(migration.contains("create table worktree_instances"));
    assert!(migration.contains("primary key (adapter_id, path)"));
    assert!(migration.contains("adapter_cursors_last_core_seq_nonnegative"));
}

#[test]
fn stor_gda_p11_migration_guards_preexisting_structures_before_create() {
    let migration = STORAGE_TEST_MIGRATIONS
        .iter()
        .find(|migration| migration.name == "0011_gdrive_durable_state.sql")
        .unwrap()
        .sql
        .to_ascii_lowercase();
    let guard = migration.find("if to_regclass").unwrap();
    let first_create = migration.find("create table gdrive_adapter_state").unwrap();
    assert!(guard < first_create);
    assert!(migration.contains("create table gdrive_durable_items"));
    assert!(migration.contains("create table gdrive_operations"));
    assert!(!migration.contains("drop table"));
}

#[tokio::test]
#[ignore = "requires explicit HAZE_SYNC_TEST_DATABASE_URL and exclusive migration evidence"]
async fn migrates_empty_pre_p10_schema_and_rejects_nonempty_legacy_state() {
    let context = connect_required_test_database_from_env().await.unwrap();
    let mut transaction = context.pool().begin().await.unwrap();

    let empty_schema = safe_schema_name("stor_p10_empty");
    create_and_select_schema(&mut transaction, &empty_schema).await;
    execute_migrations(&mut transaction, &STORAGE_TEST_MIGRATIONS[..9])
        .await
        .unwrap();
    prepare_storage_schema(&mut transaction).await.unwrap();
    validate_current_schema(&mut transaction).await.unwrap();

    let legacy_schema = safe_schema_name("stor_p10_legacy");
    create_and_select_schema(&mut transaction, &legacy_schema).await;
    execute_migrations(&mut transaction, &STORAGE_TEST_MIGRATIONS[..9])
        .await
        .unwrap();
    sqlx::query("insert into worktree_state (path) values ('Notes/legacy.md')")
        .execute(&mut *transaction)
        .await
        .unwrap();
    assert_eq!(
        prepare_storage_schema(&mut transaction).await,
        Err(TestSupportError::LegacyWorktreeStateNotEmpty)
    );
    assert_eq!(
        legacy_worktree_state_count(&mut transaction).await.unwrap(),
        1
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
#[ignore = "requires explicit HAZE_SYNC_TEST_DATABASE_URL and accepted pre-state migration evidence"]
async fn migrates_accepted_pre_gdrive_state_without_data_loss() {
    let context = connect_required_test_database_from_env().await.unwrap();
    let mut transaction = context.pool().begin().await.unwrap();
    let schema = safe_schema_name("stor_gda_pre_state");
    create_and_select_schema(&mut transaction, &schema).await;
    execute_migrations(&mut transaction, &STORAGE_TEST_MIGRATIONS[..10])
        .await
        .unwrap();
    sqlx::query(
        "insert into gdrive_mapping (path, drive_file_id) \
         values ('Notes/existing.md', 'existing-drive-id')",
    )
    .execute(&mut *transaction)
    .await
    .unwrap();

    prepare_storage_schema(&mut transaction).await.unwrap();
    validate_current_schema(&mut transaction).await.unwrap();
    let preserved = sqlx::query_scalar::<_, i64>(
        "select count(*)::bigint from gdrive_mapping \
         where path = 'Notes/existing.md' and drive_file_id = 'existing-drive-id'",
    )
    .fetch_one(&mut *transaction)
    .await
    .unwrap();
    assert_eq!(preserved, 1);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
#[ignore = "requires explicit HAZE_SYNC_TEST_DATABASE_URL and exclusive migration evidence"]
async fn fresh_and_current_schema_preparation_is_idempotent() {
    let context = connect_required_test_database_from_env().await.unwrap();
    let mut transaction = context.pool().begin().await.unwrap();
    let schema = safe_schema_name("stor_gda_current");
    create_and_select_schema(&mut transaction, &schema).await;
    prepare_storage_schema(&mut transaction).await.unwrap();
    prepare_storage_schema(&mut transaction).await.unwrap();
    validate_current_schema(&mut transaction).await.unwrap();
    transaction.rollback().await.unwrap();
}

async fn create_and_select_schema(transaction: &mut Transaction<'_, Postgres>, schema: &str) {
    let create = format!("create schema {schema}");
    sqlx::query(&create)
        .execute(&mut **transaction)
        .await
        .unwrap();
    let select = format!("set local search_path to {schema}");
    sqlx::query(&select)
        .execute(&mut **transaction)
        .await
        .unwrap();
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
