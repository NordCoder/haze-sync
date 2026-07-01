//! Storage repository primitives for Haze Sync metadata.
//!
//! These modules provide SQL helpers only. They do not implement route handlers,
//! Core revision apply policy, idempotency behavior, conflict/delete policy, or
//! adapter runtime behavior.

pub mod adapter_cursors;
pub mod operation_log;

use std::{error::Error, fmt};

/// Maximum number of changes returned by one repository page.
pub const MAX_CHANGES_LIMIT: u32 = 1_000;

/// Safe repository error type that does not expose SQL, database URLs, paths, or
/// provider payloads.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum RepositoryError {
    /// Sequence values must be zero or greater.
    InvalidSequence,
    /// Query page limits must be between one and the configured maximum.
    InvalidLimit { max: u32 },
    /// An operation kind string is not part of the V1 contract vocabulary.
    InvalidOperationKind,
    /// A requested cursor update would move the adapter backwards.
    CursorRegression,
    /// A database operation failed; details are intentionally redacted.
    DatabaseOperationFailed,
}

impl fmt::Display for RepositoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSequence => formatter.write_str("sequence must be non-negative"),
            Self::InvalidLimit { max } => {
                write!(formatter, "limit must be between 1 and {max}")
            }
            Self::InvalidOperationKind => formatter.write_str("operation kind is not supported"),
            Self::CursorRegression => formatter.write_str("cursor update would move backwards"),
            Self::DatabaseOperationFailed => formatter.write_str("storage database operation failed"),
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
