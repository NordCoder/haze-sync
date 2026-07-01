//! Security-oriented value wrappers shared across Haze Sync crates.
//!
//! These primitives intentionally avoid runtime credential loading or token
//! creation. They only provide safe in-memory wrappers that prevent accidental
//! formatting leaks through `Debug` or `Display`.

use std::fmt;

/// Fixed marker used when formatting secret-bearing values.
pub const REDACTED: &str = "[REDACTED]";

/// In-memory string wrapper for secrets, tokens, and credential-like values.
///
/// The wrapped value is intentionally accessible only through explicitly named
/// sensitive accessors. Formatting this type never prints the wrapped value.
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct SecretString(String);

impl SecretString {
    /// Wraps a sensitive string value.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the wrapped sensitive value for code that must hash or verify it.
    #[must_use]
    pub fn as_sensitive_str(&self) -> &str {
        &self.0
    }

    /// Consumes the wrapper and returns the wrapped sensitive string.
    #[must_use]
    pub fn into_sensitive_string(self) -> String {
        self.0
    }

    /// Returns whether the wrapped value is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("SecretString").field(&REDACTED).finish()
    }
}

impl fmt::Display for SecretString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(REDACTED)
    }
}

impl From<String> for SecretString {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for SecretString {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_redacts_wrapped_value() {
        let raw_value = "non_sensitive_fixture_value";
        let secret = SecretString::new(raw_value);

        let formatted = format!("{secret:?}");

        assert!(formatted.contains(REDACTED));
        assert!(!formatted.contains(raw_value));
    }

    #[test]
    fn display_redacts_wrapped_value() {
        let raw_value = "another_non_sensitive_fixture_value";
        let secret = SecretString::new(raw_value);

        let formatted = secret.to_string();

        assert_eq!(formatted, REDACTED);
        assert!(!formatted.contains(raw_value));
    }
}
