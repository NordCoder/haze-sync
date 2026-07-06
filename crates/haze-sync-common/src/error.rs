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
    InvalidPercentEncoding,
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
            Self::InvalidPercentEncoding => "invalid_percent_encoding",
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
            Self::InvalidPercentEncoding => "path contains invalid percent encoding",
            Self::InvalidHashLength => {
                "SHA-256 value must contain exactly 64 hexadecimal characters"
            }
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

    const ERROR_CASES: &[(ValidationError, &str, &str)] = &[
        (ValidationError::EmptyPath, "empty_path", "path must not be empty"),
        (
            ValidationError::AbsolutePath,
            "absolute_path",
            "path must be relative to the vault",
        ),
        (
            ValidationError::PathTraversal,
            "path_traversal",
            "path must not contain traversal segments",
        ),
        (
            ValidationError::WindowsDrivePrefix,
            "windows_drive_prefix",
            "path must not contain a Windows drive prefix",
        ),
        (
            ValidationError::WindowsSeparator,
            "windows_separator",
            "path must use forward slash separators",
        ),
        (
            ValidationError::NullByte,
            "null_byte",
            "value must not contain null bytes",
        ),
        (
            ValidationError::RuntimePath,
            "runtime_path",
            "path is reserved for runtime state and must not sync",
        ),
        (
            ValidationError::InvalidPercentEncoding,
            "invalid_percent_encoding",
            "path contains invalid percent encoding",
        ),
        (
            ValidationError::InvalidHashLength,
            "invalid_hash_length",
            "SHA-256 value must contain exactly 64 hexadecimal characters",
        ),
        (
            ValidationError::InvalidHashCharacter,
            "invalid_hash_character",
            "SHA-256 value contains non-hexadecimal characters",
        ),
        (
            ValidationError::InvalidIdentifier,
            "invalid_identifier",
            "identifier contains unsupported characters",
        ),
        (
            ValidationError::InvalidIdentifierPrefix,
            "invalid_identifier_prefix",
            "identifier does not use the required prefix",
        ),
        (
            ValidationError::InvalidAdapterRole,
            "invalid_adapter_role",
            "adapter role is not supported",
        ),
        (
            ValidationError::InvalidAdapterMode,
            "invalid_adapter_mode",
            "adapter mode is not supported",
        ),
    ];

    #[test]
    fn errors_serialize_as_safe_codes() {
        for (error, code, message) in ERROR_CASES {
            let json = serde_json::to_string(error).unwrap();

            assert_eq!(json, format!("\"{code}\""));
            assert_eq!(error.code(), *code);
            assert_eq!(error.message(), *message);
            assert_eq!(error.to_string(), *message);
        }
    }

    #[test]
    fn errors_deserialize_from_stable_codes() {
        for (error, code, _) in ERROR_CASES {
            let json = format!("\"{code}\"");
            assert_eq!(serde_json::from_str::<ValidationError>(&json).unwrap(), *error);
        }

        assert!(serde_json::from_str::<ValidationError>("\"raw_secret_token\"").is_err());
    }

    #[test]
    fn safe_messages_do_not_embed_raw_context() {
        let forbidden_fragments = [
            "/etc/passwd",
            "C:\\secret",
            "bearer",
            "oauth",
            "token",
            "postgres://",
            "stack backtrace",
        ];

        for (error, _, message) in ERROR_CASES {
            let debug = format!("{error:?}");
            for fragment in forbidden_fragments {
                assert!(!message.contains(fragment), "message={message:?}");
                assert!(!debug.to_lowercase().contains(fragment), "debug={debug:?}");
            }
        }
    }
}
