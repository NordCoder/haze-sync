use super::*;
use haze_sync_common::ContentHash;
use serde_json::json;

fn repeated_hash(ch: char) -> ContentHash {
    ContentHash::parse(&ch.to_string().repeat(64)).expect("test hash should be valid")
}

fn healthy_missing_blob_check() -> DoctorCheckResult {
    missing_blob_detection_check(MissingBlobDetectionInput::new(1, Vec::new(), 1))
}

#[test]
fn check_result_serialization_preserves_existing_safe_shape() {
    let result = object_store_exists_writable_check(ObjectStoreExistsWritableInput {
        configured: true,
        exists: Some(true),
        writable: Some(false),
    });
    let serialized = serde_json::to_string(&result).expect("doctor result should serialize");
    assert_eq!(
        serialized,
        r#"{"check_id":"object_store_exists_writable","status":"failed","message":"object store root is not writable","details":{"kind":"object_store_exists_writable","configured":true,"exists":true,"writable":false}}"#
    );
}

#[test]
fn empty_and_mixed_reports_use_honest_status_precedence() {
    assert_eq!(
        DoctorReport::from_results(Vec::new()).summary().status,
        DoctorCheckStatus::NotRun
    );

    let ok = healthy_missing_blob_check();
    let skipped = adapter_cursor_check(AdapterCursorCheckInput::new(0, 0, 0, 0));
    let not_run = DoctorCheckResult::not_run(
        DoctorCheckId::GdriveMapping,
        DoctorNotRunReason::ToolingUnavailable,
    );
    let placeholder = DoctorCheckResult::placeholder(DoctorCheckId::WorktreeDrift);
    let warning = db_connectivity_check(DbConnectivityCheckInput::offline(false));
    let failed = missing_blob_detection_check(MissingBlobDetectionInput::new(
        1,
        vec![repeated_hash('a')],
        1,
    ));

    assert_eq!(
        DoctorReport::from_results(vec![ok.clone()])
            .summary()
            .status,
        DoctorCheckStatus::Ok
    );
    assert_eq!(
        DoctorReport::from_results(vec![ok.clone(), skipped.clone()])
            .summary()
            .status,
        DoctorCheckStatus::Skipped
    );
    assert_eq!(
        DoctorReport::from_results(vec![ok.clone(), skipped.clone(), not_run.clone()])
            .summary()
            .status,
        DoctorCheckStatus::NotRun
    );
    assert_eq!(
        DoctorReport::from_results(vec![
            ok.clone(),
            skipped.clone(),
            not_run.clone(),
            placeholder.clone(),
        ])
        .summary()
        .status,
        DoctorCheckStatus::Placeholder
    );
    assert_eq!(
        DoctorReport::from_results(vec![
            ok.clone(),
            skipped,
            not_run,
            placeholder,
            warning.clone(),
        ])
        .summary()
        .status,
        DoctorCheckStatus::Warning
    );
    assert_eq!(
        DoctorReport::from_results(vec![ok, warning, failed])
            .summary()
            .status,
        DoctorCheckStatus::Failed
    );
}

#[test]
fn summary_counts_every_explicit_execution_state() {
    let report = DoctorReport::from_results(vec![
        healthy_missing_blob_check(),
        db_connectivity_check(DbConnectivityCheckInput::offline(false)),
        missing_blob_detection_check(MissingBlobDetectionInput::new(
            1,
            vec![repeated_hash('a')],
            1,
        )),
        adapter_cursor_check(AdapterCursorCheckInput::new(0, 0, 0, 0)),
        DoctorCheckResult::not_run(
            DoctorCheckId::GdriveMapping,
            DoctorNotRunReason::NotRequested,
        ),
        DoctorCheckResult::placeholder(DoctorCheckId::WorktreeDrift),
    ]);
    let summary = report.summary();

    assert_eq!(summary.total_checks, 6);
    assert_eq!(summary.ok_count, 1);
    assert_eq!(summary.warning_count, 1);
    assert_eq!(summary.failed_count, 1);
    assert_eq!(summary.skipped_count, 1);
    assert_eq!(summary.not_run_count, 1);
    assert_eq!(summary.placeholder_count, 1);
}

#[test]
fn database_check_distinguishes_skipped_not_run_and_completed() {
    assert_eq!(
        db_connectivity_check(DbConnectivityCheckInput::offline(true)).status(),
        DoctorCheckStatus::Skipped
    );
    assert_eq!(
        db_connectivity_check(DbConnectivityCheckInput {
            metadata_configured: true,
            live_check_enabled: true,
            connectivity_verified: None,
        })
        .status(),
        DoctorCheckStatus::NotRun
    );
    assert_eq!(
        db_connectivity_check(DbConnectivityCheckInput {
            metadata_configured: true,
            live_check_enabled: true,
            connectivity_verified: Some(false),
        })
        .status(),
        DoctorCheckStatus::Failed
    );
    assert_eq!(
        db_connectivity_check(DbConnectivityCheckInput {
            metadata_configured: true,
            live_check_enabled: true,
            connectivity_verified: Some(true),
        })
        .status(),
        DoctorCheckStatus::Ok
    );
}

#[test]
fn object_store_check_distinguishes_offline_partial_and_completed_facts() {
    assert_eq!(
        object_store_exists_writable_check(ObjectStoreExistsWritableInput::offline(true)).status(),
        DoctorCheckStatus::Skipped
    );
    assert_eq!(
        object_store_exists_writable_check(ObjectStoreExistsWritableInput {
            configured: true,
            exists: Some(true),
            writable: None,
        })
        .status(),
        DoctorCheckStatus::NotRun
    );
    assert_eq!(
        object_store_exists_writable_check(ObjectStoreExistsWritableInput {
            configured: true,
            exists: Some(true),
            writable: Some(true),
        })
        .status(),
        DoctorCheckStatus::Ok
    );
}

#[test]
fn missing_blob_summary_exposes_deterministic_hash_samples_not_paths() {
    let report = DoctorReport::from_results(vec![missing_blob_detection_check(
        MissingBlobDetectionInput::new(
            3,
            vec![repeated_hash('b'), repeated_hash('a'), repeated_hash('a')],
            1,
        ),
    )]);
    let serialized = serde_json::to_string(&report).expect("doctor report should serialize");

    assert!(serialized
        .contains("sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
    assert!(!serialized.contains("/srv/haze-sync/objects"));
    assert!(!serialized.contains("C:\\private"));
    assert!(!serialized.contains("Notes/a.md"));
    assert!(!serialized
        .contains("sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"));
}

#[test]
fn cursor_summary_covers_missing_orphaned_invalid_stale_and_valid_states() {
    assert_eq!(
        adapter_cursor_check(AdapterCursorCheckInput::new(2, 2, 1, 0)).status(),
        DoctorCheckStatus::Failed
    );
    assert_eq!(
        adapter_cursor_check(AdapterCursorCheckInput::new(2, 1, 0, 0)).message_code(),
        DoctorCheckMessage::MissingAdapterCursorDetected
    );
    assert_eq!(
        adapter_cursor_check(AdapterCursorCheckInput::new(1, 2, 0, 0)).message_code(),
        DoctorCheckMessage::OrphanedAdapterCursorDetected
    );
    assert_eq!(
        adapter_cursor_check(AdapterCursorCheckInput::new(2, 2, 0, 1)).message_code(),
        DoctorCheckMessage::StaleAdapterCursorDetected
    );
    assert_eq!(
        adapter_cursor_check(AdapterCursorCheckInput::new(2, 2, 0, 0)).status(),
        DoctorCheckStatus::Ok
    );
}

#[test]
fn mapping_and_worktree_summaries_use_counts_without_provider_or_path_data() {
    let mapping = gdrive_mapping_check(GdriveMappingCheckInput::new(true, 10, 0, 0, 2));
    assert_eq!(mapping.status(), DoctorCheckStatus::Warning);
    assert_eq!(
        gdrive_mapping_check(GdriveMappingCheckInput::new(true, 10, 1, 0, 0)).status(),
        DoctorCheckStatus::Failed
    );

    let worktree = worktree_drift_check(WorktreeDriftCheckInput::new(true, 10, 0, 1, 1, 0));
    assert_eq!(worktree.status(), DoctorCheckStatus::Warning);
    assert_eq!(
        worktree_drift_check(WorktreeDriftCheckInput::new(true, 10, 1, 0, 0, 0)).status(),
        DoctorCheckStatus::Failed
    );

    let serialized = serde_json::to_string(&DoctorReport::from_results(vec![mapping, worktree]))
        .expect("doctor report should serialize");
    for forbidden in [
        "/srv/haze-vault/worktree",
        "drive-file-id-secret",
        "postgres://operator:secret@db/haze",
        "Bearer token",
    ] {
        assert!(!serialized.contains(forbidden));
    }
}

#[test]
fn token_sanity_summary_exposes_counts_not_token_material() {
    let report = DoctorReport::from_results(vec![adapter_token_sanity_check(
        AdapterTokenSanityInput::new(vec![
            AdapterTokenInspection::new(true, false, true),
            AdapterTokenInspection::new(true, true, false),
            AdapterTokenInspection::new(false, false, false),
        ]),
    )]);
    let serialized = serde_json::to_string(&report).expect("doctor report should serialize");

    assert!(serialized.contains("\"enabled_adapter_count\":2"));
    assert!(serialized.contains("\"missing_token_hash_count\":1"));
    assert!(serialized.contains("\"invalid_role_count\":1"));
    assert!(!serialized.contains("credential_material"));
    assert!(!serialized
        .contains("sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"));
}

#[test]
fn not_run_and_placeholder_results_serialize_only_safe_reason_codes() {
    let report = DoctorReport::from_results(vec![
        DoctorCheckResult::not_run(
            DoctorCheckId::DbConnectivity,
            DoctorNotRunReason::DependencyUnavailable,
        ),
        DoctorCheckResult::placeholder(DoctorCheckId::WorktreeDrift),
    ]);
    let serialized = serde_json::to_string(&report).expect("doctor report should serialize");

    assert!(serialized.contains("\"status\":\"not_run\""));
    assert!(serialized.contains("\"reason\":\"dependency_unavailable\""));
    assert!(serialized.contains("\"status\":\"placeholder\""));
    assert!(serialized.contains("\"reason\":\"integration_pending\""));
}

#[test]
fn result_deserialization_rejects_arbitrary_messages_and_mismatched_details() {
    let arbitrary_message = json!({
        "check_id": "missing_blobs",
        "status": "failed",
        "message": "postgres://operator:secret@db/haze",
        "details": {
            "kind": "missing_blobs",
            "input_count": 1,
            "missing_count": 1,
            "sample_hashes": []
        }
    });
    assert!(serde_json::from_value::<DoctorCheckResult>(arbitrary_message).is_err());

    let mismatched_details = json!({
        "check_id": "missing_blobs",
        "status": "failed",
        "message": "missing required blobs detected",
        "details": {
            "kind": "adapter_token_sanity",
            "enabled_adapter_count": 1,
            "missing_token_hash_count": 0,
            "invalid_role_count": 1
        }
    });
    assert!(serde_json::from_value::<DoctorCheckResult>(mismatched_details).is_err());
}

#[test]
fn report_roundtrip_reorders_checks_and_rejects_inconsistent_summary() {
    let report = DoctorReport::from_results(vec![
        worktree_drift_check(WorktreeDriftCheckInput::new(false, 0, 0, 0, 0, 0)),
        healthy_missing_blob_check(),
    ]);
    assert_eq!(report.checks()[0].check_id(), DoctorCheckId::MissingBlobs);
    assert_eq!(report.checks()[1].check_id(), DoctorCheckId::WorktreeDrift);

    let serialized = serde_json::to_string(&report).unwrap();
    let decoded: DoctorReport = serde_json::from_str(&serialized).unwrap();
    assert_eq!(decoded, report);

    let mut inconsistent = serde_json::to_value(&report).unwrap();
    inconsistent["summary"]["status"] = json!("ok");
    assert!(serde_json::from_value::<DoctorReport>(inconsistent).is_err());
}
