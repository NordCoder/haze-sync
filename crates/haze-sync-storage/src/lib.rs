//! Storage schema, passive database models, local object store primitives, repository helpers, and test support for Haze Sync.
//!
//! This crate exposes table metadata, row-shaped model structs,
//! content-addressed object store primitives, passive SQLx repository helpers,
//! and transaction-scoped PostgreSQL advisory-lock helpers. Test support is
//! gated and intended only for tests/future integration harnesses. Production
//! database pool initialization, migration running, Core apply policy,
//! conflict/delete policy, and API route wiring are implemented in later phases.

pub mod locks;
pub mod models;
pub mod object_store;
pub mod repositories;
pub mod schema;

#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

pub use object_store::{
    LocalObjectStore, ObjectMetadata, ObjectStore, ObjectStoreError, ObjectStoreResult,
};
pub use repositories::{RepositoryError, RepositoryResult};

/// Human-readable crate role used by skeleton smoke checks and documentation.
pub const CRATE_ROLE: &str =
    "Storage schema metadata, passive database row models, object store primitives, repository helpers, locks, and test support.";

/// Returns the package name for this crate.
#[must_use]
pub const fn package_name() -> &'static str {
    "haze-sync-storage"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_name_matches_crate() {
        assert_eq!(package_name(), "haze-sync-storage");
    }
}
