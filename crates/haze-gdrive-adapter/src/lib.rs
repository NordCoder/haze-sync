//! Google Drive adapter runtime foundation.
//!
//! The crate owns configuration, redaction, provider-safe Drive metadata
//! normalization, fake-first provider abstractions, adapter-local mapping and
//! cursor state, dependency-free content hashing, full-scan import planning,
//! change-feed reconciliation boundaries, and process lifecycle scaffolding.
//! Core/API execution and concrete cursor persistence remain deferred.

pub mod change_feed;
pub mod config;
pub mod drive;
pub mod error;
pub mod hash;
pub mod runtime;
pub mod scan;
pub mod state;

pub use change_feed::{
    classify_drive_changes, run_change_feed_cycle, ChangeFeedCycleInput, ChangeFeedCycleOutcome,
    ChangeFeedError, ChangeFeedModelError, ChangePollDebouncer, ChangePollTrigger,
    ChangeProcessingError, ChangeWorkBatch, ChangeWorkItem, ChangeWorkProcessor, CursorStoreError,
    DebouncedPoll, DriveChangeEntry, DriveChangeFeedProvider, DriveChangePage, DriveChangePoll,
    DriveCursorStore, FakeDriveChangeFeedProvider, FullScanFallbackReason,
    InMemoryDriveCursorStore, ProviderBackoffPolicy, RetryDisposition,
};
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
pub use hash::ContentSha256;
pub use runtime::{AdapterRuntime, RuntimeState, StartupStatus};
pub use scan::{
    plan_full_scan, CoreUploadRequest, DeleteCandidatePlan, FullScanError, FullScanInput,
    FullScanPlan, ImportChangeKind, ImportExecution, PlannedImport, ScanSkipReason, SkippedImport,
    SkippedScanEntry, UnchangedDriveEntry,
};
pub use state::{
    CoreChangeCursor, CoreStateObservation, DriveChangeCursor, DriveEchoObservation,
    DriveStateObservation, EchoDecision, EchoGuard, EchoGuardEntry, GDriveMapping,
    MappingPersistenceBoundary, SafeTimestamp, StateError, StatePersistencePolicy, VaultPath,
};
