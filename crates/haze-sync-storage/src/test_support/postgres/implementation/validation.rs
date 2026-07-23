async fn validate_pre_gdrive_schema(
    transaction: &mut Transaction<'_, Postgres>,
) -> PostgresTestResult<()> {
    validate_worktree_schema(transaction).await?;
    if !constraint_exists(transaction, CURSOR_NONNEGATIVE_CONSTRAINT).await? {
        return Err(TestSupportError::IncompleteStorageSchema);
    }
    Ok(())
}

async fn validate_stage10_schema(
    transaction: &mut Transaction<'_, Postgres>,
) -> PostgresTestResult<()> {
    validate_pre_gdrive_schema(transaction).await?;
    validate_table_columns(
        transaction,
        table_names::GDRIVE_ADAPTER_STATE,
        GDRIVE_ADAPTER_STATE_COLUMNS,
    )
    .await?;
    validate_table_columns(
        transaction,
        table_names::GDRIVE_DURABLE_ITEMS,
        GDRIVE_DURABLE_ITEM_COLUMNS,
    )
    .await?;
    validate_table_columns(
        transaction,
        table_names::GDRIVE_OPERATIONS,
        GDRIVE_OPERATION_COLUMNS,
    )
    .await?;
    for constraint in [
        GDRIVE_STATE_VERSION_CONSTRAINT,
        GDRIVE_CURSOR_GENERATION_CONSTRAINT,
        GDRIVE_EXPORT_CHECKPOINT_CONSTRAINT,
        GDRIVE_ECHO_CONSTRAINT,
        GDRIVE_DELETE_CANDIDATE_CONSTRAINT,
    ] {
        if !constraint_exists(transaction, constraint).await? {
            return Err(TestSupportError::IncompleteStorageSchema);
        }
    }
    Ok(())
}

async fn validate_current_schema(
    transaction: &mut Transaction<'_, Postgres>,
) -> PostgresTestResult<()> {
    let tables = owned_storage_table_names(transaction).await?;
    if !same_table_set(&tables, table_names::ALL) {
        return Err(TestSupportError::IncompleteStorageSchema);
    }
    validate_stage10_schema(transaction).await?;
    validate_control_singletons(transaction).await
}

async fn validate_control_singletons(
    transaction: &mut Transaction<'_, Postgres>,
) -> PostgresTestResult<()> {
    let maintenance_count = sqlx::query_scalar::<_, i64>(
        "select count(*)::bigint from maintenance_control where singleton_id = 1",
    )
    .fetch_one(&mut **transaction)
    .await
    .map_err(|_| TestSupportError::DatabaseOperationFailed)?;
    let inventory_count = sqlx::query_scalar::<_, i64>(
        "select count(*)::bigint from adapter_inventory_state where singleton_id = 1",
    )
    .fetch_one(&mut **transaction)
    .await
    .map_err(|_| TestSupportError::DatabaseOperationFailed)?;
    let global_slot_count = sqlx::query_scalar::<_, i64>(
        "select count(*)::bigint from operational_execution_slots \
         where slot_id = 'global-destructive' and slot_kind = 'global_destructive'",
    )
    .fetch_one(&mut **transaction)
    .await
    .map_err(|_| TestSupportError::DatabaseOperationFailed)?;
    if maintenance_count == 1 && inventory_count == 1 && global_slot_count == 1 {
        Ok(())
    } else {
        Err(TestSupportError::IncompleteStorageSchema)
    }
}

async fn validate_worktree_schema(
    transaction: &mut Transaction<'_, Postgres>,
) -> PostgresTestResult<()> {
    validate_table_columns(
        transaction,
        table_names::WORKTREE_INSTANCES,
        WORKTREE_INSTANCE_COLUMNS,
    )
    .await?;
    validate_table_columns(
        transaction,
        table_names::WORKTREE_STATE,
        WORKTREE_STATE_COLUMNS,
    )
    .await
}

async fn acquire_setup_lock(transaction: &mut Transaction<'_, Postgres>) -> PostgresTestResult<()> {
    sqlx::query("select pg_advisory_xact_lock($1)")
        .bind(TEST_SETUP_ADVISORY_LOCK_KEY)
        .execute(&mut **transaction)
        .await
        .map_err(|_| TestSupportError::DatabaseOperationFailed)?;
    Ok(())
}

async fn owned_storage_table_names(
    transaction: &mut Transaction<'_, Postgres>,
) -> PostgresTestResult<Vec<String>> {
    let sql = owned_storage_table_names_sql();
    sqlx::query_scalar::<_, String>(&sql)
        .fetch_all(&mut **transaction)
        .await
        .map_err(|_| TestSupportError::DatabaseOperationFailed)
}

async fn validate_table_columns(
    transaction: &mut Transaction<'_, Postgres>,
    table_name: &str,
    expected: &[&str],
) -> PostgresTestResult<()> {
    let actual = sqlx::query_scalar::<_, String>(
        "select column_name from information_schema.columns \
         where table_schema = current_schema() and table_name = $1 \
         order by ordinal_position",
    )
    .bind(table_name)
    .fetch_all(&mut **transaction)
    .await
    .map_err(|_| TestSupportError::DatabaseOperationFailed)?;
    if actual
        .iter()
        .map(String::as_str)
        .eq(expected.iter().copied())
    {
        Ok(())
    } else {
        Err(TestSupportError::IncompleteStorageSchema)
    }
}

async fn legacy_worktree_state_count(
    transaction: &mut Transaction<'_, Postgres>,
) -> PostgresTestResult<i64> {
    sqlx::query_scalar::<_, i64>("select count(*)::bigint from worktree_state")
        .fetch_one(&mut **transaction)
        .await
        .map_err(|_| TestSupportError::DatabaseOperationFailed)
}

async fn constraint_exists(
    transaction: &mut Transaction<'_, Postgres>,
    constraint_name: &str,
) -> PostgresTestResult<bool> {
    sqlx::query_scalar::<_, bool>(
        "select exists ( \
             select 1 from pg_constraint \
             where connamespace = current_schema()::regnamespace and conname = $1 \
         )",
    )
    .bind(constraint_name)
    .fetch_one(&mut **transaction)
    .await
    .map_err(|_| TestSupportError::DatabaseOperationFailed)
}
