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
