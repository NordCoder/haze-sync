use super::*;
use chrono::TimeZone;

#[test]
fn opaque_cursor_and_operation_fingerprint_are_redacted() {
    let cursor = GDriveCursor::parse("opaque-page-token-secret").unwrap();
    let fingerprint = GDriveOperationFingerprint::parse("a".repeat(SHA256_HEX_LEN)).unwrap();

    for rendered in [
        format!("{cursor:?}"),
        cursor.to_string(),
        format!("{fingerprint:?}"),
        fingerprint.to_string(),
    ] {
        assert!(rendered.contains("[REDACTED]"));
        assert!(!rendered.contains("opaque-page-token-secret"));
        assert!(!rendered.contains(&"a".repeat(SHA256_HEX_LEN)));
        assert!(!rendered.contains("postgres://"));
        assert!(!rendered.contains("/srv/"));
    }
}

#[test]
fn cursor_generation_requires_exact_contiguous_transition() {
    let current = state_row(4, 9, 11);
    let adapter_id = AdapterId::parse("gdrive-test").unwrap();
    let operation_id = OperationId::parse("op_gdrive-1").unwrap();
    let fingerprint = GDriveOperationFingerprint::parse("b".repeat(SHA256_HEX_LEN)).unwrap();
    let cursor = GDriveCursor::parse("cursor-next").unwrap();

    let commit = |expected_generation, next_generation| GDriveStateCommit {
        adapter_id: &adapter_id,
        expected_state_version: 4,
        cursor_advance: Some(GDriveCursorAdvance {
            cursor: &cursor,
            expected_generation,
            next_generation,
        }),
        core_export_seq: None,
        item: None,
        operation: GDriveOperationInput {
            operation_id: &operation_id,
            kind: GDriveOperationKind::CursorCheckpoint,
            facts_fingerprint: &fingerprint,
            mapping_path: None,
            core_seq: None,
            drive_version: None,
        },
    };

    assert_eq!(
        validate_progress_transition(&current, &commit(8, 9)),
        Err(RepositoryError::GDriveCursorGenerationMismatch)
    );
    assert_eq!(
        validate_progress_transition(&current, &commit(9, 9)),
        Err(RepositoryError::CursorRegression)
    );
    assert_eq!(
        validate_progress_transition(&current, &commit(9, 11)),
        Err(RepositoryError::CursorGap)
    );
    assert_eq!(
        validate_progress_transition(&current, &commit(9, 10)),
        Ok(())
    );
}

#[test]
fn export_checkpoint_cannot_regress() {
    let current = state_row(2, 0, 42);
    let adapter_id = AdapterId::parse("gdrive-test").unwrap();
    let operation_id = OperationId::parse("op_gdrive-2").unwrap();
    let fingerprint = GDriveOperationFingerprint::parse("c".repeat(SHA256_HEX_LEN)).unwrap();
    let commit = GDriveStateCommit {
        adapter_id: &adapter_id,
        expected_state_version: 2,
        cursor_advance: None,
        core_export_seq: Some(41),
        item: None,
        operation: GDriveOperationInput {
            operation_id: &operation_id,
            kind: GDriveOperationKind::Export,
            facts_fingerprint: &fingerprint,
            mapping_path: None,
            core_seq: Some(41),
            drive_version: None,
        },
    };

    assert_eq!(
        validate_progress_transition(&current, &commit),
        Err(RepositoryError::CheckpointRegression)
    );
}

#[test]
fn echo_and_delete_candidate_shapes_fail_closed() {
    let path = VaultPath::parse("Notes/a.md").unwrap();
    let operation_id = OperationId::parse("op_gdrive-3").unwrap();
    let first = Utc.timestamp_opt(1_700_000_100, 0).unwrap();
    let last = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    let item = GDriveItemUpsert {
        path: &path,
        drive_file_id: Some("drive-file-1"),
        drive_parent_id: None,
        drive_name: Some("a.md"),
        mime_type: Some("text/markdown"),
        md5_checksum: None,
        head_revision_id: None,
        drive_version: None,
        drive_modified_time: None,
        core_object_id: None,
        core_revision_id: None,
        core_seq: None,
        echo_state: GDriveEchoState::Confirmed,
        echo_operation_id: Some(&operation_id),
        echo_provider_version: None,
        delete_candidate_first_seen_at: Some(first),
        delete_candidate_last_seen_at: Some(last),
        delete_candidate_generation: Some(1),
        delete_candidate_blocked: false,
        delete_confirmation_audit_id: None,
        last_imported_at: None,
        last_exported_at: None,
        last_seen_at: None,
    };

    assert_eq!(
        validate_item_input(&item),
        Err(RepositoryError::InvalidProviderMetadata)
    );
}

#[test]
fn internal_rows_redact_cursor_provider_and_operation_facts() {
    let time = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    let state = GDriveAdapterStateRow {
        adapter_id: "gdrive-test".to_owned(),
        state_format_version: 1,
        state_version: 1,
        drive_cursor: Some("raw-cursor-secret".to_owned()),
        drive_cursor_generation: 1,
        core_export_seq: 1,
        last_import_operation_id: Some("raw-operation-secret".to_owned()),
        last_export_operation_id: None,
        last_provider_mutation_operation_id: None,
        created_at: time,
        updated_at: time,
    };
    let operation = GDriveOperationRow {
        adapter_id: "gdrive-test".to_owned(),
        operation_id: "raw-operation-secret".to_owned(),
        operation_kind: "import".to_owned(),
        facts_hash: "d".repeat(SHA256_HEX_LEN),
        outcome_kind: "committed".to_owned(),
        committed_state_version: 1,
        mapping_path: Some("Notes/a.md".to_owned()),
        core_seq: Some(1),
        drive_version: Some("provider-version-secret".to_owned()),
        created_at: time,
    };

    let rendered = format!("{state:?} {operation:?}");
    let facts_hash = "d".repeat(SHA256_HEX_LEN);
    for secret in [
        "raw-cursor-secret",
        "raw-operation-secret",
        "provider-version-secret",
        facts_hash.as_str(),
    ] {
        assert!(!rendered.contains(secret));
    }
    assert!(rendered.contains("[REDACTED]"));
}

fn state_row(state_version: i64, cursor_generation: i64, checkpoint: i64) -> GDriveAdapterStateRow {
    let time = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    GDriveAdapterStateRow {
        adapter_id: "gdrive-test".to_owned(),
        state_format_version: GDRIVE_STATE_FORMAT_VERSION,
        state_version,
        drive_cursor: None,
        drive_cursor_generation: cursor_generation,
        core_export_seq: checkpoint,
        last_import_operation_id: None,
        last_export_operation_id: None,
        last_provider_mutation_operation_id: None,
        created_at: time,
        updated_at: time,
    }
}
