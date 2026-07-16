//! Passive route-level API contract helpers.
//!
//! Route helper modules parse request-shaped inputs and build response-shaped
//! metadata only. They do not register server routes, perform authentication,
//! access storage, or call Core services.

pub mod admin;
pub mod changes;
pub mod conflicts;
pub mod delete;
pub mod files;
pub mod gdrive;
