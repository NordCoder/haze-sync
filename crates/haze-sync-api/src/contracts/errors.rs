//! Safe public error response contract for the Core API.
//!
//! Error values are intentionally stable and sanitized. They are suitable for
//! JSON responses but do not include stack traces, provider payloads, local file
//! paths, credentials, request bodies, or runtime internals.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Top-level JSON error response shape returned by future API routes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Sanitized public error payload.
    pub error: PublicError,
}

impl ErrorResponse {
    /// Creates a public error response with no extra details.
    #[must_use]
    pub fn new(code: PublicErrorCode, message: impl Into<String>) -> Self {
        Self {
            error: PublicError::new(code, message),
        }
    }
}

/// Public error payload that may be serialized to clients.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicError {
    /// Stable machine-readable error code.
    pub code: PublicErrorCode,
    /// Safe human-readable message. Must not contain internals or secrets.
    pub message: String,
    /// Optional request correlation id when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// Optional safe validation details represented as string-only map or list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<SafeErrorDetails>,
}

impl PublicError {
    /// Creates a public error payload with no request id and no details.
    #[must_use]
    pub fn new(code: PublicErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            request_id: None,
            details: None,
        }
    }

    /// Attaches a safe request id to the public error payload.
    #[must_use]
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    /// Attaches sanitized detail values to the public error payload.
    #[must_use]
    pub fn with_details(mut self, details: SafeErrorDetails) -> Self {
        self.details = Some(details);
        self
    }
}

/// Stable public error codes used by the Core API contract.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublicErrorCode {
    /// Request syntax or shape was invalid.
    InvalidRequest,
    /// Vault path failed normalization or validation.
    InvalidPath,
    /// Request data failed semantic validation.
    ValidationError,
    /// Authentication token was missing or invalid.
    Unauthorized,
    /// Authentication token was missing.
    MissingToken,
    /// Authentication token was present but invalid.
    InvalidToken,
    /// Authenticated adapter role is not allowed to perform the action.
    ForbiddenRole,
    /// Requested file, revision, conflict, or other public resource was absent.
    NotFound,
    /// Safe conflict response for a write that cannot overwrite current content.
    Conflict,
    /// Same idempotency key was reused for a different write request.
    IdempotencyConflict,
    /// Upload exceeded the configured maximum size.
    PayloadTooLarge,
    /// Adapter exceeded a public rate limit.
    RateLimited,
    /// Delete was rejected by stale-base or mass-delete safety rules.
    UnsafeDelete,
    /// Ignored path was rejected by public sync policy.
    IgnoredPath,
    /// Unexpected internal failure mapped to a safe public message.
    InternalError,
}

/// Safe optional error details represented only by public strings.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SafeErrorDetails {
    /// Field-to-messages map for validation-style errors.
    Map(BTreeMap<String, Vec<String>>),
    /// Flat list for public non-field details.
    List(Vec<String>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_response_roundtrips_safe_json() {
        let response = ErrorResponse {
            error: PublicError::new(PublicErrorCode::InvalidPath, "Path contains '..' segment")
                .with_request_id("req_01J"),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("invalid_path"));
        assert!(!json.contains("stack"));
        assert!(!json.contains("secret"));

        let decoded: ErrorResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, response);
    }

    #[test]
    fn details_support_map_and_list_shapes() {
        let mut fields = BTreeMap::new();
        fields.insert("path".to_owned(), vec!["must be relative".to_owned()]);

        let mapped = SafeErrorDetails::Map(fields);
        let listed = SafeErrorDetails::List(vec!["retry later".to_owned()]);

        assert_eq!(
            serde_json::to_value(&mapped).unwrap()["path"][0],
            "must be relative"
        );
        assert_eq!(serde_json::to_value(&listed).unwrap()[0], "retry later");
    }
}
