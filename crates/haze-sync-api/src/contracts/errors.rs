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

/// Stable public error codes used by the Core and operational-control APIs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublicErrorCode {
    InvalidRequest,
    InvalidPath,
    ValidationError,
    Unauthorized,
    MissingToken,
    InvalidToken,
    ForbiddenRole,
    NotFound,
    Conflict,
    IdempotencyConflict,
    IdempotencyInProgress,
    PayloadTooLarge,
    RateLimited,
    UnsafeDelete,
    IgnoredPath,
    MaintenanceInProgress,
    ControlTransitionInProgress,
    InvalidControlTransition,
    StaleControlGeneration,
    StaleRecordVersion,
    StaleExecutorFence,
    CredentialSecretNotReplayable,
    OperationInProgress,
    ConfirmationRequired,
    ConfirmationExpired,
    OperationNotCancellable,
    UnsupportedOperationKind,
    InternalError,
}

/// Safe optional error details represented only by public strings.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SafeErrorDetails {
    Map(BTreeMap<String, Vec<String>>),
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

    #[test]
    fn validation_details_use_public_names_without_secret_values() {
        let raw_idempotency_key = "fixture-idempotency-key-01";
        let raw_bearer = "fixture-bearer-token-01";
        let mut details = BTreeMap::new();
        details.insert(
            "Idempotency-Key".to_owned(),
            vec!["is required for write routes".to_owned()],
        );
        details.insert(
            "X-Base-Revision-Id".to_owned(),
            vec!["must be a revision id or literal null".to_owned()],
        );
        details.insert("query.limit".to_owned(), vec!["must be <= 1000".to_owned()]);
        let response = ErrorResponse {
            error: PublicError::new(PublicErrorCode::ValidationError, "validation failed")
                .with_request_id("req_02J")
                .with_details(SafeErrorDetails::Map(details)),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("Idempotency-Key"));
        assert!(json.contains("X-Base-Revision-Id"));
        assert!(json.contains("query.limit"));
        assert!(!json.contains(raw_idempotency_key));
        assert!(!json.contains(raw_bearer));
        assert!(!json.contains("token_hash"));
        assert!(!json.contains("database_url"));
        assert!(!json.contains("stack_trace"));
    }

    #[test]
    fn operational_codes_serialize_to_stable_safe_names() {
        for (code, expected) in [
            (
                PublicErrorCode::MaintenanceInProgress,
                "maintenance_in_progress",
            ),
            (
                PublicErrorCode::StaleControlGeneration,
                "stale_control_generation",
            ),
            (
                PublicErrorCode::CredentialSecretNotReplayable,
                "credential_secret_not_replayable",
            ),
            (
                PublicErrorCode::IdempotencyInProgress,
                "idempotency_in_progress",
            ),
            (PublicErrorCode::StaleExecutorFence, "stale_executor_fence"),
        ] {
            assert_eq!(
                serde_json::to_string(&code).unwrap(),
                format!("\"{expected}\"")
            );
        }
    }
}
