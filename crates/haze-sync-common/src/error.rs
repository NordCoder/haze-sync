//! Safe validation errors exposed by shared Haze Sync value types.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Validation failures that are safe to serialize in public API responses.
///
/// Variants intentionally do not carry raw input values, filesystem paths,
/// provider payloads, stack traces, credentials, or runtime details.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationError {
    EmptyPath,
    AbsolutePath,
    PathTraversal,
    WindowsDrivePrefix,
    WindowsSeparator,
    NullByte,
    RuntimePath,
    InvalidHashLength,
    InvalidHashCharacter,
    InvalidIdentifier,
    InvalidIdentifierPrefix,
    InvalidAdapterRole,
    InvalidAdapterMode,
}

impl ValidationError {
    /// Stable machine-readable error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::EmptyPath => "empty_path",
            Self::AbsolutePath => "absolute_path",
            Self::PathTraversal => "path_traversal",
            Self::WindowsDrivePrefix => "windows_drive_prefix",
            Self::WindowsSeparator => "windows_separator",
            Self::NullByte => "null_byte",
            Self::RuntimePath => "runtime_path",
            Self::InvalidHashLength => "invalid_hash_length",
            Self::InvalidHashCharacter => "invalid_hash_character",
            Self::InvalidIdentifier => "invalid_identifier",
            Self::InvalidIdentifierPrefix => "invalid_identifier_prefix",
            Self::InvalidAdapterRole => "invalid_adapter_role",
            Self::InvalidAdapterMode => "invalid_adapter_mode",
        }
    }

    /// Stable human-readable message with no sensitive details.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::EmptyPath => "path must not be empty",
            Self::AbsolutePath => "path must be relative to the vault",
            Self::PathTraversal => "path must not contain traversal segments",
            Self::WindowsDrivePrefix => "path must not contain a Windows drive prefix",
            Self::WindowsSeparator => "path must use forward slash separators",
            Self::NullByte => "value must not contain null bytes",
            Self::RuntimePath => "path is reserved for runtime state and must not sync",
            Self::InvalidHashLength => "SHA-256 value must contain exactly 64 hexadecimal characters",
            Self::InvalidHashCharacter => "SHA-256 value contains non-hexadecimal characters",
            Self::InvalidIdentifier => "identifier contains unsupported characters",
            Self::InvalidIdentifierPrefix => "identifier does not use the required prefix",
            Self::InvalidAdapterRole => "adapter role is not supported",
            Self::InvalidAdapterMode => "adapter mode is not supported",
        }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for ValidationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_serializes_as_safe_code() {
        let json = serde_json::to_string(&ValidationError::PathTraversal).unwrap();
        assert_eq!(json, "\"path_traversal\"");
        assert_eq!(ValidationError::PathTraversal.code(), "path_traversal");
        assert_eq!(
            ValidationError::PathTraversal.message(),
            "path must not contain traversal segments"
        );
    }
}
