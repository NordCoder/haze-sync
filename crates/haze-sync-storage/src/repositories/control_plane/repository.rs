include!("repository/base.rs");
include!("repository/maintenance.rs");
include!("repository/inventory_controls.rs");

mod effective_records {
    use super::*;

    include!("repository/effective_evidence.rs");
}

pub use effective_records::{lock_adapter_effective_control, upsert_adapter_effective_control};

mod credential_records {
    use super::*;

    include!("repository/credentials.rs");
}

pub use credential_records::{
    cas_update_credential_lifecycle, cas_update_principal, insert_credential, insert_principal,
    lock_credential, lock_principal, read_credential_by_id, read_legacy_credential_by_sha256,
};

mod job_records {
    use super::*;

    include!("repository/jobs.rs");
}

pub use job_records::{
    append_operational_audit_event, cas_acquire_execution_slot,
    cas_block_execution_slot_uncertain, cas_reconcile_execution_slot_uncertainty,
    cas_release_execution_slot, cas_update_operational_job, ensure_scoped_execution_slot,
    insert_operational_evidence, lock_execution_slot,
};

include!("repository/runtime_lease.rs");
include!("repository/runtime_report/open.rs");
include!("repository/runtime_report/accept.rs");
include!("repository/runtime_report/uncertain.rs");
include!("repository/runtime_report/reconcile.rs");
include!("repository/facts.rs");
include!("repository/qa_corrections.rs");
