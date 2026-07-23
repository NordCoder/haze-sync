//! Core service-layer primitives for Haze Sync.
//!
//! This crate contains pure Core algorithms and traits. Runtime wiring, HTTP
//! routes, database pool creation, migrations, provider SDKs, and adapter loops
//! live outside this crate.
//!
//! The public module surface is intentionally limited to storage/API-neutral Core
//! decision primitives: revision upserts, conflict preservation, delete safety,
//! idempotency, operation-log value models, and passive doctor summaries.

pub mod conflict_saved_planner;
pub mod conflict_service;
pub mod delete_guard;
pub mod doctor;
pub mod idempotency;
pub mod operation_log;
pub mod policy_engine;
pub mod revision_service;
pub mod tombstone_service;

/// Human-readable crate role used by skeleton smoke checks and documentation.
pub const CRATE_ROLE: &str = "Core service-layer primitives for Haze Sync.";

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
