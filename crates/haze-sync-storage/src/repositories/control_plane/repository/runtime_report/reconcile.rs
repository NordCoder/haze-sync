pub async fn reconcile_uncertain_external_effect(
    transaction: &mut Transaction<'_, Postgres>,
    input: &UncertainEffectReconciliation,
) -> ControlPlaneResult<RuntimeAuthorityRow> {
    validate_identifier(&input.adapter_id)?;
    validate_identifier(&input.effect_id)?;
    if !matches!(input.resolution.as_str(), "reconciled" | "externally_fenced") {
        return Err(ControlPlaneRepositoryError::InvalidIdentifier);
    }
    if input.resolution == "externally_fenced" {
        let external_fence_id = input
            .external_fence_id
            .as_deref()
            .ok_or(ControlPlaneRepositoryError::InvalidIdentifier)?;
        validate_identifier(external_fence_id)?;
    } else if input.external_fence_id.is_some() {
        return Err(ControlPlaneRepositoryError::InvalidIdentifier);
    }

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
    if authority.runtime_lease_version != input.expected_runtime_lease_version {
        return Err(stale(cas_namespaces::RUNTIME_LEASE_VERSION));
    }
    let result = sqlx::query(
        "update gdrive_uncertain_effects set state = $3, external_fence_id = $4, resolved_at = now() \
         where adapter_id = $1 and effect_id = $2 and state = 'unresolved'",
    )
    .bind(&input.adapter_id)
    .bind(&input.effect_id)
    .bind(&input.resolution)
    .bind(&input.external_fence_id)
    .execute(&mut **transaction)
    .await
    .map_err(map_database_error)?;
    if result.rows_affected() != 1 {
        return Err(ControlPlaneRepositoryError::MissingRecord);
    }
    let remaining: i64 = sqlx::query_scalar(
        "select count(*)::bigint from gdrive_uncertain_effects \
         where adapter_id = $1 and state = 'unresolved'",
    )
    .bind(&input.adapter_id)
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_database_error)?;
    let takeover_state = if input.clear_takeover_when_empty && remaining == 0 {
        "clear"
    } else {
        authority.takeover_state.as_str()
    };
    let next = advance(authority.runtime_lease_version)?;
    let query = format!(
        "update gdrive_runtime_authorities set runtime_lease_version = $3, \
         uncertain_external_effects = $4, takeover_state = $5, updated_at = now() \
         where adapter_id = $1 and runtime_lease_version = $2 returning {RUNTIME_AUTHORITY_COLUMNS}"
    );
    let row = sqlx::query(&query)
        .bind(&input.adapter_id)
        .bind(input.expected_runtime_lease_version)
        .bind(next)
        .bind(remaining)
        .bind(takeover_state)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_database_error)?
        .ok_or_else(|| stale(cas_namespaces::RUNTIME_LEASE_VERSION))?;

    if let Some(effective) = effective {
        if effective.runtime_instance_id == authority.runtime_instance_id
            && effective.standalone_runtime_epoch == Some(authority.standalone_runtime_epoch)
            && effective.runtime_lease_version == Some(input.expected_runtime_lease_version)
            && effective.report_sequence == Some(authority.last_accepted_report_sequence)
        {
            let updated = sqlx::query(
                "update adapter_effective_controls set runtime_lease_version = $4, \
                 updated_at = now() where adapter_id = $1 and standalone_runtime_epoch = $2 \
                 and runtime_lease_version = $3",
            )
            .bind(&input.adapter_id)
            .bind(authority.standalone_runtime_epoch)
            .bind(input.expected_runtime_lease_version)
            .bind(next)
            .execute(&mut **transaction)
            .await
            .map_err(map_database_error)?;
            if updated.rows_affected() != 1 {
                return Err(ControlPlaneRepositoryError::RuntimeReportConflict);
            }
        }
    }
    runtime_authority_from_row(&row)
}
