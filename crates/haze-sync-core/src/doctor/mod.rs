//! Safe passive doctor-check and safety-report primitives.
//!
//! Core accepts already-redacted facts and derives deterministic statuses,
//! messages, details, and aggregate summaries. It never opens database
//! connections, probes filesystems or object stores, calls providers, repairs
//! state, exposes local absolute paths, or serializes credentials.

mod checks;
mod report;
mod types;

pub use checks::{
    adapter_cursor_check, adapter_token_sanity_check, db_connectivity_check, gdrive_mapping_check,
    missing_blob_detection_check, object_store_exists_writable_check, worktree_drift_check,
    AdapterCursorCheckInput, AdapterTokenInspection, AdapterTokenSanityInput,
    DbConnectivityCheckInput, GdriveMappingCheckInput, MissingBlobDetectionInput,
    ObjectStoreExistsWritableInput, WorktreeDriftCheckInput,
};
pub use report::{DoctorReport, DoctorReportSummary};
pub use types::{
    AdapterCursorDetails, AdapterTokenSanityDetails, DbConnectivityDetails, DoctorCheckDetails,
    DoctorCheckId, DoctorCheckMessage, DoctorCheckResult, DoctorCheckStatus, DoctorNotRunDetails,
    DoctorNotRunReason, DoctorPlaceholderDetails, DoctorPlaceholderReason, GdriveMappingDetails,
    MissingBlobDetectionDetails, ObjectStoreExistsWritableDetails, WorktreeDriftDetails,
};

#[cfg(test)]
mod tests;
