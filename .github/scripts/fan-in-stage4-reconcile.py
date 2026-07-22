from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one occurrence, found {count}")
    return text.replace(old, new, 1)


def patch_cli() -> None:
    doctor_path = Path("crates/haze-sync-cli/src/doctor.rs")
    doctor = doctor_path.read_text()
    doctor = replace_once(
        doctor,
        '"doctor summary: {:?} (total: {}, ok: {}, warnings: {}, failed: {}, skipped: {})",\n        report.summary.status,\n        report.summary.total_checks,\n        report.summary.ok_count,\n        report.summary.warning_count,\n        report.summary.failed_count,\n        report.summary.skipped_count',
        '"doctor summary: {:?} (total: {}, ok: {}, warnings: {}, failed: {}, skipped: {}, not_run: {}, placeholders: {})",\n        report.summary.status,\n        report.summary.total_checks,\n        report.summary.ok_count,\n        report.summary.warning_count,\n        report.summary.failed_count,\n        report.summary.skipped_count,\n        report.summary.not_run_count,\n        report.summary.placeholder_count',
        "doctor summary counters",
    )
    doctor = replace_once(
        doctor,
        "status_label(check.status),",
        "status_label(check.status()),",
        "doctor status accessor",
    )
    doctor = replace_once(
        doctor,
        '        DoctorCheckStatus::Skipped => "skipped",\n',
        '        DoctorCheckStatus::Skipped => "skipped",\n        DoctorCheckStatus::NotRun => "not_run",\n        DoctorCheckStatus::Placeholder => "placeholder",\n',
        "doctor status labels",
    )
    doctor_path.write_text(doctor)

    live_path = Path("crates/haze-sync-cli/src/doctor_live.rs")
    live = live_path.read_text()
    live = replace_once(
        live,
        "use haze_sync_core::doctor::{\n    db_connectivity_check, object_store_exists_writable_check, AdapterTokenSanityDetails,\n    DbConnectivityCheckInput, DbConnectivityDetails, DoctorCheckDetails, DoctorCheckId,\n    DoctorCheckResult, DoctorCheckStatus, DoctorReport, MissingBlobDetectionDetails,\n    ObjectStoreExistsWritableDetails, ObjectStoreExistsWritableInput,\n};",
        "use haze_sync_core::doctor::{\n    db_connectivity_check, object_store_exists_writable_check, DbConnectivityCheckInput,\n    DoctorCheckId, DoctorCheckResult, DoctorCheckStatus, DoctorNotRunReason, DoctorReport,\n    ObjectStoreExistsWritableInput,\n};",
        "doctor live imports",
    )
    live = live.replace("skipped_database_check", "not_run_database_check")
    live = live.replace("skipped_object_store_check", "not_run_object_store_check")
    live = live.replace("skipped_missing_blob_check", "not_run_missing_blob_check")
    live = live.replace("skipped_adapter_token_check", "not_run_adapter_token_check")
    live = replace_once(
        live,
        '''        ReadinessComponentState::NotReady => DoctorCheckResult::new(
            DoctorCheckId::ObjectStoreExistsWritable,
            DoctorCheckStatus::Failed,
            "object store readiness check failed",
            DoctorCheckDetails::ObjectStoreExistsWritable(ObjectStoreExistsWritableDetails {
                configured: true,
                exists: None,
                writable: None,
            }),
        ),''',
        '''        ReadinessComponentState::NotReady => DoctorCheckResult::not_run(
            DoctorCheckId::ObjectStoreExistsWritable,
            DoctorNotRunReason::DependencyUnavailable,
        ),''',
        "object store not-ready mapping",
    )
    start = live.index("fn not_run_database_check() -> DoctorCheckResult {")
    end = live.index("fn render_live_summary(", start)
    helpers = '''fn not_run_database_check() -> DoctorCheckResult {
    DoctorCheckResult::not_run(
        DoctorCheckId::DbConnectivity,
        DoctorNotRunReason::DependencyUnavailable,
    )
}

fn not_run_object_store_check() -> DoctorCheckResult {
    DoctorCheckResult::not_run(
        DoctorCheckId::ObjectStoreExistsWritable,
        DoctorNotRunReason::DependencyUnavailable,
    )
}

fn not_run_missing_blob_check() -> DoctorCheckResult {
    DoctorCheckResult::not_run(
        DoctorCheckId::MissingBlobs,
        DoctorNotRunReason::DependencyUnavailable,
    )
}

fn not_run_adapter_token_check() -> DoctorCheckResult {
    DoctorCheckResult::not_run(
        DoctorCheckId::AdapterTokenSanity,
        DoctorNotRunReason::DependencyUnavailable,
    )
}

'''
    live = live[:start] + helpers + live[end:]
    live = replace_once(
        live,
        "fn live_doctor_maps_readiness_to_core_report_and_skips_unavailable_checks()",
        "fn live_doctor_maps_readiness_to_core_report_and_reports_unavailable_checks_as_not_run()",
        "live doctor test name",
    )
    live = replace_once(
        live,
        '        assert!(output.stdout.contains("skipped: 2"));\n        assert!(output.stdout.contains("missing blob check not run"));\n        assert!(output.stdout.contains("adapter token sanity check not run"));',
        '        assert!(output.stdout.contains("not_run: 2"));\n        assert!(output\n            .stdout\n            .contains("check missing_blobs: not_run - doctor check was not run"));\n        assert!(output\n            .stdout\n            .contains("check adapter_token_sanity: not_run - doctor check was not run"));',
        "ready live doctor expectations",
    )
    live = replace_once(
        live,
        '        assert!(output.stdout.contains("failed: 2"));',
        '        assert!(output.stdout.contains("failed: 1"));\n        assert!(output.stdout.contains("not_run: 3"));',
        "not-ready live doctor expectations",
    )
    live_path.write_text(live)


def patch_gdrive() -> None:
    path = Path("crates/haze-gdrive-adapter/src/durable_state.rs")
    text = path.read_text()
    text = replace_once(
        text,
        "    GDriveLastOperationsSummaryDto, GDriveMappingFactsDto, GDriveStateCommitRequest,\n    GDriveStateCommitResponse, GDriveStateErrorCode, GDriveStateErrorResponse,\n    GDriveStateSnapshotResponse,",
        "    GDriveLastOperationsSummaryDto, GDriveMappingFactsDto, GDrivePrivateCursorStateDto,\n    GDriveStateCommitRequest, GDriveStateCommitResponse, GDriveStateErrorCode,\n    GDriveStateErrorResponse, GDriveStateSnapshotResponse,",
        "private cursor import",
    )
    text = replace_once(
        text,
        '''    fn from_snapshot(snapshot: &GDriveStateSnapshotResponse) -> Self {
        Self {
            state_format_version: snapshot.state_format_version,
            state_version: snapshot.state_version,
            cursor_generation: snapshot.cursor.generation,
            cursor_present: snapshot.cursor.present,
            core_export_checkpoint: snapshot.core_export_checkpoint,
            last_operations: snapshot.last_operations.clone(),
        }
    }''',
        '''    fn from_snapshot(snapshot: &GDriveStateSnapshotResponse) -> Self {
        let (cursor_generation, cursor_present) = match &snapshot.cursor {
            GDrivePrivateCursorStateDto::Absent { generation } => (*generation, false),
            GDrivePrivateCursorStateDto::Present { generation, .. } => (*generation, true),
        };
        Self {
            state_format_version: snapshot.state_format_version,
            state_version: snapshot.state_version,
            cursor_generation,
            cursor_present,
            core_export_checkpoint: snapshot.core_export_checkpoint,
            last_operations: snapshot.last_operations.clone(),
        }
    }''',
        "snapshot signature mapping",
    )
    text = replace_once(
        text,
        '            "cursor": { "generation": 3, "present": true },',
        '            "cursor": {\n                "state": "present",\n                "generation": 3,\n                "cursor": "sentinel-private-cursor"\n            },',
        "snapshot fixture cursor",
    )
    text = replace_once(
        text,
        '''        assert_eq!(snapshot.state_version, 7);
        let calls = client.transport().calls();''',
        '''        assert_eq!(snapshot.state_version, 7);
        match &snapshot.cursor {
            GDrivePrivateCursorStateDto::Present { generation, cursor } => {
                assert_eq!(*generation, 3);
                assert_eq!(
                    cursor.expose_for_private_commit(),
                    "sentinel-private-cursor"
                );
            }
            GDrivePrivateCursorStateDto::Absent { .. } => panic!("cursor should be present"),
        }
        let calls = client.transport().calls();''',
        "present cursor assertion",
    )
    marker = '''    #[test]
    fn origin_only_server_url_accepts_root_forms_and_rejects_ambiguous_endpoints() {'''
    absent_test = '''    #[test]
    fn absent_private_cursor_decodes_and_signs_without_provider_value() {
        let body = serde_json::to_vec(&serde_json::json!({
            "adapter_id": "gdrive-main",
            "state_format_version": 1,
            "state_version": 7,
            "cursor": { "state": "absent", "generation": 0 },
            "core_export_checkpoint": 0,
            "last_operations": {},
            "mappings": [],
            "next_after_path": null
        }))
        .unwrap();
        let client = fake_client(
            [Ok(HttpResponse::new(200, body))],
            policy(8_192, 4, 10),
        );

        let snapshot = client.get_state_page(None, 1).unwrap();
        let signature = SnapshotSignature::from_snapshot(&snapshot);
        assert!(matches!(
            snapshot.cursor,
            GDrivePrivateCursorStateDto::Absent { generation: 0 }
        ));
        assert_eq!(signature.cursor_generation, 0);
        assert!(!signature.cursor_present);
    }

'''
    text = replace_once(text, marker, absent_test + marker, "absent cursor test")
    path.write_text(text)


patch_cli()
patch_gdrive()
