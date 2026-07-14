//! JSON DTOs for the Haze Sync Core API contract.
//!
//! DTOs in this module describe metadata, request, and response shapes only.
//! File bytes are intentionally outside these JSON types.

pub mod changes;
pub mod common;
pub mod conflicts;
pub mod files;
pub mod primitives;
pub mod server;
pub mod worktree;

#[cfg(test)]
mod public_contract_tests;
