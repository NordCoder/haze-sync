//! Passive SQLx repository primitives for Haze Sync metadata.
//!
//! Repository helpers in this module execute only caller-requested SQL against a
//! caller-owned executor or transaction. They do not create pools, run
//! migrations, append operation-log entries outside their scoped repository,
//! resolve conflicts, or implement Core apply policy.

pub mod idempotency;
pub mod objects;
pub mod revisions;

pub use idempotency::{
    check_or_store_idempotency_record, compare_request_fingerprint, insert_idempotency_record,
    read_idempotency_record, IdempotencyRecordInput, IdempotencyRepositoryError,
    IdempotencyRepositoryOutcome, IdempotencyRequestComparison, IdempotencyStoreOutcome,
};

use std::{error::Error, fmt};

/// Result type returned by storage repository helpers.
pub type RepositoryResult<T> = Result<T, RepositoryError>;

/// Safe repository error boundary.
///
/// Public formatting intentionally avoids raw SQL, database URLs, filesystem
/// paths, provider payloads, stack traces, credentials, and runtime details.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum RepositoryError {
    /// A database operation failed. The underlying database error is not exposed
    /// across this storage boundary.
    DatabaseOperationFailed,
    /// A caller-provided size could not be represented by the storage schema.
    InvalidSizeBytes,
}

impl RepositoryError {
    /// Stable machine-readable error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::DatabaseOperationFailed => "storage_database_operation_failed",
            Self::InvalidSizeBytes => "invalid_size_bytes",
        }
    }

    /// Stable path-free and secret-free human-readable message.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::DatabaseOperationFailed => "storage database operation failed",
            Self::InvalidSizeBytes => "size is outside the supported storage range",
        }
    }
}

impl fmt::Display for RepositoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message())
    }
}

impl Error for RepositoryError {}

pub(crate) fn map_sqlx_error(_error: sqlx::Error) -> RepositoryError {
    RepositoryError::DatabaseOperationFailed
}

pub(crate) fn size_bytes_to_i64(size_bytes: u64) -> RepositoryResult<i64> {
    i64::try_from(size_bytes).map_err(|_| RepositoryError::InvalidSizeBytes)
}
