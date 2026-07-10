//! Built-in VPS worktree adapter foundations.
//!
//! The worktree is a local filesystem materialization of Core state. It is not
//! the sync source of truth; later phases add scanning, importing, writing, and
//! runtime behavior on top of these safe path-mapping primitives.

mod delete_guard;
mod echo_guard;
mod file_import;
mod hashing;
mod import_planner;
mod materializer;
mod path_mapping;
mod reconciliation;
mod scanner;
mod trash;

pub use delete_guard::{
    WorktreeDeleteAuthorization, WorktreeDeleteBlockReason, WorktreeDeleteCandidate,
    WorktreeDeleteGuardDecision, WorktreeDeleteGuardPolicy, WorktreeDeleteGuardPolicyError,
    WorktreeDeleteGuardSummary, WorktreeDeletePlanError, WorktreeDeleteRunError,
    WorktreeDeleteRunner, WorktreeDeleteScan, WorktreeDeleteSubmission,
    WorktreeDeleteSubmissionReport, WorktreeGuardedDeletePlan,
};
pub use echo_guard::{
    WorktreeEchoDecision, WorktreeEchoGuard, WorktreeEchoGuardError, WorktreeEchoGuardPolicy,
    WorktreeEchoLoadSummary, WorktreeEchoRecordSummary,
};
pub use file_import::{
    WorktreeImportPlan, WorktreeImportPlanner, WorktreeImportRunner, WorktreeImportSubmission,
    WorktreeImportSubmissionReport,
};
pub use import_planner::{
    WorktreeAcceptedImport, WorktreeAppliedFileState, WorktreeAppliedPathState,
    WorktreeBaseRevision, WorktreeDeleteImport, WorktreeImportAction, WorktreeImportClient,
    WorktreeImportFile, WorktreeImportOutcome, WorktreeImportPlanError, WorktreeImportRejection,
    WorktreePutImport, WorktreeStateSnapshot, WorktreeTombstoneState, WorktreeUnchangedFile,
};
pub use materializer::{
    AtomicWorktreeWriter, WorktreeDeferredMaterialization, WorktreeEchoMarker,
    WorktreeLocalImportCandidate, WorktreeMaterializationOutcome, WorktreeMaterializationRequest,
    WorktreeMaterializeError, WorktreeMaterializedFile, WorktreeMaterializer,
};
pub use path_mapping::{
    WorktreeConfig, WorktreePathError, ECHO_DIR_NAME, METADATA_DIR_NAME, TEMP_DIR_NAME,
    TRASH_DIR_NAME, WORKTREE_RUNTIME_DIR_NAME,
};
pub use reconciliation::{
    WorktreeEchoStatus, WorktreeObservedFileState, WorktreeReconciler, WorktreeReconciliationEntry,
    WorktreeReconciliationError, WorktreeReconciliationKind, WorktreeReconciliationReport,
    WorktreeReconciliationRunError, WorktreeReconciliationRunner, WorktreeReconciliationState,
    WorktreeReconciliationStateStore, WorktreeReconciliationStateTransition,
    WorktreeReconciliationSummary,
};
pub use scanner::{
    StableFileDetector, StableFileObservation, StableFileState, WorktreeFileSnapshot,
    WorktreeScanError, WorktreeScanResult, WorktreeScanSkipReason, WorktreeScanSkipped,
    WorktreeScanner,
};
pub use trash::{
    WorktreeTombstoneMaterializationOutcome, WorktreeTombstoneMaterializationRequest,
    WorktreeTrashError, WorktreeTrashManager, WorktreeTrashPolicy, WorktreeTrashRecord,
    WorktreeTrashRecordId,
};

/// Human-readable crate role used by skeleton smoke checks and documentation.
pub const CRATE_ROLE: &str = "Built-in VPS worktree adapter foundations.";

/// Returns the package name for this crate.
#[must_use]
pub const fn package_name() -> &'static str {
    "haze-sync-worktree"
}

#[cfg(test)]
mod delete_guard_reserved_tests;
#[cfg(test)]
mod delete_guard_tests;
#[cfg(test)]
mod echo_guard_tests;
#[cfg(test)]
mod file_import_tests;
#[cfg(test)]
mod materializer_safety_tests;
#[cfg(test)]
mod reconciliation_tests;
#[cfg(test)]
mod trash_integrity_tests;
#[cfg(test)]
mod trash_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_name_matches_crate() {
        assert_eq!(package_name(), "haze-sync-worktree");
    }
}
