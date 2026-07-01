//! Server configuration primitives.
//!
//! These types and loaders are intentionally not wired into server startup yet.
//! They provide deterministic env parsing and redacted config wrappers for later
//! server/API phases.

pub mod env;
pub mod error;
pub mod types;

pub use error::ConfigError;
pub use types::{
    AdapterMode, AdapterModesConfig, DatabaseConfig, DatabaseUrl, ObjectStoreConfig, ServerConfig,
    WorktreeConfig,
};
