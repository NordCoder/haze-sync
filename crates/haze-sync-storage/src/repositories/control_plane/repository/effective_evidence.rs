pub async fn upsert_adapter_effective_control(
    transaction: &mut Transaction<'_, Postgres>,
    input: &AdapterEffectiveControlInput,
) -> ControlPlaneResult<AdapterEffectiveControlRow> {
    validate_identifier(&input.adapter_id)?;
    validate_safe_json(&input.checkpoint_summary)?;
    for generation in [
        input.last_applied_adapter_control_generation,
        input.last_applied_maintenance_generation,
        input.standalone_runtime_epoch,
        input.report_sequence,
        input.runtime_lease_version,
    ]
    .into_iter()
    .flatten()
    {
        validate_non_negative(generation)?;
    }
    let query = format!(
        "insert into adapter_effective_controls (adapter_id, effective_mode, runtime_lifecycle, \
         last_applied_adapter_control_generation, last_applied_maintenance_generation, heartbeat_at, \
         last_success_at, last_cycle_started_at, last_cycle_finished_at, in_flight, connection_state, \
         safe_error_category, safe_error_code, runtime_instance_id, standalone_runtime_epoch, \
         report_sequence, runtime_lease_version, runtime_lease_expires_at, checkpoint_summary, reported_at) \
         values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20) \
         on conflict (adapter_id) do update set effective_mode = excluded.effective_mode, \
         runtime_lifecycle = excluded.runtime_lifecycle, \
         last_applied_adapter_control_generation = excluded.last_applied_adapter_control_generation, \
         last_applied_maintenance_generation = excluded.last_applied_maintenance_generation, \
         heartbeat_at = excluded.heartbeat_at, last_success_at = excluded.last_success_at, \
         last_cycle_started_at = excluded.last_cycle_started_at, \
         last_cycle_finished_at = excluded.last_cycle_finished_at, in_flight = excluded.in_flight, \
         connection_state = excluded.connection_state, safe_error_category = excluded.safe_error_category, \
         safe_error_code = excluded.safe_error_code, runtime_instance_id = excluded.runtime_instance_id, \
         standalone_runtime_epoch = excluded.standalone_runtime_epoch, \
         report_sequence = excluded.report_sequence, runtime_lease_version = excluded.runtime_lease_version, \
         runtime_lease_expires_at = excluded.runtime_lease_expires_at, \
         checkpoint_summary = excluded.checkpoint_summary, reported_at = excluded.reported_at, \
         updated_at = now() returning {EFFECTIVE_COLUMNS}"
    );
    let row = sqlx::query(&query)
        .bind(&input.adapter_id)
        .bind(&input.effective_mode)
        .bind(&input.runtime_lifecycle)
        .bind(input.last_applied_adapter_control_generation)
        .bind(input.last_applied_maintenance_generation)
        .bind(input.heartbeat_at)
        .bind(input.last_success_at)
        .bind(input.last_cycle_started_at)
        .bind(input.last_cycle_finished_at)
        .bind(input.in_flight)
        .bind(&input.connection_state)
        .bind(&input.safe_error_category)
        .bind(&input.safe_error_code)
        .bind(&input.runtime_instance_id)
        .bind(input.standalone_runtime_epoch)
        .bind(input.report_sequence)
        .bind(input.runtime_lease_version)
        .bind(input.runtime_lease_expires_at)
        .bind(&input.checkpoint_summary)
        .bind(input.reported_at)
        .fetch_one(&mut **transaction)
        .await
        .map_err(map_database_error)?;
    effective_from_row(&row)
}

/// Locks one exact adapter-effective row inside the caller transaction.
pub async fn lock_adapter_effective_control(
    transaction: &mut Transaction<'_, Postgres>,
    adapter_id: &str,
) -> ControlPlaneResult<AdapterEffectiveControlRow> {
    validate_identifier(adapter_id)?;
    let query = format!(
        "select {EFFECTIVE_COLUMNS} from adapter_effective_controls where adapter_id = $1 for update"
    );
    let row = sqlx::query(&query)
        .bind(adapter_id)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_database_error)?
        .ok_or(ControlPlaneRepositoryError::MissingRecord)?;
    effective_from_row(&row)
}

/// Inserts immutable, inventory-complete quiescence evidence.
pub async fn insert_quiescence_evidence(
    transaction: &mut Transaction<'_, Postgres>,
    input: &QuiescenceEvidenceInput,
) -> ControlPlaneResult<()> {
    validate_identifier(&input.quiescence_evidence_id)?;
    validate_identifier(&input.server_instance_id)?;
    for snapshot in &input.adapter_snapshots {
        validate_safe_json(&snapshot.checkpoint_summary)?;
    }
    if input.evidence_version < 1 {
        return Err(ControlPlaneRepositoryError::InvalidGeneration);
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
    let inventory_ids = inventory
        .iter()
        .map(|row| row.adapter_id.as_str())
        .collect::<Vec<_>>();
    let mut snapshot_ids = input
        .adapter_snapshots
        .iter()
        .map(|row| row.adapter_id.as_str())
        .collect::<Vec<_>>();
    snapshot_ids.sort_unstable();
    if snapshot_ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(ControlPlaneRepositoryError::InvalidIdentifier);
    }
    let mut expected_ids = inventory_ids;
    expected_ids.sort_unstable();
    if expected_ids != snapshot_ids {
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
            validate_identifier(&runtime.adapter_id)?;
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
                    || effective.report_sequence
                        != Some(authority.last_accepted_report_sequence)
                    || effective.runtime_lease_expires_at != authority.lease_expires_at
                {
                    return Err(ControlPlaneRepositoryError::IdempotencyConflict);
                }
            }
        }
    }

    sqlx::query(
        "insert into quiescence_evidence (quiescence_evidence_id, evidence_version, \
         maintenance_generation, adapter_inventory_generation, admission_fence_closed_at, \
         server_instance_id, server_started_at, active_authoritative_mutations, \
         open_authoritative_transactions, obsidian_authoritative_mutation_gate, evidence_digest, captured_at) \
         values ($1,$2,$3,$4,$5,$6,$7,0,0,'closed',$8,$9)",
    )
    .bind(&input.quiescence_evidence_id)
    .bind(input.evidence_version)
    .bind(input.maintenance_generation)
    .bind(input.adapter_inventory_generation)
    .bind(input.admission_fence_closed_at)
    .bind(&input.server_instance_id)
    .bind(input.server_started_at)
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
        if !snapshot_ids.contains(&runtime.adapter_id.as_str()) {
            return Err(ControlPlaneRepositoryError::InvalidIdentifier);
        }
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
    Ok(())
}

/// Appends evidence invalidation without mutating the immutable evidence bundle.
pub async fn append_quiescence_evidence_invalidation(
    transaction: &mut Transaction<'_, Postgres>,
    input: &QuiescenceEvidenceInvalidationInput,
) -> ControlPlaneResult<()> {
    validate_identifier(&input.invalidation_id)?;
    validate_identifier(&input.quiescence_evidence_id)?;
    validate_identifier(&input.invalidation_reason)?;
    validate_non_negative(input.maintenance_generation)?;
    sqlx::query(
        "insert into quiescence_evidence_invalidations (invalidation_id, \
         quiescence_evidence_id, maintenance_generation, invalidation_reason) \
         values ($1,$2,$3,$4)",
    )
    .bind(&input.invalidation_id)
    .bind(&input.quiescence_evidence_id)
    .bind(input.maintenance_generation)
    .bind(&input.invalidation_reason)
    .execute(&mut **transaction)
    .await
    .map_err(map_database_error)?;
    Ok(())
}
