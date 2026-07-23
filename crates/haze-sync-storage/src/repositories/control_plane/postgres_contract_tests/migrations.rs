#[tokio::test]
async fn fresh_schema_applies_complete_control_migration() {
    let context = connect_required_test_database_from_env().await.unwrap();
    let mut transaction = begin_isolated_schema(context.pool()).await;
    apply_all(&mut transaction).await;

    let facts = read_storage_facts(&mut transaction).await.unwrap();
    assert_eq!(
        facts.current_control_table_count,
        crate::schema::control_plane::table_names::ALL.len() as i64
    );
    assert_eq!(facts.principal_count, 0);
    assert_eq!(facts.credential_count, 0);
    assert_eq!(facts.highest_operation_sequence, 0);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn sequential_migration_preserves_existing_storage_and_maps_legacy_credentials() {
    let context = connect_required_test_database_from_env().await.unwrap();
    let mut transaction = begin_isolated_schema(context.pool()).await;
    apply_base(&mut transaction).await;
    seed_legacy_and_existing_storage(&mut transaction).await;
    apply_control(&mut transaction)
        .await
        .expect("sequential control migrations");

    let preserved: (i64, i64, i64, i64, i64, i64) = sqlx::query_as(
        "select (select count(*) from content_blobs), (select count(*) from file_revisions), \
         (select count(*) from adapter_cursors), (select count(*) from idempotency_records), \
         (select count(*) from worktree_state), (select count(*) from gdrive_durable_items)",
    )
    .fetch_one(&mut *transaction)
    .await
    .unwrap();
    assert_eq!(preserved, (1, 1, 1, 1, 1, 1));

    let principals: Vec<(String, String, bool, i64, i64, bool)> = sqlx::query_as(
        "select principal_id, role, principal_enabled, principal_version, \
         credential_set_generation, disabled_at is not null from principals order by principal_id",
    )
    .fetch_all(&mut *transaction)
    .await
    .unwrap();
    assert_eq!(
        principals,
        vec![
            ("gdrive-a".into(), "gdrive_adapter".into(), false, 1, 1, true),
            ("worktree-a".into(), "worktree_adapter".into(), true, 1, 1, false),
        ]
    );
    let credentials: Vec<(String, String, String, String, bool)> = sqlx::query_as(
        "select credential_id, principal_id, verifier_scheme, verifier_material, legacy_migrated \
         from credentials order by principal_id",
    )
    .fetch_all(&mut *transaction)
    .await
    .unwrap();
    assert_eq!(credentials.len(), 2);
    assert!(credentials.iter().all(|row| row.0.starts_with("legacy-")));
    assert_eq!(credentials[0].2, "legacy_sha256_v0");
    assert_eq!(credentials[0].3, "2".repeat(64));
    assert!(credentials.iter().all(|row| row.4));
    let credential_role_column: i64 = sqlx::query_scalar(
        "select count(*)::bigint from information_schema.columns \
         where table_schema = current_schema() and table_name = 'credentials' and column_name = 'role'",
    )
    .fetch_one(&mut *transaction)
    .await
    .unwrap();
    assert_eq!(credential_role_column, 0);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn malformed_or_ambiguous_legacy_input_rolls_back_the_whole_forward_migration() {
    let context = connect_required_test_database_from_env().await.unwrap();
    let mut transaction = begin_isolated_schema(context.pool()).await;
    apply_base(&mut transaction).await;
    sqlx::query(
        "insert into sync_adapters (adapter_id,display_name,role,token_hash,enabled) values \
         ('a','A','adapter',$1,true),('b','B','adapter',$1,true)",
    )
    .bind("f".repeat(64))
    .execute(&mut *transaction)
    .await
    .unwrap();
    (&mut *transaction).execute("savepoint before_control").await.unwrap();
    let error = apply_control(&mut transaction).await;
    assert!(error.is_err());
    (&mut *transaction)
        .execute("rollback to savepoint before_control")
        .await
        .unwrap();
    let created: i64 = sqlx::query_scalar(
        "select count(*)::bigint from information_schema.tables where table_schema = current_schema() \
         and table_name = 'maintenance_control'",
    )
    .fetch_one(&mut *transaction)
    .await
    .unwrap();
    assert_eq!(created, 0);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("select count(*)::bigint from sync_adapters")
            .fetch_one(&mut *transaction)
            .await
            .unwrap(),
        2
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn explicit_transaction_rollback_removes_control_ddl_without_touching_base_data() {
    let context = connect_required_test_database_from_env().await.unwrap();
    let mut transaction = begin_isolated_schema(context.pool()).await;
    apply_base(&mut transaction).await;
    sqlx::query(
        "insert into sync_adapters (adapter_id,display_name,role,token_hash,enabled) \
         values ('adapter-a','Adapter A','adapter',$1,true)",
    )
    .bind("e".repeat(64))
    .execute(&mut *transaction)
    .await
    .unwrap();
    (&mut *transaction).execute("savepoint before_control").await.unwrap();
    apply_control(&mut transaction).await.unwrap();
    (&mut *transaction)
        .execute("rollback to savepoint before_control")
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("select count(*)::bigint from sync_adapters")
            .fetch_one(&mut *transaction)
            .await
            .unwrap(),
        1
    );
    let created: i64 = sqlx::query_scalar(
        "select count(*)::bigint from information_schema.tables where table_schema = current_schema() \
         and table_name = any($1)",
    )
    .bind(
        &crate::schema::control_plane::table_names::ALL
            .iter()
            .map(|name| (*name).to_owned())
            .collect::<Vec<_>>(),
    )
    .fetch_one(&mut *transaction)
    .await
    .unwrap();
    assert_eq!(created, 0);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn migration_replay_fails_deterministically_without_changing_committed_rows() {
    let context = connect_required_test_database_from_env().await.unwrap();
    let mut transaction = begin_isolated_schema(context.pool()).await;
    apply_base(&mut transaction).await;
    sqlx::query(
        "insert into sync_adapters (adapter_id,display_name,role,token_hash,enabled) \
         values ('adapter-a','Adapter A','adapter',$1,true)",
    )
    .bind("e".repeat(64))
    .execute(&mut *transaction)
    .await
    .unwrap();
    apply_control(&mut transaction).await.unwrap();
    (&mut *transaction).execute("savepoint before_replay").await.unwrap();
    assert!(apply_control(&mut transaction).await.is_err());
    (&mut *transaction)
        .execute("rollback to savepoint before_replay")
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("select count(*)::bigint from principals")
            .fetch_one(&mut *transaction)
            .await
            .unwrap(),
        1
    );
    transaction.rollback().await.unwrap();
}
