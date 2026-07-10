//! Real PostgreSQL helpers for storage integration tests.
//!
//! These helpers are compiled for storage crate tests and for downstream
//! harnesses that explicitly enable the `test-support` feature. They require a
//! validated dedicated test database URL and never create, drop, or select a
//! database implicitly.

use super::{TestDatabaseUrl, TestNamespace, TestSupportError};
use crate::schema::table_names;
#[cfg(test)]
use crate::schema::INITIAL_MIGRATIONS;
use sqlx::{postgres::PgPoolOptions, Executor, PgPool};
use std::fmt;

const TEST_SETUP_ADVISORY_LOCK_KEY: i64 = 7_252_953_924_883_537_409;

type PostgresTestResult<T> = Result<T, TestSupportError>;

/// One embedded storage-schema migration used by the test harness.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TestMigration {
    /// Migration filename from the repository `migrations/` directory.
    pub name: &'static str,
    /// Migration SQL text.
    pub sql: &'static str,
}

/// Ordered storage migrations for real Postgres test setup.
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
];

/// Real Postgres test context with a validated safe database URL.
pub struct PostgresTestContext {
    url: TestDatabaseUrl,
    pool: PgPool,
    namespace: TestNamespace,
}

impl PostgresTestContext {
    /// Connects when an explicit dedicated test database is configured.
    ///
    /// This compatibility path returns `Ok(None)` only when
    /// `HAZE_SYNC_TEST_DATABASE_URL` is absent or blank. Invalid configuration and
    /// connection failures remain errors. New executable database tests should use
    /// [`Self::connect_required_from_env`] or [`Self::prepare_from_env`].
    pub async fn connect_from_env() -> PostgresTestResult<Option<Self>> {
        let Some(url) = TestDatabaseUrl::from_env()? else {
            return Ok(None);
        };

        Self::connect(url).await.map(Some)
    }

    /// Connects to the required `HAZE_SYNC_TEST_DATABASE_URL` database.
    pub async fn connect_required_from_env() -> PostgresTestResult<Self> {
        Self::connect(TestDatabaseUrl::require_from_env()?).await
    }

    /// Connects and applies the embedded schema under the setup lock.
    pub async fn prepare_from_env() -> PostgresTestResult<Self> {
        let context = Self::connect_required_from_env().await?;
        context.apply_migrations().await?;
        Ok(context)
    }

    /// Connects to a previously validated test database URL.
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

    /// Returns the underlying SQLx pool for test code.
    #[must_use]
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Returns the redacted, validated database URL metadata.
    #[must_use]
    pub const fn url(&self) -> &TestDatabaseUrl {
        &self.url
    }

    /// Returns the unique namespace reserved for the current test context.
    #[must_use]
    pub const fn namespace(&self) -> &TestNamespace {
        &self.namespace
    }

    /// Applies all embedded storage schema migrations when the schema is empty.
    ///
    /// Setup is serialized with a transaction-scoped PostgreSQL advisory lock.
    /// An already-complete schema is accepted, while a partial schema fails
    /// explicitly instead of running an ambiguous subset of migrations.
    pub async fn apply_migrations(&self) -> PostgresTestResult<()> {
        apply_storage_migrations(&self.pool).await
    }

    /// Truncates all known storage metadata tables in dependency-safe order.
    ///
    /// This is destructive by design and is available only after the database URL
    /// has passed test-name safety checks. Use it only during exclusive harness
    /// setup; normal repository tests should isolate writes in rollbacked
    /// caller-owned transactions and unique [`TestNamespace`] values.
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

/// Connects to the required real PostgreSQL test database.
pub async fn connect_required_test_database_from_env() -> PostgresTestResult<PostgresTestContext> {
    PostgresTestContext::connect_required_from_env().await
}

/// Connects when a dedicated real PostgreSQL test database is configured.
///
/// This compatibility wrapper returns `Ok(None)` only for absent or blank
/// `HAZE_SYNC_TEST_DATABASE_URL`. Invalid configuration and connection failures
/// remain errors. New mandatory database tests should use
/// [`connect_required_test_database_from_env`] or [`prepare_test_database_from_env`].
pub async fn connect_test_database_from_env() -> PostgresTestResult<Option<PostgresTestContext>> {
    PostgresTestContext::connect_from_env().await
}

/// Connects to the required test database and prepares the storage schema.
pub async fn prepare_test_database_from_env() -> PostgresTestResult<PostgresTestContext> {
    PostgresTestContext::prepare_from_env().await
}

/// Applies embedded storage migrations safely to an empty test schema.
pub async fn apply_storage_migrations(pool: &PgPool) -> PostgresTestResult<()> {
    let mut transaction = pool
        .begin()
        .await
        .map_err(|_| TestSupportError::DatabaseOperationFailed)?;
    acquire_setup_lock(&mut transaction).await?;

    let existing_table_count = storage_table_count(&mut transaction).await?;
    let expected_table_count = table_names::ALL.len() as i64;

    if existing_table_count == 0 {
        for migration in STORAGE_TEST_MIGRATIONS {
            (&mut *transaction)
                .execute(migration.sql)
                .await
                .map_err(|_| TestSupportError::DatabaseOperationFailed)?;
        }

        if storage_table_count(&mut transaction).await? != expected_table_count {
            return Err(TestSupportError::IncompleteStorageSchema);
        }
    } else if existing_table_count != expected_table_count {
        return Err(TestSupportError::IncompleteStorageSchema);
    }

    transaction
        .commit()
        .await
        .map_err(|_| TestSupportError::DatabaseOperationFailed)
}

/// Truncates all known storage metadata tables under the setup lock.
pub async fn clean_storage_tables(pool: &PgPool) -> PostgresTestResult<()> {
    let mut transaction = pool
        .begin()
        .await
        .map_err(|_| TestSupportError::DatabaseOperationFailed)?;
    acquire_setup_lock(&mut transaction).await?;

    if storage_table_count(&mut transaction).await? != table_names::ALL.len() as i64 {
        return Err(TestSupportError::IncompleteStorageSchema);
    }

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

async fn acquire_setup_lock(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> PostgresTestResult<()> {
    sqlx::query("select pg_advisory_xact_lock($1)")
        .bind(TEST_SETUP_ADVISORY_LOCK_KEY)
        .execute(&mut **transaction)
        .await
        .map_err(|_| TestSupportError::DatabaseOperationFailed)?;
    Ok(())
}

async fn storage_table_count(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> PostgresTestResult<i64> {
    let sql = storage_table_count_sql();
    sqlx::query_scalar::<_, i64>(sql.as_str())
        .fetch_one(&mut **transaction)
        .await
        .map_err(|_| TestSupportError::DatabaseOperationFailed)
}

fn storage_table_count_sql() -> String {
    let table_names = table_names::ALL
        .iter()
        .map(|table_name| format!("'{table_name}'"))
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        "select count(*)::bigint from information_schema.tables \
         where table_schema = current_schema() and table_name in ({table_names})"
    )
}

fn clean_storage_tables_sql() -> String {
    format!(
        "truncate table {audit_events}, {worktree_state}, {gdrive_mapping}, {idempotency_records}, {adapter_cursors}, {operation_log}, {conflicts}, {tombstones}, {file_revisions}, {sync_objects}, {content_blobs}, {sync_adapters} restart identity cascade",
        audit_events = table_names::AUDIT_EVENTS,
        worktree_state = table_names::WORKTREE_STATE,
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

    #[test]
    fn embedded_migrations_match_schema_metadata() {
        let actual_names: Vec<_> = STORAGE_TEST_MIGRATIONS
            .iter()
            .map(|migration| migration.name)
            .collect();

        assert_eq!(actual_names.as_slice(), INITIAL_MIGRATIONS);
        assert!(STORAGE_TEST_MIGRATIONS
            .iter()
            .all(|migration| migration.sql.contains("create table")));
    }

    #[test]
    fn schema_probe_covers_every_owned_table() {
        let sql = storage_table_count_sql();

        for table_name in table_names::ALL {
            assert!(sql.contains(table_name));
        }
        assert!(sql.contains("current_schema()"));
    }

    #[test]
    fn cleanup_sql_covers_every_owned_table_without_drop_statements() {
        let sql = clean_storage_tables_sql().to_ascii_lowercase();

        for table_name in table_names::ALL {
            assert!(sql.contains(table_name));
        }
        assert!(sql.starts_with("truncate table"));
        assert!(!sql.contains("drop database"));
        assert!(!sql.contains("drop schema"));
    }
}
