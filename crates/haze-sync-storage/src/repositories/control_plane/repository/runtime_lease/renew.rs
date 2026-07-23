pub async fn cas_renew_runtime_lease(
    transaction: &mut Transaction<'_, Postgres>,
    input: &RuntimeLeaseRenewal,
) -> ControlPlaneResult<RuntimeAuthorityRow> {
    validate_identifier(&input.adapter_id)?;
    validate_identifier(&input.runtime_instance_id)?;
    validate_non_negative(input.expected_runtime_lease_version)?;
    validate_non_negative(input.standalone_runtime_epoch)?;
    let current = lock_runtime_authority(transaction, &input.adapter_id).await?;
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
    let database_now: DateTime<Utc> = sqlx::query_scalar("select now()")
        .fetch_one(&mut **transaction)
        .await
        .map_err(map_database_error)?;
    if current.runtime_lease_version != input.expected_runtime_lease_version
        || current.runtime_instance_id.as_deref() != Some(input.runtime_instance_id.as_str())
        || current.standalone_runtime_epoch != input.standalone_runtime_epoch
        || current.lease_token_digest.as_ref() != Some(&input.lease_proof_digest)
        || current
            .lease_expires_at
            .map_or(true, |expiry| expiry <= database_now)
    {
        return Err(stale(cas_namespaces::RUNTIME_LEASE_VERSION));
    }
    if let Some(effective) = &effective {
        if effective.runtime_instance_id.as_deref() != Some(input.runtime_instance_id.as_str())
            || effective.standalone_runtime_epoch != Some(input.standalone_runtime_epoch)
            || effective.runtime_lease_version != Some(input.expected_runtime_lease_version)
            || effective.report_sequence != Some(current.last_accepted_report_sequence)
        {
            return Err(ControlPlaneRepositoryError::RuntimeReportConflict);
        }
    }
    if input.heartbeat_at > input.expires_at || input.expires_at <= database_now {
        return Err(ControlPlaneRepositoryError::InvalidGeneration);
    }
    let next = advance(input.expected_runtime_lease_version)?;
    let query = format!(
        "update gdrive_runtime_authorities set runtime_lease_version = $6, \
         lease_heartbeat_at = $7, lease_expires_at = $8, updated_at = now() \
         where adapter_id = $1 and runtime_instance_id = $2 and runtime_lease_version = $3 \
         and standalone_runtime_epoch = $4 and lease_token_digest = $5 \
         returning {RUNTIME_AUTHORITY_COLUMNS}"
    );
    let row = sqlx::query(&query)
        .bind(&input.adapter_id)
        .bind(&input.runtime_instance_id)
        .bind(input.expected_runtime_lease_version)
        .bind(input.standalone_runtime_epoch)
        .bind(input.lease_proof_digest.as_str())
        .bind(next)
        .bind(input.heartbeat_at)
        .bind(input.expires_at)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_database_error)?
        .ok_or_else(|| stale(cas_namespaces::RUNTIME_LEASE_VERSION))?;
    if effective.is_some() {
        let updated = sqlx::query(
            "update adapter_effective_controls set runtime_lease_version = $5, \
             runtime_lease_expires_at = $6, updated_at = now() where adapter_id = $1 \
             and runtime_instance_id = $2 and standalone_runtime_epoch = $3 \
             and runtime_lease_version = $4",
        )
        .bind(&input.adapter_id)
        .bind(&input.runtime_instance_id)
        .bind(input.standalone_runtime_epoch)
        .bind(input.expected_runtime_lease_version)
        .bind(next)
        .bind(input.expires_at)
        .execute(&mut **transaction)
        .await
        .map_err(map_database_error)?;
        if updated.rows_affected() != 1 {
            return Err(ControlPlaneRepositoryError::RuntimeReportConflict);
        }
    }
    runtime_authority_from_row(&row)
}
