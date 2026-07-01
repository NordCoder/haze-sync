//! Safe public error responses for server shell routes.
//!
//! The route shell uses these local HTTP errors for endpoints whose Core
//! implementation belongs to later phases. Placeholder write routes return
//! `not_implemented` and never claim that data was persisted.

use axum::{http::StatusCode, Json};
use serde::Serialize;

/// Top-level safe JSON error response returned by route shell placeholders.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ShellErrorResponse {
    /// Sanitized public error payload.
    pub error: ShellPublicError,
}

impl ShellErrorResponse {
    /// Creates a safe public error response.
    #[must_use]
    pub fn new(code: ShellErrorCode, message: impl Into<String>) -> Self {
        Self {
            error: ShellPublicError {
                code,
                message: message.into(),
            },
        }
    }
}

/// Sanitized public error payload.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ShellPublicError {
    /// Stable machine-readable public error code.
    pub code: ShellErrorCode,
    /// Safe human-readable message.
    pub message: String,
}

/// Route-shell-specific public error codes.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ShellErrorCode {
    /// The route exists but Core behavior is intentionally not wired yet.
    NotImplemented,
}

/// Returns a 501 response for Core API surfaces that are only route shells.
#[must_use]
pub fn not_implemented_response() -> (StatusCode, Json<ShellErrorResponse>) {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(ShellErrorResponse::new(
            ShellErrorCode::NotImplemented,
            "Core operation is not implemented in this server route shell.",
        )),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_error_serializes_safe_not_implemented_code() {
        let json = serde_json::to_value(ShellErrorResponse::new(
            ShellErrorCode::NotImplemented,
            "Core operation is not implemented in this server route shell.",
        ))
        .expect("shell error should serialize");

        assert_eq!(json["error"]["code"], "not_implemented");
        assert!(!json.to_string().contains("secret"));
        assert!(!json.to_string().contains("stack"));
        assert!(!json.to_string().contains("/srv/"));
    }
}
