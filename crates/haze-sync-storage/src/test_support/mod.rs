//! Test-only helpers for storage integration tests.
//!
//! This module is available only for crate tests or when the explicit
//! `test-support` feature is enabled. It is not production storage behavior and
//! must not be used by runtime server startup code.

pub mod env;
pub mod ids;
pub mod object_root;

#[cfg(feature = "test-support")]
pub mod postgres;

pub use env::{
    TestDatabaseUrl, TestDatabaseUrlSource, DATABASE_URL_ENV, HAZE_SYNC_TEST_DATABASE_URL_ENV,
};
pub use ids::{unique_test_id, TestNamespace};
pub use object_root::TestObjectRoot;

#[cfg(feature = "test-support")]
pub use postgres::{
    apply_storage_migrations, clean_storage_tables, connect_test_database_from_env,
    PostgresTestContext, TestMigration, STORAGE_TEST_MIGRATIONS,
};

use std::{error::Error, fmt};

/// Safe, redacted errors produced by storage test-support helpers.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum TestSupportError {
    /// The configured database URL does not use a PostgreSQL URL scheme.
    UnsupportedDatabaseUrlScheme,
    /// The configured database URL does not include an explicit database name.
    MissingDatabaseName,
    /// The configured database name is not safe for destructive test helpers.
    UnsafeDatabaseName { database_name: String },
    /// The configured database URL could not be interpreted safely.
    InvalidDatabaseUrl,
    /// An environment variable was set but was not valid Unicode.
    EnvironmentVariableNotUnicode { name: &'static str },
    /// A test-only filesystem helper failed without exposing local paths.
    FilesystemOperationFailed,
    /// A test-only database helper failed without exposing connection details.
    DatabaseOperationFailed,
}

impl fmt::Display for TestSupportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedDatabaseUrlScheme => {
                formatter.write_str("test database URL must use postgres:// or postgresql://")
            }
            Self::MissingDatabaseName => {
                formatter.write_str("test database URL must include an explicit database name")
            }
            Self::UnsafeDatabaseName { database_name } => write!(
                formatter,
                "refusing to use database '{database_name}' for storage tests; use a test-named database"
            ),
            Self::InvalidDatabaseUrl => {
                formatter.write_str("test database URL is invalid or unsupported")
            }
            Self::EnvironmentVariableNotUnicode { name } => {
                write!(formatter, "environment variable {name} is not valid Unicode")
            }
            Self::FilesystemOperationFailed => {
                formatter.write_str("test filesystem operation failed")
            }
            Self::DatabaseOperationFailed => formatter.write_str("test database operation failed"),
        }
    }
}

impl Error for TestSupportError {}
