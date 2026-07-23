pub async fn open_runtime_mutation_permit(
    transaction: &mut Transaction<'_, Postgres>,
    input: &MutationPermitInput,
) -> ControlPlaneResult<RuntimeAuthorityRow> {
    validate_identifier(&input.permit_id)?;
    validate_identifier(&input.adapter_id)?;
    validate_identifier(&input.runtime_instance_id)?;
    validate_identifier(&input.step_id)?;
    validate_non_negative(input.standalone_runtime_epoch)?;
    validate_non_negative(input.expected_runtime_lease_version)?;
    validate_non_negative(input.adapter_control_generation)?;
    validate_non_negative(input.maintenance_generation)?;

    // Keep the shared control lock order stable: maintenance, desired, runtime authority,
    // then effective state. This allows quiescence capture and report acceptance to compose
    // with the same caller-owned transaction without inverse lock ordering.
    let maintenance = lock_maintenance_control(transaction).await?;
    let desired = lock_adapter_desired_control(transaction, &input.adapter_id).await?;
    let authority = lock_runtime_authority(transaction, &input.adapter_id).await?;
    let effective = lock_adapter_effective_control(transaction, &input.adapter_id).await?;

    if authority.standalone_runtime_epoch != input.standalone_runtime_epoch {
        return Err(ControlPlaneRepositoryError::StaleRuntimeEpoch);
    }
    if authority.runtime_lease_version != input.expected_runtime_lease_version
        || authority.runtime_instance_id.as_deref() != Some(input.runtime_instance_id.as_str())
        || authority.lease_token_digest.as_ref() != Some(&input.lease_proof_digest)
    {
        return Err(stale(cas_namespaces::RUNTIME_LEASE_VERSION));
    }
    let database_now: DateTime<Utc> = sqlx::query_scalar("select now()")
        .fetch_one(&mut **transaction)
        .await
        .map_err(map_database_error)?;
    let lease_expires_at = authority
        .lease_expires_at
        .ok_or_else(|| stale(cas_namespaces::RUNTIME_LEASE_VERSION))?;
    if lease_expires_at <= database_now {
        return Err(stale(cas_namespaces::RUNTIME_LEASE_VERSION));
    }
    if input.bounded_expires_at <= database_now || input.bounded_expires_at > lease_expires_at {
        return Err(ControlPlaneRepositoryError::InvalidGeneration);
    }
    if authority.takeover_state != "clear" || authority.uncertain_external_effects != 0 {
        return Err(ControlPlaneRepositoryError::OperationInProgress);
    }
    if desired.adapter_control_generation != input.adapter_control_generation
        || maintenance.maintenance_generation != input.maintenance_generation
    {
        return Err(stale(cas_namespaces::ADAPTER_CONTROL_GENERATION));
    }
    if effective.runtime_instance_id.as_deref() != Some(input.runtime_instance_id.as_str())
        || effective.standalone_runtime_epoch != Some(input.standalone_runtime_epoch)
        || effective.runtime_lease_version != Some(input.expected_runtime_lease_version)
        || effective.report_sequence != Some(authority.last_accepted_report_sequence)
        || effective.last_applied_adapter_control_generation
            != Some(input.adapter_control_generation)
        || effective.last_applied_maintenance_generation != Some(input.maintenance_generation)
    {
        return Err(ControlPlaneRepositoryError::RuntimeReportConflict);
    }

    sqlx::query(
        "insert into gdrive_mutation_permits (permit_id, adapter_id, runtime_instance_id, \
         standalone_runtime_epoch, runtime_lease_version, adapter_control_generation, \
         maintenance_generation, step_id, permit_proof_digest, bounded_expires_at, state) \
         values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,'open')",
    )
    .bind(&input.permit_id)
    .bind(&input.adapter_id)
    .bind(&input.runtime_instance_id)
    .bind(input.standalone_runtime_epoch)
    .bind(input.expected_runtime_lease_version)
    .bind(input.adapter_control_generation)
    .bind(input.maintenance_generation)
    .bind(&input.step_id)
    .bind(input.permit_proof_digest.as_str())
    .bind(input.bounded_expires_at)
    .execute(&mut **transaction)
    .await
    .map_err(map_database_error)?;

    let next = advance(authority.runtime_lease_version)?;
    let query = format!(
        "update gdrive_runtime_authorities set runtime_lease_version = $3, \
         open_mutation_permits = open_mutation_permits + 1, updated_at = now() \
         where adapter_id = $1 and runtime_lease_version = $2 returning {RUNTIME_AUTHORITY_COLUMNS}"
    );
    let row = sqlx::query(&query)
        .bind(&input.adapter_id)
        .bind(authority.runtime_lease_version)
        .bind(next)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_database_error)?
        .ok_or_else(|| stale(cas_namespaces::RUNTIME_LEASE_VERSION))?;
    let updated = sqlx::query(
        "update adapter_effective_controls set runtime_lifecycle = 'running', in_flight = true, \
         runtime_lease_version = $5, runtime_lease_expires_at = $6, updated_at = now() \
         where adapter_id = $1 and runtime_instance_id = $2 and standalone_runtime_epoch = $3 \
         and runtime_lease_version = $4",
    )
    .bind(&input.adapter_id)
    .bind(&input.runtime_instance_id)
    .bind(input.standalone_runtime_epoch)
    .bind(input.expected_runtime_lease_version)
    .bind(next)
    .bind(lease_expires_at)
    .execute(&mut **transaction)
    .await
    .map_err(map_database_error)?;
    if updated.rows_affected() != 1 {
        return Err(ControlPlaneRepositoryError::RuntimeReportConflict);
    }
    runtime_authority_from_row(&row)
}
