//! Real PostgreSQL helpers for storage integration tests.
//!
//! These helpers are compiled only with the explicit `test-support` feature.
//! They require a validated test database URL and never create, drop, or select a
//! database implicitly.

use super::{TestDatabaseUrl, TestNamespace, TestSupportError};
use crate::schema::table_names;
use sqlx::{postgres::PgPoolOptions, Executor, PgPool};
use std::fmt;

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
    /// Connects to the configured test database if an explicit URL is present.
    ///
    /// Returns `Ok(None)` when neither `HAZE_SYNC_TEST_DATABASE_URL` nor
    /// `DATABASE_URL` is set. Callers can use that result to skip ignored or
    /// opt-in integration tests without failing normal local/unit test runs.
    pub async fn connect_from_env() -> Result<Option<Self>, TestSupportError> {
        let Some(url) = TestDatabaseUrl::from_env()? else {
            return Ok(None);
        };

        Self::connect(url).await.map(Some)
    }

    /// Connects to a previously validated test database URL.
    pub async fn connect(url: TestDatabaseUrl) -> Result<Self, TestSupportError> {
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

    /// Applies all embedded storage schema migrations to the connected database.
    ///
    /// This helper is intentionally test-only and assumes a fresh safe test
    /// database. It is not a production migration runner and does not maintain a
    /// schema history table.
    pub async fn apply_migrations(&self) -> Result<(), TestSupportError> {
        apply_storage_migrations(&self.pool).await
    }

    /// Truncates all known storage metadata tables in dependency-safe order.
    ///
    /// This is destructive by design and is available only after the database URL
    /// has passed test-name safety checks. It never drops databases or schemas.
    pub async fn clean_storage_tables(&self) -> Result<(), TestSupportError> {
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

/// Connects to the configured real Postgres test database if available.
pub async fn connect_test_database_from_env() -> Result<Option<PostgresTestContext>, TestSupportError> {
    PostgresTestContext::connect_from_env().await
}

/// Applies all embedded storage schema migrations to a test database.
pub async fn apply_storage_migrations(pool: &PgPool) -> Result<(), TestSupportError> {
    for migration in STORAGE_TEST_MIGRATIONS {
        pool.execute(migration.sql)
            .await
            .map_err(|_| TestSupportError::DatabaseOperationFailed)?;
    }

    Ok(())
}

/// Truncates storage metadata tables in dependency-safe order.
pub async fn clean_storage_tables(pool: &PgPool) -> Result<(), TestSupportError> {
    let sql = format!(
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
    );

    pool.execute(sql.as_str())
        .await
        .map_err(|_| TestSupportError::DatabaseOperationFailed)?;

    Ok(())
}
