async fn insert_or_replay_operational_job(
    transaction: &mut Transaction<'_, Postgres>,
    input: &OperationalJobInput,
) -> ControlPlaneResult<IdempotencyInsertOutcome<OperationalJobRow>> {
    job_records::insert_or_replay_operational_job(transaction, input).await
}

#[cfg(test)]
async fn insert_or_replay_credential_issuance(
    transaction: &mut Transaction<'_, Postgres>,
    input: &CredentialIssuanceInput,
) -> ControlPlaneResult<IdempotencyInsertOutcome<CredentialIssuanceRow>> {
    credential_records::insert_or_replay_credential_issuance(transaction, input).await
}

fn issuance_from_row(row: &PgRow) -> ControlPlaneResult<CredentialIssuanceRow> {
    Ok(CredentialIssuanceRow {
        requester_principal_id: row_column!(row, "requester_principal_id"),
        idempotency_key_digest: required_digest(row, "idempotency_key_digest")?,
        request_fingerprint: required_digest(row, "request_fingerprint")?,
        issuance_operation_id: row_column!(row, "issuance_operation_id"),
        action: row_column!(row, "action"),
        target_principal_id: row_column!(row, "target_principal_id"),
        affected_credential_ids: row_column!(row, "affected_credential_ids"),
        committed_outcome: row_column!(row, "committed_outcome"),
        safe_result: row_column!(row, "safe_result"),
        committed_at: row_column!(row, "committed_at"),
    })
}

/// Appends an immutable invalidation after taking a parent-row lock that
/// conflicts with complete evidence admission reads.
pub async fn append_quiescence_evidence_invalidation(
    transaction: &mut Transaction<'_, Postgres>,
    input: &QuiescenceEvidenceInvalidationInput,
) -> ControlPlaneResult<()> {
    validate_identifier(&input.invalidation_id)?;
    validate_identifier(&input.quiescence_evidence_id)?;
    validate_identifier(&input.invalidation_reason)?;
    validate_non_negative(input.maintenance_generation)?;

    sqlx::query_scalar::<_, String>(
        "select quiescence_evidence_id from quiescence_evidence \
         where quiescence_evidence_id = $1 for update",
    )
    .bind(&input.quiescence_evidence_id)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_database_error)?
    .ok_or(ControlPlaneRepositoryError::MissingRecord)?;

    sqlx::query(
        "insert into quiescence_evidence_invalidations (invalidation_id, \
         quiescence_evidence_id, maintenance_generation, invalidation_reason) \
         values ($1,$2,$3,$4)",
    )
    .bind(&input.invalidation_id)
    .bind(&input.quiescence_evidence_id)
    .bind(input.maintenance_generation)
    .bind(&input.invalidation_reason)
    .execute(&mut **transaction)
    .await
    .map_err(map_database_error)?;
    Ok(())
}
