const MAINTENANCE_COLUMNS: &str = "schema_version, maintenance_generation, state, \
    transition_operation_id, transition_requested_by, transition_requested_at, state_entered_at, \
    admission_fence_closed, admission_fence_closed_at, quiescence_evidence_version, \
    quiescence_evidence_id, active_maintenance_job_id, safe_error_category, updated_at";
const DESIRED_COLUMNS: &str = "adapter_id, desired_enabled, desired_mode, \
    adapter_control_generation, maintenance_generation, maintenance_hold, updated_by, updated_at";
const EFFECTIVE_COLUMNS: &str = "adapter_id, effective_mode, runtime_lifecycle, \
    last_applied_adapter_control_generation, last_applied_maintenance_generation, heartbeat_at, \
    last_success_at, last_cycle_started_at, last_cycle_finished_at, in_flight, connection_state, \
    safe_error_category, safe_error_code, runtime_instance_id, standalone_runtime_epoch, \
    report_sequence, runtime_lease_version, runtime_lease_expires_at, checkpoint_summary, \
    reported_at, updated_at";
const PRINCIPAL_COLUMNS: &str = "principal_id, principal_kind, role, principal_enabled, \
    principal_version, credential_set_generation, created_at, updated_at, disabled_at";
const CREDENTIAL_COLUMNS: &str = "credential_id, credential_version, principal_id, \
    issuance_operation_id, credential_kind, status, verifier_scheme, verifier_material, \
    legacy_migrated, created_at, not_before, expires_at, grace_expires_at, \
    rotated_from_credential_id, revoked_at, revoked_by, last_used_at, updated_at";
const JOB_COLUMNS: &str = "operation_id, job_version, kind, maintenance_required, destructive, \
    requester_principal_id, requester_credential_id, state, dry_run, confirmation_required, \
    confirmation_digest, confirmation_expires_at, confirmation_consumed_at, \
    expected_maintenance_generation, created_at, updated_at, started_at, completed_at, \
    cancel_requested_at, safe_summary, safe_error_category, artifact_manifest_id, \
    artifact_manifest_digest, checkpoint, execution_scope_digest, executor_id, executor_fence, \
    lease_token_digest, lease_heartbeat_at, lease_expires_at, destructive_execution_slot_id";
const SLOT_COLUMNS: &str = "slot_id, slot_version, slot_kind, execution_scope_digest, \
    operation_id, maintenance_generation, executor_fence, blocked_uncertain, acquired_at, \
    released_at, updated_at";
const RUNTIME_AUTHORITY_COLUMNS: &str = "adapter_id, runtime_lease_version, \
    standalone_runtime_epoch, runtime_instance_id, lease_token_digest, lease_heartbeat_at, \
    lease_expires_at, last_accepted_report_sequence, last_accepted_report_fingerprint, \
    open_mutation_permits, uncertain_external_effects, takeover_state, updated_at";

macro_rules! row_column {
    ($row:expr, $name:literal) => {
        $row.try_get($name)
            .map_err(|_| ControlPlaneRepositoryError::DatabaseOperationFailed)?
    };
}

fn map_database_error(error: sqlx::Error) -> ControlPlaneRepositoryError {
    let Some(database_error) = error.as_database_error() else {
        return ControlPlaneRepositoryError::DatabaseOperationFailed;
    };
    if database_error.code().as_deref() == Some("55000") {
        return ControlPlaneRepositoryError::ImmutableRecord;
    }
    match database_error.constraint() {
        Some(
            "credentials_one_active_per_principal_uq"
            | "credentials_one_grace_per_principal_uq"
            | "operational_execution_slots_one_global_uq"
            | "operational_execution_slots_scope_active_uq",
        ) => ControlPlaneRepositoryError::OperationInProgress,
        Some(
            "credential_issuance_idempotency_pkey"
            | "operational_idempotency_pkey"
            | "gdrive_runtime_reports_pkey"
            | "gdrive_mutation_permits_adapter_id_standalone_runtime_epoch_step_id_key",
        ) => ControlPlaneRepositoryError::IdempotencyConflict,
        _ => ControlPlaneRepositoryError::DatabaseOperationFailed,
    }
}

fn validate_identifier(value: &str) -> ControlPlaneResult<()> {
    if value.is_empty() || value.len() > 1024 || value.chars().any(char::is_control) {
        return Err(ControlPlaneRepositoryError::InvalidIdentifier);
    }
    Ok(())
}

fn validate_safe_json(value: &Value) -> ControlPlaneResult<()> {
    const PROHIBITED_KEYS: &[&str] = &[
        "password",
        "plaintext",
        "secret",
        "token",
        "verifier",
        "salt",
        "pepper",
        "idempotency_key",
        "confirmation_value",
        "oauth",
        "database_url",
        "provider_payload",
        "raw_cursor",
        "absolute_path",
    ];
    const PROHIBITED_VALUES: &[&str] = &[
        "postgres://",
        "postgresql://",
        "bearer ",
        "hs1.",
        "/srv/",
        "stack backtrace",
    ];
    match value {
        Value::Object(map) => {
            for (key, nested) in map {
                let normalized = key.to_ascii_lowercase();
                if normalized == "plaintext_available" {
                    if nested != &Value::Bool(false) {
                        return Err(ControlPlaneRepositoryError::InvalidSecretMaterial);
                    }
                    continue;
                }
                if PROHIBITED_KEYS
                    .iter()
                    .any(|fragment| normalized.contains(fragment))
                {
                    return Err(ControlPlaneRepositoryError::InvalidSecretMaterial);
                }
                validate_safe_json(nested)?;
            }
        }
        Value::Array(values) => {
            for nested in values {
                validate_safe_json(nested)?;
            }
        }
        Value::String(text) => {
            let normalized = text.to_ascii_lowercase();
            if PROHIBITED_VALUES
                .iter()
                .any(|fragment| normalized.contains(fragment))
            {
                return Err(ControlPlaneRepositoryError::InvalidSecretMaterial);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
    Ok(())
}

fn validate_non_negative(value: i64) -> ControlPlaneResult<()> {
    if value < 0 {
        return Err(ControlPlaneRepositoryError::InvalidGeneration);
    }
    Ok(())
}

fn advance(value: i64) -> ControlPlaneResult<i64> {
    validate_non_negative(value)?;
    value
        .checked_add(1)
        .ok_or(ControlPlaneRepositoryError::VersionOverflow)
}

fn stale(namespace: &'static str) -> ControlPlaneRepositoryError {
    ControlPlaneRepositoryError::StaleVersion { namespace }
}

async fn lock_idempotency_scope(
    transaction: &mut Transaction<'_, Postgres>,
    namespace: &str,
    parts: &[&str],
) -> ControlPlaneResult<()> {
    let mut identity = String::with_capacity(256);
    identity.push_str(namespace);
    for part in parts {
        identity.push('\u{1f}');
        identity.push_str(part);
    }
    sqlx::query("select pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(identity)
        .execute(&mut **transaction)
        .await
        .map_err(map_database_error)?;
    Ok(())
}

fn maintenance_from_row(row: &PgRow) -> ControlPlaneResult<MaintenanceControlRow> {
    Ok(MaintenanceControlRow {
        schema_version: row_column!(row, "schema_version"),
        maintenance_generation: row_column!(row, "maintenance_generation"),
        state: row_column!(row, "state"),
        transition_operation_id: row_column!(row, "transition_operation_id"),
        transition_requested_by: row_column!(row, "transition_requested_by"),
        transition_requested_at: row_column!(row, "transition_requested_at"),
        state_entered_at: row_column!(row, "state_entered_at"),
        admission_fence_closed: row_column!(row, "admission_fence_closed"),
        admission_fence_closed_at: row_column!(row, "admission_fence_closed_at"),
        quiescence_evidence_version: row_column!(row, "quiescence_evidence_version"),
        quiescence_evidence_id: row_column!(row, "quiescence_evidence_id"),
        active_maintenance_job_id: row_column!(row, "active_maintenance_job_id"),
        safe_error_category: row_column!(row, "safe_error_category"),
        updated_at: row_column!(row, "updated_at"),
    })
}

fn desired_from_row(row: &PgRow) -> ControlPlaneResult<AdapterDesiredControlRow> {
    Ok(AdapterDesiredControlRow {
        adapter_id: row_column!(row, "adapter_id"),
        desired_enabled: row_column!(row, "desired_enabled"),
        desired_mode: row_column!(row, "desired_mode"),
        adapter_control_generation: row_column!(row, "adapter_control_generation"),
        maintenance_generation: row_column!(row, "maintenance_generation"),
        maintenance_hold: row_column!(row, "maintenance_hold"),
        updated_by: row_column!(row, "updated_by"),
        updated_at: row_column!(row, "updated_at"),
    })
}

fn effective_from_row(row: &PgRow) -> ControlPlaneResult<AdapterEffectiveControlRow> {
    Ok(AdapterEffectiveControlRow {
        adapter_id: row_column!(row, "adapter_id"),
        effective_mode: row_column!(row, "effective_mode"),
        runtime_lifecycle: row_column!(row, "runtime_lifecycle"),
        last_applied_adapter_control_generation: row_column!(
            row,
            "last_applied_adapter_control_generation"
        ),
        last_applied_maintenance_generation: row_column!(row, "last_applied_maintenance_generation"),
        heartbeat_at: row_column!(row, "heartbeat_at"),
        last_success_at: row_column!(row, "last_success_at"),
        last_cycle_started_at: row_column!(row, "last_cycle_started_at"),
        last_cycle_finished_at: row_column!(row, "last_cycle_finished_at"),
        in_flight: row_column!(row, "in_flight"),
        connection_state: row_column!(row, "connection_state"),
        safe_error_category: row_column!(row, "safe_error_category"),
        safe_error_code: row_column!(row, "safe_error_code"),
        runtime_instance_id: row_column!(row, "runtime_instance_id"),
        standalone_runtime_epoch: row_column!(row, "standalone_runtime_epoch"),
        report_sequence: row_column!(row, "report_sequence"),
        runtime_lease_version: row_column!(row, "runtime_lease_version"),
        runtime_lease_expires_at: row_column!(row, "runtime_lease_expires_at"),
        checkpoint_summary: row_column!(row, "checkpoint_summary"),
        reported_at: row_column!(row, "reported_at"),
        updated_at: row_column!(row, "updated_at"),
    })
}

fn principal_from_row(row: &PgRow) -> ControlPlaneResult<PrincipalRow> {
    Ok(PrincipalRow {
        principal_id: row_column!(row, "principal_id"),
        principal_kind: row_column!(row, "principal_kind"),
        role: row_column!(row, "role"),
        principal_enabled: row_column!(row, "principal_enabled"),
        principal_version: row_column!(row, "principal_version"),
        credential_set_generation: row_column!(row, "credential_set_generation"),
        created_at: row_column!(row, "created_at"),
        updated_at: row_column!(row, "updated_at"),
        disabled_at: row_column!(row, "disabled_at"),
    })
}

fn credential_from_row(row: &PgRow) -> ControlPlaneResult<CredentialRow> {
    Ok(CredentialRow {
        credential_id: row_column!(row, "credential_id"),
        credential_version: row_column!(row, "credential_version"),
        principal_id: row_column!(row, "principal_id"),
        issuance_operation_id: row_column!(row, "issuance_operation_id"),
        credential_kind: row_column!(row, "credential_kind"),
        status: row_column!(row, "status"),
        verifier_scheme: row_column!(row, "verifier_scheme"),
        verifier_material: VerifierMaterial::parse(row_column!(row, "verifier_material"))?,
        legacy_migrated: row_column!(row, "legacy_migrated"),
        created_at: row_column!(row, "created_at"),
        not_before: row_column!(row, "not_before"),
        expires_at: row_column!(row, "expires_at"),
        grace_expires_at: row_column!(row, "grace_expires_at"),
        rotated_from_credential_id: row_column!(row, "rotated_from_credential_id"),
        revoked_at: row_column!(row, "revoked_at"),
        revoked_by: row_column!(row, "revoked_by"),
        last_used_at: row_column!(row, "last_used_at"),
        updated_at: row_column!(row, "updated_at"),
    })
}

fn optional_digest(row: &PgRow, name: &str) -> ControlPlaneResult<Option<SecretDigest>> {
    let value = row
        .try_get::<Option<String>, _>(name)
        .map_err(|_| ControlPlaneRepositoryError::DatabaseOperationFailed)?;
    value.map(SecretDigest::parse).transpose()
}

fn required_digest(row: &PgRow, name: &str) -> ControlPlaneResult<SecretDigest> {
    SecretDigest::parse(
        row.try_get::<String, _>(name)
            .map_err(|_| ControlPlaneRepositoryError::DatabaseOperationFailed)?,
    )
}

fn job_from_row(row: &PgRow) -> ControlPlaneResult<OperationalJobRow> {
    Ok(OperationalJobRow {
        operation_id: row_column!(row, "operation_id"),
        job_version: row_column!(row, "job_version"),
        kind: row_column!(row, "kind"),
        maintenance_required: row_column!(row, "maintenance_required"),
        destructive: row_column!(row, "destructive"),
        requester_principal_id: row_column!(row, "requester_principal_id"),
        requester_credential_id: row_column!(row, "requester_credential_id"),
        state: row_column!(row, "state"),
        dry_run: row_column!(row, "dry_run"),
        confirmation_required: row_column!(row, "confirmation_required"),
        confirmation_digest: optional_digest(row, "confirmation_digest")?,
        confirmation_expires_at: row_column!(row, "confirmation_expires_at"),
        confirmation_consumed_at: row_column!(row, "confirmation_consumed_at"),
        expected_maintenance_generation: row_column!(row, "expected_maintenance_generation"),
        created_at: row_column!(row, "created_at"),
        updated_at: row_column!(row, "updated_at"),
        started_at: row_column!(row, "started_at"),
        completed_at: row_column!(row, "completed_at"),
        cancel_requested_at: row_column!(row, "cancel_requested_at"),
        safe_summary: row_column!(row, "safe_summary"),
        safe_error_category: row_column!(row, "safe_error_category"),
        artifact_manifest_id: row_column!(row, "artifact_manifest_id"),
        artifact_manifest_digest: optional_digest(row, "artifact_manifest_digest")?,
        checkpoint: row_column!(row, "checkpoint"),
        execution_scope_digest: required_digest(row, "execution_scope_digest")?,
        executor_id: row_column!(row, "executor_id"),
        executor_fence: row_column!(row, "executor_fence"),
        lease_token_digest: optional_digest(row, "lease_token_digest")?,
        lease_heartbeat_at: row_column!(row, "lease_heartbeat_at"),
        lease_expires_at: row_column!(row, "lease_expires_at"),
        destructive_execution_slot_id: row_column!(row, "destructive_execution_slot_id"),
    })
}

fn slot_from_row(row: &PgRow) -> ControlPlaneResult<ExecutionSlotRow> {
    Ok(ExecutionSlotRow {
        slot_id: row_column!(row, "slot_id"),
        slot_version: row_column!(row, "slot_version"),
        slot_kind: row_column!(row, "slot_kind"),
        execution_scope_digest: optional_digest(row, "execution_scope_digest")?,
        operation_id: row_column!(row, "operation_id"),
        maintenance_generation: row_column!(row, "maintenance_generation"),
        executor_fence: row_column!(row, "executor_fence"),
        blocked_uncertain: row_column!(row, "blocked_uncertain"),
        acquired_at: row_column!(row, "acquired_at"),
        released_at: row_column!(row, "released_at"),
        updated_at: row_column!(row, "updated_at"),
    })
}

fn runtime_authority_from_row(row: &PgRow) -> ControlPlaneResult<RuntimeAuthorityRow> {
    Ok(RuntimeAuthorityRow {
        adapter_id: row_column!(row, "adapter_id"),
        runtime_lease_version: row_column!(row, "runtime_lease_version"),
        standalone_runtime_epoch: row_column!(row, "standalone_runtime_epoch"),
        runtime_instance_id: row_column!(row, "runtime_instance_id"),
        lease_token_digest: optional_digest(row, "lease_token_digest")?,
        lease_heartbeat_at: row_column!(row, "lease_heartbeat_at"),
        lease_expires_at: row_column!(row, "lease_expires_at"),
        last_accepted_report_sequence: row_column!(row, "last_accepted_report_sequence"),
        last_accepted_report_fingerprint: optional_digest(
            row,
            "last_accepted_report_fingerprint",
        )?,
        open_mutation_permits: row_column!(row, "open_mutation_permits"),
        uncertain_external_effects: row_column!(row, "uncertain_external_effects"),
        takeover_state: row_column!(row, "takeover_state"),
        updated_at: row_column!(row, "updated_at"),
    })
}
