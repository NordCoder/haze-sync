//! Google Drive adapter runtime foundation.
//!
//! The crate currently owns configuration, redaction, and process lifecycle
//! scaffolding only. Provider and Core/API calls are intentionally deferred.

pub mod config;
pub mod error;
pub mod runtime;

pub use config::{
    AdapterConfig, AdapterMode, DeleteSafetyConfig, RuntimeIntervals, SecretPath, SecretString,
};
pub use error::{ConfigError, ConfigErrorCategory, RuntimeError, RuntimeErrorCategory};
pub use runtime::{AdapterRuntime, RuntimeState, StartupStatus};
