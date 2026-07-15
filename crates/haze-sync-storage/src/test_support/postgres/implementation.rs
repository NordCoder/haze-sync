const TEST_SETUP_ADVISORY_LOCK_KEY: i64 = 7_252_953_924_883_537_409;

const PRE_STOR_P10_WORKTREE_STATE_COLUMNS: &[&str] = &[
    "path",
    "last_applied_revision_id",
    "last_seen_sha256",
    "last_seen_mtime",
    "dirty",
    "last_scanned_at",
    "last_written_by_adapter",
];
const WORKTREE_INSTANCE_COLUMNS: &[&str] = &[
    "adapter_id",
    "root_fingerprint",
    "state_format_version",
    "created_at",
    "updated_at",
];
const WORKTREE_STATE_COLUMNS: &[&str] = &[
    "adapter_id",
    "path",
    "state_kind",
    "state_format_version",
    "last_applied_revision_id",
    "content_sha256",
    "observation_schema_version",
    "observed_size_bytes",
    "observed_mtime",
    "created_at",
    "updated_at",
];
const GDRIVE_ADAPTER_STATE_COLUMNS: &[&str] = &[
    "adapter_id",
    "state_format_version",
    "state_version",
    "drive_cursor",
    "drive_cursor_generation",
    "core_export_seq",
    "last_import_operation_id",
    "last_export_operation_id",
    "last_provider_mutation_operation_id",
    "created_at",
    "updated_at",
];
const GDRIVE_DURABLE_ITEM_COLUMNS: &[&str] = &[
    "adapter_id",
    "path",
    "drive_file_id",
    "drive_parent_id",
    "drive_name",
    "mime_type",
    "md5_checksum",
    "head_revision_id",
    "drive_version",
    "drive_modified_time",
    "core_object_id",
    "core_revision_id",
    "core_seq",
    "echo_state",
    "echo_operation_id",
    "echo_provider_version",
    "delete_candidate_first_seen_at",
    "delete_candidate_last_seen_at",
    "delete_candidate_generation",
    "delete_candidate_blocked",
    "delete_confirmation_audit_id",
    "last_imported_at",
    "last_exported_at",
    "last_seen_at",
    "created_at",
    "updated_at",
];
const GDRIVE_OPERATION_COLUMNS: &[&str] = &[
    "adapter_id",
    "operation_id",
    "operation_kind",
    "facts_hash",
    "outcome_kind",
    "committed_state_version",
    "mapping_path",
    "core_seq",
    "drive_version",
    "created_at",
];

const CURSOR_NONNEGATIVE_CONSTRAINT: &str = "adapter_cursors_last_core_seq_nonnegative";
const GDRIVE_STATE_VERSION_CONSTRAINT: &str = "gdrive_adapter_state_version_nonnegative";
const GDRIVE_CURSOR_GENERATION_CONSTRAINT: &str =
    "gdrive_adapter_cursor_generation_nonnegative";
const GDRIVE_EXPORT_CHECKPOINT_CONSTRAINT: &str =
    "gdrive_adapter_core_export_seq_nonnegative";
const GDRIVE_ECHO_CONSTRAINT: &str = "gdrive_durable_items_echo_consistent";
const GDRIVE_DELETE_CANDIDATE_CONSTRAINT: &str =
    "gdrive_durable_items_delete_candidate_consistent";

type PostgresTestResult<T> = Result<T, TestSupportError>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TestMigration {
    pub name: &'static str,
    pub sql: &'static str,
}

pub const STORAGE_TEST_MIGRATIONS: &[TestMigration] = &[
    TestMigration {
        name: "0001_sync_adapters.sql",
        sql: include_str!("../../../../migrations/0001_sync_adapters.sql"),
    },
    TestMigration {
        name: "0002_content_blobs.sql",
        sql: include_str!("../../../../migrations/0002_content_blobs.sql"),
    },
    TestMigration {
        name: "0003_sync_objects_file_revisions.sql",
        sql: include_str!("../../../../migrations/0003_sync_objects_file_revisions.sql"),
    },
    TestMigration {
        name: "0004_operation_log.sql",
        sql: include_str!("../../../../migrations/0004_operation_log.sql"),
    },
    TestMigration {
        name: "0005_tombstones_conflicts.sql",
        sql: include_str!("../../../../migrations/0005_tombstones_conflicts.sql"),
    },
    TestMigration {
        name: "0006_cursors_idempotency.sql",
        sql: include_str!("../../../../migrations/0006_cursors_idempotency.sql"),
    },
    TestMigration {
        name: "0007_gdrive_mapping.sql",
        sql: include_str!("../../../../migrations/0007_gdrive_mapping.sql"),
    },
    TestMigration {
        name: "0008_worktree_state.sql",
        sql: include_str!("../../../../migrations/0008_worktree_state.sql"),
    },
    TestMigration {
        name: "0009_audit_events.sql",
        sql: include_str!("../../../../migrations/0009_audit_events.sql"),
    },
    TestMigration {
        name: "0010_worktree_durable_state.sql",
        sql: include_str!("../../../../migrations/0010_worktree_durable_state.sql"),
    },
    TestMigration {
        name: "0011_gdrive_durable_state.sql",
        sql: include_str!("../../../../migrations/0011_gdrive_durable_state.sql"),
    },
];

pub struct PostgresTestContext {
    url: TestDatabaseUrl,
    pool: PgPool,
    namespace: TestNamespace,
}

impl PostgresTestContext {
    /// Optional compatibility path for tests explicitly named as optional.
    pub async fn connect_from_env() -> PostgresTestResult<Option<Self>> {
        let Some(url) = TestDatabaseUrl::from_env()? else {
            return Ok(None);
        };
        Self::connect(url).await.map(Some)
    }

    pub async fn connect_required_from_env() -> PostgresTestResult<Self> {
        Self::connect(TestDatabaseUrl::require_from_env()?).await
    }

    pub async fn prepare_from_env() -> PostgresTestResult<Self> {
        let context = Self::connect_required_from_env().await?;
        context.apply_migrations().await?;
        Ok(context)
    }

    pub async fn connect(url: TestDatabaseUrl) -> PostgresTestResult<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(url.as_sensitive_str())
            .await
            .map_err(|_| TestSupportError::DatabaseOperationFailed)?;
        let namespace = TestNamespace::new("storage-pg");
        Ok(Self {
            url,
            pool,
            namespace,
        })
    }

    #[must_use]
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    #[must_use]
    pub const fn url(&self) -> &TestDatabaseUrl {
        &self.url
    }

    #[must_use]
    pub const fn namespace(&self) -> &TestNamespace {
        &self.namespace
    }

    /// Prepare a fresh, exact pre-STOR-P10, exact pre-STOR-GDA-P1, or exact
    /// current schema.
    pub async fn apply_migrations(&self) -> PostgresTestResult<()> {
        apply_storage_migrations(&self.pool).await
    }

    /// Destructive exclusive-harness cleanup for the exact current schema.
    pub async fn clean_storage_tables(&self) -> PostgresTestResult<()> {
        clean_storage_tables(&self.pool).await
    }
}

impl fmt::Debug for PostgresTestContext {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PostgresTestContext")
            .field("url", &self.url)
            .field("namespace", &self.namespace)
            .finish_non_exhaustive()
    }
}

pub async fn connect_required_test_database_from_env() -> PostgresTestResult<PostgresTestContext> {
    PostgresTestContext::connect_required_from_env().await
}

pub async fn connect_test_database_from_env() -> PostgresTestResult<Option<PostgresTestContext>> {
    PostgresTestContext::connect_from_env().await
}

pub async fn prepare_test_database_from_env() -> PostgresTestResult<PostgresTestContext> {
    PostgresTestContext::prepare_from_env().await
}

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
    } else if same_table_set(&tables, table_names::PRE_STOR_GDA_P11) {
        validate_pre_gdrive_schema(transaction).await?;
        execute_named_migrations(transaction, &["0011_gdrive_durable_state.sql"]).await?;
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

async fn validate_pre_gdrive_schema(
    transaction: &mut Transaction<'_, Postgres>,
) -> PostgresTestResult<()> {
    validate_worktree_schema(transaction).await?;
    if !constraint_exists(transaction, CURSOR_NONNEGATIVE_CONSTRAINT).await? {
        return Err(TestSupportError::IncompleteStorageSchema);
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

fn same_table_set(actual: &[String], expected: &[&str]) -> bool {
    let mut actual = actual.iter().map(String::as_str).collect::<Vec<_>>();
    let mut expected = expected.to_vec();
    actual.sort_unstable();
    expected.sort_unstable();
    actual == expected
}

fn owned_storage_table_names_sql() -> String {
    let table_names = table_names::ALL
        .iter()
        .map(|table_name| format!("'{table_name}'"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "select table_name from information_schema.tables \
         where table_schema = current_schema() \
           and table_type = 'BASE TABLE' \
           and table_name in ({table_names}) \
         order by table_name"
    )
}

fn clean_storage_tables_sql() -> String {
    format!(
        "truncate table {gdrive_operations}, {gdrive_durable_items}, {gdrive_adapter_state}, {audit_events}, {worktree_state}, {worktree_instances}, {gdrive_mapping}, {idempotency_records}, {adapter_cursors}, {operation_log}, {conflicts}, {tombstones}, {file_revisions}, {sync_objects}, {content_blobs}, {sync_adapters} restart identity cascade",
        gdrive_operations = table_names::GDRIVE_OPERATIONS,
        gdrive_durable_items = table_names::GDRIVE_DURABLE_ITEMS,
        gdrive_adapter_state = table_names::GDRIVE_ADAPTER_STATE,
        audit_events = table_names::AUDIT_EVENTS,
        worktree_state = table_names::WORKTREE_STATE,
        worktree_instances = table_names::WORKTREE_INSTANCES,
        gdrive_mapping = table_names::GDRIVE_MAPPING,
        idempotency_records = table_names::IDEMPOTENCY_RECORDS,
        adapter_cursors = table_names::ADAPTER_CURSORS,
        operation_log = table_names::OPERATION_LOG,
        conflicts = table_names::CONFLICTS,
        tombstones = table_names::TOMBSTONES,
        file_revisions = table_names::FILE_REVISIONS,
        sync_objects = table_names::SYNC_OBJECTS,
        content_blobs = table_names::CONTENT_BLOBS,
        sync_adapters = table_names::SYNC_ADAPTERS,
    )
}
