//! Shared domain and value primitives for Haze Sync.
//!
//! This crate contains deterministic, JSON-serializable value types only.
//! Runtime behavior, persistence, provider calls, HTTP routing, and sync policy
//! belong to higher-level Haze Sync components.

pub mod adapter;
pub mod error;
pub mod hash;
pub mod ids;
pub mod path;
pub mod security;

pub use adapter::{AdapterMode, AdapterRole};
pub use error::ValidationError;
pub use hash::{ContentHash, Sha256};
pub use ids::{AdapterId, ConflictId, OperationId, RevisionId};
pub use path::VaultPath;

/// Human-readable crate role used by skeleton smoke checks and documentation.
pub const CRATE_ROLE: &str = "Shared domain and value primitives for Haze Sync.";

/// Returns the package name for this crate.
#[must_use]
pub const fn package_name() -> &'static str {
    "haze-sync-common"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_name_matches_crate() {
        assert_eq!(package_name(), "haze-sync-common");
    }

    #[test]
    fn public_exports_are_usable_together() {
        let path = VaultPath::parse("Notes/a.md").unwrap();
        let adapter_id = AdapterId::parse("iphone-anna").unwrap();
        let revision_id = RevisionId::parse("rev_01JTEST").unwrap();
        let operation_id = OperationId::parse("op_01JTEST").unwrap();
        let conflict_id = ConflictId::parse("conf_01JTEST").unwrap();
        let hash: ContentHash = Sha256::parse(
            "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        )
        .unwrap();

        assert_eq!(path.as_str(), "Notes/a.md");
        assert_eq!(adapter_id.as_str(), "iphone-anna");
        assert_eq!(revision_id.as_str(), "rev_01JTEST");
        assert_eq!(operation_id.as_str(), "op_01JTEST");
        assert_eq!(conflict_id.as_str(), "conf_01JTEST");
        assert_eq!(
            hash.to_string(),
            "sha256:0000000000000000000000000000000000000000000000000000000000000000"
        );
        assert_eq!(AdapterRole::ObsidianPlugin.to_string(), "obsidian_plugin");
        assert_eq!(AdapterMode::Bidirectional.to_string(), "bidirectional");
        assert_eq!(ValidationError::InvalidIdentifier.code(), "invalid_identifier");
    }

    #[test]
    fn security_primitives_are_accessed_through_module_path() {
        let secret = security::SecretString::new("fixture_secret_value");

        assert_eq!(secret.to_string(), security::REDACTED);
        assert_eq!(secret.as_sensitive_str(), "fixture_secret_value");
    }
}
