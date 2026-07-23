pub async fn accept_ordered_runtime_report(
    transaction: &mut Transaction<'_, Postgres>,
    input: &RuntimeReportInput,
) -> ControlPlaneResult<RuntimeReportOutcome> {
    validate_identifier(&input.adapter_id)?;
    validate_identifier(&input.runtime_instance_id)?;
    validate_safe_json(&input.checkpoint_summary)?;
    if input.report_sequence < 1 {
        return Err(ControlPlaneRepositoryError::StaleRuntimeReport);
    }
    // Match the common control lock order used by permit admission and evidence capture.
    let maintenance = lock_maintenance_control(transaction).await?;
    let desired = lock_adapter_desired_control(transaction, &input.adapter_id).await?;
    let authority = lock_runtime_authority(transaction, &input.adapter_id).await?;
    let effective_query = format!(
        "select {EFFECTIVE_COLUMNS} from adapter_effective_controls where adapter_id = $1 for update"
    );
    let effective = sqlx::query(&effective_query)
        .bind(&input.adapter_id)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_database_error)?
        .as_ref()
        .map(effective_from_row)
        .transpose()?;
    if authority.standalone_runtime_epoch != input.standalone_runtime_epoch {
        return Err(ControlPlaneRepositoryError::StaleRuntimeEpoch);
    }
    if authority.runtime_instance_id.as_deref() != Some(input.runtime_instance_id.as_str()) {
        return Err(ControlPlaneRepositoryError::StaleRuntimeEpoch);
    }
    if authority.last_accepted_report_sequence > 0 {
        let current_effective = effective
            .as_ref()
            .ok_or(ControlPlaneRepositoryError::RuntimeReportConflict)?;
        if current_effective.runtime_instance_id.as_deref()
            != Some(input.runtime_instance_id.as_str())
            || current_effective.standalone_runtime_epoch
                != Some(input.standalone_runtime_epoch)
            || current_effective.runtime_lease_version
                != Some(authority.runtime_lease_version)
            || current_effective.report_sequence
                != Some(authority.last_accepted_report_sequence)
        {
            return Err(ControlPlaneRepositoryError::RuntimeReportConflict);
        }
    }
    let database_now: DateTime<Utc> = sqlx::query_scalar("select now()")
        .fetch_one(&mut **transaction)
        .await
        .map_err(map_database_error)?;
    if authority.lease_token_digest.as_ref() != Some(&input.lease_proof_digest)
        || authority
            .lease_expires_at
            .map_or(true, |expiry| expiry <= database_now)
    {
        return Err(stale(cas_namespaces::RUNTIME_LEASE_VERSION));
    }
    if input.report_sequence == authority.last_accepted_report_sequence {
        if authority.last_accepted_report_fingerprint.as_ref() == Some(&input.report_fingerprint) {
            return Ok(RuntimeReportOutcome::Replay(authority));
        }
        return Err(ControlPlaneRepositoryError::RuntimeReportConflict);
    }
    if input.report_sequence != authority.last_accepted_report_sequence + 1 {
        return Err(ControlPlaneRepositoryError::StaleRuntimeReport);
    }
    if input.expected_runtime_lease_version != authority.runtime_lease_version {
        return Err(stale(cas_namespaces::RUNTIME_LEASE_VERSION));
    }
    if input.adapter_control_generation > desired.adapter_control_generation
        || input.maintenance_generation > maintenance.maintenance_generation
    {
        return Err(ControlPlaneRepositoryError::RuntimeReportConflict);
    }

    let mut close_permit_ids = input.close_permit_ids.clone();
    for permit_id in &close_permit_ids {
        validate_identifier(permit_id)?;
    }
    close_permit_ids.sort_unstable();
    if close_permit_ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(ControlPlaneRepositoryError::RuntimeReportConflict);
    }
    let current_open_permits: i64 = sqlx::query_scalar(
        "select count(*)::bigint from gdrive_mutation_permits where adapter_id = $1 \
         and standalone_runtime_epoch = $2 and state = 'open'",
    )
    .bind(&input.adapter_id)
    .bind(input.standalone_runtime_epoch)
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_database_error)?;
    let matching_open_permits: i64 = if close_permit_ids.is_empty() {
        0
    } else {
        sqlx::query_scalar(
            "select count(*)::bigint from gdrive_mutation_permits where adapter_id = $1 \
             and standalone_runtime_epoch = $2 and state = 'open' and permit_id = any($3)",
        )
        .bind(&input.adapter_id)
        .bind(input.standalone_runtime_epoch)
        .bind(&close_permit_ids)
        .fetch_one(&mut **transaction)
        .await
        .map_err(map_database_error)?
    };
    if matching_open_permits != close_permit_ids.len() as i64 {
        return Err(ControlPlaneRepositoryError::RuntimeReportConflict);
    }
    let open_permits = current_open_permits
        .checked_sub(matching_open_permits)
        .ok_or(ControlPlaneRepositoryError::RuntimeReportConflict)?;
    if input.in_flight != (open_permits > 0) {
        return Err(ControlPlaneRepositoryError::RuntimeReportConflict);
    }
    if !close_permit_ids.is_empty() {
        let result = sqlx::query(
            "update gdrive_mutation_permits set state = 'closed', \
             terminal_outcome = 'reported_terminal', checkpoint_summary = $4, closed_at = now() \
             where adapter_id = $1 and standalone_runtime_epoch = $2 and state = 'open' \
             and permit_id = any($3)",
        )
        .bind(&input.adapter_id)
        .bind(input.standalone_runtime_epoch)
        .bind(&close_permit_ids)
        .bind(&input.checkpoint_summary)
        .execute(&mut **transaction)
        .await
        .map_err(map_database_error)?;
        if result.rows_affected() != matching_open_permits as u64 {
            return Err(ControlPlaneRepositoryError::RuntimeReportConflict);
        }
    }
    let next_version = advance(authority.runtime_lease_version)?;
    sqlx::query(
        "insert into gdrive_runtime_reports (adapter_id, standalone_runtime_epoch, report_sequence, \
         runtime_instance_id, accepted_runtime_lease_version, report_fingerprint, \
         adapter_control_generation, maintenance_generation, in_flight, open_mutation_permits, \
         checkpoint_summary, safe_error_category, safe_error_code, reported_at) \
         values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)",
    )
    .bind(&input.adapter_id)
    .bind(input.standalone_runtime_epoch)
    .bind(input.report_sequence)
    .bind(&input.runtime_instance_id)
    .bind(next_version)
    .bind(input.report_fingerprint.as_str())
    .bind(input.adapter_control_generation)
    .bind(input.maintenance_generation)
    .bind(input.in_flight)
    .bind(open_permits)
    .bind(&input.checkpoint_summary)
    .bind(&input.safe_error_category)
    .bind(&input.safe_error_code)
    .bind(input.reported_at)
    .execute(&mut **transaction)
    .await
    .map_err(map_database_error)?;

    upsert_adapter_effective_control(
        transaction,
        &AdapterEffectiveControlInput {
            adapter_id: input.adapter_id.clone(),
            effective_mode: input.effective_mode.clone(),
            runtime_lifecycle: input.runtime_lifecycle.clone(),
            last_applied_adapter_control_generation: Some(input.adapter_control_generation),
            last_applied_maintenance_generation: Some(input.maintenance_generation),
            heartbeat_at: Some(input.reported_at),
            last_success_at: None,
            last_cycle_started_at: None,
            last_cycle_finished_at: None,
            in_flight: input.in_flight,
            connection_state: input.connection_state.clone(),
            safe_error_category: input.safe_error_category.clone(),
            safe_error_code: input.safe_error_code.clone(),
            runtime_instance_id: Some(input.runtime_instance_id.clone()),
            standalone_runtime_epoch: Some(input.standalone_runtime_epoch),
            report_sequence: Some(input.report_sequence),
            runtime_lease_version: Some(next_version),
            runtime_lease_expires_at: authority.lease_expires_at,
            checkpoint_summary: input.checkpoint_summary.clone(),
            reported_at: input.reported_at,
        },
    )
    .await?;

    let query = format!(
        "update gdrive_runtime_authorities set runtime_lease_version = $3, \
         last_accepted_report_sequence = $4, last_accepted_report_fingerprint = $5, \
         open_mutation_permits = $6, updated_at = now() where adapter_id = $1 \
         and runtime_lease_version = $2 returning {RUNTIME_AUTHORITY_COLUMNS}"
    );
    let row = sqlx::query(&query)
        .bind(&input.adapter_id)
        .bind(authority.runtime_lease_version)
        .bind(next_version)
        .bind(input.report_sequence)
        .bind(input.report_fingerprint.as_str())
        .bind(open_permits)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_database_error)?
        .ok_or_else(|| stale(cas_namespaces::RUNTIME_LEASE_VERSION))?;
    Ok(RuntimeReportOutcome::Accepted(runtime_authority_from_row(
        &row,
    )?))
}
