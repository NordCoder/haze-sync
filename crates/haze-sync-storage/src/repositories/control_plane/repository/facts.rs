pub async fn read_storage_facts(
    transaction: &mut Transaction<'_, Postgres>,
) -> ControlPlaneResult<StorageFacts> {
    let schema_name: String = sqlx::query_scalar("select current_schema()")
        .fetch_one(&mut **transaction)
        .await
        .map_err(map_database_error)?;
    let control_tables = crate::schema::control_plane::table_names::ALL
        .iter()
        .map(|name| (*name).to_owned())
        .collect::<Vec<_>>();
    let current_control_table_count: i64 = sqlx::query_scalar(
        "select count(*)::bigint from information_schema.tables where table_schema = current_schema() \
         and table_name = any($1)",
    )
    .bind(&control_tables)
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_database_error)?;
    let mut table_counts = Vec::new();
    for table in crate::schema::table_names::ALL {
        table_counts.push(((*table).to_owned(), scalar_count(transaction, table).await?));
    }
    let principal_count = scalar_count(transaction, "principals").await?;
    let credential_count = scalar_count(transaction, "credentials").await?;
    let operational_job_count = scalar_count(transaction, "operational_jobs").await?;
    let operational_audit_count = scalar_count(transaction, "operational_audit_events").await?;
    let maintenance_generation: i64 = sqlx::query_scalar(
        "select maintenance_generation from maintenance_control where singleton_id = 1",
    )
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_database_error)?;
    let adapter_inventory_generation: i64 = sqlx::query_scalar(
        "select adapter_inventory_generation from adapter_inventory_state where singleton_id = 1",
    )
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_database_error)?;
    let highest_operation_sequence: i64 =
        sqlx::query_scalar("select coalesce(max(seq), 0)::bigint from operation_log")
            .fetch_one(&mut **transaction)
            .await
            .map_err(map_database_error)?;
    let (object_blob_count, object_blob_bytes): (i64, i64) = sqlx::query_as(
        "select count(*)::bigint, coalesce(sum(size_bytes), 0)::bigint from content_blobs",
    )
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_database_error)?;
    let (worktree_file_count, worktree_file_bytes): (i64, i64) = sqlx::query_as(
        "select count(*)::bigint, coalesce(sum(observed_size_bytes), 0)::bigint \
         from worktree_state where state_kind = 'present'",
    )
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_database_error)?;
    Ok(StorageFacts {
        schema_name,
        migration_head: crate::schema::control_plane::CURRENT_MIGRATION_HEAD.to_owned(),
        current_control_table_count,
        table_counts,
        principal_count,
        credential_count,
        operational_job_count,
        operational_audit_count,
        maintenance_generation,
        adapter_inventory_generation,
        highest_operation_sequence,
        object_blob_count,
        object_blob_bytes,
        worktree_file_count,
        worktree_file_bytes,
        restore_excluded_table_count: crate::schema::control_plane::table_names::PRODUCT_RESTORE_EXCLUSIONS
            .len() as i64,
    })
}

async fn scalar_count(
    transaction: &mut Transaction<'_, Postgres>,
    table: &str,
) -> ControlPlaneResult<i64> {
    let query = format!("select count(*)::bigint from {table}");
    sqlx::query_scalar(&query)
        .fetch_one(&mut **transaction)
        .await
        .map_err(map_database_error)
}
