//! Safe Google Drive change-feed polling and reconciliation planning.
//!
//! Provider polling, cursor persistence, retry policy, work classification, and
//! trigger debouncing remain adapter-local and dependency-injected. This module
//! performs no Core/API calls, provider mutations, direct database access, or
//! background scheduling.

mod model;
mod policy;
mod provider;
mod runner;

pub use model::{
    ChangeFeedCycleInput, ChangeFeedCycleOutcome, ChangeFeedModelError, ChangeProcessingError,
    ChangeWorkBatch, ChangeWorkItem, ChangeWorkProcessor, CursorStoreError, DriveChangeEntry,
    DriveChangePage, DriveChangePoll, DriveCursorStore, FullScanFallbackReason,
    InMemoryDriveCursorStore,
};
pub use policy::{
    ChangePollDebouncer, ChangePollTrigger, DebouncedPoll, ProviderBackoffPolicy, RetryDisposition,
};
pub use provider::{DriveChangeFeedProvider, FakeDriveChangeFeedProvider};
pub use runner::{classify_drive_changes, run_change_feed_cycle, ChangeFeedError};

#[cfg(test)]
mod tests;
