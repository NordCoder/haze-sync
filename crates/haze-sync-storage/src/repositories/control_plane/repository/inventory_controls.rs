pub async fn lock_adapter_inventory_state(
    transaction: &mut Transaction<'_, Postgres>,
) -> ControlPlaneResult<AdapterInventoryStateRow> {
    let row = sqlx::query(
        "select adapter_inventory_generation, updated_at from adapter_inventory_state \
         where singleton_id = 1 for update",
    )
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_database_error)?
    .ok_or(ControlPlaneRepositoryError::MissingRecord)?;
    Ok(AdapterInventoryStateRow {
        adapter_inventory_generation: row_column!(&row, "adapter_inventory_generation"),
        updated_at: row_column!(&row, "updated_at"),
    })
}

/// Returns one complete active inventory snapshot and the generation it belongs to.
pub async fn read_adapter_inventory_snapshot(
    transaction: &mut Transaction<'_, Postgres>,
) -> ControlPlaneResult<(AdapterInventoryStateRow, Vec<AdapterInventoryRow>)> {
    let state = lock_adapter_inventory_state(transaction).await?;
    let rows = sqlx::query(
        "select adapter_id, adapter_kind, control_authority, inventory_generation, \
         configured_at, retired_at from adapter_inventory where retired_at is null \
         order by adapter_id for share",
    )
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_database_error)?;
    let inventory = rows
        .iter()
        .map(|row| {
            Ok(AdapterInventoryRow {
                adapter_id: row_column!(row, "adapter_id"),
                adapter_kind: row_column!(row, "adapter_kind"),
                control_authority: row_column!(row, "control_authority"),
                inventory_generation: row_column!(row, "inventory_generation"),
                configured_at: row_column!(row, "configured_at"),
                retired_at: row_column!(row, "retired_at"),
            })
        })
        .collect::<ControlPlaneResult<Vec<_>>>()?;
    if inventory
        .iter()
        .any(|item| item.inventory_generation != state.adapter_inventory_generation)
    {
        return Err(ControlPlaneRepositoryError::DatabaseOperationFailed);
    }
    Ok((state, inventory))
}

/// Replaces the active inventory logically. Missing identities are retired, never hard-deleted.
pub async fn cas_replace_adapter_inventory(
    transaction: &mut Transaction<'_, Postgres>,
    expected_generation: i64,
    items: &[AdapterInventoryItem],
) -> ControlPlaneResult<(AdapterInventoryStateRow, Vec<AdapterInventoryRow>)> {
    let state = lock_adapter_inventory_state(transaction).await?;
    if state.adapter_inventory_generation != expected_generation {
        return Err(stale(cas_namespaces::ADAPTER_INVENTORY_GENERATION));
    }
    let next_generation = advance(expected_generation)?;
    let mut ids = Vec::with_capacity(items.len());
    for item in items {
        validate_identifier(&item.adapter_id)?;
        validate_identifier(&item.adapter_kind)?;
        validate_identifier(&item.control_authority)?;
        if ids.contains(&item.adapter_id) {
            return Err(ControlPlaneRepositoryError::InvalidIdentifier);
        }
        ids.push(item.adapter_id.clone());
    }

    sqlx::query(
        "update adapter_inventory set retired_at = coalesce(retired_at, now()), \
         inventory_generation = $1 where retired_at is null and not (adapter_id = any($2))",
    )
    .bind(next_generation)
    .bind(&ids)
    .execute(&mut **transaction)
    .await
    .map_err(map_database_error)?;

    for item in items {
        sqlx::query(
            "insert into adapter_inventory (adapter_id, adapter_kind, control_authority, \
             inventory_generation, configured_at, retired_at) values ($1, $2, $3, $4, $5, $6) \
             on conflict (adapter_id) do update set adapter_kind = excluded.adapter_kind, \
             control_authority = excluded.control_authority, inventory_generation = excluded.inventory_generation, \
             configured_at = excluded.configured_at, retired_at = excluded.retired_at",
        )
        .bind(&item.adapter_id)
        .bind(&item.adapter_kind)
        .bind(&item.control_authority)
        .bind(next_generation)
        .bind(item.configured_at)
        .bind(item.retired_at)
        .execute(&mut **transaction)
        .await
        .map_err(map_database_error)?;
    }

    let updated = sqlx::query(
        "update adapter_inventory_state set adapter_inventory_generation = $2, updated_at = now() \
         where singleton_id = 1 and adapter_inventory_generation = $1 \
         returning adapter_inventory_generation, updated_at",
    )
    .bind(expected_generation)
    .bind(next_generation)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_database_error)?
    .ok_or_else(|| stale(cas_namespaces::ADAPTER_INVENTORY_GENERATION))?;
    let updated_state = AdapterInventoryStateRow {
        adapter_inventory_generation: row_column!(&updated, "adapter_inventory_generation"),
        updated_at: row_column!(&updated, "updated_at"),
    };
    let (_, snapshot) = read_adapter_inventory_snapshot(transaction).await?;
    Ok((updated_state, snapshot))
}

pub async fn initialize_adapter_desired_control(
    transaction: &mut Transaction<'_, Postgres>,
    adapter_id: &str,
    update: &AdapterDesiredControlUpdate,
) -> ControlPlaneResult<AdapterDesiredControlRow> {
    validate_identifier(adapter_id)?;
    validate_non_negative(update.maintenance_generation)?;
    let query = format!(
        "insert into adapter_desired_controls (adapter_id, desired_enabled, desired_mode, \
         adapter_control_generation, maintenance_generation, maintenance_hold, updated_by) \
         values ($1, $2, $3, 0, $4, $5, $6) on conflict (adapter_id) do nothing \
         returning {DESIRED_COLUMNS}"
    );
    let inserted = sqlx::query(&query)
        .bind(adapter_id)
        .bind(update.desired_enabled)
        .bind(&update.desired_mode)
        .bind(update.maintenance_generation)
        .bind(update.maintenance_hold)
        .bind(&update.updated_by)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_database_error)?;
    if let Some(row) = inserted {
        return desired_from_row(&row);
    }
    lock_adapter_desired_control(transaction, adapter_id).await
}

pub async fn lock_adapter_desired_control(
    transaction: &mut Transaction<'_, Postgres>,
    adapter_id: &str,
) -> ControlPlaneResult<AdapterDesiredControlRow> {
    validate_identifier(adapter_id)?;
    let query = format!(
        "select {DESIRED_COLUMNS} from adapter_desired_controls where adapter_id = $1 for update"
    );
    let row = sqlx::query(&query)
        .bind(adapter_id)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_database_error)?
        .ok_or(ControlPlaneRepositoryError::MissingRecord)?;
    desired_from_row(&row)
}

pub async fn cas_update_adapter_desired_control(
    transaction: &mut Transaction<'_, Postgres>,
    adapter_id: &str,
    expected_generation: i64,
    update: &AdapterDesiredControlUpdate,
) -> ControlPlaneResult<AdapterDesiredControlRow> {
    validate_identifier(adapter_id)?;
    validate_non_negative(update.maintenance_generation)?;
    let next_generation = advance(expected_generation)?;
    let query = format!(
        "update adapter_desired_controls set desired_enabled = $3, desired_mode = $4, \
         adapter_control_generation = $2, maintenance_generation = $5, maintenance_hold = $6, \
         updated_by = $7, updated_at = now() where adapter_id = $1 and \
         adapter_control_generation = $8 returning {DESIRED_COLUMNS}"
    );
    let row = sqlx::query(&query)
        .bind(adapter_id)
        .bind(next_generation)
        .bind(update.desired_enabled)
        .bind(&update.desired_mode)
        .bind(update.maintenance_generation)
        .bind(update.maintenance_hold)
        .bind(&update.updated_by)
        .bind(expected_generation)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_database_error)?
        .ok_or_else(|| stale(cas_namespaces::ADAPTER_CONTROL_GENERATION))?;
    desired_from_row(&row)
}
