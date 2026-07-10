//! Core-to-Google-Drive export planning and replay-safe fake-first apply runner.
//!
//! Core/API reads, provider mutation, mapping/cursor persistence, and echo state
//! are dependency-injected. No concrete HTTP, OAuth, database, or background
//! runtime wiring is owned here.

mod core;
mod model;
mod provider;
mod runner;
mod state_store;

pub use core::{CoreClientError, CoreClientErrorCategory, CoreExportClient, FakeCoreExportClient};
pub use model::{
    CoreExportChange, CoreExportPage, CoreFileContent, DriveCreateTarget, ExportCycleInput,
    ExportCycleOutcome, ExportExecution, ExportModelError, ExportPlanItem, ExportSkipReason,
    VerifiedExportSource, DEFAULT_CORE_CHANGE_PAGE_LIMIT,
};
pub use provider::{
    DriveCreateExportRequest, DriveExportError, DriveExportErrorCategory, DriveExportProvider,
    DriveExportReceipt, DriveTrashExportRequest, DriveUpdateExportRequest, ExportRetryDisposition,
    ExportRetryPolicy, FakeDriveExportProvider,
};
pub use runner::{
    cursor_after_page, echo_observation_for_mapping, plan_core_export, run_export_cycle,
    ExportError,
};
pub use state_store::{ExportStateError, ExportStateStore, InMemoryExportStateStore};

#[cfg(test)]
mod tests;
