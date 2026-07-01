//! Adapter authentication primitives for Haze Sync API contracts.
//!
//! This module provides safe value types and pure helpers only. It does not wire
//! HTTP middleware, read persistent storage, create tokens, or perform runtime
//! authentication lookups.

use haze_sync_common::security::{SecretString, REDACTED};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt, str::FromStr};
use subtle::ConstantTimeEq;

/// Adapter roles accepted by the V1 API contract.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterRole {
    /// Obsidian plugin adapter running inside the user's vault.
    ObsidianPlugin,
    /// Google Drive API adapter running on the VPS.
    GdriveAdapter,
    /// Built-in VPS worktree adapter.
    WorktreeAdapter,
    /// Administrative principal for token, policy, repair, and status actions.
    Admin,
}

impl AdapterRole {
    /// Returns whether this role may read file content and metadata from Core.
    #[must_use]
    pub const fn can_read_files(self) -> bool {
        matches!(
            self,
            Self::ObsidianPlugin | Self::GdriveAdapter | Self::WorktreeAdapter | Self::Admin
        )
    }

    /// Returns whether this role may submit file writes to Core.
    #[must_use]
    pub const fn can_write_files(self) -> bool {
        matches!(
            self,
            Self::ObsidianPlugin | Self::GdriveAdapter | Self::WorktreeAdapter | Self::Admin
        )
    }

    /// Returns whether this role may resolve user-visible conflicts.
    #[must_use]
    pub const fn can_resolve_conflicts(self) -> bool {
        matches!(self, Self::ObsidianPlugin | Self::Admin)
    }

    /// Returns whether this role may perform administrative operations.
    #[must_use]
    pub const fn can_admin(self) -> bool {
        matches!(self, Self::Admin)
    }
}

impl fmt::Display for AdapterRole {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ObsidianPlugin => "obsidian_plugin",
            Self::GdriveAdapter => "gdrive_adapter",
            Self::WorktreeAdapter => "worktree_adapter",
            Self::Admin => "admin",
        })
    }
}

impl FromStr for AdapterRole {
    type Err = AuthError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "obsidian_plugin" => Ok(Self::ObsidianPlugin),
            "gdrive_adapter" => Ok(Self::GdriveAdapter),
            "worktree_adapter" => Ok(Self::WorktreeAdapter),
            "admin" => Ok(Self::Admin),
            _ => Err(AuthError::UnknownAdapterRole),
        }
    }
}

/// Authenticated adapter identity after token verification.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AdapterPrincipal {
    adapter_id: String,
    role: AdapterRole,
}

impl AdapterPrincipal {
    /// Creates a principal for a verified adapter.
    pub fn new(adapter_id: impl Into<String>, role: AdapterRole) -> Result<Self, AuthError> {
        let adapter_id = adapter_id.into();
        if adapter_id.trim().is_empty() {
            return Err(AuthError::EmptyAdapterId);
        }

        Ok(Self { adapter_id, role })
    }

    /// Stable adapter identifier.
    #[must_use]
    pub fn adapter_id(&self) -> &str {
        &self.adapter_id
    }

    /// Adapter authorization role.
    #[must_use]
    pub const fn role(&self) -> AdapterRole {
        self.role
    }
}

/// Bearer token wrapper that redacts itself in all formatting output.
#[derive(Clone, Eq, PartialEq)]
pub struct BearerToken {
    secret: SecretString,
}

impl BearerToken {
    /// Creates a bearer token wrapper from an already-extracted token value.
    pub fn new(token: impl Into<String>) -> Result<Self, AuthError> {
        let token = token.into();
        if token.is_empty() || token.contains(['\r', '\n']) {
            return Err(AuthError::InvalidBearerToken);
        }

        Ok(Self {
            secret: SecretString::new(token),
        })
    }

    /// Parses an Authorization header in the form `Bearer <token>`.
    pub fn parse_authorization_header(header: &str) -> Result<Self, AuthError> {
        let token = header
            .strip_prefix("Bearer ")
            .ok_or(AuthError::InvalidAuthorizationHeader)?;
        if token.trim().is_empty() || token.trim() != token {
            return Err(AuthError::InvalidBearerToken);
        }

        Self::new(token.to_owned())
    }

    /// Returns the sensitive token value for hashing or verification code only.
    #[must_use]
    pub fn as_sensitive_str(&self) -> &str {
        self.secret.as_sensitive_str()
    }

    /// Computes a SHA-256 token hash representation.
    #[must_use]
    pub fn sha256_hash(&self) -> TokenHash {
        let digest = Sha256::digest(self.secret.as_sensitive_str().as_bytes());
        TokenHash {
            algorithm: TokenHashAlgorithm::Sha256,
            digest_hex: hex::encode(digest),
        }
    }
}

impl fmt::Debug for BearerToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("BearerToken").field(&REDACTED).finish()
    }
}

/// Token hash algorithms understood by these pure primitives.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenHashAlgorithm {
    /// SHA-256 hash helper suitable for deterministic tests and primitive wiring.
    Sha256,
}

impl fmt::Display for TokenHashAlgorithm {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Sha256 => "sha256",
        })
    }
}

/// Server-side token hash representation.
#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
pub struct TokenHash {
    algorithm: TokenHashAlgorithm,
    digest_hex: String,
}

impl TokenHash {
    /// Builds a SHA-256 token hash from a hexadecimal digest.
    pub fn from_sha256_hex(digest_hex: impl Into<String>) -> Result<Self, AuthError> {
        let digest_hex = digest_hex.into().to_ascii_lowercase();
        let decoded = hex::decode(&digest_hex).map_err(|_| AuthError::InvalidTokenHash)?;
        if decoded.len() != 32 {
            return Err(AuthError::InvalidTokenHash);
        }

        Ok(Self {
            algorithm: TokenHashAlgorithm::Sha256,
            digest_hex,
        })
    }

    /// Hash algorithm identifier.
    #[must_use]
    pub const fn algorithm(&self) -> TokenHashAlgorithm {
        self.algorithm
    }

    /// Hex digest used for storage and constant-time comparison.
    #[must_use]
    pub fn digest_hex(&self) -> &str {
        &self.digest_hex
    }
}

impl fmt::Debug for TokenHash {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TokenHash")
            .field("algorithm", &self.algorithm)
            .field("digest_hex", &REDACTED)
            .finish()
    }
}

impl fmt::Display for TokenHash {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}", self.algorithm, REDACTED)
    }
}

/// Pure token verification interface for later middleware/storage wiring.
pub trait TokenVerifier {
    /// Returns true only when `token` matches the stored `expected_hash`.
    fn verify(&self, token: &BearerToken, expected_hash: &TokenHash) -> bool;
}

/// SHA-256 verifier for deterministic primitive tests and future wiring.
#[derive(Clone, Copy, Debug, Default)]
pub struct Sha256TokenVerifier;

impl TokenVerifier for Sha256TokenVerifier {
    fn verify(&self, token: &BearerToken, expected_hash: &TokenHash) -> bool {
        verify_sha256_bearer_token(token, expected_hash)
    }
}

/// Verifies a bearer token against a SHA-256 token hash with constant-time digest comparison.
#[must_use]
pub fn verify_sha256_bearer_token(token: &BearerToken, expected_hash: &TokenHash) -> bool {
    if expected_hash.algorithm != TokenHashAlgorithm::Sha256 {
        return false;
    }

    let actual_hash = token.sha256_hash();
    actual_hash
        .digest_hex
        .as_bytes()
        .ct_eq(expected_hash.digest_hex.as_bytes())
        .into()
}

/// Safe public authentication errors.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthError {
    /// Authorization header is absent or does not use the Bearer scheme.
    InvalidAuthorizationHeader,
    /// Bearer token value is empty or malformed.
    InvalidBearerToken,
    /// Stored token hash is malformed.
    InvalidTokenHash,
    /// Adapter role string is not recognized.
    UnknownAdapterRole,
    /// Adapter principal was constructed without a stable adapter identifier.
    EmptyAdapterId,
}

impl fmt::Display for AuthError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidAuthorizationHeader => "invalid authorization header",
            Self::InvalidBearerToken => "invalid bearer token",
            Self::InvalidTokenHash => "invalid token hash",
            Self::UnknownAdapterRole => "unknown adapter role",
            Self::EmptyAdapterId => "empty adapter id",
        })
    }
}

impl Error for AuthError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bearer_authorization_header() {
        let token = BearerToken::parse_authorization_header("Bearer non_sensitive_fixture_value")
            .expect("header should parse");

        assert_eq!(token.as_sensitive_str(), "non_sensitive_fixture_value");
    }

    #[test]
    fn token_debug_redacts_raw_value() {
        let raw_value = "non_sensitive_fixture_token";
        let token = BearerToken::new(raw_value).expect("fixture token should be valid");

        let formatted = format!("{token:?}");

        assert!(formatted.contains(REDACTED));
        assert!(!formatted.contains(raw_value));
    }

    #[test]
    fn token_hash_debug_redacts_digest() {
        let token = BearerToken::new("non_sensitive_fixture_token")
            .expect("fixture token should be valid");
        let hash = token.sha256_hash();

        let formatted = format!("{hash:?}");

        assert!(formatted.contains(REDACTED));
        assert!(!formatted.contains(hash.digest_hex()));
    }

    #[test]
    fn verifies_matching_sha256_token_hash() {
        let token = BearerToken::new("non_sensitive_fixture_token")
            .expect("fixture token should be valid");
        let hash = token.sha256_hash();

        assert!(verify_sha256_bearer_token(&token, &hash));
        assert!(Sha256TokenVerifier.verify(&token, &hash));
    }

    #[test]
    fn rejects_non_matching_sha256_token_hash() {
        let token = BearerToken::new("non_sensitive_fixture_token")
            .expect("fixture token should be valid");
        let other_token = BearerToken::new("different_non_sensitive_fixture_token")
            .expect("fixture token should be valid");
        let other_hash = other_token.sha256_hash();

        assert!(!verify_sha256_bearer_token(&token, &other_hash));
    }

    #[test]
    fn validates_stored_sha256_hash_shape() {
        let token = BearerToken::new("non_sensitive_fixture_token")
            .expect("fixture token should be valid");
        let hash = token.sha256_hash();

        let parsed = TokenHash::from_sha256_hex(hash.digest_hex()).expect("hash should parse");

        assert_eq!(parsed, hash);
        assert_eq!(TokenHash::from_sha256_hex("abcd"), Err(AuthError::InvalidTokenHash));
    }

    #[test]
    fn parses_and_serializes_roles() {
        assert_eq!(
            "obsidian_plugin".parse::<AdapterRole>(),
            Ok(AdapterRole::ObsidianPlugin)
        );
        assert_eq!(AdapterRole::Admin.to_string(), "admin");
        assert_eq!(
            serde_json::to_string(&AdapterRole::GdriveAdapter).expect("role serializes"),
            "\"gdrive_adapter\""
        );
    }

    #[test]
    fn role_permissions_follow_contract_boundaries() {
        assert!(AdapterRole::ObsidianPlugin.can_resolve_conflicts());
        assert!(!AdapterRole::GdriveAdapter.can_resolve_conflicts());
        assert!(!AdapterRole::WorktreeAdapter.can_admin());
        assert!(AdapterRole::Admin.can_admin());
    }
}
