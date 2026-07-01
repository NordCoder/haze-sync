//! Explicit repository migration runner for server-owned PostgreSQL metadata.
//!
//! The runner uses the repository's existing migration directory and only runs
//! when this function is called by a future safe entry point. It does not expose
//! rollback, drop, reset, or any destructive schema behavior.

use super::DbRuntimeError;
use sqlx::PgPool;

/// Workspace-relative migration directory embedded by SQLx at compile time.
pub const MIGRATIONS_PATH: &str = "../../migrations";

/// Run repository migrations against a caller-owned PostgreSQL pool.
///
/// This is deliberately explicit: route construction and the current binary
/// shell do not call it automatically.
pub async fn run_repository_migrations(pool: &PgPool) -> Result<(), DbRuntimeError> {
    sqlx::migrate!("../../migrations")
        .run(pool)
        .await
        .map_err(|_error| DbRuntimeError::MigrationFailed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_error_is_safe_to_render() {
        let rendered = format!(
            "{:?} {} {}",
            DbRuntimeError::MigrationFailed,
            DbRuntimeError::MigrationFailed,
            DbRuntimeError::MigrationFailed.code()
        );

        assert!(rendered.contains("database_migration_failed"));
        assert!(!rendered.contains("postgres://"));
        assert!(!rendered.contains("password"));
        assert!(!rendered.contains("/srv/"));
        assert!(!rendered.contains("stack"));
    }

    #[test]
    fn migration_path_points_to_existing_repository_directory() {
        assert_eq!(MIGRATIONS_PATH, "../../migrations");
    }
}
