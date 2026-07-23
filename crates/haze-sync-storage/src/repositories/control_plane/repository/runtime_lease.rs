pub async fn initialize_runtime_authority(
    transaction: &mut Transaction<'_, Postgres>,
    adapter_id: &str,
) -> ControlPlaneResult<RuntimeAuthorityRow> {
    validate_identifier(adapter_id)?;
    let query = format!(
        "insert into gdrive_runtime_authorities (adapter_id) values ($1) \
         on conflict (adapter_id) do nothing returning {RUNTIME_AUTHORITY_COLUMNS}"
    );
    if let Some(row) = sqlx::query(&query)
        .bind(adapter_id)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_database_error)?
    {
        return runtime_authority_from_row(&row);
    }
    lock_runtime_authority(transaction, adapter_id).await
}

pub async fn lock_runtime_authority(
    transaction: &mut Transaction<'_, Postgres>,
    adapter_id: &str,
) -> ControlPlaneResult<RuntimeAuthorityRow> {
    validate_identifier(adapter_id)?;
    let query = format!(
        "select {RUNTIME_AUTHORITY_COLUMNS} from gdrive_runtime_authorities \
         where adapter_id = $1 for update"
    );
    let row = sqlx::query(&query)
        .bind(adapter_id)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_database_error)?
        .ok_or(ControlPlaneRepositoryError::MissingRecord)?;
    runtime_authority_from_row(&row)
}

pub async fn cas_acquire_runtime_lease(
    transaction: &mut Transaction<'_, Postgres>,
    input: &RuntimeLeaseAcquisition,
) -> ControlPlaneResult<RuntimeAuthorityRow> {
    validate_identifier(&input.adapter_id)?;
    validate_identifier(&input.runtime_instance_id)?;
    let inventory_kind: Option<String> = sqlx::query_scalar(
        "select adapter_kind from adapter_inventory where adapter_id = $1 and retired_at is null",
    )
    .bind(&input.adapter_id)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_database_error)?;
    if inventory_kind.as_deref() != Some("gdrive") {
        return Err(ControlPlaneRepositoryError::MissingRecord);
    }
    let current = lock_runtime_authority(transaction, &input.adapter_id).await?;
    if current.runtime_lease_version != input.expected_runtime_lease_version {
        return Err(stale(cas_namespaces::RUNTIME_LEASE_VERSION));
    }
    let database_now: DateTime<Utc> = sqlx::query_scalar("select now()")
        .fetch_one(&mut **transaction)
        .await
        .map_err(map_database_error)?;
    if input.lease_heartbeat_at > input.lease_expires_at || input.lease_expires_at <= database_now {
        return Err(ControlPlaneRepositoryError::InvalidGeneration);
    }
    let replacing_owner = current.runtime_instance_id.is_some();
    if input.takeover && !replacing_owner {
        return Err(ControlPlaneRepositoryError::RuntimeLeaseConflict);
    }
    if replacing_owner && current.lease_expires_at.is_some_and(|expiry| expiry > database_now) {
        return Err(ControlPlaneRepositoryError::RuntimeLeaseConflict);
    }
    if replacing_owner && !input.takeover {
        return Err(ControlPlaneRepositoryError::RuntimeLeaseConflict);
    }
    if replacing_owner && input.takeover {
        sqlx::query(
            "insert into gdrive_uncertain_effects (effect_id, adapter_id, \
             standalone_runtime_epoch, permit_id, step_id, effect_identity_digest, state, \
             safe_metadata) select 'takeover-' || md5(adapter_id || ':' || \
             standalone_runtime_epoch::text || ':' || permit_id), adapter_id, \
             standalone_runtime_epoch, permit_id, step_id, permit_proof_digest, 'unresolved', \
             jsonb_build_object('source','expired_runtime_takeover') \
             from gdrive_mutation_permits where adapter_id = $1 and \
             standalone_runtime_epoch = $2 and state = 'open' \
             on conflict (effect_id) do nothing",
        )
        .bind(&input.adapter_id)
        .bind(current.standalone_runtime_epoch)
        .execute(&mut **transaction)
        .await
        .map_err(map_database_error)?;
        sqlx::query(
            "update gdrive_mutation_permits set state = 'uncertain', \
             terminal_outcome = 'lease_takeover', closed_at = now() where adapter_id = $1 \
             and standalone_runtime_epoch = $2 and state = 'open'",
        )
        .bind(&input.adapter_id)
        .bind(current.standalone_runtime_epoch)
        .execute(&mut **transaction)
        .await
        .map_err(map_database_error)?;
    }
    let unresolved_effects: i64 = sqlx::query_scalar(
        "select count(*)::bigint from gdrive_uncertain_effects \
         where adapter_id = $1 and state = 'unresolved'",
    )
    .bind(&input.adapter_id)
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_database_error)?;
    let next_version = advance(current.runtime_lease_version)?;
    let next_epoch = advance(current.standalone_runtime_epoch)?;
    let takeover_state = if input.takeover
        || replacing_owner
        || current.uncertain_external_effects != 0
    {
        "reconciliation_required"
    } else {
        "clear"
    };
    let query = format!(
        "update gdrive_runtime_authorities set runtime_lease_version = $3, \
         standalone_runtime_epoch = $4, runtime_instance_id = $5, lease_token_digest = $6, \
         lease_heartbeat_at = $7, lease_expires_at = $8, last_accepted_report_sequence = 0, \
         last_accepted_report_fingerprint = null, open_mutation_permits = 0, \
         uncertain_external_effects = $10, takeover_state = $9, updated_at = now() \
         where adapter_id = $1 and runtime_lease_version = $2 \
         returning {RUNTIME_AUTHORITY_COLUMNS}"
    );
    let row = sqlx::query(&query)
        .bind(&input.adapter_id)
        .bind(input.expected_runtime_lease_version)
        .bind(next_version)
        .bind(next_epoch)
        .bind(&input.runtime_instance_id)
        .bind(input.lease_token_digest.as_str())
        .bind(input.lease_heartbeat_at)
        .bind(input.lease_expires_at)
        .bind(takeover_state)
        .bind(unresolved_effects)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_database_error)?
        .ok_or_else(|| stale(cas_namespaces::RUNTIME_LEASE_VERSION))?;
    runtime_authority_from_row(&row)
}

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
        || current.lease_expires_at.is_none_or(|expiry| expiry <= database_now)
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
