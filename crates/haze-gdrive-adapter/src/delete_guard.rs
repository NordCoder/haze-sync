//! Conservative Drive delete-candidate reconciliation and mass-delete safety.
//!
//! This module consumes authoritative full-scan facts, requires repeated absence
//! before proposing a Core delete, applies adapter and injected Core guardrails,
//! and keeps manual unlock unavailable without an audited external contract. It
//! performs no Drive hard deletes, direct database writes, or runtime scheduling.

mod core;
mod model;
mod runner;
mod state_store;

pub use core::{
    CoreDeleteError, CoreDeleteErrorCategory, CoreDeleteGateway, CoreDeleteGuardRequest,
    CoreDeleteRequest, CoreDeleteResponse, FakeCoreDeleteGateway,
};
pub use model::{
    CompleteDeleteScan, ConfirmedDeleteCandidate, CoreDeleteGuardBlockReason,
    CoreDeleteGuardDecision, CoreDeleteRejectedReason, DeleteBlockReason, DeleteExecution,
    DeleteModelError, DeleteReconciliationInput, DeleteReconciliationOutcome, DeleteRejection,
    DeleteSafetyNotice, DeleteScanIssue, DeleteScanObservation, ManualDeleteUnlockAvailability,
    MovedProviderIdentity, GDRIVE_ADAPTER_ID,
};
pub use runner::{run_delete_reconciliation, DeleteReconciliationError};
pub use state_store::{
    DeleteCandidateStateStore, DeleteStateError, InMemoryDeleteCandidateStateStore,
};

#[cfg(test)]
mod tests;
