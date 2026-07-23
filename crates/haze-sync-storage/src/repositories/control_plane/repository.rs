include!("repository/base.rs");
include!("repository/maintenance.rs");
include!("repository/inventory_controls.rs");

// The raw effective-state writer is private. Standalone GDrive authority is
// publicly mutable only through the complete ordered-report transaction below.
#[allow(dead_code)]
mod effective_records {
    use super::*;

    include!("repository/effective_evidence.rs");
}

pub use effective_records::lock_adapter_effective_control;
use effective_records::upsert_adapter_effective_control;

/// Writes effective state for hosted/non-standalone adapters only.
///
/// The inventory row is locked in the caller transaction so an adapter cannot
/// change authority while the bounded write is being validated. Runtime lease,
/// epoch, report, and lease-expiry fields are reserved for ordered standalone
/// GDrive reports.
pub async fn upsert_hosted_adapter_effective_control(
    transaction: &mut Transaction<'_, Postgres>,
    input: &AdapterEffectiveControlInput,
) -> ControlPlaneResult<AdapterEffectiveControlRow> {
    validate_identifier(&input.adapter_id)?;
    let inventory = sqlx::query(
        "select adapter_kind, control_authority from adapter_inventory \
         where adapter_id = $1 and retired_at is null for share",
    )
    .bind(&input.adapter_id)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_database_error)?
    .ok_or(ControlPlaneRepositoryError::MissingRecord)?;
    let adapter_kind: String = row_column!(&inventory, "adapter_kind");
    let control_authority: String = row_column!(&inventory, "control_authority");
    validate_identifier(&adapter_kind)?;
    validate_identifier(&control_authority)?;

    if control_authority == "standalone" {
        return Err(ControlPlaneRepositoryError::RuntimeLeaseConflict);
    }
    if input.runtime_instance_id.is_some()
        || input.standalone_runtime_epoch.is_some()
        || input.report_sequence.is_some()
        || input.runtime_lease_version.is_some()
        || input.runtime_lease_expires_at.is_some()
    {
        return Err(ControlPlaneRepositoryError::RuntimeReportConflict);
    }

    upsert_adapter_effective_control(transaction, input).await
}

// The former blocking issuance path remains private for regression coverage.
#[allow(dead_code)]
mod credential_records {
    use super::*;

    include!("repository/credentials.rs");
}

pub use credential_records::{
    insert_principal, lock_credential, lock_principal, read_credential_by_id,
    read_legacy_credential_by_sha256,
};

fn verify_principal_cas(
    current: &PrincipalRow,
    expected_principal_version: i64,
    expected_credential_set_generation: i64,
) -> ControlPlaneResult<()> {
    if current.principal_version != expected_principal_version {
        return Err(stale(cas_namespaces::PRINCIPAL_VERSION));
    }
    if current.credential_set_generation != expected_credential_set_generation {
        return Err(stale(cas_namespaces::CREDENTIAL_SET_GENERATION));
    }
    Ok(())
}

/// Applies principal CAS precedence: principal version, then credential set.
pub async fn cas_update_principal(
    transaction: &mut Transaction<'_, Postgres>,
    principal_id: &str,
    expected_principal_version: i64,
    expected_credential_set_generation: i64,
    update: &PrincipalUpdate,
) -> ControlPlaneResult<PrincipalRow> {
    let current = credential_records::lock_principal(transaction, principal_id).await?;
    verify_principal_cas(
        &current,
        expected_principal_version,
        expected_credential_set_generation,
    )?;
    credential_records::cas_update_principal(
        transaction,
        principal_id,
        expected_principal_version,
        expected_credential_set_generation,
        update,
    )
    .await
}

/// Creates a credential only after exact principal and set CAS validation.
pub async fn insert_credential(
    transaction: &mut Transaction<'_, Postgres>,
    input: &NewCredential,
    expected_principal_version: i64,
    expected_credential_set_generation: i64,
) -> ControlPlaneResult<CredentialMutationResult> {
    let current = credential_records::lock_principal(transaction, &input.principal_id).await?;
    verify_principal_cas(
        &current,
        expected_principal_version,
        expected_credential_set_generation,
    )?;
    credential_records::insert_credential(
        transaction,
        input,
        expected_principal_version,
        expected_credential_set_generation,
    )
    .await
}

/// Applies lifecycle CAS precedence: principal, set, then credential version.
pub async fn cas_update_credential_lifecycle(
    transaction: &mut Transaction<'_, Postgres>,
    credential_id: &str,
    expected_credential_version: i64,
    expected_principal_version: i64,
    expected_credential_set_generation: i64,
    update: &CredentialLifecycleUpdate,
) -> ControlPlaneResult<CredentialMutationResult> {
    let observed = credential_records::read_credential_by_id(transaction, credential_id)
        .await?
        .ok_or(ControlPlaneRepositoryError::MissingRecord)?;
    let principal =
        credential_records::lock_principal(transaction, &observed.principal_id).await?;
    let current = credential_records::lock_credential(transaction, credential_id).await?;
    if current.principal_id != observed.principal_id {
        return Err(ControlPlaneRepositoryError::DatabaseOperationFailed);
    }
    verify_principal_cas(
        &principal,
        expected_principal_version,
        expected_credential_set_generation,
    )?;
    if current.credential_version != expected_credential_version {
        return Err(stale(cas_namespaces::CREDENTIAL_VERSION));
    }

    credential_records::cas_update_credential_lifecycle(
        transaction,
        credential_id,
        expected_credential_version,
        expected_principal_version,
        expected_credential_set_generation,
        update,
    )
    .await
}

mod job_records {
    use super::*;

    include!("repository/jobs.rs");
}

pub use job_records::{
    append_operational_audit_event, cas_acquire_execution_slot, ensure_scoped_execution_slot,
    lock_execution_slot,
};

fn terminal_job_state(state: &str) -> bool {
    matches!(state, "succeeded" | "failed" | "cancelled")
}

/// Applies job CAS precedence: lifecycle, job version, fence, then lease proof.
pub async fn cas_update_operational_job(
    transaction: &mut Transaction<'_, Postgres>,
    operation_id: &str,
    expected_job_version: i64,
    expected_executor_fence: Option<i64>,
    expected_lease_proof: Option<&SecretDigest>,
    update: &OperationalJobUpdate,
) -> ControlPlaneResult<OperationalJobRow> {
    let current = job_records::lock_operational_job(transaction, operation_id).await?;
    if terminal_job_state(&current.state) {
        return Err(ControlPlaneRepositoryError::ImmutableRecord);
    }
    if current.job_version != expected_job_version {
        return Err(stale(cas_namespaces::JOB_VERSION));
    }
    if expected_executor_fence.is_some_and(|fence| current.executor_fence != fence)
        || update.executor_fence < current.executor_fence
    {
        return Err(stale(cas_namespaces::EXECUTOR_FENCE));
    }
    if expected_lease_proof
        .is_some_and(|proof| current.lease_token_digest.as_ref() != Some(proof))
    {
        return Err(stale(cas_namespaces::EXECUTOR_LEASE));
    }

    job_records::cas_update_operational_job(
        transaction,
        operation_id,
        expected_job_version,
        expected_executor_fence,
        expected_lease_proof,
        update,
    )
    .await
}

fn verify_slot_owner(
    current: &ExecutionSlotRow,
    expected_slot_version: i64,
    operation_id: &str,
    executor_fence: i64,
) -> ControlPlaneResult<()> {
    if current.slot_version != expected_slot_version {
        return Err(stale(cas_namespaces::SLOT_VERSION));
    }
    if current.operation_id.as_deref() != Some(operation_id) {
        return Err(ControlPlaneRepositoryError::OperationInProgress);
    }
    if current.executor_fence != Some(executor_fence) {
        return Err(stale(cas_namespaces::EXECUTOR_FENCE));
    }
    Ok(())
}

pub async fn cas_release_execution_slot(
    transaction: &mut Transaction<'_, Postgres>,
    slot_id: &str,
    expected_slot_version: i64,
    operation_id: &str,
    executor_fence: i64,
) -> ControlPlaneResult<ExecutionSlotRow> {
    let current = job_records::lock_execution_slot(transaction, slot_id).await?;
    verify_slot_owner(
        &current,
        expected_slot_version,
        operation_id,
        executor_fence,
    )?;
    if current.blocked_uncertain {
        return Err(ControlPlaneRepositoryError::OperationInProgress);
    }
    job_records::cas_release_execution_slot(
        transaction,
        slot_id,
        expected_slot_version,
        operation_id,
        executor_fence,
    )
    .await
}

pub async fn cas_block_execution_slot_uncertain(
    transaction: &mut Transaction<'_, Postgres>,
    slot_id: &str,
    expected_slot_version: i64,
    operation_id: &str,
    executor_fence: i64,
) -> ControlPlaneResult<ExecutionSlotRow> {
    let current = job_records::lock_execution_slot(transaction, slot_id).await?;
    verify_slot_owner(
        &current,
        expected_slot_version,
        operation_id,
        executor_fence,
    )?;
    job_records::cas_block_execution_slot_uncertain(
        transaction,
        slot_id,
        expected_slot_version,
        operation_id,
        executor_fence,
    )
    .await
}

pub async fn cas_reconcile_execution_slot_uncertainty(
    transaction: &mut Transaction<'_, Postgres>,
    slot_id: &str,
    expected_slot_version: i64,
    operation_id: &str,
    executor_fence: i64,
) -> ControlPlaneResult<ExecutionSlotRow> {
    let current = job_records::lock_execution_slot(transaction, slot_id).await?;
    verify_slot_owner(
        &current,
        expected_slot_version,
        operation_id,
        executor_fence,
    )?;
    if !current.blocked_uncertain {
        return Err(ControlPlaneRepositoryError::OperationInProgress);
    }
    job_records::cas_reconcile_execution_slot_uncertainty(
        transaction,
        slot_id,
        expected_slot_version,
        operation_id,
        executor_fence,
    )
    .await
}

/// Binds immutable evidence to the current exact job version.
pub async fn insert_operational_evidence(
    transaction: &mut Transaction<'_, Postgres>,
    input: &OperationalEvidenceInput,
) -> ControlPlaneResult<()> {
    let current = job_records::lock_operational_job(transaction, &input.operation_id).await?;
    if current.job_version != input.job_version {
        return Err(stale(cas_namespaces::JOB_VERSION));
    }
    job_records::insert_operational_evidence(transaction, input).await
}

include!("repository/runtime_lease.rs");
include!("repository/runtime_report/open.rs");

mod ordered_runtime_reports {
    use super::*;

    include!("repository/runtime_report/accept.rs");
}

/// The sole public standalone GDrive effective-authority mutation transaction.
pub async fn accept_ordered_runtime_report(
    transaction: &mut Transaction<'_, Postgres>,
    input: &RuntimeReportInput,
) -> ControlPlaneResult<RuntimeReportOutcome> {
    validate_identifier(&input.adapter_id)?;
    validate_identifier(&input.runtime_instance_id)?;

    // Preserve the canonical lock order and duplicate semantics before invoking
    // the complete raw transaction, which repeats the locks defensively.
    let maintenance = lock_maintenance_control(transaction).await?;
    let desired = lock_adapter_desired_control(transaction, &input.adapter_id).await?;
    let authority = lock_runtime_authority(transaction, &input.adapter_id).await?;

    if authority.standalone_runtime_epoch != input.standalone_runtime_epoch
        || authority.runtime_instance_id.as_deref() != Some(input.runtime_instance_id.as_str())
    {
        return Err(ControlPlaneRepositoryError::StaleRuntimeEpoch);
    }
    if input.report_sequence != authority.last_accepted_report_sequence
        && input.report_sequence == authority.last_accepted_report_sequence + 1
    {
        if input.expected_runtime_lease_version != authority.runtime_lease_version {
            return Err(stale(cas_namespaces::RUNTIME_LEASE_VERSION));
        }
        if desired.adapter_control_generation != input.adapter_control_generation {
            return Err(stale(cas_namespaces::ADAPTER_CONTROL_GENERATION));
        }
        if maintenance.maintenance_generation != input.maintenance_generation {
            return Err(stale(cas_namespaces::MAINTENANCE_GENERATION));
        }
    }

    ordered_runtime_reports::accept_ordered_runtime_report(transaction, input).await
}

include!("repository/runtime_report/uncertain.rs");
include!("repository/runtime_report/reconcile.rs");
include!("repository/facts.rs");
include!("repository/qa_private.rs");
include!("repository/qa_corrections.rs");
