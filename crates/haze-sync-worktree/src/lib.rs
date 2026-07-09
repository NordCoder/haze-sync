//! Built-in VPS worktree adapter foundations.
//!
//! The worktree is a local filesystem materialization of Core state. It is not
//! the sync source of truth; later phases add scanning, importing, writing, and
//! runtime behavior on top of these safe path-mapping primitives.

mod import_planner;
mod path_mapping;
mod scanner;

pub use import_planner::{
    WorktreeAcceptedImport, WorktreeAppliedFileState, WorktreeAppliedPathState,
    WorktreeBaseRevision, WorktreeDeleteImport, WorktreeImportAction, WorktreeImportClient,
    WorktreeImportFile, WorktreeImportOutcome, WorktreeImportPlan, WorktreeImportPlanError,
    WorktreeImportPlanner, WorktreeImportRejection, WorktreeImportRunner, WorktreeImportSubmission,
    WorktreeImportSubmissionReport, WorktreePutImport, WorktreeStateSnapshot,
    WorktreeTombstoneState, WorktreeUnchangedFile,
};
pub use path_mapping::{
    WorktreeConfig, WorktreePathError, ECHO_DIR_NAME, METADATA_DIR_NAME, TEMP_DIR_NAME,
    TRASH_DIR_NAME, WORKTREE_RUNTIME_DIR_NAME,
};
pub use scanner::{
    StableFileDetector, StableFileObservation, StableFileState, WorktreeFileSnapshot,
    WorktreeScanError, WorktreeScanResult, WorktreeScanSkipReason, WorktreeScanSkipped,
    WorktreeScanner,
};

/// Human-readable crate role used by skeleton smoke checks and documentation.
pub const CRATE_ROLE: &str = "Built-in VPS worktree adapter foundations.";

/// Returns the package name for this crate.
#[must_use]
pub const fn package_name() -> &'static str {
    "haze-sync-worktree"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_name_matches_crate() {
        assert_eq!(package_name(), "haze-sync-worktree");
    }
}
