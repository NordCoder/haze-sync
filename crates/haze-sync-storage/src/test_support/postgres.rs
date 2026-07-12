//! Real PostgreSQL helpers for storage integration tests.
//!
//! These helpers are compiled for storage crate tests and downstream harnesses
//! that explicitly enable `test-support`. They require a validated dedicated test
//! database URL and never create, drop, or select a database implicitly.

use super::{TestDatabaseUrl, TestNamespace, TestSupportError};
use crate::schema::table_names;
#[cfg(test)]
use crate::schema::INITIAL_MIGRATIONS;
use sqlx::{postgres::PgPoolOptions, Executor, PgPool, Postgres, Transaction};
use std::fmt;

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

const CURSOR_NONNEGATIVE_CONSTRAINT: &str = "adapter_cursors_last_core_seq_nonnegative";

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

    /// Prepare a fresh, exact pre-STOR-P10, or exact current schema.
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

/// Applies all migrations to a fresh schema, only STOR-P10 to an exact empty
/// pre-P10 schema, and no migration to an exact current schema.
///
/// A non-empty legacy path-only `worktree_state` fails before any destructive SQL
/// because its rows cannot be assigned to an adapter/root binding safely.
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
        let migration = STORAGE_TEST_MIGRATIONS
            .last()
            .filter(|migration| migration.name == "0010_worktree_durable_state.sql")
            .ok_or(TestSupportError::IncompleteStorageSchema)?;
        execute_migrations(transaction, std::slice::from_ref(migration)).await?;
    } else {
        return Err(TestSupportError::IncompleteStorageSchema);
    }

    validate_current_schema(transaction).await
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

async fn validate_current_schema(
    transaction: &mut Transaction<'_, Postgres>,
) -> PostgresTestResult<()> {
    let tables = owned_storage_table_names(transaction).await?;
    if !same_table_set(&tables, table_names::ALL) {
        return Err(TestSupportError::IncompleteStorageSchema);
    }

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
    .await?;
    if !constraint_exists(transaction, CURSOR_NONNEGATIVE_CONSTRAINT).await? {
        return Err(TestSupportError::IncompleteStorageSchema);
    }

    Ok(())
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
        "truncate table {audit_events}, {worktree_state}, {worktree_instances}, {gdrive_mapping}, {idempotency_records}, {adapter_cursors}, {operation_log}, {conflicts}, {tombstones}, {file_revisions}, {sync_objects}, {content_blobs}, {sync_adapters} restart identity cascade",
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{connect_required_test_database_from_env, unique_test_id};

    #[test]
    fn embedded_migrations_match_schema_metadata() {
        let actual_names: Vec<_> = STORAGE_TEST_MIGRATIONS
            .iter()
            .map(|migration| migration.name)
            .collect();

        assert_eq!(actual_names.as_slice(), INITIAL_MIGRATIONS);
        assert!(STORAGE_TEST_MIGRATIONS
            .iter()
            .all(|migration| migration.sql.contains("create table")
                || migration.sql.contains("alter table")));
    }

    #[test]
    fn schema_probe_covers_current_and_pre_p10_owned_tables() {
        let sql = owned_storage_table_names_sql();

        for table_name in table_names::ALL {
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
            .last()
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
    #[ignore = "requires explicit HAZE_SYNC_TEST_DATABASE_URL and exclusive migration evidence"]
    async fn fresh_and_current_schema_preparation_is_idempotent() {
        let context = connect_required_test_database_from_env().await.unwrap();
        let mut transaction = context.pool().begin().await.unwrap();
        let schema = safe_schema_name("stor_p10_current");
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
}
