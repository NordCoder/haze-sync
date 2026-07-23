pub async fn record_uncertain_external_effect(
    transaction: &mut Transaction<'_, Postgres>,
    input: &UncertainEffectInput,
) -> ControlPlaneResult<RuntimeAuthorityRow> {
    validate_identifier(&input.effect_id)?;
    validate_identifier(&input.adapter_id)?;
    validate_identifier(&input.runtime_instance_id)?;
    validate_safe_json(&input.safe_metadata)?;
    validate_identifier(&input.step_id)?;
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
    if authority.runtime_lease_version != input.expected_runtime_lease_version
        || authority.runtime_instance_id.as_deref() != Some(input.runtime_instance_id.as_str())
        || authority.lease_token_digest.as_ref() != Some(&input.lease_proof_digest)
    {
        return Err(stale(cas_namespaces::RUNTIME_LEASE_VERSION));
    }
    if let Some(effective) = &effective {
        if effective.runtime_instance_id.as_deref() != Some(input.runtime_instance_id.as_str())
            || effective.standalone_runtime_epoch != Some(input.standalone_runtime_epoch)
            || effective.runtime_lease_version != Some(input.expected_runtime_lease_version)
            || effective.report_sequence != Some(authority.last_accepted_report_sequence)
        {
            return Err(ControlPlaneRepositoryError::RuntimeReportConflict);
        }
    }
    let database_now: DateTime<Utc> = sqlx::query_scalar("select now()")
        .fetch_one(&mut **transaction)
        .await
        .map_err(map_database_error)?;
    if authority
        .lease_expires_at
        .map_or(true, |expiry| expiry <= database_now)
    {
        return Err(stale(cas_namespaces::RUNTIME_LEASE_VERSION));
    }
    if let Some(permit_id) = &input.permit_id {
        validate_identifier(permit_id)?;
        let result = sqlx::query(
            "update gdrive_mutation_permits set state = 'uncertain', terminal_outcome = 'uncertain', \
             closed_at = now() where permit_id = $1 and adapter_id = $2 \
             and standalone_runtime_epoch = $3 and state = 'open'",
        )
        .bind(permit_id)
        .bind(&input.adapter_id)
        .bind(input.standalone_runtime_epoch)
        .execute(&mut **transaction)
        .await
        .map_err(map_database_error)?;
        if result.rows_affected() != 1 {
            return Err(ControlPlaneRepositoryError::RuntimeReportConflict);
        }
    }
    sqlx::query(
        "insert into gdrive_uncertain_effects (effect_id, adapter_id, standalone_runtime_epoch, \
         permit_id, step_id, effect_identity_digest, state, safe_metadata) \
         values ($1,$2,$3,$4,$5,$6,'unresolved',$7)",
    )
    .bind(&input.effect_id)
    .bind(&input.adapter_id)
    .bind(input.standalone_runtime_epoch)
    .bind(&input.permit_id)
    .bind(&input.step_id)
    .bind(input.effect_identity_digest.as_str())
    .bind(&input.safe_metadata)
    .execute(&mut **transaction)
    .await
    .map_err(map_database_error)?;
    let open_permits: i64 = sqlx::query_scalar(
        "select count(*)::bigint from gdrive_mutation_permits where adapter_id = $1 \
         and standalone_runtime_epoch = $2 and state = 'open'",
    )
    .bind(&input.adapter_id)
    .bind(input.standalone_runtime_epoch)
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_database_error)?;
    let next = advance(authority.runtime_lease_version)?;
    let query = format!(
        "update gdrive_runtime_authorities set runtime_lease_version = $3, \
         open_mutation_permits = $4, uncertain_external_effects = uncertain_external_effects + 1, \
         takeover_state = 'reconciliation_required', updated_at = now() where adapter_id = $1 \
         and runtime_lease_version = $2 returning {RUNTIME_AUTHORITY_COLUMNS}"
    );
    let row = sqlx::query(&query)
        .bind(&input.adapter_id)
        .bind(authority.runtime_lease_version)
        .bind(next)
        .bind(open_permits)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_database_error)?
        .ok_or_else(|| stale(cas_namespaces::RUNTIME_LEASE_VERSION))?;
    if effective.is_some() {
        let updated = sqlx::query(
            "update adapter_effective_controls set runtime_lease_version = $5, \
             updated_at = now() where adapter_id = $1 and runtime_instance_id = $2 \
             and standalone_runtime_epoch = $3 and runtime_lease_version = $4",
        )
        .bind(&input.adapter_id)
        .bind(&input.runtime_instance_id)
        .bind(input.standalone_runtime_epoch)
        .bind(input.expected_runtime_lease_version)
        .bind(next)
        .execute(&mut **transaction)
        .await
        .map_err(map_database_error)?;
        if updated.rows_affected() != 1 {
            return Err(ControlPlaneRepositoryError::RuntimeReportConflict);
        }
    }
    runtime_authority_from_row(&row)
}
