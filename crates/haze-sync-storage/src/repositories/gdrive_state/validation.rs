fn validate_commit_input(input: &GDriveStateCommit<'_>) -> RepositoryResult<()> {
    validate_sequence(input.expected_state_version)?;
    if let Some(core_export_seq) = input.core_export_seq {
        validate_sequence(core_export_seq)?;
    }
    if let Some(core_seq) = input.operation.core_seq {
        validate_sequence(core_seq)?;
    }
    validate_optional_provider_identifier(input.operation.drive_version)?;
    if let Some(item) = input.item.as_ref() {
        validate_item_input(item)?;
    }
    Ok(())
}

fn validate_progress_transition(
    current: &GDriveAdapterStateRow,
    input: &GDriveStateCommit<'_>,
) -> RepositoryResult<()> {
    if let Some(advance) = input.cursor_advance.as_ref() {
        validate_sequence(advance.expected_generation)?;
        validate_sequence(advance.next_generation)?;
        if advance.expected_generation != current.drive_cursor_generation {
            return Err(RepositoryError::GDriveCursorGenerationMismatch);
        }
        let contiguous = advance
            .expected_generation
            .checked_add(1)
            .ok_or(RepositoryError::CursorOverflow)?;
        if advance.next_generation <= advance.expected_generation {
            return Err(RepositoryError::CursorRegression);
        }
        if advance.next_generation != contiguous {
            return Err(RepositoryError::CursorGap);
        }
    }
    if input
        .core_export_seq
        .is_some_and(|checkpoint| checkpoint < current.core_export_seq)
    {
        return Err(RepositoryError::CheckpointRegression);
    }
    Ok(())
}

fn validate_item_input(input: &GDriveItemUpsert<'_>) -> RepositoryResult<()> {
    validate_optional_provider_identifier(input.drive_file_id)?;
    validate_optional_provider_identifier(input.drive_parent_id)?;
    validate_optional_provider_text(input.drive_name)?;
    validate_optional_provider_text(input.mime_type)?;
    validate_optional_md5(input.md5_checksum)?;
    validate_optional_provider_identifier(input.head_revision_id)?;
    validate_optional_provider_identifier(input.drive_version)?;
    validate_optional_provider_identifier(input.core_object_id)?;
    validate_optional_provider_identifier(input.echo_provider_version)?;
    validate_optional_provider_identifier(input.delete_confirmation_audit_id)?;
    if let Some(core_seq) = input.core_seq {
        validate_sequence(core_seq)?;
    }

    match input.echo_state {
        GDriveEchoState::None => {
            if input.echo_operation_id.is_some() || input.echo_provider_version.is_some() {
                return Err(RepositoryError::InvalidProviderMetadata);
            }
        }
        GDriveEchoState::Pending => {
            if input.echo_operation_id.is_none() || input.echo_provider_version.is_some() {
                return Err(RepositoryError::InvalidProviderMetadata);
            }
        }
        GDriveEchoState::Confirmed => {
            if input.echo_operation_id.is_none() || input.echo_provider_version.is_none() {
                return Err(RepositoryError::InvalidProviderMetadata);
            }
        }
    }

    match (
        input.delete_candidate_first_seen_at,
        input.delete_candidate_last_seen_at,
        input.delete_candidate_generation,
    ) {
        (None, None, None) => {
            if input.delete_candidate_blocked || input.delete_confirmation_audit_id.is_some() {
                return Err(RepositoryError::InvalidProviderMetadata);
            }
        }
        (Some(first), Some(last), Some(generation)) => {
            validate_sequence(generation)?;
            if last < first {
                return Err(RepositoryError::InvalidProviderMetadata);
            }
        }
        _ => return Err(RepositoryError::InvalidProviderMetadata),
    }

    Ok(())
}

fn gdrive_state_from_pg(row: &PgRow) -> RepositoryResult<GDriveAdapterStateRow> {
    let adapter_id: String = row.try_get("adapter_id").map_err(map_sqlx_error)?;
    AdapterId::parse(&adapter_id).map_err(|_| RepositoryError::InvalidIdentifier)?;
    let state_format_version: i32 = row
        .try_get("state_format_version")
        .map_err(map_sqlx_error)?;
    if state_format_version != GDRIVE_STATE_FORMAT_VERSION {
        return Err(RepositoryError::UnsupportedGDriveStateVersion);
    }
    let state_version: i64 = row.try_get("state_version").map_err(map_sqlx_error)?;
    let drive_cursor_generation: i64 = row
        .try_get("drive_cursor_generation")
        .map_err(map_sqlx_error)?;
    let core_export_seq: i64 = row.try_get("core_export_seq").map_err(map_sqlx_error)?;
    validate_sequence(state_version)?;
    validate_sequence(drive_cursor_generation)?;
    validate_sequence(core_export_seq)?;
    let drive_cursor: Option<String> = row.try_get("drive_cursor").map_err(map_sqlx_error)?;
    if let Some(cursor) = drive_cursor.as_deref() {
        GDriveCursor::parse(cursor.to_owned())?;
    }
    let last_import_operation_id: Option<String> = row
        .try_get("last_import_operation_id")
        .map_err(map_sqlx_error)?;
    let last_export_operation_id: Option<String> = row
        .try_get("last_export_operation_id")
        .map_err(map_sqlx_error)?;
    let last_provider_mutation_operation_id: Option<String> = row
        .try_get("last_provider_mutation_operation_id")
        .map_err(map_sqlx_error)?;
    for operation_id in [
        last_import_operation_id.as_deref(),
        last_export_operation_id.as_deref(),
        last_provider_mutation_operation_id.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        OperationId::parse(operation_id).map_err(|_| RepositoryError::InvalidIdentifier)?;
    }

    Ok(GDriveAdapterStateRow {
        adapter_id,
        state_format_version,
        state_version,
        drive_cursor,
        drive_cursor_generation,
        core_export_seq,
        last_import_operation_id,
        last_export_operation_id,
        last_provider_mutation_operation_id,
        created_at: row.try_get("created_at").map_err(map_sqlx_error)?,
        updated_at: row.try_get("updated_at").map_err(map_sqlx_error)?,
    })
}

fn gdrive_item_from_pg(row: &PgRow) -> RepositoryResult<GDriveDurableItemRow> {
    let mapped = GDriveDurableItemRow {
        adapter_id: row.try_get("adapter_id").map_err(map_sqlx_error)?,
        path: row.try_get("path").map_err(map_sqlx_error)?,
        drive_file_id: row.try_get("drive_file_id").map_err(map_sqlx_error)?,
        drive_parent_id: row.try_get("drive_parent_id").map_err(map_sqlx_error)?,
        drive_name: row.try_get("drive_name").map_err(map_sqlx_error)?,
        mime_type: row.try_get("mime_type").map_err(map_sqlx_error)?,
        md5_checksum: row.try_get("md5_checksum").map_err(map_sqlx_error)?,
        head_revision_id: row.try_get("head_revision_id").map_err(map_sqlx_error)?,
        drive_version: row.try_get("drive_version").map_err(map_sqlx_error)?,
        drive_modified_time: row.try_get("drive_modified_time").map_err(map_sqlx_error)?,
        core_object_id: row.try_get("core_object_id").map_err(map_sqlx_error)?,
        core_revision_id: row.try_get("core_revision_id").map_err(map_sqlx_error)?,
        core_seq: row.try_get("core_seq").map_err(map_sqlx_error)?,
        echo_state: row.try_get("echo_state").map_err(map_sqlx_error)?,
        echo_operation_id: row.try_get("echo_operation_id").map_err(map_sqlx_error)?,
        echo_provider_version: row.try_get("echo_provider_version").map_err(map_sqlx_error)?,
        delete_candidate_first_seen_at: row
            .try_get("delete_candidate_first_seen_at")
            .map_err(map_sqlx_error)?,
        delete_candidate_last_seen_at: row
            .try_get("delete_candidate_last_seen_at")
            .map_err(map_sqlx_error)?,
        delete_candidate_generation: row
            .try_get("delete_candidate_generation")
            .map_err(map_sqlx_error)?,
        delete_candidate_blocked: row
            .try_get("delete_candidate_blocked")
            .map_err(map_sqlx_error)?,
        delete_confirmation_audit_id: row
            .try_get("delete_confirmation_audit_id")
            .map_err(map_sqlx_error)?,
        last_imported_at: row.try_get("last_imported_at").map_err(map_sqlx_error)?,
        last_exported_at: row.try_get("last_exported_at").map_err(map_sqlx_error)?,
        last_seen_at: row.try_get("last_seen_at").map_err(map_sqlx_error)?,
        created_at: row.try_get("created_at").map_err(map_sqlx_error)?,
        updated_at: row.try_get("updated_at").map_err(map_sqlx_error)?,
    };
    validate_item_row(mapped)
}

fn validate_item_row(row: GDriveDurableItemRow) -> RepositoryResult<GDriveDurableItemRow> {
    AdapterId::parse(&row.adapter_id).map_err(|_| RepositoryError::InvalidIdentifier)?;
    let path = VaultPath::parse(&row.path).map_err(|_| RepositoryError::InvalidPath)?;
    if path.as_str() != row.path {
        return Err(RepositoryError::InvalidPath);
    }
    validate_optional_provider_identifier(row.drive_file_id.as_deref())?;
    validate_optional_provider_identifier(row.drive_parent_id.as_deref())?;
    validate_optional_provider_text(row.drive_name.as_deref())?;
    validate_optional_provider_text(row.mime_type.as_deref())?;
    validate_optional_md5(row.md5_checksum.as_deref())?;
    validate_optional_provider_identifier(row.head_revision_id.as_deref())?;
    validate_optional_provider_identifier(row.drive_version.as_deref())?;
    validate_optional_provider_identifier(row.core_object_id.as_deref())?;
    validate_optional_provider_identifier(row.echo_provider_version.as_deref())?;
    validate_optional_provider_identifier(row.delete_confirmation_audit_id.as_deref())?;
    if let Some(revision_id) = row.core_revision_id.as_deref() {
        RevisionId::parse(revision_id).map_err(|_| RepositoryError::InvalidIdentifier)?;
    }
    if let Some(core_seq) = row.core_seq {
        validate_sequence(core_seq)?;
    }
    GDriveEchoState::parse(&row.echo_state)?;
    if let Some(operation_id) = row.echo_operation_id.as_deref() {
        OperationId::parse(operation_id).map_err(|_| RepositoryError::InvalidIdentifier)?;
    }
    if let Some(generation) = row.delete_candidate_generation {
        validate_sequence(generation)?;
    }
    Ok(row)
}

fn gdrive_operation_from_pg(row: &PgRow) -> RepositoryResult<GDriveOperationRow> {
    let mapped = GDriveOperationRow {
        adapter_id: row.try_get("adapter_id").map_err(map_sqlx_error)?,
        operation_id: row.try_get("operation_id").map_err(map_sqlx_error)?,
        operation_kind: row.try_get("operation_kind").map_err(map_sqlx_error)?,
        facts_hash: row.try_get("facts_hash").map_err(map_sqlx_error)?,
        outcome_kind: row.try_get("outcome_kind").map_err(map_sqlx_error)?,
        committed_state_version: row
            .try_get("committed_state_version")
            .map_err(map_sqlx_error)?,
        mapping_path: row.try_get("mapping_path").map_err(map_sqlx_error)?,
        core_seq: row.try_get("core_seq").map_err(map_sqlx_error)?,
        drive_version: row.try_get("drive_version").map_err(map_sqlx_error)?,
        created_at: row.try_get("created_at").map_err(map_sqlx_error)?,
    };
    AdapterId::parse(&mapped.adapter_id).map_err(|_| RepositoryError::InvalidIdentifier)?;
    OperationId::parse(&mapped.operation_id).map_err(|_| RepositoryError::InvalidIdentifier)?;
    GDriveOperationKind::parse(&mapped.operation_kind)?;
    GDriveOperationFingerprint::parse(mapped.facts_hash.clone())?;
    if mapped.outcome_kind != "committed" {
        return Err(RepositoryError::InvalidProviderMetadata);
    }
    validate_sequence(mapped.committed_state_version)?;
    if let Some(path) = mapped.mapping_path.as_deref() {
        VaultPath::parse(path).map_err(|_| RepositoryError::InvalidPath)?;
    }
    if let Some(core_seq) = mapped.core_seq {
        validate_sequence(core_seq)?;
    }
    validate_optional_provider_identifier(mapped.drive_version.as_deref())?;
    Ok(mapped)
}

fn validate_optional_provider_identifier(value: Option<&str>) -> RepositoryResult<()> {
    value.map_or(Ok(()), validate_provider_identifier)
}

fn validate_provider_identifier(value: &str) -> RepositoryResult<()> {
    if value.is_empty()
        || value.len() > MAX_IDENTIFIER_LEN
        || value
            .chars()
            .any(|character| character.is_control() || character.is_whitespace())
    {
        return Err(RepositoryError::InvalidIdentifier);
    }
    Ok(())
}

fn validate_optional_provider_text(value: Option<&str>) -> RepositoryResult<()> {
    let Some(value) = value else {
        return Ok(());
    };
    if value.is_empty()
        || value.len() > MAX_PROVIDER_TEXT_LEN
        || value.chars().any(char::is_control)
    {
        return Err(RepositoryError::InvalidProviderMetadata);
    }
    Ok(())
}

fn validate_optional_md5(value: Option<&str>) -> RepositoryResult<()> {
    let Some(value) = value else {
        return Ok(());
    };
    if value.len() != MD5_HEX_LEN || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(RepositoryError::InvalidHash);
    }
    Ok(())
}
