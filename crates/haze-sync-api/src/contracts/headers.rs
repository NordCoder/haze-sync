//! Public header contract types for Core API requests.
//!
//! These types document and validate header value shapes for later route layers.
//! They do not implement Axum extractors or authenticate any adapter token.

use core::fmt;

/// Header name for bearer adapter authentication.
pub const AUTHORIZATION_HEADER: &str = "Authorization";

/// Header name required for idempotent writes.
pub const IDEMPOTENCY_KEY_HEADER: &str = "Idempotency-Key";

/// Header name carrying the expected upload content hash.
pub const X_CONTENT_SHA256_HEADER: &str = "X-Content-SHA256";

/// Header name carrying the known base revision or explicit null.
pub const X_BASE_REVISION_ID_HEADER: &str = "X-Base-Revision-Id";

const SHA256_PREFIX: &str = "sha256:";
const SHA256_HEX_LEN: usize = 64;

/// Safe parse errors for API header contract values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HeaderValueError {
    /// Header value was empty or only whitespace.
    Empty,
    /// Authorization header did not use the Bearer scheme.
    InvalidBearerScheme,
    /// Header value contained whitespace where the contract forbids it.
    ContainsWhitespace,
    /// Header value contained an ASCII control character.
    ContainsControlCharacter,
    /// Header value exceeded the contract-local maximum length.
    TooLong { max: usize },
    /// X-Content-SHA256 was not sha256: followed by 64 hexadecimal characters.
    InvalidSha256,
}

impl fmt::Display for HeaderValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("header value must not be empty"),
            Self::InvalidBearerScheme => f.write_str("authorization header must use Bearer scheme"),
            Self::ContainsWhitespace => f.write_str("header value must not contain whitespace"),
            Self::ContainsControlCharacter => {
                f.write_str("header value must not contain control characters")
            }
            Self::TooLong { max } => write!(f, "header value must not exceed {max} bytes"),
            Self::InvalidSha256 => f.write_str(
                "content hash header must be sha256: followed by 64 hexadecimal characters",
            ),
        }
    }
}

impl std::error::Error for HeaderValueError {}

/// Extracted bearer token value from Authorization: Bearer <adapter_token>.
///
/// Debug output is intentionally redacted because bearer tokens are secrets.
#[derive(Clone, PartialEq, Eq)]
pub struct BearerToken(String);

impl BearerToken {
    /// Parses an Authorization header value using the Bearer scheme.
    pub fn parse_authorization_header(value: impl AsRef<str>) -> Result<Self, HeaderValueError> {
        let value = value.as_ref();
        let token = value
            .strip_prefix("Bearer ")
            .ok_or(HeaderValueError::InvalidBearerScheme)?;
        Self::new(token)
    }

    /// Creates a bearer token marker from an already extracted token value.
    pub fn new(token: impl Into<String>) -> Result<Self, HeaderValueError> {
        let token = token.into();
        validate_non_empty_atom(&token, 4096)?;
        Ok(Self(token))
    }

    /// Exposes the raw token only for a later auth verifier.
    ///
    /// Callers must not serialize, log, or include this value in public errors.
    #[must_use]
    pub fn expose_for_auth(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for BearerToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("BearerToken(<redacted>)")
    }
}

/// Contract type for the Idempotency-Key write header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdempotencyKey(String);

impl IdempotencyKey {
    /// Creates an idempotency key value for later route/service layers.
    pub fn new(value: impl Into<String>) -> Result<Self, HeaderValueError> {
        let value = value.into();
        validate_non_empty_atom(&value, 512)?;
        Ok(Self(value))
    }

    /// Returns the validated idempotency key.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Contract type for X-Content-SHA256 upload verification header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContentSha256Header(String);

impl ContentSha256Header {
    /// Parses a sha256:<hex> header value.
    pub fn parse(value: impl Into<String>) -> Result<Self, HeaderValueError> {
        let value = value.into();
        validate_sha256_header(&value)?;
        Ok(Self(value.to_ascii_lowercase()))
    }

    /// Returns the full sha256:<hex> value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the hexadecimal digest without the sha256: prefix.
    #[must_use]
    pub fn hex_digest(&self) -> &str {
        &self.0[SHA256_PREFIX.len()..]
    }
}

/// Contract type for X-Base-Revision-Id supporting explicit null.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BaseRevisionIdHeader {
    /// Header value was the literal null string, meaning the adapter does not
    /// know a safe base revision.
    Null,
    /// Header value was an explicit revision identifier.
    Revision(String),
}

impl BaseRevisionIdHeader {
    /// Parses either the literal null value or a non-empty revision id atom.
    pub fn parse(value: impl Into<String>) -> Result<Self, HeaderValueError> {
        let value = value.into();
        if value == "null" {
            return Ok(Self::Null);
        }

        validate_non_empty_atom(&value, 256)?;
        Ok(Self::Revision(value))
    }

    /// Returns Some(revision_id) for known bases or None for explicit null.
    #[must_use]
    pub fn as_optional_revision_id(&self) -> Option<&str> {
        match self {
            Self::Null => None,
            Self::Revision(revision_id) => Some(revision_id.as_str()),
        }
    }

    /// Returns true when the header explicitly represented an unknown base.
    #[must_use]
    pub const fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }
}

fn validate_non_empty_atom(value: &str, max_len: usize) -> Result<(), HeaderValueError> {
    if value.trim().is_empty() {
        return Err(HeaderValueError::Empty);
    }

    if value.len() > max_len {
        return Err(HeaderValueError::TooLong { max: max_len });
    }

    if value.chars().any(char::is_whitespace) {
        return Err(HeaderValueError::ContainsWhitespace);
    }

    if value.chars().any(char::is_control) {
        return Err(HeaderValueError::ContainsControlCharacter);
    }

    Ok(())
}

fn validate_sha256_header(value: &str) -> Result<(), HeaderValueError> {
    let hex = value
        .strip_prefix(SHA256_PREFIX)
        .ok_or(HeaderValueError::InvalidSha256)?;

    if hex.len() != SHA256_HEX_LEN || !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(HeaderValueError::InvalidSha256);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bearer_token_debug_is_redacted() {
        let token = BearerToken::parse_authorization_header("Bearer secret-token").unwrap();

        assert_eq!(format!("{token:?}"), "BearerToken(<redacted>)");
        assert_eq!(token.expose_for_auth(), "secret-token");
    }

    #[test]
    fn sha256_header_requires_prefixed_hex() {
        let good = format!("sha256:{}", "a".repeat(64));
        let parsed = ContentSha256Header::parse(good).unwrap();

        assert_eq!(parsed.hex_digest(), "a".repeat(64));
        assert!(ContentSha256Header::parse("abc").is_err());
    }

    #[test]
    fn base_revision_header_supports_explicit_null() {
        assert_eq!(
            BaseRevisionIdHeader::parse("null").unwrap(),
            BaseRevisionIdHeader::Null
        );
        assert_eq!(
            BaseRevisionIdHeader::parse("rev_01J")
                .unwrap()
                .as_optional_revision_id(),
            Some("rev_01J")
        );
    }
}
