//! Storage schema, passive database models, and local object store primitives for Haze Sync.
//!
//! This crate currently exposes table metadata, row-shaped model structs, and
//! content-addressed object store primitives. Repository functions, SQL execution,
//! migration running, production database connectivity, and Core storage policy
//! are implemented in later phases.

pub mod models;
pub mod object_store;
pub mod schema;

/// Human-readable crate role used by skeleton smoke checks and documentation.
pub const CRATE_ROLE: &str = "Storage schema metadata, passive database row models, and object store primitives.";

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
