pub async fn apply_storage_migrations(pool: &PgPool) -> PostgresTestResult<()> {
    let mut transaction = pool
        .begin()
        .await
        .map_err(|_| TestSupportError::DatabaseOperationFailed)?;
    prepare_storage_schema(&mut transaction).await?;
    transaction
        .commit()
        .await
        .map_err(|_| TestSupportError::DatabaseOperationFailed)
}

pub async fn clean_storage_tables(pool: &PgPool) -> PostgresTestResult<()> {
    let mut transaction = pool
        .begin()
        .await
        .map_err(|_| TestSupportError::DatabaseOperationFailed)?;
    acquire_setup_lock(&mut transaction).await?;
    validate_current_schema(&mut transaction).await?;
    let cleanup_sql = clean_storage_tables_sql();
    (&mut *transaction)
        .execute(cleanup_sql.as_str())
        .await
        .map_err(|_| TestSupportError::DatabaseOperationFailed)?;
    transaction
        .commit()
        .await
        .map_err(|_| TestSupportError::DatabaseOperationFailed)
}

async fn prepare_storage_schema(
    transaction: &mut Transaction<'_, Postgres>,
) -> PostgresTestResult<()> {
    acquire_setup_lock(transaction).await?;
    let tables = owned_storage_table_names(transaction).await?;

    if tables.is_empty() {
        execute_migrations(transaction, STORAGE_TEST_MIGRATIONS).await?;
    } else if same_table_set(&tables, table_names::ALL) {
        validate_current_schema(transaction).await?;
        return Ok(());
    } else if same_table_set(&tables, table_names::PRE_CONTROL_P12) {
        validate_stage10_schema(transaction).await?;
        execute_named_migrations(
            transaction,
            &[
                "0012_operational_control_storage.sql",
                "0013_operational_jobs_audit.sql",
                "0014_gdrive_runtime_authority.sql",
                "0015_qa_contract_corrections.sql",
            ],
        )
        .await?;
    } else if same_table_set(&tables, table_names::PRE_STOR_GDA_P11) {
        validate_pre_gdrive_schema(transaction).await?;
        execute_named_migrations(
            transaction,
            &[
                "0011_gdrive_durable_state.sql",
                "0012_operational_control_storage.sql",
                "0013_operational_jobs_audit.sql",
                "0014_gdrive_runtime_authority.sql",
                "0015_qa_contract_corrections.sql",
            ],
        )
        .await?;
    } else if same_table_set(&tables, table_names::PRE_STOR_P10) {
        validate_table_columns(
            transaction,
            table_names::WORKTREE_STATE,
            PRE_STOR_P10_WORKTREE_STATE_COLUMNS,
        )
        .await?;
        if legacy_worktree_state_count(transaction).await? != 0 {
            return Err(TestSupportError::LegacyWorktreeStateNotEmpty);
        }
        execute_named_migrations(
            transaction,
            &[
                "0010_worktree_durable_state.sql",
                "0011_gdrive_durable_state.sql",
                "0012_operational_control_storage.sql",
                "0013_operational_jobs_audit.sql",
                "0014_gdrive_runtime_authority.sql",
                "0015_qa_contract_corrections.sql",
            ],
        )
        .await?;
    } else {
        return Err(TestSupportError::IncompleteStorageSchema);
    }

    validate_current_schema(transaction).await
}

async fn execute_named_migrations(
    transaction: &mut Transaction<'_, Postgres>,
    names: &[&str],
) -> PostgresTestResult<()> {
    for name in names {
        let migration = STORAGE_TEST_MIGRATIONS
            .iter()
            .find(|migration| migration.name == *name)
            .ok_or(TestSupportError::IncompleteStorageSchema)?;
        (&mut **transaction)
            .execute(migration.sql)
            .await
            .map_err(|_| TestSupportError::DatabaseOperationFailed)?;
    }
    Ok(())
}

async fn execute_migrations(
    transaction: &mut Transaction<'_, Postgres>,
    migrations: &[TestMigration],
) -> PostgresTestResult<()> {
    for migration in migrations {
        (&mut **transaction)
            .execute(migration.sql)
            .await
            .map_err(|_| TestSupportError::DatabaseOperationFailed)?;
    }
    Ok(())
}
