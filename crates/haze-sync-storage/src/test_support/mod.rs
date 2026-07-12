//! Test-only helpers for storage integration tests.
//!
//! This module is available only for crate tests or when the explicit
//! `test-support` feature is enabled. It is not production storage behavior and
//! must not be used by runtime server startup code.

pub mod env;
pub mod ids;
pub mod object_root;

#[cfg(any(test, feature = "test-support"))]
pub mod postgres;

pub use env::{
    TestDatabaseUrl, TestDatabaseUrlSource, DATABASE_URL_ENV, HAZE_SYNC_TEST_DATABASE_URL_ENV,
};
pub use ids::{unique_test_id, TestNamespace};
pub use object_root::TestObjectRoot;

#[cfg(any(test, feature = "test-support"))]
pub use postgres::{
    apply_storage_migrations, clean_storage_tables, connect_required_test_database_from_env,
    connect_test_database_from_env, prepare_test_database_from_env, PostgresTestContext,
    TestMigration, STORAGE_TEST_MIGRATIONS,
};

use std::{error::Error, fmt};

/// Safe, redacted errors produced by storage test-support helpers.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum TestSupportError {
    MissingTestDatabaseUrl,
    UnsupportedDatabaseUrlScheme,
    MissingDatabaseName,
    UnsafeDatabaseName,
    InvalidDatabaseUrl,
    EnvironmentVariableNotUnicode {
        name: &'static str,
    },
    IncompleteStorageSchema,
    /// Pre-STOR-P10 path-only state cannot be assigned to an adapter/root safely.
    LegacyWorktreeStateNotEmpty,
    FilesystemOperationFailed,
    DatabaseOperationFailed,
}

impl fmt::Display for TestSupportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingTestDatabaseUrl => formatter.write_str(
                "HAZE_SYNC_TEST_DATABASE_URL is required for storage database tests",
            ),
            Self::UnsupportedDatabaseUrlScheme => {
                formatter.write_str("test database URL must use postgres:// or postgresql://")
            }
            Self::MissingDatabaseName => {
                formatter.write_str("test database URL must include an explicit database name")
            }
            Self::UnsafeDatabaseName => formatter.write_str(
                "refusing to use the configured database for storage tests; use a dedicated test-named database",
            ),
            Self::InvalidDatabaseUrl => {
                formatter.write_str("test database URL is invalid or unsupported")
            }
            Self::EnvironmentVariableNotUnicode { name } => {
                write!(formatter, "environment variable {name} is not valid Unicode")
            }
            Self::IncompleteStorageSchema => formatter.write_str(
                "test database contains a partial or incompatible storage schema; use a clean dedicated test database",
            ),
            Self::LegacyWorktreeStateNotEmpty => formatter.write_str(
                "legacy Worktree state requires explicit operator migration before STOR-P10",
            ),
            Self::FilesystemOperationFailed => {
                formatter.write_str("test filesystem operation failed")
            }
            Self::DatabaseOperationFailed => formatter.write_str("test database operation failed"),
        }
    }
}

impl Error for TestSupportError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_messages_are_redacted_and_actionable() {
        let errors = [
            TestSupportError::MissingTestDatabaseUrl,
            TestSupportError::UnsupportedDatabaseUrlScheme,
            TestSupportError::MissingDatabaseName,
            TestSupportError::UnsafeDatabaseName,
            TestSupportError::InvalidDatabaseUrl,
            TestSupportError::EnvironmentVariableNotUnicode {
                name: HAZE_SYNC_TEST_DATABASE_URL_ENV,
            },
            TestSupportError::IncompleteStorageSchema,
            TestSupportError::LegacyWorktreeStateNotEmpty,
            TestSupportError::FilesystemOperationFailed,
            TestSupportError::DatabaseOperationFailed,
        ];

        for error in errors {
            let displayed = error.to_string().to_ascii_lowercase();
            assert!(!displayed.contains("postgres://user:"));
            assert!(!displayed.contains("password"));
            assert!(!displayed.contains("/srv/"));
            assert!(!displayed.contains("root_fingerprint"));
            assert!(std::error::Error::source(&error).is_none());
        }
    }
}
