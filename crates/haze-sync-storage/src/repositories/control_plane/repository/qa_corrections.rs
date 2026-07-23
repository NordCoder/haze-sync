fn complete_quiescence_evidence_from_row(
    row: &PgRow,
) -> ControlPlaneResult<CompleteQuiescenceEvidenceRow> {
    Ok(CompleteQuiescenceEvidenceRow {
        quiescence_evidence_id: row_column!(row, "quiescence_evidence_id"),
        schema_version: row_column!(row, "schema_version"),
        evidence_version: row_column!(row, "evidence_version"),
        maintenance_generation: row_column!(row, "maintenance_generation"),
        adapter_inventory_generation: row_column!(row, "adapter_inventory_generation"),
        admission_fence_closed_at: row_column!(row, "admission_fence_closed_at"),
        server_instance_id: row_column!(row, "server_instance_id"),
        server_started_at: row_column!(row, "server_started_at"),
        active_authoritative_mutations: row_column!(row, "active_authoritative_mutations"),
        open_authoritative_transactions: row_column!(row, "open_authoritative_transactions"),
        obsidian_authoritative_mutation_gate: row_column!(
            row,
            "obsidian_authoritative_mutation_gate"
        ),
        active_cli_write_jobs: row_column!(row, "active_cli_write_jobs"),
        object_store_writer_count: row_column!(row, "object_store_writer_count"),
        database_writer_count: row_column!(row, "database_writer_count"),
        worktree_external_writer_evidence: row_column!(
            row,
            "worktree_external_writer_evidence"
        ),
        evidence_digest: required_digest(row, "evidence_digest")?,
        captured_at: row_column!(row, "captured_at"),
    })
}

fn quiescence_adapter_snapshot_from_row(
    row: &PgRow,
) -> ControlPlaneResult<QuiescenceAdapterSnapshotRow> {
    Ok(QuiescenceAdapterSnapshotRow {
        quiescence_evidence_id: row_column!(row, "quiescence_evidence_id"),
        adapter_id: row_column!(row, "adapter_id"),
        adapter_kind: row_column!(row, "adapter_kind"),
        control_authority: row_column!(row, "control_authority"),
        adapter_control_generation: row_column!(row, "adapter_control_generation"),
        maintenance_generation: row_column!(row, "maintenance_generation"),
        desired_enabled: row_column!(row, "desired_enabled"),
        desired_mode: row_column!(row, "desired_mode"),
        last_applied_adapter_control_generation: row_column!(
            row,
            "last_applied_adapter_control_generation"
        ),
        last_applied_maintenance_generation: row_column!(
            row,
            "last_applied_maintenance_generation"
        ),
        runtime_lifecycle: row_column!(row, "runtime_lifecycle"),
        connection_state: row_column!(row, "connection_state"),
        in_flight: row_column!(row, "in_flight"),
        drain_proof: row_column!(row, "drain_proof"),
        external_fence_id: row_column!(row, "external_fence_id"),
        checkpoint_summary: row_column!(row, "checkpoint_summary"),
        captured_at: row_column!(row, "captured_at"),
    })
}

fn quiescence_runtime_snapshot_from_row(
    row: &PgRow,
) -> ControlPlaneResult<QuiescenceRuntimeSnapshotRow> {
    Ok(QuiescenceRuntimeSnapshotRow {
        quiescence_evidence_id: row_column!(row, "quiescence_evidence_id"),
        adapter_id: row_column!(row, "adapter_id"),
        runtime_instance_id: row_column!(row, "runtime_instance_id"),
        runtime_lease_version: row_column!(row, "runtime_lease_version"),
        standalone_runtime_epoch: row_column!(row, "standalone_runtime_epoch"),
        last_accepted_report_sequence: row_column!(row, "last_accepted_report_sequence"),
        last_accepted_report_fingerprint: required_digest(
            row,
            "last_accepted_report_fingerprint",
        )?,
        open_mutation_permits: row_column!(row, "open_mutation_permits"),
        uncertain_external_effects: row_column!(row, "uncertain_external_effects"),
        takeover_state: row_column!(row, "takeover_state"),
        runtime_lease_expires_at: row_column!(row, "runtime_lease_expires_at"),
        external_fence_id: row_column!(row, "external_fence_id"),
        captured_at: row_column!(row, "captured_at"),
    })
}

/// Inserts one recovery-complete immutable evidence bundle in the caller transaction.
pub async fn insert_complete_quiescence_evidence(
    transaction: &mut Transaction<'_, Postgres>,
    input: &CompleteQuiescenceEvidenceInput,
) -> ControlPlaneResult<CompleteQuiescenceEvidenceBundle> {
    validate_identifier(&input.quiescence_evidence_id)?;
    validate_identifier(&input.server_instance_id)?;
    validate_safe_json(&input.worktree_external_writer_evidence)?;
    if input.worktree_external_writer_evidence == serde_json::json!({})
        || input.worktree_external_writer_evidence == serde_json::json!([])
    {
        return Err(ControlPlaneRepositoryError::InvalidIdentifier);
    }
    if input.evidence_version < 1
        || input.active_authoritative_mutations != 0
        || input.open_authoritative_transactions != 0
        || input.active_cli_write_jobs != 0
        || input.object_store_writer_count != 0
        || input.database_writer_count != 0
    {
        return Err(ControlPlaneRepositoryError::InvalidGeneration);
    }
    for snapshot in &input.adapter_snapshots {
        validate_safe_json(&snapshot.checkpoint_summary)?;
    }

    let maintenance = lock_maintenance_control(transaction).await?;
    if maintenance.maintenance_generation != input.maintenance_generation
        || !maintenance.admission_fence_closed
        || maintenance.admission_fence_closed_at != Some(input.admission_fence_closed_at)
    {
        return Err(stale(cas_namespaces::MAINTENANCE_GENERATION));
    }
    let (inventory_state, inventory) = read_adapter_inventory_snapshot(transaction).await?;
    if inventory_state.adapter_inventory_generation != input.adapter_inventory_generation {
        return Err(stale(cas_namespaces::ADAPTER_INVENTORY_GENERATION));
    }

    let mut inventory_ids = inventory
        .iter()
        .map(|row| row.adapter_id.as_str())
        .collect::<Vec<_>>();
    let mut snapshot_ids = input
        .adapter_snapshots
        .iter()
        .map(|row| row.adapter_id.as_str())
        .collect::<Vec<_>>();
    inventory_ids.sort_unstable();
    snapshot_ids.sort_unstable();
    if snapshot_ids.windows(2).any(|pair| pair[0] == pair[1])
        || inventory_ids != snapshot_ids
    {
        return Err(ControlPlaneRepositoryError::InvalidIdentifier);
    }

    let mut expected_runtime_ids = input
        .adapter_snapshots
        .iter()
        .filter(|snapshot| snapshot.adapter_kind == "gdrive")
        .map(|snapshot| snapshot.adapter_id.as_str())
        .collect::<Vec<_>>();
    let mut runtime_ids = input
        .runtime_snapshots
        .iter()
        .map(|snapshot| snapshot.adapter_id.as_str())
        .collect::<Vec<_>>();
    expected_runtime_ids.sort_unstable();
    runtime_ids.sort_unstable();
    if runtime_ids.windows(2).any(|pair| pair[0] == pair[1])
        || expected_runtime_ids != runtime_ids
    {
        return Err(ControlPlaneRepositoryError::InvalidIdentifier);
    }

    for snapshot in &input.adapter_snapshots {
        for identifier in [
            snapshot.adapter_id.as_str(),
            snapshot.adapter_kind.as_str(),
            snapshot.control_authority.as_str(),
            snapshot.desired_mode.as_str(),
            snapshot.runtime_lifecycle.as_str(),
            snapshot.connection_state.as_str(),
            snapshot.drain_proof.as_str(),
        ] {
            validate_identifier(identifier)?;
        }
        if let Some(external_fence_id) = &snapshot.external_fence_id {
            validate_identifier(external_fence_id)?;
        }
        let inventory_row = inventory
            .iter()
            .find(|row| row.adapter_id == snapshot.adapter_id)
            .ok_or(ControlPlaneRepositoryError::MissingRecord)?;
        if inventory_row.adapter_kind != snapshot.adapter_kind
            || inventory_row.control_authority != snapshot.control_authority
            || snapshot.maintenance_generation != input.maintenance_generation
        {
            return Err(ControlPlaneRepositoryError::IdempotencyConflict);
        }
        let desired = lock_adapter_desired_control(transaction, &snapshot.adapter_id).await?;
        if desired.adapter_control_generation != snapshot.adapter_control_generation
            || desired.maintenance_generation != snapshot.maintenance_generation
            || desired.desired_enabled != snapshot.desired_enabled
            || desired.desired_mode != snapshot.desired_mode
        {
            return Err(stale(cas_namespaces::ADAPTER_CONTROL_GENERATION));
        }

        let runtime = input
            .runtime_snapshots
            .iter()
            .find(|runtime| runtime.adapter_id == snapshot.adapter_id);
        let authority = if snapshot.adapter_kind == "gdrive" {
            let runtime = runtime.ok_or(ControlPlaneRepositoryError::MissingRecord)?;
            validate_identifier(&runtime.runtime_instance_id)?;
            if let Some(external_fence_id) = &runtime.external_fence_id {
                validate_identifier(external_fence_id)?;
            }
            let authority = lock_runtime_authority(transaction, &runtime.adapter_id).await?;
            if authority.runtime_instance_id.as_deref()
                != Some(runtime.runtime_instance_id.as_str())
                || authority.runtime_lease_version != runtime.runtime_lease_version
                || authority.standalone_runtime_epoch != runtime.standalone_runtime_epoch
                || authority.last_accepted_report_sequence
                    != runtime.last_accepted_report_sequence
                || authority.last_accepted_report_fingerprint.as_ref()
                    != Some(&runtime.last_accepted_report_fingerprint)
                || authority.open_mutation_permits != 0
                || authority.uncertain_external_effects != 0
                || authority.takeover_state != "clear"
                || authority.lease_expires_at != runtime.runtime_lease_expires_at
            {
                return Err(ControlPlaneRepositoryError::IdempotencyConflict);
            }
            Some((authority, runtime))
        } else {
            None
        };

        if snapshot.drain_proof == "acknowledged" {
            let effective = lock_adapter_effective_control(transaction, &snapshot.adapter_id).await?;
            if effective.last_applied_adapter_control_generation
                != snapshot.last_applied_adapter_control_generation
                || effective.last_applied_maintenance_generation
                    != snapshot.last_applied_maintenance_generation
                || effective.runtime_lifecycle != snapshot.runtime_lifecycle
                || effective.connection_state != snapshot.connection_state
                || effective.in_flight
                || snapshot.last_applied_adapter_control_generation
                    != Some(snapshot.adapter_control_generation)
                || snapshot.last_applied_maintenance_generation
                    != Some(snapshot.maintenance_generation)
            {
                return Err(ControlPlaneRepositoryError::IdempotencyConflict);
            }
            if let Some((authority, runtime)) = authority {
                if effective.runtime_instance_id.as_deref()
                    != Some(runtime.runtime_instance_id.as_str())
                    || effective.standalone_runtime_epoch
                        != Some(runtime.standalone_runtime_epoch)
                    || effective.runtime_lease_version != Some(authority.runtime_lease_version)
                    || effective.report_sequence != Some(authority.last_accepted_report_sequence)
                    || effective.runtime_lease_expires_at != authority.lease_expires_at
                {
                    return Err(ControlPlaneRepositoryError::IdempotencyConflict);
                }
            }
        } else if snapshot.drain_proof != "externally_fenced"
            || snapshot.external_fence_id.is_none()
        {
            return Err(ControlPlaneRepositoryError::InvalidIdentifier);
        }
    }

    sqlx::query(
        "insert into quiescence_evidence (quiescence_evidence_id, evidence_version, \
         maintenance_generation, adapter_inventory_generation, admission_fence_closed_at, \
         server_instance_id, server_started_at, active_authoritative_mutations, \
         open_authoritative_transactions, obsidian_authoritative_mutation_gate, \
         active_cli_write_jobs, object_store_writer_count, database_writer_count, \
         worktree_external_writer_evidence, evidence_digest, captured_at) \
         values ($1,$2,$3,$4,$5,$6,$7,$8,$9,'closed',$10,$11,$12,$13,$14,$15)",
    )
    .bind(&input.quiescence_evidence_id)
    .bind(input.evidence_version)
    .bind(input.maintenance_generation)
    .bind(input.adapter_inventory_generation)
    .bind(input.admission_fence_closed_at)
    .bind(&input.server_instance_id)
    .bind(input.server_started_at)
    .bind(input.active_authoritative_mutations)
    .bind(input.open_authoritative_transactions)
    .bind(input.active_cli_write_jobs)
    .bind(input.object_store_writer_count)
    .bind(input.database_writer_count)
    .bind(&input.worktree_external_writer_evidence)
    .bind(input.evidence_digest.as_str())
    .bind(input.captured_at)
    .execute(&mut **transaction)
    .await
    .map_err(map_database_error)?;

    for snapshot in &input.adapter_snapshots {
        sqlx::query(
            "insert into quiescence_adapter_snapshots (quiescence_evidence_id, adapter_id, \
             adapter_kind, control_authority, adapter_control_generation, maintenance_generation, \
             desired_enabled, desired_mode, last_applied_adapter_control_generation, \
             last_applied_maintenance_generation, runtime_lifecycle, connection_state, in_flight, \
             drain_proof, external_fence_id, checkpoint_summary, captured_at) \
             values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,false,$13,$14,$15,$16)",
        )
        .bind(&input.quiescence_evidence_id)
        .bind(&snapshot.adapter_id)
        .bind(&snapshot.adapter_kind)
        .bind(&snapshot.control_authority)
        .bind(snapshot.adapter_control_generation)
        .bind(snapshot.maintenance_generation)
        .bind(snapshot.desired_enabled)
        .bind(&snapshot.desired_mode)
        .bind(snapshot.last_applied_adapter_control_generation)
        .bind(snapshot.last_applied_maintenance_generation)
        .bind(&snapshot.runtime_lifecycle)
        .bind(&snapshot.connection_state)
        .bind(&snapshot.drain_proof)
        .bind(&snapshot.external_fence_id)
        .bind(&snapshot.checkpoint_summary)
        .bind(snapshot.captured_at)
        .execute(&mut **transaction)
        .await
        .map_err(map_database_error)?;
    }

    for runtime in &input.runtime_snapshots {
        sqlx::query(
            "insert into quiescence_runtime_snapshots (quiescence_evidence_id, adapter_id, \
             runtime_instance_id, runtime_lease_version, standalone_runtime_epoch, \
             last_accepted_report_sequence, last_accepted_report_fingerprint, open_mutation_permits, \
             uncertain_external_effects, takeover_state, runtime_lease_expires_at, external_fence_id, \
             captured_at) values ($1,$2,$3,$4,$5,$6,$7,0,0,'clear',$8,$9,$10)",
        )
        .bind(&input.quiescence_evidence_id)
        .bind(&runtime.adapter_id)
        .bind(&runtime.runtime_instance_id)
        .bind(runtime.runtime_lease_version)
        .bind(runtime.standalone_runtime_epoch)
        .bind(runtime.last_accepted_report_sequence)
        .bind(runtime.last_accepted_report_fingerprint.as_str())
        .bind(runtime.runtime_lease_expires_at)
        .bind(&runtime.external_fence_id)
        .bind(runtime.captured_at)
        .execute(&mut **transaction)
        .await
        .map_err(map_database_error)?;
    }

    lock_complete_quiescence_evidence(transaction, &input.quiescence_evidence_id).await
}

/// Re-reads and share-locks one complete immutable evidence bundle.
pub async fn lock_complete_quiescence_evidence(
    transaction: &mut Transaction<'_, Postgres>,
    quiescence_evidence_id: &str,
) -> ControlPlaneResult<CompleteQuiescenceEvidenceBundle> {
    validate_identifier(quiescence_evidence_id)?;
    let evidence = sqlx::query(
        "select quiescence_evidence_id, schema_version, evidence_version, maintenance_generation, \
         adapter_inventory_generation, admission_fence_closed_at, server_instance_id, \
         server_started_at, active_authoritative_mutations, open_authoritative_transactions, \
         obsidian_authoritative_mutation_gate, active_cli_write_jobs, object_store_writer_count, \
         database_writer_count, worktree_external_writer_evidence, evidence_digest, captured_at \
         from quiescence_evidence where quiescence_evidence_id = $1 for share",
    )
    .bind(quiescence_evidence_id)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_database_error)?
    .ok_or(ControlPlaneRepositoryError::MissingRecord)?;

    let adapter_rows = sqlx::query(
        "select quiescence_evidence_id, adapter_id, adapter_kind, control_authority, \
         adapter_control_generation, maintenance_generation, desired_enabled, desired_mode, \
         last_applied_adapter_control_generation, last_applied_maintenance_generation, \
         runtime_lifecycle, connection_state, in_flight, drain_proof, external_fence_id, \
         checkpoint_summary, captured_at from quiescence_adapter_snapshots \
         where quiescence_evidence_id = $1 order by adapter_id for share",
    )
    .bind(quiescence_evidence_id)
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_database_error)?;
    let runtime_rows = sqlx::query(
        "select quiescence_evidence_id, adapter_id, runtime_instance_id, runtime_lease_version, \
         standalone_runtime_epoch, last_accepted_report_sequence, last_accepted_report_fingerprint, \
         open_mutation_permits, uncertain_external_effects, takeover_state, runtime_lease_expires_at, \
         external_fence_id, captured_at from quiescence_runtime_snapshots \
         where quiescence_evidence_id = $1 order by adapter_id for share",
    )
    .bind(quiescence_evidence_id)
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_database_error)?;
    let invalidation_rows = sqlx::query(
        "select invalidation_id, quiescence_evidence_id, maintenance_generation, \
         invalidation_reason, invalidated_at from quiescence_evidence_invalidations \
         where quiescence_evidence_id = $1 order by invalidated_at, invalidation_id for share",
    )
    .bind(quiescence_evidence_id)
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_database_error)?;

    Ok(CompleteQuiescenceEvidenceBundle {
        evidence: complete_quiescence_evidence_from_row(&evidence)?,
        adapter_snapshots: adapter_rows
            .iter()
            .map(quiescence_adapter_snapshot_from_row)
            .collect::<ControlPlaneResult<Vec<_>>>()?,
        runtime_snapshots: runtime_rows
            .iter()
            .map(quiescence_runtime_snapshot_from_row)
            .collect::<ControlPlaneResult<Vec<_>>>()?,
        invalidations: invalidation_rows
            .iter()
            .map(|row| {
                Ok(QuiescenceEvidenceInvalidationRow {
                    invalidation_id: row_column!(row, "invalidation_id"),
                    quiescence_evidence_id: row_column!(row, "quiescence_evidence_id"),
                    maintenance_generation: row_column!(row, "maintenance_generation"),
                    invalidation_reason: row_column!(row, "invalidation_reason"),
                    invalidated_at: row_column!(row, "invalidated_at"),
                })
            })
            .collect::<ControlPlaneResult<Vec<_>>>()?,
    })
}

async fn load_complete_operational_job(
    transaction: &mut Transaction<'_, Postgres>,
    operation_id: &str,
    lock: bool,
) -> ControlPlaneResult<CompleteOperationalJobRow> {
    validate_identifier(operation_id)?;
    let lock_clause = if lock { " for update" } else { "" };
    let query = format!(
        "select {JOB_COLUMNS}, idempotency_scope, idempotency_key_digest, request_fingerprint \
         from operational_jobs where operation_id = $1{lock_clause}"
    );
    let row = sqlx::query(&query)
        .bind(operation_id)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_database_error)?
        .ok_or(ControlPlaneRepositoryError::MissingRecord)?;
    let expected_adapter_generations = sqlx::query_as::<_, (String, i64)>(
        "select adapter_id, adapter_control_generation from operational_job_adapter_generations \
         where operation_id = $1 order by adapter_id",
    )
    .bind(operation_id)
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_database_error)?;
    Ok(CompleteOperationalJobRow {
        job: job_from_row(&row)?,
        idempotency_scope: row_column!(&row, "idempotency_scope"),
        idempotency_key_digest: required_digest(&row, "idempotency_key_digest")?,
        request_fingerprint: required_digest(&row, "request_fingerprint")?,
        expected_adapter_generations,
    })
}

/// Reads every canonical operational-job confirmation/idempotency binding.
pub async fn read_complete_operational_job(
    transaction: &mut Transaction<'_, Postgres>,
    operation_id: &str,
) -> ControlPlaneResult<CompleteOperationalJobRow> {
    load_complete_operational_job(transaction, operation_id, false).await
}

/// Locks and reads every canonical operational-job confirmation/idempotency binding.
pub async fn lock_complete_operational_job(
    transaction: &mut Transaction<'_, Postgres>,
    operation_id: &str,
) -> ControlPlaneResult<CompleteOperationalJobRow> {
    load_complete_operational_job(transaction, operation_id, true).await
}

/// Creates/replays a job and returns the complete canonical durable binding.
pub async fn insert_or_replay_complete_operational_job(
    transaction: &mut Transaction<'_, Postgres>,
    input: &OperationalJobInput,
) -> ControlPlaneResult<IdempotencyInsertOutcome<CompleteOperationalJobRow>> {
    let outcome = insert_or_replay_operational_job(transaction, input).await?;
    match outcome {
        IdempotencyInsertOutcome::Inserted(row) => Ok(IdempotencyInsertOutcome::Inserted(
            lock_complete_operational_job(transaction, &row.operation_id).await?,
        )),
        IdempotencyInsertOutcome::Replay(row) => Ok(IdempotencyInsertOutcome::Replay(
            lock_complete_operational_job(transaction, &row.operation_id).await?,
        )),
    }
}

async fn read_credential_issuance_scope(
    transaction: &mut Transaction<'_, Postgres>,
    input: &CredentialIssuanceInput,
) -> ControlPlaneResult<Option<CredentialIssuanceRow>> {
    sqlx::query(
        "select requester_principal_id, idempotency_key_digest, request_fingerprint, \
         issuance_operation_id, action, target_principal_id, affected_credential_ids, \
         committed_outcome, safe_result, committed_at from credential_issuance_idempotency \
         where requester_principal_id = $1 and idempotency_key_digest = $2",
    )
    .bind(&input.requester_principal_id)
    .bind(input.idempotency_key_digest.as_str())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_database_error)?
    .as_ref()
    .map(issuance_from_row)
    .transpose()
}

fn classify_credential_issuance_replay(
    existing: CredentialIssuanceRow,
    input: &CredentialIssuanceInput,
) -> ControlPlaneResult<CredentialIssuanceReservationOutcome> {
    if existing.request_fingerprint != input.request_fingerprint {
        return Err(ControlPlaneRepositoryError::IdempotencyConflict);
    }
    Ok(CredentialIssuanceReservationOutcome::Replay(existing))
}

/// Non-blocking issuance reservation. A concurrent winner yields `InProgress`.
pub async fn reserve_or_replay_credential_issuance(
    transaction: &mut Transaction<'_, Postgres>,
    input: &CredentialIssuanceInput,
) -> ControlPlaneResult<CredentialIssuanceReservationOutcome> {
    validate_identifier(&input.requester_principal_id)?;
    validate_identifier(&input.issuance_operation_id)?;
    validate_identifier(&input.action)?;
    validate_identifier(&input.target_principal_id)?;
    validate_identifier(&input.committed_outcome)?;
    validate_safe_json(&input.affected_credential_ids)?;
    validate_safe_json(&input.safe_result)?;

    if let Some(existing) = read_credential_issuance_scope(transaction, input).await? {
        return classify_credential_issuance_replay(existing, input);
    }

    let mut identity = String::with_capacity(256);
    identity.push_str("credential_issuance");
    identity.push('\u{1f}');
    identity.push_str(&input.requester_principal_id);
    identity.push('\u{1f}');
    identity.push_str(input.idempotency_key_digest.as_str());
    let acquired: bool = sqlx::query_scalar(
        "select pg_try_advisory_xact_lock(hashtextextended($1, 0))",
    )
    .bind(identity)
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_database_error)?;
    if !acquired {
        return Ok(CredentialIssuanceReservationOutcome::InProgress);
    }

    if let Some(existing) = read_credential_issuance_scope(transaction, input).await? {
        return classify_credential_issuance_replay(existing, input);
    }

    let row = sqlx::query(
        "insert into credential_issuance_idempotency (requester_principal_id, \
         idempotency_key_digest, request_fingerprint, issuance_operation_id, action, \
         target_principal_id, affected_credential_ids, committed_outcome, safe_result, \
         plaintext_available) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,false) \
         returning requester_principal_id, idempotency_key_digest, request_fingerprint, \
         issuance_operation_id, action, target_principal_id, affected_credential_ids, \
         committed_outcome, safe_result, committed_at",
    )
    .bind(&input.requester_principal_id)
    .bind(input.idempotency_key_digest.as_str())
    .bind(input.request_fingerprint.as_str())
    .bind(&input.issuance_operation_id)
    .bind(&input.action)
    .bind(&input.target_principal_id)
    .bind(&input.affected_credential_ids)
    .bind(&input.committed_outcome)
    .bind(&input.safe_result)
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_database_error)?;
    Ok(CredentialIssuanceReservationOutcome::Inserted(
        issuance_from_row(&row)?,
    ))
}
