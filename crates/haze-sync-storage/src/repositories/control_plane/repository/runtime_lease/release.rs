pub async fn cas_release_runtime_lease(
    transaction: &mut Transaction<'_, Postgres>,
    adapter_id: &str,
    runtime_instance_id: &str,
    expected_runtime_lease_version: i64,
    standalone_runtime_epoch: i64,
    lease_proof_digest: &SecretDigest,
) -> ControlPlaneResult<RuntimeAuthorityRow> {
    validate_identifier(adapter_id)?;
    validate_identifier(runtime_instance_id)?;
    let current = lock_runtime_authority(transaction, adapter_id).await?;
    if current.runtime_lease_version != expected_runtime_lease_version
        || current.runtime_instance_id.as_deref() != Some(runtime_instance_id)
        || current.standalone_runtime_epoch != standalone_runtime_epoch
        || current.lease_token_digest.as_ref() != Some(lease_proof_digest)
    {
        return Err(stale(cas_namespaces::RUNTIME_LEASE_VERSION));
    }
    if current.open_mutation_permits != 0
        || current.uncertain_external_effects != 0
        || current.takeover_state != "clear"
    {
        return Err(ControlPlaneRepositoryError::OperationInProgress);
    }
    let effective_query = format!(
        "select {EFFECTIVE_COLUMNS} from adapter_effective_controls where adapter_id = $1 for share"
    );
    let effective = sqlx::query(&effective_query)
        .bind(adapter_id)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_database_error)?
        .as_ref()
        .map(effective_from_row)
        .transpose()?
        .ok_or(ControlPlaneRepositoryError::OperationInProgress)?;
    if effective.runtime_instance_id.as_deref() != Some(runtime_instance_id)
        || effective.standalone_runtime_epoch != Some(standalone_runtime_epoch)
        || effective.report_sequence != Some(current.last_accepted_report_sequence)
        || effective.runtime_lease_version != Some(current.runtime_lease_version)
        || effective.in_flight
    {
        return Err(ControlPlaneRepositoryError::OperationInProgress);
    }
    let next = advance(expected_runtime_lease_version)?;
    let query = format!(
        "update gdrive_runtime_authorities set runtime_lease_version = $6, runtime_instance_id = null, \
         lease_token_digest = null, lease_heartbeat_at = null, lease_expires_at = null, \
         updated_at = now() where adapter_id = $1 and runtime_instance_id = $2 \
         and runtime_lease_version = $3 and standalone_runtime_epoch = $4 and lease_token_digest = $5 \
         returning {RUNTIME_AUTHORITY_COLUMNS}"
    );
    let row = sqlx::query(&query)
        .bind(adapter_id)
        .bind(runtime_instance_id)
        .bind(expected_runtime_lease_version)
        .bind(standalone_runtime_epoch)
        .bind(lease_proof_digest.as_str())
        .bind(next)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_database_error)?
        .ok_or_else(|| stale(cas_namespaces::RUNTIME_LEASE_VERSION))?;
    runtime_authority_from_row(&row)
}
