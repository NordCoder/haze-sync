//! Shared PostgreSQL lease for Server tests.
//!
//! Component CI intentionally provides one ephemeral test database. Server tests
//! that apply migrations and truncate shared tables must therefore run through one
//! process-local lease instead of racing each other or reapplying non-idempotent
//! migrations.

use haze_sync_storage::test_support::{connect_test_database_from_env, PostgresTestContext};
use sqlx::PgPool;
use std::sync::OnceLock;
use tokio::sync::{Mutex, MutexGuard};

static TEST_DATABASE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub(crate) struct TestDatabaseLease {
    context: PostgresTestContext,
    _guard: MutexGuard<'static, ()>,
}

impl TestDatabaseLease {
    #[must_use]
    pub(crate) fn pool(&self) -> &PgPool {
        self.context.pool()
    }
}

pub(crate) async fn acquire_required() -> TestDatabaseLease {
    acquire_optional()
        .await
        .expect("Component CI must provide a strict test database")
}

pub(crate) async fn acquire_optional() -> Option<TestDatabaseLease> {
    let guard = TEST_DATABASE_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .await;
    let context = connect_test_database_from_env()
        .await
        .expect("test database lookup should stay safe")?;

    ensure_schema(&context).await;
    context
        .clean_storage_tables()
        .await
        .expect("tables should clean");

    Some(TestDatabaseLease {
        context,
        _guard: guard,
    })
}

async fn ensure_schema(context: &PostgresTestContext) {
    let schema_ready: bool = sqlx::query_scalar(
        "select to_regclass('public.audit_events') is not null",
    )
    .fetch_one(context.pool())
    .await
    .expect("test schema probe should succeed");

    if !schema_ready {
        context
            .apply_migrations()
            .await
            .expect("migrations should apply");
    }
}
