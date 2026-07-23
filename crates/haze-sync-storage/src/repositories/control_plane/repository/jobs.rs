pub async fn insert_or_replay_operational_job(
    transaction: &mut Transaction<'_, Postgres>,
    input: &OperationalJobInput,
) -> ControlPlaneResult<IdempotencyInsertOutcome<OperationalJobRow>> {
    validate_safe_json(&input.safe_summary)?;
    if let Some(checkpoint) = &input.checkpoint {
        validate_safe_json(checkpoint)?;
    }
    for identifier in [
        input.operation_id.as_str(),
        input.kind.as_str(),
        input.requester_principal_id.as_str(),
        input.idempotency_scope.as_str(),
        input.state.as_str(),
    ] {
        validate_identifier(identifier)?;
    }
    lock_idempotency_scope(
        transaction,
        "operational_job",
        &[
            &input.requester_principal_id,
            &input.kind,
            input.idempotency_key_digest.as_str(),
        ],
    )
    .await?;
    let existing = sqlx::query(
        "select request_fingerprint, operation_id from operational_idempotency \
         where requester_principal_id = $1 and operation_kind = $2 and idempotency_key_digest = $3",
    )
    .bind(&input.requester_principal_id)
    .bind(&input.kind)
    .bind(input.idempotency_key_digest.as_str())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_database_error)?;
    if let Some(existing) = existing {
        let fingerprint = required_digest(&existing, "request_fingerprint")?;
        if fingerprint != input.request_fingerprint {
            return Err(ControlPlaneRepositoryError::IdempotencyConflict);
        }
        let operation_id: String = column!(&existing, "operation_id");
        return Ok(IdempotencyInsertOutcome::Replay(
            lock_operational_job(transaction, &operation_id).await?,
        ));
    }

    let query = format!(
        "insert into operational_jobs (operation_id, job_version, kind, maintenance_required, \
         destructive, requester_principal_id, requester_credential_id, idempotency_scope, \
         idempotency_key_digest, request_fingerprint, state, dry_run, confirmation_required, \
         confirmation_digest, confirmation_expires_at, expected_maintenance_generation, \
         safe_summary, artifact_manifest_id, artifact_manifest_digest, checkpoint, \
         execution_scope_digest, retry_of_operation_id) \
         values ($1,1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21) \
         returning {JOB_COLUMNS}"
    );
    let row = sqlx::query(&query)
        .bind(&input.operation_id)
        .bind(&input.kind)
        .bind(input.maintenance_required)
        .bind(input.destructive)
        .bind(&input.requester_principal_id)
        .bind(&input.requester_credential_id)
        .bind(&input.idempotency_scope)
        .bind(input.idempotency_key_digest.as_str())
        .bind(input.request_fingerprint.as_str())
        .bind(&input.state)
        .bind(input.dry_run)
        .bind(input.confirmation_required)
        .bind(input.confirmation_digest.as_ref().map(SecretDigest::as_str))
        .bind(input.confirmation_expires_at)
        .bind(input.expected_maintenance_generation)
        .bind(&input.safe_summary)
        .bind(&input.artifact_manifest_id)
        .bind(input.artifact_manifest_digest.as_ref().map(SecretDigest::as_str))
        .bind(&input.checkpoint)
        .bind(input.execution_scope_digest.as_str())
        .bind(&input.retry_of_operation_id)
        .fetch_one(&mut **transaction)
        .await
        .map_err(map_database_error)?;

    for (adapter_id, generation) in &input.expected_adapter_generations {
        validate_identifier(adapter_id)?;
        validate_non_negative(*generation)?;
        sqlx::query(
            "insert into operational_job_adapter_generations \
             (operation_id, adapter_id, adapter_control_generation) values ($1,$2,$3)",
        )
        .bind(&input.operation_id)
        .bind(adapter_id)
        .bind(generation)
        .execute(&mut **transaction)
        .await
        .map_err(map_database_error)?;
    }
    sqlx::query(
        "insert into operational_idempotency (requester_principal_id, operation_kind, \
         idempotency_key_digest, request_fingerprint, operation_id) values ($1,$2,$3,$4,$5)",
    )
    .bind(&input.requester_principal_id)
    .bind(&input.kind)
    .bind(input.idempotency_key_digest.as_str())
    .bind(input.request_fingerprint.as_str())
    .bind(&input.operation_id)
    .execute(&mut **transaction)
    .await
    .map_err(map_database_error)?;
    Ok(IdempotencyInsertOutcome::Inserted(job_from_row(&row)?))
}

pub async fn lock_operational_job(
    transaction: &mut Transaction<'_, Postgres>,
    operation_id: &str,
) -> ControlPlaneResult<OperationalJobRow> {
    validate_identifier(operation_id)?;
    let query = format!(
        "select {JOB_COLUMNS} from operational_jobs where operation_id = $1 for update"
    );
    let row = sqlx::query(&query)
        .bind(operation_id)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_database_error)?
        .ok_or(ControlPlaneRepositoryError::MissingRecord)?;
    job_from_row(&row)
}

pub async fn cas_update_operational_job(
    transaction: &mut Transaction<'_, Postgres>,
    operation_id: &str,
    expected_job_version: i64,
    expected_executor_fence: Option<i64>,
    expected_lease_proof: Option<&SecretDigest>,
    update: &OperationalJobUpdate,
) -> ControlPlaneResult<OperationalJobRow> {
    validate_identifier(operation_id)?;
    validate_safe_json(&update.safe_summary)?;
    if let Some(checkpoint) = &update.checkpoint {
        validate_safe_json(checkpoint)?;
    }
    let next_version = advance(expected_job_version)?;
    if let Some(fence) = expected_executor_fence {
        validate_non_negative(fence)?;
    }
    let query = format!(
        "update operational_jobs set job_version = $4, state = $5, safe_summary = $6, \
         safe_error_category = $7, artifact_manifest_id = $8, artifact_manifest_digest = $9, \
         checkpoint = $10, started_at = $11, completed_at = $12, cancel_requested_at = $13, \
         confirmation_consumed_at = $14, executor_id = $15, executor_fence = $16, \
         lease_token_digest = $17, lease_heartbeat_at = $18, lease_expires_at = $19, \
         destructive_execution_slot_id = $20, updated_at = now() \
         where operation_id = $1 and job_version = $2 and $16 >= executor_fence \
         and ($3::bigint is null or executor_fence = $3) \
         and ($21::text is null or lease_token_digest = $21) returning {JOB_COLUMNS}"
    );
    let row = sqlx::query(&query)
        .bind(operation_id)
        .bind(expected_job_version)
        .bind(expected_executor_fence)
        .bind(next_version)
        .bind(&update.state)
        .bind(&update.safe_summary)
        .bind(&update.safe_error_category)
        .bind(&update.artifact_manifest_id)
        .bind(update.artifact_manifest_digest.as_ref().map(SecretDigest::as_str))
        .bind(&update.checkpoint)
        .bind(update.started_at)
        .bind(update.completed_at)
        .bind(update.cancel_requested_at)
        .bind(update.confirmation_consumed_at)
        .bind(&update.executor_id)
        .bind(update.executor_fence)
        .bind(update.lease_token_digest.as_ref().map(SecretDigest::as_str))
        .bind(update.lease_heartbeat_at)
        .bind(update.lease_expires_at)
        .bind(&update.destructive_execution_slot_id)
        .bind(expected_lease_proof.map(SecretDigest::as_str))
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_database_error)?
        .ok_or_else(|| {
            if expected_executor_fence.is_some() {
                stale(cas_namespaces::EXECUTOR_FENCE)
            } else {
                stale(cas_namespaces::JOB_VERSION)
            }
        })?;
    job_from_row(&row)
}

pub async fn lock_execution_slot(
    transaction: &mut Transaction<'_, Postgres>,
    slot_id: &str,
) -> ControlPlaneResult<ExecutionSlotRow> {
    validate_identifier(slot_id)?;
    let query = format!(
        "select {SLOT_COLUMNS} from operational_execution_slots where slot_id = $1 for update"
    );
    let row = sqlx::query(&query)
        .bind(slot_id)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_database_error)?
        .ok_or(ControlPlaneRepositoryError::MissingRecord)?;
    slot_from_row(&row)
}

pub async fn ensure_scoped_execution_slot(
    transaction: &mut Transaction<'_, Postgres>,
    slot_id: &str,
    execution_scope_digest: &SecretDigest,
) -> ControlPlaneResult<ExecutionSlotRow> {
    validate_identifier(slot_id)?;
    let query = format!(
        "insert into operational_execution_slots (slot_id, slot_kind, execution_scope_digest) \
         values ($1,'scoped',$2) on conflict (slot_id) do nothing returning {SLOT_COLUMNS}"
    );
    if let Some(row) = sqlx::query(&query)
        .bind(slot_id)
        .bind(execution_scope_digest.as_str())
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_database_error)?
    {
        return slot_from_row(&row);
    }
    let row = lock_execution_slot(transaction, slot_id).await?;
    if row.execution_scope_digest.as_ref() != Some(execution_scope_digest) {
        return Err(ControlPlaneRepositoryError::IdempotencyConflict);
    }
    Ok(row)
}

pub async fn cas_acquire_execution_slot(
    transaction: &mut Transaction<'_, Postgres>,
    slot_id: &str,
    expected_slot_version: i64,
    operation_id: &str,
    maintenance_generation: Option<i64>,
    executor_fence: i64,
) -> ControlPlaneResult<ExecutionSlotRow> {
    validate_identifier(slot_id)?;
    validate_identifier(operation_id)?;
    validate_non_negative(expected_slot_version)?;
    if let Some(generation) = maintenance_generation {
        validate_non_negative(generation)?;
    }
    validate_non_negative(executor_fence)?;
    let current = lock_execution_slot(transaction, slot_id).await?;
    if current.slot_version != expected_slot_version {
        return Err(stale(cas_namespaces::SLOT_VERSION));
    }
    if current.operation_id.is_some() || current.blocked_uncertain {
        return Err(ControlPlaneRepositoryError::OperationInProgress);
    }
    let next = advance(expected_slot_version)?;
    let query = format!(
        "update operational_execution_slots set slot_version = $3, operation_id = $4, \
         maintenance_generation = $5, executor_fence = $6, blocked_uncertain = false, \
         acquired_at = now(), released_at = null, updated_at = now() \
         where slot_id = $1 and slot_version = $2 and operation_id is null and not blocked_uncertain \
         returning {SLOT_COLUMNS}"
    );
    let row = sqlx::query(&query)
        .bind(slot_id)
        .bind(expected_slot_version)
        .bind(next)
        .bind(operation_id)
        .bind(maintenance_generation)
        .bind(executor_fence)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_database_error)?
        .ok_or(ControlPlaneRepositoryError::OperationInProgress)?;
    slot_from_row(&row)
}

pub async fn cas_release_execution_slot(
    transaction: &mut Transaction<'_, Postgres>,
    slot_id: &str,
    expected_slot_version: i64,
    operation_id: &str,
    executor_fence: i64,
) -> ControlPlaneResult<ExecutionSlotRow> {
    validate_identifier(slot_id)?;
    validate_identifier(operation_id)?;
    validate_non_negative(expected_slot_version)?;
    validate_non_negative(executor_fence)?;
    let current = lock_execution_slot(transaction, slot_id).await?;
    if current.slot_version != expected_slot_version
        || current.operation_id.as_deref() != Some(operation_id)
        || current.executor_fence != Some(executor_fence)
    {
        return Err(stale(cas_namespaces::SLOT_VERSION));
    }
    if current.blocked_uncertain {
        return Err(ControlPlaneRepositoryError::OperationInProgress);
    }
    let next = advance(expected_slot_version)?;
    let query = format!(
        "update operational_execution_slots set slot_version = $5, operation_id = null, \
         maintenance_generation = null, executor_fence = null, blocked_uncertain = false, \
         acquired_at = null, released_at = now(), updated_at = now() where slot_id = $1 \
         and slot_version = $2 and operation_id = $3 and executor_fence = $4 \
         and not blocked_uncertain returning {SLOT_COLUMNS}"
    );
    let row = sqlx::query(&query)
        .bind(slot_id)
        .bind(expected_slot_version)
        .bind(operation_id)
        .bind(executor_fence)
        .bind(next)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_database_error)?
        .ok_or(ControlPlaneRepositoryError::OperationInProgress)?;
    slot_from_row(&row)
}

pub async fn cas_block_execution_slot_uncertain(
    transaction: &mut Transaction<'_, Postgres>,
    slot_id: &str,
    expected_slot_version: i64,
    operation_id: &str,
    executor_fence: i64,
) -> ControlPlaneResult<ExecutionSlotRow> {
    validate_identifier(slot_id)?;
    validate_identifier(operation_id)?;
    validate_non_negative(expected_slot_version)?;
    validate_non_negative(executor_fence)?;
    let next = advance(expected_slot_version)?;
    let query = format!(
        "update operational_execution_slots set slot_version = $5, blocked_uncertain = true, \
         updated_at = now() where slot_id = $1 and slot_version = $2 and operation_id = $3 \
         and executor_fence = $4 returning {SLOT_COLUMNS}"
    );
    let row = sqlx::query(&query)
        .bind(slot_id)
        .bind(expected_slot_version)
        .bind(operation_id)
        .bind(executor_fence)
        .bind(next)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_database_error)?
        .ok_or_else(|| stale(cas_namespaces::SLOT_VERSION))?;
    slot_from_row(&row)
}

pub async fn cas_reconcile_execution_slot_uncertainty(
    transaction: &mut Transaction<'_, Postgres>,
    slot_id: &str,
    expected_slot_version: i64,
    operation_id: &str,
    executor_fence: i64,
) -> ControlPlaneResult<ExecutionSlotRow> {
    validate_identifier(slot_id)?;
    validate_identifier(operation_id)?;
    validate_non_negative(expected_slot_version)?;
    validate_non_negative(executor_fence)?;
    let next = advance(expected_slot_version)?;
    let query = format!(
        "update operational_execution_slots set slot_version = $5, blocked_uncertain = false, \
         updated_at = now() where slot_id = $1 and slot_version = $2 and operation_id = $3 \
         and executor_fence = $4 and blocked_uncertain returning {SLOT_COLUMNS}"
    );
    let row = sqlx::query(&query)
        .bind(slot_id)
        .bind(expected_slot_version)
        .bind(operation_id)
        .bind(executor_fence)
        .bind(next)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_database_error)?
        .ok_or_else(|| stale(cas_namespaces::SLOT_VERSION))?;
    slot_from_row(&row)
}

pub async fn insert_operational_evidence(
    transaction: &mut Transaction<'_, Postgres>,
    input: &OperationalEvidenceInput,
) -> ControlPlaneResult<()> {
    validate_identifier(&input.evidence_id)?;
    validate_identifier(&input.operation_id)?;
    validate_safe_json(&input.safe_metadata)?;
    if input.job_version < 1 || input.evidence_version < 1 {
        return Err(ControlPlaneRepositoryError::InvalidGeneration);
    }
    sqlx::query(
        "insert into operational_job_evidence (evidence_id, operation_id, job_version, \
         evidence_kind, evidence_schema, evidence_version, evidence_digest, status, safe_metadata, \
         source_environment_id, target_environment_id, helper_attempt_id) \
         values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)",
    )
    .bind(&input.evidence_id)
    .bind(&input.operation_id)
    .bind(input.job_version)
    .bind(&input.evidence_kind)
    .bind(&input.evidence_schema)
    .bind(input.evidence_version)
    .bind(input.evidence_digest.as_str())
    .bind(&input.status)
    .bind(&input.safe_metadata)
    .bind(&input.source_environment_id)
    .bind(&input.target_environment_id)
    .bind(&input.helper_attempt_id)
    .execute(&mut **transaction)
    .await
    .map_err(map_database_error)?;
    Ok(())
}

pub async fn append_operational_audit_event(
    transaction: &mut Transaction<'_, Postgres>,
    input: &OperationalAuditEventInput,
) -> ControlPlaneResult<()> {
    validate_identifier(&input.audit_id)?;
    validate_safe_json(&input.safe_metadata)?;
    validate_identifier(&input.event_type)?;
    validate_identifier(&input.outcome)?;
    validate_identifier(&input.actor_origin)?;
    validate_identifier(&input.target_type)?;
    sqlx::query(
        "insert into operational_audit_events (audit_id, occurred_at, event_type, outcome, \
         actor_principal_id, actor_credential_id, actor_role, actor_origin, request_id, correlation_id, \
         target_type, target_id, operation_id, maintenance_generation, adapter_control_generation, \
         principal_version, credential_set_generation, credential_version, job_version, executor_fence, \
         runtime_lease_version, standalone_runtime_epoch, runtime_report_sequence, issuance_operation_id, \
         previous_state, next_state, safe_error_category, artifact_manifest_id, safe_metadata, corrects_audit_id) \
         values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20, \
         $21,$22,$23,$24,$25,$26,$27,$28,$29,$30)",
    )
    .bind(&input.audit_id)
    .bind(input.occurred_at)
    .bind(&input.event_type)
    .bind(&input.outcome)
    .bind(&input.actor_principal_id)
    .bind(&input.actor_credential_id)
    .bind(&input.actor_role)
    .bind(&input.actor_origin)
    .bind(&input.request_id)
    .bind(&input.correlation_id)
    .bind(&input.target_type)
    .bind(&input.target_id)
    .bind(&input.operation_id)
    .bind(input.maintenance_generation)
    .bind(input.adapter_control_generation)
    .bind(input.principal_version)
    .bind(input.credential_set_generation)
    .bind(input.credential_version)
    .bind(input.job_version)
    .bind(input.executor_fence)
    .bind(input.runtime_lease_version)
    .bind(input.standalone_runtime_epoch)
    .bind(input.runtime_report_sequence)
    .bind(&input.issuance_operation_id)
    .bind(&input.previous_state)
    .bind(&input.next_state)
    .bind(&input.safe_error_category)
    .bind(&input.artifact_manifest_id)
    .bind(&input.safe_metadata)
    .bind(&input.corrects_audit_id)
    .execute(&mut **transaction)
    .await
    .map_err(map_database_error)?;
    Ok(())
}
