/// Completes a Server-owned automatic maintenance transition without allocating
/// a new maintenance generation.
///
/// Transition legality remains a Server decision. Storage compares the exact
/// current generation and state, then replaces the row fields atomically while
/// preserving `maintenance_generation`.
pub async fn cas_complete_maintenance_transition(
    transaction: &mut Transaction<'_, Postgres>,
    expected_generation: i64,
    expected_state: &str,
    update: &MaintenanceControlUpdate,
) -> ControlPlaneResult<MaintenanceControlRow> {
    validate_non_negative(expected_generation)?;
    validate_non_negative(update.quiescence_evidence_version)?;
    validate_identifier(expected_state)?;
    validate_identifier(&update.state)?;
    let query = format!(
        "update maintenance_control set state = $3, transition_operation_id = $4, \
         transition_requested_by = $5, transition_requested_at = $6, state_entered_at = $7, \
         admission_fence_closed = $8, admission_fence_closed_at = $9, \
         quiescence_evidence_version = $10, quiescence_evidence_id = $11, \
         active_maintenance_job_id = $12, safe_error_category = $13, updated_at = now() \
         where singleton_id = 1 and maintenance_generation = $1 and state = $2 \
         returning {MAINTENANCE_COLUMNS}"
    );
    let row = sqlx::query(&query)
        .bind(expected_generation)
        .bind(expected_state)
        .bind(&update.state)
        .bind(&update.transition_operation_id)
        .bind(&update.transition_requested_by)
        .bind(update.transition_requested_at)
        .bind(update.state_entered_at)
        .bind(update.admission_fence_closed)
        .bind(update.admission_fence_closed_at)
        .bind(update.quiescence_evidence_version)
        .bind(&update.quiescence_evidence_id)
        .bind(&update.active_maintenance_job_id)
        .bind(&update.safe_error_category)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_database_error)?
        .ok_or_else(|| stale(cas_namespaces::MAINTENANCE_GENERATION))?;
    maintenance_from_row(&row)
}

/// Returns encoded verifier material only for the private Server authentication
/// boundary. Formatting of the row and material remains permanently redacted.
#[must_use]
pub fn credential_verifier_material_for_private_authentication(
    credential: &CredentialRow,
) -> &str {
    credential.verifier_material.as_str()
}

/// Reads one principal's complete credential set in stable credential-id order.
pub async fn read_credentials_by_principal(
    transaction: &mut Transaction<'_, Postgres>,
    principal_id: &str,
) -> ControlPlaneResult<Vec<CredentialRow>> {
    validate_identifier(principal_id)?;
    let query = format!(
        "select {CREDENTIAL_COLUMNS} from credentials where principal_id = $1 order by credential_id"
    );
    let rows = sqlx::query(&query)
        .bind(principal_id)
        .fetch_all(&mut **transaction)
        .await
        .map_err(map_database_error)?;
    rows.iter().map(credential_from_row).collect()
}

async fn update_rotation_credential(
    transaction: &mut Transaction<'_, Postgres>,
    principal_id: &str,
    mutation: &CredentialRotationMutation,
) -> ControlPlaneResult<CredentialRow> {
    validate_identifier(&mutation.credential_id)?;
    validate_identifier(&mutation.expected_status)?;
    validate_identifier(&mutation.update.status)?;
    if let Some(revoked_by) = &mutation.update.revoked_by {
        validate_identifier(revoked_by)?;
    }
    let next_version = advance(mutation.expected_credential_version)?;
    let query = format!(
        "update credentials set credential_version = $5, status = $6, grace_expires_at = $7, \
         revoked_at = $8, revoked_by = $9, expires_at = $10, updated_at = now() \
         where credential_id = $1 and principal_id = $2 and credential_version = $3 and status = $4 \
         returning {CREDENTIAL_COLUMNS}"
    );
    let row = sqlx::query(&query)
        .bind(&mutation.credential_id)
        .bind(principal_id)
        .bind(mutation.expected_credential_version)
        .bind(&mutation.expected_status)
        .bind(next_version)
        .bind(&mutation.update.status)
        .bind(mutation.update.grace_expires_at)
        .bind(mutation.update.revoked_at)
        .bind(&mutation.update.revoked_by)
        .bind(mutation.update.expires_at)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_database_error)?
        .ok_or_else(|| stale(cas_namespaces::CREDENTIAL_VERSION))?;
    credential_from_row(&row)
}

async fn insert_rotation_credential(
    transaction: &mut Transaction<'_, Postgres>,
    input: &NewCredential,
) -> ControlPlaneResult<CredentialRow> {
    for identifier in [
        input.credential_id.as_str(),
        input.principal_id.as_str(),
        input.issuance_operation_id.as_str(),
        input.credential_kind.as_str(),
        input.status.as_str(),
        input.verifier_scheme.as_str(),
    ] {
        validate_identifier(identifier)?;
    }
    if input.verifier_scheme == "legacy_sha256_v0" {
        return Err(ControlPlaneRepositoryError::LegacyVerifierCreationForbidden);
    }
    if input.status != "active" {
        return Err(ControlPlaneRepositoryError::InvalidIdentifier);
    }
    let query = format!(
        "insert into credentials (credential_id, credential_version, principal_id, \
         issuance_operation_id, credential_kind, status, verifier_scheme, verifier_material, \
         legacy_migrated, created_at, not_before, expires_at, grace_expires_at, \
         rotated_from_credential_id, updated_at) \
         values ($1,1,$2,$3,$4,$5,$6,$7,false,$8,$9,$10,$11,$12,$8) \
         returning {CREDENTIAL_COLUMNS}"
    );
    let row = sqlx::query(&query)
        .bind(&input.credential_id)
        .bind(&input.principal_id)
        .bind(&input.issuance_operation_id)
        .bind(&input.credential_kind)
        .bind(&input.status)
        .bind(&input.verifier_scheme)
        .bind(input.verifier_material.as_str())
        .bind(input.created_at)
        .bind(input.not_before)
        .bind(input.expires_at)
        .bind(input.grace_expires_at)
        .bind(&input.rotated_from_credential_id)
        .fetch_one(&mut **transaction)
        .await
        .map_err(map_database_error)?;
    credential_from_row(&row)
}

/// Applies all caller-selected credential rotation changes and advances the
/// principal credential-set generation exactly once.
pub async fn rotate_credential_set(
    transaction: &mut Transaction<'_, Postgres>,
    input: &CredentialSetRotationInput,
) -> ControlPlaneResult<CredentialSetRotationResult> {
    validate_non_negative(input.expected_principal_version)?;
    validate_non_negative(input.expected_credential_set_generation)?;
    if input.affected_credentials.is_empty()
        || input.new_credential.principal_id.is_empty()
        || input.new_credential.status != "active"
    {
        return Err(ControlPlaneRepositoryError::InvalidIdentifier);
    }

    let principal = credential_records::lock_principal(
        transaction,
        &input.new_credential.principal_id,
    )
    .await?;
    verify_principal_cas(
        &principal,
        input.expected_principal_version,
        input.expected_credential_set_generation,
    )?;

    let mut ordered = input.affected_credentials.iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| left.credential_id.cmp(&right.credential_id));
    if ordered
        .windows(2)
        .any(|pair| pair[0].credential_id == pair[1].credential_id)
    {
        return Err(ControlPlaneRepositoryError::InvalidIdentifier);
    }

    for mutation in &ordered {
        let current = credential_records::lock_credential(transaction, &mutation.credential_id).await?;
        if current.principal_id != principal.principal_id
            || current.credential_version != mutation.expected_credential_version
            || current.status != mutation.expected_status
        {
            return Err(stale(cas_namespaces::CREDENTIAL_VERSION));
        }
    }

    // Revoke/non-grace targets first so the partial unique grace index is free
    // before the former active record is moved into grace.
    let mut affected = Vec::with_capacity(ordered.len());
    for mutation in ordered
        .iter()
        .copied()
        .filter(|mutation| mutation.update.status != "grace")
    {
        affected.push(
            update_rotation_credential(transaction, &principal.principal_id, mutation).await?,
        );
    }
    for mutation in ordered
        .iter()
        .copied()
        .filter(|mutation| mutation.update.status == "grace")
    {
        affected.push(
            update_rotation_credential(transaction, &principal.principal_id, mutation).await?,
        );
    }

    let new_credential = insert_rotation_credential(transaction, &input.new_credential).await?;
    let next_principal_version = advance(input.expected_principal_version)?;
    let next_set_generation = advance(input.expected_credential_set_generation)?;
    let query = format!(
        "update principals set principal_version = $4, credential_set_generation = $5, \
         updated_at = now() where principal_id = $1 and principal_version = $2 \
         and credential_set_generation = $3 returning {PRINCIPAL_COLUMNS}"
    );
    let row = sqlx::query(&query)
        .bind(&principal.principal_id)
        .bind(input.expected_principal_version)
        .bind(input.expected_credential_set_generation)
        .bind(next_principal_version)
        .bind(next_set_generation)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_database_error)?
        .ok_or_else(|| stale(cas_namespaces::CREDENTIAL_SET_GENERATION))?;
    let principal = principal_from_row(&row)?;
    affected.sort_by(|left, right| left.credential_id.cmp(&right.credential_id));
    Ok(CredentialSetRotationResult {
        principal,
        new_credential,
        affected_credentials: affected,
    })
}
