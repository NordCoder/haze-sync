//! Storage schema, passive database models, local object store primitives, repository helpers, and test support for Haze Sync.
//!
//! This crate currently exposes table metadata, row-shaped model structs,
//! content-addressed object store primitives, and narrow repository helpers.
//! Test support is gated and intended only for tests/future integration harnesses.
//! Repository functions do not create global pools, run migrations, or start
//! production runtime behavior.

pub mod models;
pub mod object_store;
pub mod repositories;
pub mod schema;

#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

pub use object_store::{
    LocalObjectStore, ObjectMetadata, ObjectStore, ObjectStoreError, ObjectStoreResult,
};

/// Human-readable crate role used by skeleton smoke checks and documentation.
pub const CRATE_ROLE: &str =
    "Storage schema metadata, passive database row models, object store primitives, repositories, and test support.";

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
