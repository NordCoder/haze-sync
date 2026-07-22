//! Passive SQLx repository primitives for Haze Sync metadata.
//!
//! Repository helpers execute caller-requested SQL against caller-owned
//! executors or transactions. They do not create pools, run production migration
//! policy, resolve conflicts, or implement Core/Worktree/provider semantics.

pub mod adapter_cursors;
pub mod conflicts;
pub mod content_blobs;
pub mod gdrive_mapping;
pub mod gdrive_state;
pub mod idempotency;
pub mod objects;
pub mod operation_log;
pub mod revisions;
pub mod tombstones;
pub mod worktree_state;

pub use idempotency::{
    check_or_store_idempotency_record, compare_request_fingerprint, insert_idempotency_record,
    read_idempotency_record, IdempotencyRecordInput, IdempotencyRepositoryError,
    IdempotencyRepositoryOutcome, IdempotencyRequestComparison, IdempotencyStoreOutcome,
};

use std::{error::Error, fmt};

/// Maximum number of rows returned by one repository page.
pub const MAX_CHANGES_LIMIT: u32 = 1_000;

/// Result type returned by storage repository helpers.
pub type RepositoryResult<T> = Result<T, RepositoryError>;

/// Safe repository error boundary.
///
/// Formatting intentionally avoids raw SQL, database URLs, filesystem paths,
/// provider payloads, root fingerprints, cursor payloads, row contents, stack
/// traces, credentials, and runtime details.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum RepositoryError {
    InvalidSequence,
    InvalidLimit {
        max: u32,
    },
    InvalidPath,
    InvalidIdentifier,
    InvalidHash,
    InvalidProviderMetadata,
    InvalidOperationKind,
    InvalidConflictStatus,
    /// A requested cursor update would move backwards.
    CursorRegression,
    /// A requested cursor transition skipped one or more sequence values.
    CursorGap,
    /// The caller's expected cursor does not match the locked persisted value.
    CursorStaleExpected,
    /// Exact contiguous advancement was requested before cursor initialization.
    CursorMissing,
    /// Exact contiguous sequence calculation overflowed the supported range.
    CursorOverflow,
    /// A Worktree row or input uses an unsupported durable-state version.
    UnsupportedWorktreeStateVersion,
    /// An existing adapter binding does not match the supplied root/version.
    WorktreeInstanceBindingMismatch,
    /// A Worktree path-state kind is unknown or inconsistent.
    InvalidWorktreeStateKind,
    /// Reconciliation observation fields are incomplete or inconsistent.
    InvalidWorktreeObservation,
    /// A GDrive aggregate row uses an unsupported durable-state version.
    UnsupportedGDriveStateVersion,
    /// GDrive compare-and-commit was requested before aggregate initialization.
    GDriveStateMissing,
    /// The caller's expected GDrive state version is stale.
    GDriveStateStaleExpected,
    /// The caller's expected Drive cursor generation is stale.
    GDriveCursorGenerationMismatch,
    /// The requested Core export checkpoint would move backwards.
    CheckpointRegression,
    /// One GDrive operation identity was reused with different persisted facts.
    GDriveOperationConflict,
    /// Optimistic aggregate state version cannot be advanced safely.
    StateVersionOverflow,
    DatabaseOperationFailed,
    InvalidSizeBytes,
}

impl RepositoryError {
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidSequence => "invalid_sequence",
            Self::InvalidLimit { .. } => "invalid_limit",
            Self::InvalidPath => "invalid_storage_path",
            Self::InvalidIdentifier => "invalid_storage_identifier",
            Self::InvalidHash => "invalid_storage_hash",
            Self::InvalidProviderMetadata => "invalid_provider_metadata",
            Self::InvalidOperationKind => "invalid_operation_kind",
            Self::InvalidConflictStatus => "invalid_conflict_status",
            Self::CursorRegression => "cursor_regression",
            Self::CursorGap => "cursor_gap",
            Self::CursorStaleExpected => "cursor_stale_expected",
            Self::CursorMissing => "cursor_missing",
            Self::CursorOverflow => "cursor_overflow",
            Self::UnsupportedWorktreeStateVersion => "unsupported_worktree_state_version",
            Self::WorktreeInstanceBindingMismatch => "worktree_instance_binding_mismatch",
            Self::InvalidWorktreeStateKind => "invalid_worktree_state_kind",
            Self::InvalidWorktreeObservation => "invalid_worktree_observation",
            Self::UnsupportedGDriveStateVersion => "unsupported_gdrive_state_version",
            Self::GDriveStateMissing => "gdrive_state_missing",
            Self::GDriveStateStaleExpected => "gdrive_state_stale_expected",
            Self::GDriveCursorGenerationMismatch => "gdrive_cursor_generation_mismatch",
            Self::CheckpointRegression => "checkpoint_regression",
            Self::GDriveOperationConflict => "gdrive_operation_conflict",
            Self::StateVersionOverflow => "state_version_overflow",
            Self::DatabaseOperationFailed => "storage_database_operation_failed",
            Self::InvalidSizeBytes => "invalid_size_bytes",
        }
    }

    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::InvalidSequence => "sequence must be non-negative",
            Self::InvalidLimit { .. } => "limit is outside the supported range",
            Self::InvalidPath => "storage path is invalid",
            Self::InvalidIdentifier => "storage identifier is invalid",
            Self::InvalidHash => "storage hash is invalid",
            Self::InvalidProviderMetadata => "provider metadata is invalid",
            Self::InvalidOperationKind => "operation kind is not supported",
            Self::InvalidConflictStatus => "conflict status is not supported",
            Self::CursorRegression => "cursor update would move backwards",
            Self::CursorGap => "cursor update must advance exactly one sequence",
            Self::CursorStaleExpected => "cursor expected value is stale",
            Self::CursorMissing => "cursor must be initialized before advancement",
            Self::CursorOverflow => "cursor sequence cannot be advanced safely",
            Self::UnsupportedWorktreeStateVersion => {
                "worktree durable-state version is not supported"
            }
            Self::WorktreeInstanceBindingMismatch => {
                "worktree instance binding does not match persisted state"
            }
            Self::InvalidWorktreeStateKind => "worktree path-state kind is invalid",
            Self::InvalidWorktreeObservation => "worktree reconciliation observation is invalid",
            Self::UnsupportedGDriveStateVersion => "gdrive durable-state version is not supported",
            Self::GDriveStateMissing => "gdrive durable state must be initialized",
            Self::GDriveStateStaleExpected => "gdrive expected state version is stale",
            Self::GDriveCursorGenerationMismatch => "gdrive cursor generation is stale",
            Self::CheckpointRegression => "checkpoint update would move backwards",
            Self::GDriveOperationConflict => {
                "gdrive operation identity conflicts with persisted facts"
            }
            Self::StateVersionOverflow => "state version cannot be advanced safely",
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

pub(crate) fn validate_sequence(sequence: i64) -> RepositoryResult<()> {
    if sequence < 0 {
        return Err(RepositoryError::InvalidSequence);
    }
    Ok(())
}

pub(crate) fn validate_limit(limit: u32) -> RepositoryResult<()> {
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
        "root_fingerprint",
        "external_cursor_json",
        "provider payload",
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
            RepositoryError::InvalidPath,
            RepositoryError::InvalidIdentifier,
            RepositoryError::InvalidHash,
            RepositoryError::InvalidProviderMetadata,
            RepositoryError::InvalidOperationKind,
            RepositoryError::InvalidConflictStatus,
            RepositoryError::CursorRegression,
            RepositoryError::CursorGap,
            RepositoryError::CursorStaleExpected,
            RepositoryError::CursorMissing,
            RepositoryError::CursorOverflow,
            RepositoryError::UnsupportedWorktreeStateVersion,
            RepositoryError::WorktreeInstanceBindingMismatch,
            RepositoryError::InvalidWorktreeStateKind,
            RepositoryError::InvalidWorktreeObservation,
            RepositoryError::UnsupportedGDriveStateVersion,
            RepositoryError::GDriveStateMissing,
            RepositoryError::GDriveStateStaleExpected,
            RepositoryError::GDriveCursorGenerationMismatch,
            RepositoryError::CheckpointRegression,
            RepositoryError::GDriveOperationConflict,
            RepositoryError::StateVersionOverflow,
            RepositoryError::DatabaseOperationFailed,
            RepositoryError::InvalidSizeBytes,
        ];

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
        assert_safe_error_text(&error.to_string());
        assert!(std::error::Error::source(&error).is_none());
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
