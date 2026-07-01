//! Storage schema, passive database models, local object store primitives, repositories, and test support for Haze Sync.
//!
//! This crate currently exposes table metadata, row-shaped model structs, local
//! content-addressed object store primitives, operation-log/cursor repositories,
//! and gated test support. Repository helpers execute storage SQL only; Core file
//! apply policy, conflict/delete behavior, idempotency behavior, adapters, route
//! handlers, migration running, and production database connectivity are
//! implemented in later phases.

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
