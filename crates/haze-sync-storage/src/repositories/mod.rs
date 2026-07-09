//! Passive SQLx repository primitives for Haze Sync metadata.
//!
//! Repository helpers in this module execute only caller-requested SQL against a
//! caller-owned executor or transaction. They do not create pools, run
//! migrations, resolve conflicts, or implement Core apply policy.

pub mod adapter_cursors;
pub mod conflicts;
pub mod content_blobs;
pub mod idempotency;
pub mod objects;
pub mod operation_log;
pub mod revisions;
pub mod tombstones;

pub use idempotency::{
    check_or_store_idempotency_record, compare_request_fingerprint, insert_idempotency_record,
    read_idempotency_record, IdempotencyRecordInput, IdempotencyRepositoryError,
    IdempotencyRepositoryOutcome, IdempotencyRequestComparison, IdempotencyStoreOutcome,
};

use std::{error::Error, fmt};

/// Maximum number of changes returned by one repository page.
pub const MAX_CHANGES_LIMIT: u32 = 1_000;

/// Result type returned by storage repository helpers.
pub type RepositoryResult<T> = Result<T, RepositoryError>;

/// Safe repository error boundary.
///
/// Public formatting intentionally avoids raw SQL, database URLs, filesystem
/// paths, provider payloads, stack traces, credentials, and runtime details.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum RepositoryError {
    /// Sequence values must be zero or greater.
    InvalidSequence,
    /// Query page limits must be between one and the configured maximum.
    InvalidLimit { max: u32 },
    /// An operation kind string is not part of the V1 contract vocabulary.
    InvalidOperationKind,
    /// A conflict status string is not part of the V1 contract vocabulary.
    InvalidConflictStatus,
    /// A requested cursor update would move the adapter backwards.
    CursorRegression,
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
            Self::InvalidSequence => "invalid_sequence",
            Self::InvalidLimit { .. } => "invalid_limit",
            Self::InvalidOperationKind => "invalid_operation_kind",
            Self::InvalidConflictStatus => "invalid_conflict_status",
            Self::CursorRegression => "cursor_regression",
            Self::DatabaseOperationFailed => "storage_database_operation_failed",
            Self::InvalidSizeBytes => "invalid_size_bytes",
        }
    }

    /// Stable path-free and secret-free human-readable message.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::InvalidSequence => "sequence must be non-negative",
            Self::InvalidLimit { .. } => "limit is outside the supported range",
            Self::InvalidOperationKind => "operation kind is not supported",
            Self::InvalidConflictStatus => "conflict status is not supported",
            Self::CursorRegression => "cursor update would move backwards",
            Self::DatabaseOperationFailed => "storage database operation failed",
            Self::InvalidSizeBytes => "size is outside the supported storage range",
        }
    }
}

impl fmt::Display for RepositoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimit { max } => write!(formatter, "limit must be between 1 and {max}"),
            _ => formatter.write_str(self.message()),
        }
    }
}

impl Error for RepositoryError {}

pub(crate) fn validate_sequence(sequence: i64) -> Result<(), RepositoryError> {
    if sequence < 0 {
        return Err(RepositoryError::InvalidSequence);
    }

    Ok(())
}

pub(crate) fn validate_limit(limit: u32) -> Result<(), RepositoryError> {
    if limit == 0 || limit > MAX_CHANGES_LIMIT {
        return Err(RepositoryError::InvalidLimit {
            max: MAX_CHANGES_LIMIT,
        });
    }

    Ok(())
}

pub(crate) fn map_sqlx_error(_error: sqlx::Error) -> RepositoryError {
    RepositoryError::DatabaseOperationFailed
}

pub(crate) fn size_bytes_to_i64(size_bytes: u64) -> RepositoryResult<i64> {
    i64::try_from(size_bytes).map_err(|_| RepositoryError::InvalidSizeBytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    const UNSAFE_ERROR_FRAGMENTS: &[&str] = &[
        "postgres://",
        "password",
        "secret",
        "select ",
        "/srv/",
        "idempotency-key",
        "oauth",
        "stack backtrace",
    ];

    #[test]
    fn validates_changes_limits() {
        assert_eq!(validate_limit(1), Ok(()));
        assert_eq!(validate_limit(MAX_CHANGES_LIMIT), Ok(()));
        assert_eq!(
            validate_limit(0),
            Err(RepositoryError::InvalidLimit {
                max: MAX_CHANGES_LIMIT
            })
        );
        assert_eq!(
            validate_limit(MAX_CHANGES_LIMIT + 1),
            Err(RepositoryError::InvalidLimit {
                max: MAX_CHANGES_LIMIT
            })
        );
    }

    #[test]
    fn validates_non_negative_sequences() {
        assert_eq!(validate_sequence(0), Ok(()));
        assert_eq!(validate_sequence(42), Ok(()));
        assert_eq!(validate_sequence(-1), Err(RepositoryError::InvalidSequence));
    }

    #[test]
    fn validates_size_bytes_schema_range() {
        assert_eq!(size_bytes_to_i64(0), Ok(0));
        assert_eq!(size_bytes_to_i64(i64::MAX as u64), Ok(i64::MAX));
        assert_eq!(
            size_bytes_to_i64(i64::MAX as u64 + 1),
            Err(RepositoryError::InvalidSizeBytes)
        );
    }

    #[test]
    fn repository_error_codes_messages_and_display_are_stable_and_safe() {
        let errors = [
            RepositoryError::InvalidSequence,
            RepositoryError::InvalidLimit {
                max: MAX_CHANGES_LIMIT,
            },
            RepositoryError::InvalidOperationKind,
            RepositoryError::InvalidConflictStatus,
            RepositoryError::CursorRegression,
            RepositoryError::DatabaseOperationFailed,
            RepositoryError::InvalidSizeBytes,
        ];

        let codes = errors.map(RepositoryError::code);
        assert_eq!(
            codes,
            [
                "invalid_sequence",
                "invalid_limit",
                "invalid_operation_kind",
                "invalid_conflict_status",
                "cursor_regression",
                "storage_database_operation_failed",
                "invalid_size_bytes",
            ]
        );

        for error in errors {
            assert_safe_error_text(error.code());
            assert_safe_error_text(error.message());
            assert_safe_error_text(&error.to_string());
            assert!(std::error::Error::source(&error).is_none());
        }
    }

    #[test]
    fn mapped_sqlx_error_discards_raw_database_details() {
        let raw = sqlx::Error::Io(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "postgres://user:secret@db/prod select * from tokens /srv/prod Idempotency-Key",
        ));

        let error = map_sqlx_error(raw);

        assert_eq!(error, RepositoryError::DatabaseOperationFailed);
        assert_eq!(error.code(), "storage_database_operation_failed");
        assert_safe_error_text(error.message());
        assert_safe_error_text(&error.to_string());
        assert!(std::error::Error::source(&error).is_none());
    }

    #[test]
    fn conflict_status_error_is_safe() {
        let error = RepositoryError::InvalidConflictStatus;
        assert_eq!(error.code(), "invalid_conflict_status");
        assert_safe_error_text(error.message());
        assert_safe_error_text(&error.to_string());
    }

    fn assert_safe_error_text(text: &str) {
        let lowered = text.to_ascii_lowercase();
        for fragment in UNSAFE_ERROR_FRAGMENTS {
            assert!(
                !lowered.contains(fragment),
                "repository error text leaked unsafe fragment `{fragment}` in `{text}`"
            );
        }
    }
}
