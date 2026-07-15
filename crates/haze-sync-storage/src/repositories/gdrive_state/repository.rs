pub async fn initialize_gdrive_adapter_state(
    transaction: &mut Transaction<'_, Postgres>,
    adapter_id: &AdapterId,
) -> RepositoryResult<GDriveAdapterStateRow> {
    let row = sqlx::query(INITIALIZE_STATE_SQL)
        .bind(adapter_id.as_str())
        .bind(GDRIVE_STATE_FORMAT_VERSION)
        .fetch_one(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    gdrive_state_from_pg(&row)
}

pub async fn load_gdrive_state_snapshot(
    transaction: &mut Transaction<'_, Postgres>,
    adapter_id: &AdapterId,
    after_path: Option<&VaultPath>,
    limit: u32,
) -> RepositoryResult<Option<GDriveStateSnapshotPage>> {
    validate_limit(limit)?;
    let state_sql = format!(
        "select {STATE_COLUMNS} from gdrive_adapter_state where adapter_id = $1 for share"
    );
    let Some(state_row) = sqlx::query(&state_sql)
        .bind(adapter_id.as_str())
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?
    else {
        return Ok(None);
    };
    let state = gdrive_state_from_pg(&state_row)?;

    let item_sql = format!(
        "select {ITEM_COLUMNS} from gdrive_durable_items \
         where adapter_id = $1 and ($2::text is null or path > $2) \
         order by path asc limit $3"
    );
    let rows = sqlx::query(&item_sql)
        .bind(adapter_id.as_str())
        .bind(after_path.map(VaultPath::as_str))
        .bind(i64::from(limit) + 1)
        .fetch_all(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    let mut items = rows
        .iter()
        .map(gdrive_item_from_pg)
        .collect::<RepositoryResult<Vec<_>>>()?;
    let has_more = items.len() > limit as usize;
    if has_more {
        items.truncate(limit as usize);
    }
    let next_after_path = if has_more {
        items
            .last()
            .map(|item| VaultPath::parse(&item.path).map_err(|_| RepositoryError::InvalidPath))
            .transpose()?
    } else {
        None
    };

    Ok(Some(GDriveStateSnapshotPage {
        state,
        items,
        next_after_path,
    }))
}

pub async fn compare_and_commit_gdrive_state(
    transaction: &mut Transaction<'_, Postgres>,
    input: &GDriveStateCommit<'_>,
) -> RepositoryResult<GDriveCommitOutcome> {
    validate_commit_input(input)?;

    let locked = sqlx::query(LOCK_STATE_SQL)
        .bind(input.adapter_id.as_str())
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(RepositoryError::GDriveStateMissing)?;
    let current = gdrive_state_from_pg(&locked)?;

    if let Some(replay) = load_operation(
        transaction,
        input.adapter_id,
        input.operation.operation_id,
    )
    .await?
    {
        if replay.facts_hash == input.operation.facts_fingerprint.as_str()
            && replay.operation_kind == input.operation.kind.as_str()
        {
            return Ok(GDriveCommitOutcome::Replayed { operation: replay });
        }
        return Err(RepositoryError::GDriveOperationConflict);
    }

    if current.state_version != input.expected_state_version {
        return Err(RepositoryError::GDriveStateStaleExpected);
    }
    validate_progress_transition(&current, input)?;
    let next_state_version = current
        .state_version
        .checked_add(1)
        .ok_or(RepositoryError::StateVersionOverflow)?;

    if let Some(item) = input.item.as_ref() {
        upsert_item(transaction, input.adapter_id, item).await?;
    }

    let cursor = input
        .cursor_advance
        .as_ref()
        .map(|advance| advance.cursor.as_str());
    let cursor_generation = input
        .cursor_advance
        .as_ref()
        .map(|advance| advance.next_generation);
    let operation_id = input.operation.operation_id.as_str();
    let updated = sqlx::query(UPDATE_STATE_SQL)
        .bind(input.adapter_id.as_str())
        .bind(input.expected_state_version)
        .bind(next_state_version)
        .bind(cursor)
        .bind(cursor_generation)
        .bind(input.core_export_seq)
        .bind(input.operation.kind.as_str())
        .bind(operation_id)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(RepositoryError::GDriveStateStaleExpected)?;
    let state = gdrive_state_from_pg(&updated)?;

    let operation_row = sqlx::query(INSERT_OPERATION_SQL)
        .bind(input.adapter_id.as_str())
        .bind(operation_id)
        .bind(input.operation.kind.as_str())
        .bind(input.operation.facts_fingerprint.as_str())
        .bind(next_state_version)
        .bind(input.operation.mapping_path.map(VaultPath::as_str))
        .bind(input.operation.core_seq)
        .bind(input.operation.drive_version)
        .fetch_one(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    let operation = gdrive_operation_from_pg(&operation_row)?;

    Ok(GDriveCommitOutcome::Committed { state, operation })
}

async fn load_operation(
    transaction: &mut Transaction<'_, Postgres>,
    adapter_id: &AdapterId,
    operation_id: &OperationId,
) -> RepositoryResult<Option<GDriveOperationRow>> {
    let row = sqlx::query(SELECT_OPERATION_SQL)
        .bind(adapter_id.as_str())
        .bind(operation_id.as_str())
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    row.as_ref().map(gdrive_operation_from_pg).transpose()
}

async fn upsert_item(
    transaction: &mut Transaction<'_, Postgres>,
    adapter_id: &AdapterId,
    input: &GDriveItemUpsert<'_>,
) -> RepositoryResult<GDriveDurableItemRow> {
    let row = sqlx::query(UPSERT_ITEM_SQL)
        .bind(adapter_id.as_str())
        .bind(input.path.as_str())
        .bind(input.drive_file_id)
        .bind(input.drive_parent_id)
        .bind(input.drive_name)
        .bind(input.mime_type)
        .bind(input.md5_checksum)
        .bind(input.head_revision_id)
        .bind(input.drive_version)
        .bind(input.drive_modified_time)
        .bind(input.core_object_id)
        .bind(input.core_revision_id.map(RevisionId::as_str))
        .bind(input.core_seq)
        .bind(input.echo_state.as_str())
        .bind(input.echo_operation_id.map(OperationId::as_str))
        .bind(input.echo_provider_version)
        .bind(input.delete_candidate_first_seen_at)
        .bind(input.delete_candidate_last_seen_at)
        .bind(input.delete_candidate_generation)
        .bind(input.delete_candidate_blocked)
        .bind(input.delete_confirmation_audit_id)
        .bind(input.last_imported_at)
        .bind(input.last_exported_at)
        .bind(input.last_seen_at)
        .fetch_one(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    gdrive_item_from_pg(&row)
}
