use super::*;
use crate::test_support::connect_required_test_database_from_env;
use chrono::TimeZone;

#[tokio::test]
#[ignore = "requires explicit HAZE_SYNC_TEST_DATABASE_URL for STOR-GDA-P1 durable-state evidence"]
async fn gdrive_compare_and_commit_is_versioned_replay_safe_isolated_and_rollback_safe() {
    let context = connect_required_test_database_from_env().await.unwrap();
    context.apply_migrations().await.unwrap();
    let namespace = context.namespace();
    let adapter_a = AdapterId::parse(&namespace.adapter_id("gdrive-a")).unwrap();
    let adapter_b = AdapterId::parse(&namespace.adapter_id("gdrive-b")).unwrap();
    let path_a = VaultPath::parse(&namespace.vault_path("a.md")).unwrap();
    let path_b = VaultPath::parse(&namespace.vault_path("b.md")).unwrap();
    let import_operation = OperationId::parse(&namespace.operation_id("import-b")).unwrap();
    let export_operation = OperationId::parse(&namespace.operation_id("export-a")).unwrap();
    let confirm_operation = OperationId::parse(&namespace.operation_id("confirm-b")).unwrap();
    let rollback_operation = OperationId::parse(&namespace.operation_id("rollback-a")).unwrap();
    let rejected_operation = OperationId::parse(&namespace.operation_id("rejected")).unwrap();
    let cursor_1 = GDriveCursor::parse("opaque-cursor-1").unwrap();
    let cursor_2 = GDriveCursor::parse("opaque-cursor-2").unwrap();
    let fingerprint_1 = fingerprint('1');
    let fingerprint_2 = fingerprint('2');
    let fingerprint_3 = fingerprint('3');
    let fingerprint_4 = fingerprint('4');
    let conflicting_fingerprint = fingerprint('f');
    let first_seen = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    let last_seen = Utc.timestamp_opt(1_700_000_100, 0).unwrap();

    let mut transaction = context.pool().begin().await.unwrap();
    for adapter in [&adapter_a, &adapter_b] {
        insert_test_adapter(&mut transaction, adapter).await;
        let initialized = initialize_gdrive_adapter_state(&mut transaction, adapter)
            .await
            .unwrap();
        assert_eq!(initialized.state_version, 0);
        assert_eq!(initialized.drive_cursor_generation, 0);
        assert_eq!(initialized.core_export_seq, 0);
    }

    let empty = load_gdrive_state_snapshot(&mut transaction, &adapter_a, None, 10)
        .await
        .unwrap()
        .unwrap();
    assert!(empty.items.is_empty());

    let import_commit = GDriveStateCommit {
        adapter_id: &adapter_a,
        expected_state_version: 0,
        cursor_advance: Some(GDriveCursorAdvance {
            cursor: &cursor_1,
            expected_generation: 0,
            next_generation: 1,
        }),
        core_export_seq: Some(5),
        item: Some(item_input(
            &path_b,
            "drive-file-b",
            GDriveEchoState::Pending,
            Some(&import_operation),
            None,
            Some((first_seen, last_seen, 1, false)),
            5,
        )),
        operation: operation_input(
            &import_operation,
            GDriveOperationKind::Import,
            &fingerprint_1,
            Some(&path_b),
            Some(5),
        ),
    };
    let committed = compare_and_commit_gdrive_state(&mut transaction, &import_commit)
        .await
        .unwrap();
    assert!(matches!(committed, GDriveCommitOutcome::Committed { .. }));

    let replay = compare_and_commit_gdrive_state(&mut transaction, &import_commit)
        .await
        .unwrap();
    assert!(matches!(replay, GDriveCommitOutcome::Replayed { .. }));

    let conflicting_replay = GDriveStateCommit {
        operation: operation_input(
            &import_operation,
            GDriveOperationKind::Import,
            &conflicting_fingerprint,
            Some(&path_b),
            Some(5),
        ),
        ..import_commit.clone()
    };
    assert_eq!(
        compare_and_commit_gdrive_state(&mut transaction, &conflicting_replay).await,
        Err(RepositoryError::GDriveOperationConflict)
    );

    for rejected in [
        rejected_commit(
            &adapter_a,
            0,
            None,
            Some(6),
            &rejected_operation,
            &fingerprint_2,
        ),
        rejected_commit(
            &adapter_a,
            1,
            Some(GDriveCursorAdvance {
                cursor: &cursor_2,
                expected_generation: 0,
                next_generation: 1,
            }),
            None,
            &rejected_operation,
            &fingerprint_2,
        ),
    ] {
        let expected = if rejected.expected_state_version == 0 {
            RepositoryError::GDriveStateStaleExpected
        } else {
            RepositoryError::GDriveCursorGenerationMismatch
        };
        assert_eq!(
            compare_and_commit_gdrive_state(&mut transaction, &rejected).await,
            Err(expected)
        );
    }

    for (next_generation, expected) in [
        (1, RepositoryError::CursorRegression),
        (3, RepositoryError::CursorGap),
    ] {
        let rejected = rejected_commit(
            &adapter_a,
            1,
            Some(GDriveCursorAdvance {
                cursor: &cursor_2,
                expected_generation: 1,
                next_generation,
            }),
            None,
            &rejected_operation,
            &fingerprint_2,
        );
        assert_eq!(
            compare_and_commit_gdrive_state(&mut transaction, &rejected).await,
            Err(expected)
        );
    }

    let checkpoint_regression = rejected_commit(
        &adapter_a,
        1,
        None,
        Some(4),
        &rejected_operation,
        &fingerprint_2,
    );
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
        item: Some(item_input(
            &path_a,
            "drive-file-a",
            GDriveEchoState::None,
            None,
            None,
            None,
            6,
        )),
        operation: operation_input(
            &export_operation,
            GDriveOperationKind::Export,
            &fingerprint_2,
            Some(&path_a),
            Some(6),
        ),
    };
    compare_and_commit_gdrive_state(&mut transaction, &export_commit)
        .await
        .unwrap();

    let confirm_commit = GDriveStateCommit {
        adapter_id: &adapter_a,
        expected_state_version: 2,
        cursor_advance: None,
        core_export_seq: None,
        item: Some(item_input(
            &path_b,
            "drive-file-b",
            GDriveEchoState::Confirmed,
            Some(&import_operation),
            Some("provider-version-2"),
            Some((first_seen, last_seen, 1, true)),
            6,
        )),
        operation: operation_input(
            &confirm_operation,
            GDriveOperationKind::ProviderMutation,
            &fingerprint_3,
            Some(&path_b),
            Some(6),
        ),
    };
    compare_and_commit_gdrive_state(&mut transaction, &confirm_commit)
        .await
        .unwrap();

    let bounded = load_gdrive_state_snapshot(&mut transaction, &adapter_a, None, 1)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(bounded.state.state_version, 3);
    assert_eq!(bounded.state.drive_cursor_generation, 2);
    assert_eq!(bounded.state.core_export_seq, 6);
    assert_eq!(bounded.items.len(), 1);
    assert_eq!(bounded.items[0].path, path_a.as_str());
    assert_eq!(bounded.next_after_path.as_ref(), Some(&path_a));

    sqlx::query("savepoint stor_gda_rollback")
        .execute(&mut *transaction)
        .await
        .unwrap();
    let rollback_commit = GDriveStateCommit {
        adapter_id: &adapter_a,
        expected_state_version: 3,
        cursor_advance: None,
        core_export_seq: Some(7),
        item: Some(item_input(
            &path_a,
            "drive-file-a",
            GDriveEchoState::None,
            None,
            None,
            Some((first_seen, last_seen, 2, false)),
            7,
        )),
        operation: operation_input(
            &rollback_operation,
            GDriveOperationKind::DeleteCandidate,
            &fingerprint_4,
            Some(&path_a),
            Some(7),
        ),
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
    assert!(after_rollback
        .items
        .iter()
        .find(|item| item.path == path_a.as_str())
        .unwrap()
        .delete_candidate_generation
        .is_none());

    let isolated = load_gdrive_state_snapshot(&mut transaction, &adapter_b, None, 10)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(isolated.state.state_version, 0);
    assert!(isolated.items.is_empty());
    transaction.rollback().await.unwrap();
}

#[tokio::test]
#[ignore = "requires explicit HAZE_SYNC_TEST_DATABASE_URL for concurrent STOR-GDA-P1 evidence"]
async fn concurrent_loser_observes_stale_expected_after_winner_commits() {
    let context = connect_required_test_database_from_env().await.unwrap();
    context.apply_migrations().await.unwrap();
    let adapter_id = AdapterId::parse(&context.namespace().adapter_id("gdrive-race")).unwrap();
    let winner_operation = OperationId::parse(&context.namespace().operation_id("winner")).unwrap();
    let loser_operation = OperationId::parse(&context.namespace().operation_id("loser")).unwrap();
    let winner_fingerprint = fingerprint('5');
    let loser_fingerprint = fingerprint('6');

    let mut setup = context.pool().begin().await.unwrap();
    insert_test_adapter(&mut setup, &adapter_id).await;
    initialize_gdrive_adapter_state(&mut setup, &adapter_id)
        .await
        .unwrap();
    setup.commit().await.unwrap();

    let mut winner_transaction = context.pool().begin().await.unwrap();
    let winner_commit = rejected_commit(
        &adapter_id,
        0,
        None,
        Some(1),
        &winner_operation,
        &winner_fingerprint,
    );
    let winner = compare_and_commit_gdrive_state(&mut winner_transaction, &winner_commit)
        .await
        .unwrap();

    let loser_adapter = adapter_id.clone();
    let loser_pool = context.pool().clone();
    let loser_handle = tokio::spawn(async move {
        let mut loser_transaction = loser_pool.begin().await.unwrap();
        let loser_commit = rejected_commit(
            &loser_adapter,
            0,
            None,
            Some(2),
            &loser_operation,
            &loser_fingerprint,
        );
        let result = compare_and_commit_gdrive_state(&mut loser_transaction, &loser_commit).await;
        loser_transaction.rollback().await.unwrap();
        result
    });

    tokio::task::yield_now().await;
    winner_transaction.commit().await.unwrap();
    assert!(matches!(winner, GDriveCommitOutcome::Committed { .. }));
    assert_eq!(
        loser_handle.await.unwrap(),
        Err(RepositoryError::GDriveStateStaleExpected)
    );

    let mut cleanup = context.pool().begin().await.unwrap();
    sqlx::query("delete from gdrive_operations where adapter_id = $1")
        .bind(adapter_id.as_str())
        .execute(&mut *cleanup)
        .await
        .unwrap();
    sqlx::query("delete from gdrive_adapter_state where adapter_id = $1")
        .bind(adapter_id.as_str())
        .execute(&mut *cleanup)
        .await
        .unwrap();
    sqlx::query("delete from sync_adapters where adapter_id = $1")
        .bind(adapter_id.as_str())
        .execute(&mut *cleanup)
        .await
        .unwrap();
    cleanup.commit().await.unwrap();
}

async fn insert_test_adapter(transaction: &mut Transaction<'_, Postgres>, adapter_id: &AdapterId) {
    sqlx::query(
        "insert into sync_adapters (adapter_id, display_name, role, token_hash) \
         values ($1, $2, 'gdrive_adapter', 'synthetic-test-token-hash')",
    )
    .bind(adapter_id.as_str())
    .bind(format!("Test {}", adapter_id.as_str()))
    .execute(&mut **transaction)
    .await
    .unwrap();
}

fn fingerprint(character: char) -> GDriveOperationFingerprint {
    GDriveOperationFingerprint::parse(character.to_string().repeat(SHA256_HEX_LEN)).unwrap()
}

fn operation_input<'a>(
    operation_id: &'a OperationId,
    kind: GDriveOperationKind,
    fingerprint: &'a GDriveOperationFingerprint,
    path: Option<&'a VaultPath>,
    core_seq: Option<i64>,
) -> GDriveOperationInput<'a> {
    GDriveOperationInput {
        operation_id,
        kind,
        facts_fingerprint: fingerprint,
        mapping_path: path,
        core_seq,
        drive_version: None,
    }
}

fn rejected_commit<'a>(
    adapter_id: &'a AdapterId,
    expected_state_version: i64,
    cursor_advance: Option<GDriveCursorAdvance<'a>>,
    core_export_seq: Option<i64>,
    operation_id: &'a OperationId,
    fingerprint: &'a GDriveOperationFingerprint,
) -> GDriveStateCommit<'a> {
    GDriveStateCommit {
        adapter_id,
        expected_state_version,
        cursor_advance,
        core_export_seq,
        item: None,
        operation: operation_input(
            operation_id,
            GDriveOperationKind::Export,
            fingerprint,
            None,
            core_export_seq,
        ),
    }
}

fn item_input<'a>(
    path: &'a VaultPath,
    drive_file_id: &'a str,
    echo_state: GDriveEchoState,
    echo_operation_id: Option<&'a OperationId>,
    echo_provider_version: Option<&'a str>,
    candidate: Option<(DateTime<Utc>, DateTime<Utc>, i64, bool)>,
    core_seq: i64,
) -> GDriveItemUpsert<'a> {
    let (first_seen, last_seen, generation, blocked) = candidate
        .map(|(first, last, generation, blocked)| {
            (Some(first), Some(last), Some(generation), blocked)
        })
        .unwrap_or((None, None, None, false));
    GDriveItemUpsert {
        path,
        drive_file_id: Some(drive_file_id),
        drive_parent_id: Some("drive-parent"),
        drive_name: Some("note.md"),
        mime_type: Some("text/markdown"),
        md5_checksum: Some("aabbccddeeff00112233445566778899"),
        head_revision_id: Some("drive-head"),
        drive_version: Some("drive-version"),
        drive_modified_time: last_seen,
        core_object_id: None,
        core_revision_id: None,
        core_seq: Some(core_seq),
        echo_state,
        echo_operation_id,
        echo_provider_version,
        delete_candidate_first_seen_at: first_seen,
        delete_candidate_last_seen_at: last_seen,
        delete_candidate_generation: generation,
        delete_candidate_blocked: blocked,
        delete_confirmation_audit_id: None,
        last_imported_at: last_seen,
        last_exported_at: None,
        last_seen_at: last_seen,
    }
}
