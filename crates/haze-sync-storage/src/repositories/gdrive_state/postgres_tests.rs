use super::*;
use crate::test_support::connect_required_test_database_from_env;
use chrono::TimeZone;

#[tokio::test]
#[ignore = "requires explicit HAZE_SYNC_TEST_DATABASE_URL for STOR-GDA-P1 durable-state evidence"]
async fn durable_gdrive_state_is_versioned_isolated_replay_safe_and_transactional() {
    let context = connect_required_test_database_from_env().await.unwrap();
    context.apply_migrations().await.unwrap();

    let namespace = context.namespace();
    let adapter_a = AdapterId::parse(&namespace.adapter_id("gdrive-a")).unwrap();
    let adapter_b = AdapterId::parse(&namespace.adapter_id("gdrive-b")).unwrap();
    let path_a = VaultPath::parse(&namespace.vault_path("a.md")).unwrap();
    let path_b = VaultPath::parse(&namespace.vault_path("b.md")).unwrap();
    let op_import = OperationId::parse(&namespace.operation_id("import-b")).unwrap();
    let op_export = OperationId::parse(&namespace.operation_id("export-a")).unwrap();
    let op_confirm = OperationId::parse(&namespace.operation_id("confirm-b")).unwrap();
    let op_rollback = OperationId::parse(&namespace.operation_id("rollback-a")).unwrap();
    let op_stale = OperationId::parse(&namespace.operation_id("stale")).unwrap();
    let cursor_1 = GDriveCursor::parse("opaque-cursor-generation-1").unwrap();
    let cursor_2 = GDriveCursor::parse("opaque-cursor-generation-2").unwrap();
    let fingerprint_1 = GDriveOperationFingerprint::parse("1".repeat(SHA256_HEX_LEN)).unwrap();
    let fingerprint_2 = GDriveOperationFingerprint::parse("2".repeat(SHA256_HEX_LEN)).unwrap();
    let fingerprint_3 = GDriveOperationFingerprint::parse("3".repeat(SHA256_HEX_LEN)).unwrap();
    let fingerprint_4 = GDriveOperationFingerprint::parse("4".repeat(SHA256_HEX_LEN)).unwrap();
    let conflicting_fingerprint =
        GDriveOperationFingerprint::parse("f".repeat(SHA256_HEX_LEN)).unwrap();
    let first_seen = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    let last_seen = Utc.timestamp_opt(1_700_000_100, 0).unwrap();

    let mut transaction = context.pool().begin().await.unwrap();
    for adapter in [&adapter_a, &adapter_b] {
        sqlx::query(
            "insert into sync_adapters (adapter_id, display_name, role, token_hash) \
             values ($1, $2, 'gdrive_adapter', 'synthetic-test-token-hash')",
        )
        .bind(adapter.as_str())
        .bind(format!("Test {}", adapter.as_str()))
        .execute(&mut *transaction)
        .await
        .unwrap();
        let state = initialize_gdrive_adapter_state(&mut *transaction, adapter)
            .await
            .unwrap();
        assert_eq!(state.state_version, 0);
        assert_eq!(state.drive_cursor_generation, 0);
        assert_eq!(state.core_export_seq, 0);
    }

    let empty = load_gdrive_state_snapshot(&mut transaction, &adapter_a, None, 10)
        .await
        .unwrap()
        .unwrap();
    assert!(empty.items.is_empty());
    assert_eq!(empty.state.state_version, 0);

    let import_commit = GDriveStateCommit {
        adapter_id: &adapter_a,
        expected_state_version: 0,
        cursor_advance: Some(GDriveCursorAdvance {
            cursor: &cursor_1,
            expected_generation: 0,
            next_generation: 1,
        }),
        core_export_seq: Some(5),
        item: Some(GDriveItemUpsert {
            path: &path_b,
            drive_file_id: Some("drive-file-b"),
            drive_parent_id: Some("drive-parent"),
            drive_name: Some("b.md"),
            mime_type: Some("text/markdown"),
            md5_checksum: Some("aabbccddeeff00112233445566778899"),
            head_revision_id: Some("head-b-1"),
            drive_version: Some("version-b-1"),
            drive_modified_time: Some(first_seen),
            core_object_id: None,
            core_revision_id: None,
            core_seq: Some(5),
            echo_state: GDriveEchoState::Pending,
            echo_operation_id: Some(&op_import),
            echo_provider_version: None,
            delete_candidate_first_seen_at: Some(first_seen),
            delete_candidate_last_seen_at: Some(last_seen),
            delete_candidate_generation: Some(1),
            delete_candidate_blocked: false,
            delete_confirmation_audit_id: None,
            last_imported_at: Some(last_seen),
            last_exported_at: None,
            last_seen_at: Some(last_seen),
        }),
        operation: GDriveOperationInput {
            operation_id: &op_import,
            kind: GDriveOperationKind::Import,
            facts_fingerprint: &fingerprint_1,
            mapping_path: Some(&path_b),
            core_seq: Some(5),
            drive_version: Some("version-b-1"),
        },
    };

    let committed = compare_and_commit_gdrive_state(&mut transaction, &import_commit)
        .await
        .unwrap();
    let GDriveCommitOutcome::Committed { state, operation } = committed else {
        panic!("first operation must commit");
    };
    assert_eq!(state.state_version, 1);
    assert_eq!(state.drive_cursor_generation, 1);
    assert_eq!(state.core_export_seq, 5);
    assert_eq!(operation.committed_state_version, 1);

    let replayed = compare_and_commit_gdrive_state(&mut transaction, &import_commit)
        .await
        .unwrap();
    let GDriveCommitOutcome::Replayed { operation } = replayed else {
        panic!("same operation and facts must replay");
    };
    assert_eq!(operation.committed_state_version, 1);

    let conflicting_replay = GDriveStateCommit {
        operation: GDriveOperationInput {
            facts_fingerprint: &conflicting_fingerprint,
            ..import_commit.operation.clone()
        },
        ..import_commit.clone()
    };
    assert_eq!(
        compare_and_commit_gdrive_state(&mut transaction, &conflicting_replay).await,
        Err(RepositoryError::GDriveOperationConflict)
    );

    let stale_commit = GDriveStateCommit {
        adapter_id: &adapter_a,
        expected_state_version: 0,
        cursor_advance: None,
        core_export_seq: Some(6),
        item: None,
        operation: GDriveOperationInput {
            operation_id: &op_stale,
            kind: GDriveOperationKind::Export,
            facts_fingerprint: &fingerprint_2,
            mapping_path: None,
            core_seq: Some(6),
            drive_version: None,
        },
    };
    assert_eq!(
        compare_and_commit_gdrive_state(&mut transaction, &stale_commit).await,
        Err(RepositoryError::GDriveStateStaleExpected)
    );

    for (expected_generation, next_generation, expected_error) in [
        (0, 1, RepositoryError::GDriveCursorGenerationMismatch),
        (1, 1, RepositoryError::CursorRegression),
        (1, 3, RepositoryError::CursorGap),
    ] {
        let cursor_failure = GDriveStateCommit {
            adapter_id: &adapter_a,
            expected_state_version: 1,
            cursor_advance: Some(GDriveCursorAdvance {
                cursor: &cursor_2,
                expected_generation,
                next_generation,
            }),
            core_export_seq: None,
            item: None,
            operation: GDriveOperationInput {
                operation_id: &op_stale,
                kind: GDriveOperationKind::CursorCheckpoint,
                facts_fingerprint: &fingerprint_2,
                mapping_path: None,
                core_seq: None,
                drive_version: None,
            },
        };
        assert_eq!(
            compare_and_commit_gdrive_state(&mut transaction, &cursor_failure).await,
            Err(expected_error)
        );
    }

    let checkpoint_regression = GDriveStateCommit {
        adapter_id: &adapter_a,
        expected_state_version: 1,
        cursor_advance: None,
        core_export_seq: Some(4),
        item: None,
        operation: GDriveOperationInput {
            operation_id: &op_stale,
            kind: GDriveOperationKind::Export,
            facts_fingerprint: &fingerprint_2,
            mapping_path: None,
            core_seq: Some(4),
            drive_version: None,
        },
    };
    assert_eq!(
        compare_and_commit_gdrive_state(&mut transaction, &checkpoint_regression).await,
        Err(RepositoryError::CheckpointRegression)
    );

    let export_commit = GDriveStateCommit {
        adapter_id: &adapter_a,
        expected_state_version: 1,
        cursor_advance: Some(GDriveCursorAdvance {
            cursor: &cursor_2,
            expected_generation: 1,
            next_generation: 2,
        }),
        core_export_seq: Some(6),
        item: Some(GDriveItemUpsert {
            path: &path_a,
            drive_file_id: Some("drive-file-a"),
            drive_parent_id: Some("drive-parent"),
            drive_name: Some("a.md"),
            mime_type: Some("text/markdown"),
            md5_checksum: None,
            head_revision_id: Some("head-a-1"),
            drive_version: Some("version-a-1"),
            drive_modified_time: Some(last_seen),
            core_object_id: None,
            core_revision_id: None,
            core_seq: Some(6),
            echo_state: GDriveEchoState::None,
            echo_operation_id: None,
            echo_provider_version: None,
            delete_candidate_first_seen_at: None,
            delete_candidate_last_seen_at: None,
            delete_candidate_generation: None,
            delete_candidate_blocked: false,
            delete_confirmation_audit_id: None,
            last_imported_at: None,
            last_exported_at: Some(last_seen),
            last_seen_at: Some(last_seen),
        }),
        operation: GDriveOperationInput {
            operation_id: &op_export,
            kind: GDriveOperationKind::Export,
            facts_fingerprint: &fingerprint_2,
            mapping_path: Some(&path_a),
            core_seq: Some(6),
            drive_version: Some("version-a-1"),
        },
    };
    let result = compare_and_commit_gdrive_state(&mut transaction, &export_commit)
        .await
        .unwrap();
    assert!(matches!(result, GDriveCommitOutcome::Committed { .. }));

    let bounded = load_gdrive_state_snapshot(&mut transaction, &adapter_a, None, 1)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(bounded.state.state_version, 2);
    assert_eq!(bounded.state.drive_cursor_generation, 2);
    assert_eq!(bounded.state.core_export_seq, 6);
    assert_eq!(bounded.items.len(), 1);
    assert_eq!(bounded.items[0].path, path_a.as_str());
    assert_eq!(bounded.next_after_path.as_ref(), Some(&path_a));

    let confirm_commit = GDriveStateCommit {
        adapter_id: &adapter_a,
        expected_state_version: 2,
        cursor_advance: None,
        core_export_seq: None,
        item: Some(GDriveItemUpsert {
            path: &path_b,
            drive_file_id: Some("drive-file-b"),
            drive_parent_id: Some("drive-parent"),
            drive_name: Some("b.md"),
            mime_type: Some("text/markdown"),
            md5_checksum: Some("aabbccddeeff00112233445566778899"),
            head_revision_id: Some("head-b-2"),
            drive_version: Some("version-b-2"),
            drive_modified_time: Some(last_seen),
            core_object_id: None,
            core_revision_id: None,
            core_seq: Some(6),
            echo_state: GDriveEchoState::Confirmed,
            echo_operation_id: Some(&op_import),
            echo_provider_version: Some("version-b-2"),
            delete_candidate_first_seen_at: Some(first_seen),
            delete_candidate_last_seen_at: Some(last_seen),
            delete_candidate_generation: Some(1),
            delete_candidate_blocked: true,
            delete_confirmation_audit_id: None,
            last_imported_at: Some(last_seen),
            last_exported_at: Some(last_seen),
            last_seen_at: Some(last_seen),
        }),
        operation: GDriveOperationInput {
            operation_id: &op_confirm,
            kind: GDriveOperationKind::ProviderMutation,
            facts_fingerprint: &fingerprint_3,
            mapping_path: Some(&path_b),
            core_seq: Some(6),
            drive_version: Some("version-b-2"),
        },
    };
    compare_and_commit_gdrive_state(&mut transaction, &confirm_commit)
        .await
        .unwrap();

    sqlx::query("savepoint stor_gda_rollback")
        .execute(&mut *transaction)
        .await
        .unwrap();
    let rollback_commit = GDriveStateCommit {
        adapter_id: &adapter_a,
        expected_state_version: 3,
        cursor_advance: None,
        core_export_seq: Some(7),
        item: Some(GDriveItemUpsert {
            path: &path_a,
            drive_file_id: Some("drive-file-a"),
            drive_parent_id: Some("drive-parent"),
            drive_name: Some("a.md"),
            mime_type: Some("text/markdown"),
            md5_checksum: None,
            head_revision_id: Some("head-a-2"),
            drive_version: Some("version-a-2"),
            drive_modified_time: Some(last_seen),
            core_object_id: None,
            core_revision_id: None,
            core_seq: Some(7),
            echo_state: GDriveEchoState::None,
            echo_operation_id: None,
            echo_provider_version: None,
            delete_candidate_first_seen_at: Some(first_seen),
            delete_candidate_last_seen_at: Some(last_seen),
            delete_candidate_generation: Some(2),
            delete_candidate_blocked: false,
            delete_confirmation_audit_id: None,
            last_imported_at: None,
            last_exported_at: Some(last_seen),
            last_seen_at: Some(last_seen),
        }),
        operation: GDriveOperationInput {
            operation_id: &op_rollback,
            kind: GDriveOperationKind::DeleteCandidate,
            facts_fingerprint: &fingerprint_4,
            mapping_path: Some(&path_a),
            core_seq: Some(7),
            drive_version: Some("version-a-2"),
        },
    };
    compare_and_commit_gdrive_state(&mut transaction, &rollback_commit)
        .await
        .unwrap();
    sqlx::query("rollback to savepoint stor_gda_rollback")
        .execute(&mut *transaction)
        .await
        .unwrap();

    let after_rollback = load_gdrive_state_snapshot(&mut transaction, &adapter_a, None, 10)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(after_rollback.state.state_version, 3);
    assert_eq!(after_rollback.state.core_export_seq, 6);
    let item_a = after_rollback
        .items
        .iter()
        .find(|item| item.path == path_a.as_str())
        .unwrap();
    assert!(item_a.delete_candidate_generation.is_none());

    let isolated = load_gdrive_state_snapshot(&mut transaction, &adapter_b, None, 10)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(isolated.state.state_version, 0);
    assert!(isolated.items.is_empty());

    transaction.rollback().await.unwrap();
}
