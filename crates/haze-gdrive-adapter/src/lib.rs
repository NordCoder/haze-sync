//! Google Drive adapter runtime foundation.
//!
//! The crate currently owns configuration, redaction, provider-safe Drive
//! metadata normalization, fake-first provider abstractions, and process
//! lifecycle scaffolding. Core/API calls are intentionally deferred.

pub mod config;
pub mod drive;
pub mod error;
pub mod runtime;

pub use config::{
    AdapterConfig, AdapterMode, DeleteSafetyConfig, RuntimeIntervals, SecretPath, SecretString,
};
pub use drive::{
    classify_drive_metadata, normalize_drive_metadata, DriveEntryClassification, DriveEntryKind,
    DriveMetadata, DriveMutationOutcome, DriveProvider, DriveUpdateRequest, DriveUploadRequest,
    FakeDriveProvider, NormalizedDriveEntry, ProviderError, ProviderErrorCategory,
    SupportedFileType, UnsupportedEntryReason,
};
pub use error::{ConfigError, ConfigErrorCategory, RuntimeError, RuntimeErrorCategory};
pub use runtime::{AdapterRuntime, RuntimeState, StartupStatus};
