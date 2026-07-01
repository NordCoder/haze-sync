//! Idempotency primitives for safe write replay handling.
//!
//! These types model retry-safety metadata only. They do not store raw request
//! bodies, file bytes, bearer tokens, provider payloads, database URLs, or
//! runtime details.

use haze_sync_common::{AdapterId, Sha256};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use sha2::{Digest, Sha256 as Sha256Digest};
use std::{collections::BTreeMap, error::Error, fmt, str::FromStr};

const MAX_IDEMPOTENCY_KEY_LEN: usize = 512;
const MIN_STATUS_CODE: u16 = 100;
const MAX_STATUS_CODE: u16 = 599;

/// Safe, validated idempotency key from the `Idempotency-Key` header.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct IdempotencyKey(String);

impl IdempotencyKey {
    /// Parses and validates an idempotency key.
    pub fn parse(input: &str) -> Result<Self, IdempotencyError> {
        if input.is_empty()
            || input.len() > MAX_IDEMPOTENCY_KEY_LEN
            || input.as_bytes().iter().any(|byte| !is_visible_header_byte(*byte))
        {
            return Err(IdempotencyError::InvalidKey);
        }

        Ok(Self(input.to_owned()))
    }

    /// Borrows the canonical key string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the key and returns the canonical key string.
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl AsRef<str> for IdempotencyKey {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for IdempotencyKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for IdempotencyKey {
    type Err = IdempotencyError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Self::parse(input)
    }
}

impl TryFrom<&str> for IdempotencyKey {
    type Error = IdempotencyError;

    fn try_from(input: &str) -> Result<Self, Self::Error> {
        Self::parse(input)
    }
}

impl Serialize for IdempotencyKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for IdempotencyKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(serde::de::Error::custom)
    }
}

/// Idempotency comparison scope. V1 scopes keys to an authenticated adapter.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IdempotencyScope {
    adapter_id: AdapterId,
}

impl IdempotencyScope {
    /// Creates an adapter-scoped idempotency namespace.
    #[must_use]
    pub const fn adapter(adapter_id: AdapterId) -> Self {
        Self { adapter_id }
    }

    /// Returns the adapter identifier used as the idempotency scope.
    #[must_use]
    pub const fn adapter_id(&self) -> &AdapterId {
        &self.adapter_id
    }
}

/// SHA-256 request fingerprint computed from canonical safe request metadata.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct RequestFingerprint(Sha256);

impl RequestFingerprint {
    /// Wraps a parsed SHA-256 value as a request fingerprint.
    #[must_use]
    pub const fn from_sha256(hash: Sha256) -> Self {
        Self(hash)
    }

    /// Parses a canonical or plain SHA-256 fingerprint string.
    pub fn parse(input: &str) -> Result<Self, IdempotencyError> {
        Sha256::parse(input)
            .map(Self)
            .map_err(|_| IdempotencyError::InvalidRequestFingerprint)
    }

    /// Hashes safe JSON request metadata after deterministic canonicalization.
    #[must_use]
    pub fn from_safe_json_metadata(metadata: &Value) -> Self {
        let mut canonical = String::new();
        append_canonical_json(metadata, &mut canonical);
        Self::from_safe_canonical_bytes(canonical.as_bytes())
    }

    /// Hashes already-canonical safe metadata bytes.
    #[must_use]
    pub fn from_safe_canonical_bytes(bytes: &[u8]) -> Self {
        let digest = Sha256Digest::digest(bytes);
        let mut output = [0_u8; 32];
        output.copy_from_slice(&digest);
        Self(Sha256::from_bytes(output))
    }

    /// Returns the underlying SHA-256 value.
    #[must_use]
    pub const fn as_sha256(&self) -> &Sha256 {
        &self.0
    }

    /// Consumes the fingerprint and returns the underlying SHA-256 value.
    #[must_use]
    pub const fn into_sha256(self) -> Sha256 {
        self.0
    }
}

impl fmt::Display for RequestFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, formatter)
    }
}

impl From<Sha256> for RequestFingerprint {
    fn from(hash: Sha256) -> Self {
        Self::from_sha256(hash)
    }
}

impl From<RequestFingerprint> for Sha256 {
    fn from(fingerprint: RequestFingerprint) -> Self {
        fingerprint.into_sha256()
    }
}

/// JSON-safe response snapshot stored for a successful idempotent write.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct StoredIdempotencyResponse {
    status_code: u16,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    headers: BTreeMap<String, String>,
    body: Value,
}

impl StoredIdempotencyResponse {
    /// Creates a stored response from safe public JSON body and optional headers.
    pub fn new(
        status_code: u16,
        body: Value,
        headers: BTreeMap<String, String>,
    ) -> Result<Self, IdempotencyError> {
        if !(MIN_STATUS_CODE..=MAX_STATUS_CODE).contains(&status_code) {
            return Err(IdempotencyError::InvalidStoredResponse);
        }

        let mut normalized_headers = BTreeMap::new();
        for (name, value) in headers {
            let normalized_name = validate_response_header_name(&name)?;
            validate_response_header_value(&value)?;
            normalized_headers.insert(normalized_name, value);
        }

        Ok(Self {
            status_code,
            headers: normalized_headers,
            body,
        })
    }

    /// Creates a stored JSON response without replayed headers.
    pub fn json(status_code: u16, body: Value) -> Result<Self, IdempotencyError> {
        Self::new(status_code, body, BTreeMap::new())
    }

    /// HTTP status code to return for replay.
    #[must_use]
    pub const fn status_code(&self) -> u16 {
        self.status_code
    }

    /// Safe replay headers. Sensitive headers are rejected by the constructor.
    #[must_use]
    pub const fn headers(&self) -> &BTreeMap<String, String> {
        &self.headers
    }

    /// JSON response body to replay.
    #[must_use]
    pub const fn body(&self) -> &Value {
        &self.body
    }
}

/// Stored idempotency record used to evaluate request replay behavior.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct StoredIdempotencyRecord {
    scope: IdempotencyScope,
    key: IdempotencyKey,
    request_fingerprint: RequestFingerprint,
    response: StoredIdempotencyResponse,
}

impl StoredIdempotencyRecord {
    /// Creates an idempotency record from validated safe components.
    #[must_use]
    pub const fn new(
        scope: IdempotencyScope,
        key: IdempotencyKey,
        request_fingerprint: RequestFingerprint,
        response: StoredIdempotencyResponse,
    ) -> Self {
        Self {
            scope,
            key,
            request_fingerprint,
            response,
        }
    }

    #[must_use]
    pub const fn scope(&self) -> &IdempotencyScope {
        &self.scope
    }

    #[must_use]
    pub const fn key(&self) -> &IdempotencyKey {
        &self.key
    }

    #[must_use]
    pub const fn request_fingerprint(&self) -> &RequestFingerprint {
        &self.request_fingerprint
    }

    #[must_use]
    pub const fn response(&self) -> &StoredIdempotencyResponse {
        &self.response
    }
}

/// Result of checking an incoming request against stored idempotency metadata.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum IdempotencyReplayOutcome {
    NewRequest,
    ReplaySameRequest { response: StoredIdempotencyResponse },
    ConflictDifferentRequest {
        stored_request_fingerprint: RequestFingerprint,
        incoming_request_fingerprint: RequestFingerprint,
    },
}

/// Pure idempotency decision helper.
pub struct IdempotencyService;

impl IdempotencyService {
    /// Compares an optional stored record with an incoming request fingerprint.
    #[must_use]
    pub fn evaluate(
        existing: Option<&StoredIdempotencyRecord>,
        incoming_request_fingerprint: RequestFingerprint,
    ) -> IdempotencyReplayOutcome {
        let Some(existing) = existing else {
            return IdempotencyReplayOutcome::NewRequest;
        };

        if *existing.request_fingerprint() == incoming_request_fingerprint {
            IdempotencyReplayOutcome::ReplaySameRequest {
                response: existing.response().clone(),
            }
        } else {
            IdempotencyReplayOutcome::ConflictDifferentRequest {
                stored_request_fingerprint: *existing.request_fingerprint(),
                incoming_request_fingerprint,
            }
        }
    }
}

/// Safe public idempotency errors. Variants do not carry raw input values.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IdempotencyError {
    InvalidKey,
    InvalidRequestFingerprint,
    InvalidStoredResponse,
    UnsafeResponseHeader,
}

impl IdempotencyError {
    /// Stable machine-readable error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidKey => "invalid_idempotency_key",
            Self::InvalidRequestFingerprint => "invalid_request_fingerprint",
            Self::InvalidStoredResponse => "invalid_stored_response",
            Self::UnsafeResponseHeader => "unsafe_response_header",
        }
    }

    /// Stable human-readable message with no sensitive details.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::InvalidKey => "idempotency key is invalid",
            Self::InvalidRequestFingerprint => "request fingerprint is invalid",
            Self::InvalidStoredResponse => "stored idempotency response is invalid",
            Self::UnsafeResponseHeader => "stored response header is not safe to replay",
        }
    }
}

impl fmt::Display for IdempotencyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message())
    }
}

impl Error for IdempotencyError {}

fn append_canonical_json(value: &Value, output: &mut String) {
    match value {
        Value::Null => output.push_str("null"),
        Value::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
        Value::Number(number) => output.push_str(&number.to_string()),
        Value::String(value) => output.push_str(
            &serde_json::to_string(value).expect("serializing a JSON string cannot fail"),
        ),
        Value::Array(values) => {
            output.push('[');
            for (index, item) in values.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                append_canonical_json(item, output);
            }
            output.push(']');
        }
        Value::Object(values) => {
            let mut entries: Vec<_> = values.iter().collect();
            entries.sort_by_key(|(left_key, _)| *left_key);
            output.push('{');
            for (index, (key, value)) in entries.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                output.push_str(
                    &serde_json::to_string(key).expect("serializing a JSON object key cannot fail"),
                );
                output.push(':');
                append_canonical_json(value, output);
            }
            output.push('}');
        }
    }
}

fn validate_response_header_name(name: &str) -> Result<String, IdempotencyError> {
    if name.is_empty() || name.as_bytes().iter().any(|byte| !is_header_name_byte(*byte)) {
        return Err(IdempotencyError::UnsafeResponseHeader);
    }

    let normalized = name.to_ascii_lowercase();
    if matches!(
        normalized.as_str(),
        "authorization" | "cookie" | "proxy-authorization" | "set-cookie" | "x-api-key"
    ) {
        return Err(IdempotencyError::UnsafeResponseHeader);
    }

    Ok(normalized)
}

fn validate_response_header_value(value: &str) -> Result<(), IdempotencyError> {
    if value
        .as_bytes()
        .iter()
        .any(|byte| matches!(*byte, b'\0' | b'\r' | b'\n') || *byte == 0x7f || *byte < b' ')
    {
        return Err(IdempotencyError::UnsafeResponseHeader);
    }

    Ok(())
}

fn is_visible_header_byte(byte: u8) -> bool {
    matches!(byte, b'!'..=b'~')
}

fn is_header_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-')
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn adapter_id() -> AdapterId {
        AdapterId::parse("iphone-anna").expect("fixture adapter id should parse")
    }

    fn response() -> StoredIdempotencyResponse {
        StoredIdempotencyResponse::json(200, json!({ "status": "accepted", "seq": 12_381 }))
            .expect("fixture response should be safe")
    }

    fn record(fingerprint: RequestFingerprint) -> StoredIdempotencyRecord {
        StoredIdempotencyRecord::new(
            IdempotencyScope::adapter(adapter_id()),
            IdempotencyKey::parse("iphone:iphone-anna:op-001").unwrap(),
            fingerprint,
            response(),
        )
    }

    #[test]
    fn same_key_same_fingerprint_returns_replay() {
        let fingerprint = RequestFingerprint::from_safe_json_metadata(&json!({
            "base_revision_id": "rev_123",
            "content_sha256": format!("sha256:{}", "a".repeat(64)),
            "method": "PUT",
            "path": "Projects/Haze/plan.md"
        }));
        let stored = record(fingerprint);

        assert_eq!(
            IdempotencyService::evaluate(Some(&stored), fingerprint),
            IdempotencyReplayOutcome::ReplaySameRequest {
                response: response()
            }
        );
    }

    #[test]
    fn same_key_different_fingerprint_returns_conflict() {
        let stored_fingerprint = RequestFingerprint::from_safe_json_metadata(&json!({
            "content_sha256": format!("sha256:{}", "a".repeat(64)),
            "method": "PUT",
            "path": "Projects/Haze/plan.md"
        }));
        let incoming_fingerprint = RequestFingerprint::from_safe_json_metadata(&json!({
            "content_sha256": format!("sha256:{}", "b".repeat(64)),
            "method": "PUT",
            "path": "Projects/Haze/plan.md"
        }));
        let stored = record(stored_fingerprint);

        assert_eq!(
            IdempotencyService::evaluate(Some(&stored), incoming_fingerprint),
            IdempotencyReplayOutcome::ConflictDifferentRequest {
                stored_request_fingerprint: stored_fingerprint,
                incoming_request_fingerprint: incoming_fingerprint,
            }
        );
    }

    #[test]
    fn missing_record_returns_new_request() {
        let fingerprint = RequestFingerprint::from_safe_json_metadata(&json!({
            "method": "DELETE",
            "path": "Projects/Haze/old.md"
        }));

        assert_eq!(
            IdempotencyService::evaluate(None, fingerprint),
            IdempotencyReplayOutcome::NewRequest
        );
    }

    #[test]
    fn invalid_key_is_rejected() {
        assert_eq!(IdempotencyKey::parse("").unwrap_err(), IdempotencyError::InvalidKey);
        assert_eq!(
            IdempotencyKey::parse("contains space").unwrap_err(),
            IdempotencyError::InvalidKey
        );
        assert_eq!(
            IdempotencyKey::parse("bad\0key").unwrap_err(),
            IdempotencyError::InvalidKey
        );
        assert_eq!(
            IdempotencyKey::parse(&"a".repeat(MAX_IDEMPOTENCY_KEY_LEN + 1)).unwrap_err(),
            IdempotencyError::InvalidKey
        );
    }

    #[test]
    fn stored_response_roundtrips_through_serde() {
        let mut headers = BTreeMap::new();
        headers.insert("X-Revision-Id".to_owned(), "rev_124".to_owned());
        headers.insert("Content-Type".to_owned(), "application/json".to_owned());
        let response = StoredIdempotencyResponse::new(
            201,
            json!({ "status": "accepted", "path": "Projects/Haze/plan.md" }),
            headers,
        )
        .expect("fixture response should be safe");

        let serialized = serde_json::to_string(&response).unwrap();
        assert!(serialized.contains("content-type"));
        assert!(!serialized.contains("Authorization"));

        let decoded: StoredIdempotencyResponse = serde_json::from_str(&serialized).unwrap();
        assert_eq!(decoded, response);
    }

    #[test]
    fn sensitive_headers_are_rejected() {
        let mut headers = BTreeMap::new();
        headers.insert("Authorization".to_owned(), "Bearer token".to_owned());

        assert_eq!(
            StoredIdempotencyResponse::new(200, json!({}), headers).unwrap_err(),
            IdempotencyError::UnsafeResponseHeader
        );
    }

    #[test]
    fn canonical_json_metadata_is_key_order_independent() {
        let left = RequestFingerprint::from_safe_json_metadata(&json!({
            "path": "a.md",
            "method": "PUT",
            "content_sha256": "sha256:abc"
        }));
        let right = RequestFingerprint::from_safe_json_metadata(&json!({
            "content_sha256": "sha256:abc",
            "method": "PUT",
            "path": "a.md"
        }));

        assert_eq!(left, right);
    }
}
