//! JSON-serializable API contracts and DTOs for the Haze Sync Core HTTP API.
//!
//! This crate defines contract data shapes only. It does not register Axum
//! routes, start a server, verify credentials, access storage, or implement Core
//! sync behavior.

pub mod contracts;
pub mod dto;

/// Human-readable crate role used by skeleton smoke checks and documentation.
pub const CRATE_ROLE: &str = "JSON-serializable API contracts and DTOs for the Haze Sync Core HTTP API.";

/// Returns the package name for this crate.
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
