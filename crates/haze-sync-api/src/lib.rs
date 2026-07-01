//! Placeholder crate for future API contracts and route surfaces.
//!
//! Wave 0 intentionally exposes only a tiny placeholder API. Real V1 behavior is
//! implemented by later phases.

pub mod auth;

/// Human-readable crate role used by skeleton smoke checks and documentation.
pub const CRATE_ROLE: &str = "Placeholder crate for future API contracts and route surfaces.";

/// Returns the package name for this placeholder crate.
#[must_use]
pub const fn package_name() -> &'static str {
    "haze-sync-api"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_name_matches_crate() {
        assert_eq!(package_name(), "haze-sync-api");
    }
}
