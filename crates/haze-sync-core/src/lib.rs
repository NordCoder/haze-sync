//! Placeholder crate for the future Sync Core.
//!
//! Wave 0 intentionally exposes only a tiny placeholder API. Real V1 behavior is
//! implemented by later phases.

/// Human-readable crate role used by skeleton smoke checks and documentation.
pub const CRATE_ROLE: &str = "Placeholder crate for the future Sync Core.";

/// Returns the package name for this placeholder crate.
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
