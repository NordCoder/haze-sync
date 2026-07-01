//! Storage schema and passive database models for Haze Sync.
//!
//! This crate currently exposes table metadata and row-shaped model structs only.
//! Repository functions, SQL execution, migration running, production database
//! connectivity, and Core storage policy are implemented in later phases.

pub mod models;
pub mod schema;

#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

/// Human-readable crate role used by skeleton smoke checks and documentation.
pub const CRATE_ROLE: &str = "Storage schema metadata and passive database row models.";

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
