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
