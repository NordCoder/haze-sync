//! Haze Sync server scaffold binary.
//!
//! This phase exposes deterministic HTTP route boundaries and server-side
//! database/readiness helpers. It does not start a production listener, auto-run
//! migrations, verify adapter tokens, or perform adapter runtime behavior.

pub mod config;
pub mod db;
pub mod http;
pub mod readiness;
pub mod routes;
pub mod state;

/// Human-readable crate role used by smoke checks and documentation.
pub const CRATE_ROLE: &str =
    "Haze Sync HTTP server scaffolding, configuration, DB readiness, migrations, and W2/W3 route wiring.";

/// Returns the package name for this crate.
#[must_use]
pub const fn package_name() -> &'static str {
    "haze-sync-server"
}

fn main() {
    let _app = routes::build_router();
    println!("haze-sync-server scaffold");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_name_matches_crate() {
        assert_eq!(package_name(), "haze-sync-server");
    }
}
