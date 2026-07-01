//! PostgreSQL runtime helpers for the Haze Sync server.
//!
//! This module owns server-side database pool construction, lightweight ping
//! checks, and an explicit migration runner entry point. It never logs or
//! renders raw database URLs, does not use global mutable pools, and does not run
//! migrations automatically from route construction or binary startup.

use crate::config::DatabaseConfig;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{error::Error, fmt, time::Duration};

pub mod migrations;

/// Conservative default connection count for the single-user V1 server.
pub const DEFAULT_MAX_CONNECTIONS: u32 = 5;
/// Conservative default acquire timeout for readiness and startup checks.
pub const DEFAULT_ACQUIRE_TIMEOUT: Duration = Duration::from_secs(5);

/// Safe database runtime errors.
///
/// Formatting intentionally excludes database URLs, SQL text, credentials,
/// filesystem paths, stack traces, and driver internals.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum DbRuntimeError {
    /// A lazy pool could not be constructed from the configured database URL.
    PoolCreationFailed,
    /// Connecting to PostgreSQL failed.
    ConnectionFailed,
    /// A lightweight database ping failed.
    PingFailed,
    /// Repository migrations failed.
    MigrationFailed,
}

impl DbRuntimeError {
    /// Stable machine-readable error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::PoolCreationFailed => "database_pool_creation_failed",
            Self::ConnectionFailed => "database_connection_failed",
            Self::PingFailed => "database_ping_failed",
            Self::MigrationFailed => "database_migration_failed",
        }
    }

    /// Stable path-free and secret-free human-readable message.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::PoolCreationFailed => "database pool could not be created",
            Self::ConnectionFailed => "database connection failed",
            Self::PingFailed => "database connectivity check failed",
            Self::MigrationFailed => "database migrations failed",
        }
    }
}

impl fmt::Display for DbRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message())
    }
}

impl Error for DbRuntimeError {}

/// Build PostgreSQL pool options without opening a database connection.
#[must_use]
pub fn pg_pool_options() -> PgPoolOptions {
    PgPoolOptions::new()
        .max_connections(DEFAULT_MAX_CONNECTIONS)
        .acquire_timeout(DEFAULT_ACQUIRE_TIMEOUT)
}

/// Create a lazy PostgreSQL pool from safe server config.
///
/// This parses pool configuration but does not connect to the database. Callers
/// that need to prove connectivity must explicitly call [`connect_pg_pool`] or
/// [`ping_pg_pool`].
pub fn create_lazy_pg_pool(config: &DatabaseConfig) -> Result<PgPool, DbRuntimeError> {
    pg_pool_options()
        .connect_lazy(config.database_url.as_sensitive_str())
        .map_err(|_error| DbRuntimeError::PoolCreationFailed)
}

/// Create and connect a PostgreSQL pool from safe server config.
///
/// The raw database URL is read from the redacted config wrapper and is never
/// returned in this function's error value.
pub async fn connect_pg_pool(config: &DatabaseConfig) -> Result<PgPool, DbRuntimeError> {
    pg_pool_options()
        .connect(config.database_url.as_sensitive_str())
        .await
        .map_err(|_error| DbRuntimeError::ConnectionFailed)
}

/// Run a lightweight PostgreSQL connectivity check.
pub async fn ping_pg_pool(pool: &PgPool) -> Result<(), DbRuntimeError> {
    let _one = sqlx::query_scalar::<_, i64>("select 1")
        .fetch_one(pool)
        .await
        .map_err(|_error| DbRuntimeError::PingFailed)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DatabaseUrl;

    #[tokio::test]
    async fn lazy_pool_creation_does_not_connect() {
        let config = DatabaseConfig {
            database_url: DatabaseUrl::new(
                "postgres://haze_sync:secret_password@postgres.invalid/haze_sync",
            )
            .expect("database URL wrapper should parse"),
        };

        let pool = create_lazy_pg_pool(&config).expect("lazy pool should be created without IO");

        assert!(!pool.is_closed());
    }

    #[test]
    fn database_errors_are_redacted() {
        let raw_url = "postgres://user:secret_password@postgres.invalid/haze_sync";
        let rendered = format!(
            "{:?} {} {} {} {}",
            DbRuntimeError::ConnectionFailed,
            DbRuntimeError::PoolCreationFailed,
            DbRuntimeError::PingFailed,
            DbRuntimeError::MigrationFailed,
            DbRuntimeError::ConnectionFailed.code()
        );

        assert!(!rendered.contains(raw_url));
        assert!(!rendered.contains("secret_password"));
        assert!(!rendered.contains("postgres://"));
    }

    #[test]
    fn pool_options_use_safe_defaults() {
        let _options = pg_pool_options();

        assert_eq!(DEFAULT_MAX_CONNECTIONS, 5);
        assert_eq!(DEFAULT_ACQUIRE_TIMEOUT, Duration::from_secs(5));
    }
}
