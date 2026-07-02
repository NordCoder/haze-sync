//! Passive route-level API contract helpers.
//!
//! Route helper modules parse request-shaped inputs and build response-shaped
//! metadata only. They do not register server routes, perform authentication,
//! access storage, or call Core services.

pub mod changes;
pub mod delete;
pub mod files;
