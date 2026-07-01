//! Sync Core primitives and services for Haze Sync.
//!
//! This crate currently exposes deterministic Core-domain helpers that do not
//! start server runtimes, access providers, persist file bytes, or perform route
//! handling.

pub mod idempotency;

/// Human-readable crate role used by skeleton smoke checks and documentation.
pub const CRATE_ROLE: &str = "Sync Core primitives and services for Haze Sync.";

/// Returns the package name for this crate.
#[must_use]
pub const fn package_name() -> &'static str {
    "haze-sync-core"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_name_matches_crate() {
        assert_eq!(package_name(), "haze-sync-core");
    }
}
