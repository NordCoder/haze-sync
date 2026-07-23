pub async fn insert_principal(
    transaction: &mut Transaction<'_, Postgres>,
    principal_id: &str,
    principal_kind: &str,
    role: &str,
    enabled: bool,
    created_at: DateTime<Utc>,
) -> ControlPlaneResult<PrincipalRow> {
    validate_identifier(principal_id)?;
    validate_identifier(principal_kind)?;
    validate_identifier(role)?;
    let query = format!(
        "insert into principals (principal_id, principal_kind, role, principal_enabled, \
         principal_version, credential_set_generation, created_at, updated_at, disabled_at) \
         values ($1,$2,$3,$4,1,1,$5,$5,$6) returning {PRINCIPAL_COLUMNS}"
    );
    let row = sqlx::query(&query)
        .bind(principal_id)
        .bind(principal_kind)
        .bind(role)
        .bind(enabled)
        .bind(created_at)
        .bind(if enabled { None } else { Some(created_at) })
        .fetch_one(&mut **transaction)
        .await
        .map_err(map_database_error)?;
    principal_from_row(&row)
}

pub async fn lock_principal(
    transaction: &mut Transaction<'_, Postgres>,
    principal_id: &str,
) -> ControlPlaneResult<PrincipalRow> {
    validate_identifier(principal_id)?;
    let query = format!(
        "select {PRINCIPAL_COLUMNS} from principals where principal_id = $1 for update"
    );
    let row = sqlx::query(&query)
        .bind(principal_id)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_database_error)?
        .ok_or(ControlPlaneRepositoryError::MissingRecord)?;
    principal_from_row(&row)
}

pub async fn cas_update_principal(
    transaction: &mut Transaction<'_, Postgres>,
    principal_id: &str,
    expected_principal_version: i64,
    expected_credential_set_generation: i64,
    update: &PrincipalUpdate,
) -> ControlPlaneResult<PrincipalRow> {
    validate_identifier(principal_id)?;
    validate_identifier(&update.role)?;
    let current = lock_principal(transaction, principal_id).await?;
    if current.principal_version != expected_principal_version
        || current.credential_set_generation != expected_credential_set_generation
    {
        return Err(stale(cas_namespaces::PRINCIPAL_VERSION));
    }
    let next_principal_version = advance(expected_principal_version)?;
    let must_advance_credential_set =
        current.role != update.role || update.advance_credential_set_generation;
    let next_set_generation = if must_advance_credential_set {
        advance(expected_credential_set_generation)?
    } else {
        expected_credential_set_generation
    };
    let query = format!(
        "update principals set role = $4, principal_enabled = $5, principal_version = $6, \
         credential_set_generation = $7, disabled_at = $8, updated_at = now() \
         where principal_id = $1 and principal_version = $2 and credential_set_generation = $3 \
         returning {PRINCIPAL_COLUMNS}"
    );
    let row = sqlx::query(&query)
        .bind(principal_id)
        .bind(expected_principal_version)
        .bind(expected_credential_set_generation)
        .bind(&update.role)
        .bind(update.principal_enabled)
        .bind(next_principal_version)
        .bind(next_set_generation)
        .bind(update.disabled_at)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_database_error)?
        .ok_or_else(|| stale(cas_namespaces::PRINCIPAL_VERSION))?;
    principal_from_row(&row)
}

async fn advance_principal_credential_set(
    transaction: &mut Transaction<'_, Postgres>,
    principal_id: &str,
    expected_principal_version: i64,
    expected_credential_set_generation: i64,
) -> ControlPlaneResult<PrincipalRow> {
    let next_principal_version = advance(expected_principal_version)?;
    let next_set_generation = advance(expected_credential_set_generation)?;
    let query = format!(
        "update principals set principal_version = $4, credential_set_generation = $5, \
         updated_at = now() where principal_id = $1 and principal_version = $2 \
         and credential_set_generation = $3 returning {PRINCIPAL_COLUMNS}"
    );
    let row = sqlx::query(&query)
        .bind(principal_id)
        .bind(expected_principal_version)
        .bind(expected_credential_set_generation)
        .bind(next_principal_version)
        .bind(next_set_generation)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_database_error)?
        .ok_or_else(|| stale(cas_namespaces::CREDENTIAL_SET_GENERATION))?;
    principal_from_row(&row)
}

pub async fn insert_credential(
    transaction: &mut Transaction<'_, Postgres>,
    input: &NewCredential,
    expected_principal_version: i64,
    expected_credential_set_generation: i64,
) -> ControlPlaneResult<CredentialMutationResult> {
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
    let principal = lock_principal(transaction, &input.principal_id).await?;
    if principal.principal_version != expected_principal_version
        || principal.credential_set_generation != expected_credential_set_generation
    {
        return Err(stale(cas_namespaces::CREDENTIAL_SET_GENERATION));
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
    let credential = credential_from_row(&row)?;
    let principal = advance_principal_credential_set(
        transaction,
        &input.principal_id,
        expected_principal_version,
        expected_credential_set_generation,
    )
    .await?;
    Ok(CredentialMutationResult {
        principal,
        credential,
    })
}

pub async fn read_credential_by_id(
    transaction: &mut Transaction<'_, Postgres>,
    credential_id: &str,
) -> ControlPlaneResult<Option<CredentialRow>> {
    validate_identifier(credential_id)?;
    let query = format!(
        "select {CREDENTIAL_COLUMNS} from credentials where credential_id = $1"
    );
    sqlx::query(&query)
        .bind(credential_id)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_database_error)?
        .as_ref()
        .map(credential_from_row)
        .transpose()
}

/// Private exact compatibility lookup. Callers must keep generic authentication failure behavior.
pub async fn read_legacy_credential_by_sha256(
    transaction: &mut Transaction<'_, Postgres>,
    legacy_sha256: &SecretDigest,
) -> ControlPlaneResult<Option<CredentialRow>> {
    let query = format!(
        "select {CREDENTIAL_COLUMNS} from credentials where verifier_scheme = 'legacy_sha256_v0' \
         and verifier_material = $1"
    );
    sqlx::query(&query)
        .bind(legacy_sha256.as_str())
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_database_error)?
        .as_ref()
        .map(credential_from_row)
        .transpose()
}

pub async fn lock_credential(
    transaction: &mut Transaction<'_, Postgres>,
    credential_id: &str,
) -> ControlPlaneResult<CredentialRow> {
    validate_identifier(credential_id)?;
    let query = format!(
        "select {CREDENTIAL_COLUMNS} from credentials where credential_id = $1 for update"
    );
    let row = sqlx::query(&query)
        .bind(credential_id)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_database_error)?
        .ok_or(ControlPlaneRepositoryError::MissingRecord)?;
    credential_from_row(&row)
}

pub async fn cas_update_credential_lifecycle(
    transaction: &mut Transaction<'_, Postgres>,
    credential_id: &str,
    expected_credential_version: i64,
    expected_principal_version: i64,
    expected_credential_set_generation: i64,
    update: &CredentialLifecycleUpdate,
) -> ControlPlaneResult<CredentialMutationResult> {
    validate_identifier(credential_id)?;
    validate_identifier(&update.status)?;
    if let Some(revoked_by) = &update.revoked_by {
        validate_identifier(revoked_by)?;
    }

    let observed = read_credential_by_id(transaction, credential_id)
        .await?
        .ok_or(ControlPlaneRepositoryError::MissingRecord)?;
    let principal = lock_principal(transaction, &observed.principal_id).await?;
    let current = lock_credential(transaction, credential_id).await?;
    if current.principal_id != observed.principal_id {
        return Err(ControlPlaneRepositoryError::DatabaseOperationFailed);
    }
    if current.credential_version != expected_credential_version {
        return Err(stale(cas_namespaces::CREDENTIAL_VERSION));
    }
    if principal.principal_version != expected_principal_version
        || principal.credential_set_generation != expected_credential_set_generation
    {
        return Err(stale(cas_namespaces::CREDENTIAL_SET_GENERATION));
    }
    let next_version = advance(expected_credential_version)?;
    let query = format!(
        "update credentials set credential_version = $3, status = $4, grace_expires_at = $5, \
         revoked_at = $6, revoked_by = $7, expires_at = $8, updated_at = now() \
         where credential_id = $1 and credential_version = $2 returning {CREDENTIAL_COLUMNS}"
    );
    let row = sqlx::query(&query)
        .bind(credential_id)
        .bind(expected_credential_version)
        .bind(next_version)
        .bind(&update.status)
        .bind(update.grace_expires_at)
        .bind(update.revoked_at)
        .bind(&update.revoked_by)
        .bind(update.expires_at)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_database_error)?
        .ok_or_else(|| stale(cas_namespaces::CREDENTIAL_VERSION))?;
    let credential = credential_from_row(&row)?;
    let principal = advance_principal_credential_set(
        transaction,
        &current.principal_id,
        expected_principal_version,
        expected_credential_set_generation,
    )
    .await?;
    Ok(CredentialMutationResult {
        principal,
        credential,
    })
}

fn issuance_from_row(row: &PgRow) -> ControlPlaneResult<CredentialIssuanceRow> {
    Ok(CredentialIssuanceRow {
        requester_principal_id: column!(row, "requester_principal_id"),
        idempotency_key_digest: required_digest(row, "idempotency_key_digest")?,
        request_fingerprint: required_digest(row, "request_fingerprint")?,
        issuance_operation_id: column!(row, "issuance_operation_id"),
        action: column!(row, "action"),
        target_principal_id: column!(row, "target_principal_id"),
        affected_credential_ids: column!(row, "affected_credential_ids"),
        committed_outcome: column!(row, "committed_outcome"),
        safe_result: column!(row, "safe_result"),
        committed_at: column!(row, "committed_at"),
    })
}

pub async fn insert_or_replay_credential_issuance(
    transaction: &mut Transaction<'_, Postgres>,
    input: &CredentialIssuanceInput,
) -> ControlPlaneResult<IdempotencyInsertOutcome<CredentialIssuanceRow>> {
    validate_identifier(&input.requester_principal_id)?;
    validate_identifier(&input.issuance_operation_id)?;
    validate_safe_json(&input.affected_credential_ids)?;
    validate_safe_json(&input.safe_result)?;
    validate_identifier(&input.target_principal_id)?;
    lock_idempotency_scope(
        transaction,
        "credential_issuance",
        &[&input.requester_principal_id, input.idempotency_key_digest.as_str()],
    )
    .await?;
    let existing = sqlx::query(
        "select requester_principal_id, idempotency_key_digest, request_fingerprint, \
         issuance_operation_id, action, target_principal_id, affected_credential_ids, \
         committed_outcome, safe_result, committed_at from credential_issuance_idempotency \
         where requester_principal_id = $1 and idempotency_key_digest = $2",
    )
    .bind(&input.requester_principal_id)
    .bind(input.idempotency_key_digest.as_str())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_database_error)?;
    if let Some(row) = existing {
        let existing = issuance_from_row(&row)?;
        if existing.request_fingerprint != input.request_fingerprint {
            return Err(ControlPlaneRepositoryError::IdempotencyConflict);
        }
        return Ok(IdempotencyInsertOutcome::Replay(existing));
    }

    let row = sqlx::query(
        "insert into credential_issuance_idempotency (requester_principal_id, \
         idempotency_key_digest, request_fingerprint, issuance_operation_id, action, \
         target_principal_id, affected_credential_ids, committed_outcome, safe_result, \
         plaintext_available) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,false) \
         returning requester_principal_id, idempotency_key_digest, request_fingerprint, \
         issuance_operation_id, action, target_principal_id, affected_credential_ids, \
         committed_outcome, safe_result, committed_at",
    )
    .bind(&input.requester_principal_id)
    .bind(input.idempotency_key_digest.as_str())
    .bind(input.request_fingerprint.as_str())
    .bind(&input.issuance_operation_id)
    .bind(&input.action)
    .bind(&input.target_principal_id)
    .bind(&input.affected_credential_ids)
    .bind(&input.committed_outcome)
    .bind(&input.safe_result)
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_database_error)?;
    Ok(IdempotencyInsertOutcome::Inserted(issuance_from_row(&row)?))
}
