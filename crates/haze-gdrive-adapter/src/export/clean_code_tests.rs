use super::*;
use crate::config::AdapterMode;
use crate::hash::ContentSha256;
use crate::state::{CoreChangeCursor, EchoGuard, GDriveMapping, SafeTimestamp, VaultPath};

fn timestamp() -> SafeTimestamp {
    SafeTimestamp::new("2026-07-10T18:30:00Z").expect("timestamp")
}

fn path(value: &str) -> VaultPath {
    VaultPath::new(value).expect("path")
}

fn upsert_change(
    operation_id: &str,
    path_value: &str,
    revision_id: &str,
    content: &[u8],
    updated_by: &str,
) -> (CoreExportChange, CoreFileContent) {
    let vault_path = path(path_value);
    let hash = ContentSha256::from_content(content);
    let change = CoreExportChange::upsert_file(
        10,
        operation_id,
        vault_path.clone(),
        revision_id,
        hash,
        content.len() as u64,
        updated_by,
    )
    .expect("change");
    let source = CoreFileContent::new(
        vault_path,
        revision_id,
        hash,
        content.len() as u64,
        content.to_vec(),
    )
    .expect("source");
    (change, source)
}

fn mapping(path_value: &str, drive_version: Option<&str>) -> GDriveMapping {
    let mut mapping = GDriveMapping::new(
        path(path_value),
        "drive-file",
        "drive-folder",
        path_value.rsplit('/').next().expect("name"),
    )
    .expect("mapping");
    mapping.drive_version = drive_version.map(str::to_owned);
    mapping.core_revision = Some("revision-old".to_owned());
    mapping.core_sequence = Some(9);
    mapping
}

#[test]
fn own_origin_change_is_consumed_without_download_or_provider_mutation() {
    let (change, _source) = upsert_change(
        "operation-own-origin",
        "Notes/own.md",
        "revision-10",
        b"own",
        "gdrive-adapter",
    );
    let page = CoreExportPage::new(10, 11, false, vec![change]).expect("page");
    let core = FakeCoreExportClient::new().with_page(page);
    let mut provider = FakeDriveExportProvider::new();
    let mut state = InMemoryExportStateStore::new(CoreChangeCursor::new(10), "root");
    let mut echo_guard = EchoGuard::new();
    let input = ExportCycleInput::new(
        "gdrive-adapter",
        AdapterMode::ExportOnly,
        false,
        timestamp(),
    )
    .expect("input");

    let outcome = run_export_cycle(
        &core,
        &mut provider,
        &mut state,
        &mut echo_guard,
        &ExportRetryPolicy::default(),
        input,
    )
    .expect("cycle");

    assert_eq!(outcome.work_items_planned, 1);
    assert_eq!(provider.mutation_count(), 0);
    assert_eq!(state.mapping_save_count(), 0);
    assert_eq!(state.cursor().next_sequence, 11);
    assert!(outcome.cursor_saved);
}

#[test]
fn update_and_trash_require_provider_revision_preconditions_even_in_dry_run() {
    let (upsert, source) = upsert_change(
        "operation-update",
        "Notes/precondition.md",
        "revision-10",
        b"updated",
        "other-adapter",
    );
    let missing_version_mapping = mapping("Notes/precondition.md", None);

    let update_error = plan_core_export(
        upsert,
        Some(missing_version_mapping.clone()),
        None,
        Some(source),
        AdapterMode::ExportOnly,
        true,
    )
    .expect_err("missing update precondition");

    let tombstone = CoreExportChange::tombstone(
        10,
        "operation-trash",
        path("Notes/precondition.md"),
        "tombstone-10",
        "other-adapter",
    )
    .expect("tombstone");
    let trash_error = plan_core_export(
        tombstone,
        Some(missing_version_mapping),
        None,
        None,
        AdapterMode::ExportOnly,
        true,
    )
    .expect_err("missing trash precondition");

    assert_eq!(
        update_error,
        ExportError::MissingProviderRevisionPrecondition
    );
    assert_eq!(
        trash_error,
        ExportError::MissingProviderRevisionPrecondition
    );
}

#[test]
fn planner_rejects_mapping_for_a_different_vault_path() {
    let (change, source) = upsert_change(
        "operation-path",
        "Notes/expected.md",
        "revision-10",
        b"content",
        "other-adapter",
    );

    let error = plan_core_export(
        change,
        Some(mapping("Notes/other.md", Some("provider-v1"))),
        None,
        Some(source),
        AdapterMode::ExportOnly,
        false,
    )
    .expect_err("mapping path mismatch");

    assert_eq!(error, ExportError::MappingPathMismatch);
}

#[test]
fn fake_provider_rejects_reused_operation_id_with_changed_request_fingerprint() {
    let content = b"same-content".to_vec();
    let hash = ContentSha256::from_content(&content);
    let mut provider = FakeDriveExportProvider::new();

    provider
        .create_file(DriveCreateExportRequest {
            operation_id: "operation-reused".to_owned(),
            parent_id: "root".to_owned(),
            name: "note.md".to_owned(),
            mime_type: "text/markdown".to_owned(),
            content_sha256: hash,
            content: content.clone(),
        })
        .expect("first mutation");

    let error = provider
        .create_file(DriveCreateExportRequest {
            operation_id: "operation-reused".to_owned(),
            parent_id: "root".to_owned(),
            name: "note.md".to_owned(),
            mime_type: "text/plain".to_owned(),
            content_sha256: hash,
            content,
        })
        .expect_err("idempotency mismatch");

    assert_eq!(error.category(), DriveExportErrorCategory::Conflict);
    assert_eq!(provider.mutation_count(), 1);
}

#[test]
fn fake_provider_verifies_content_before_returning_an_idempotent_replay() {
    let content = b"verified-content".to_vec();
    let hash = ContentSha256::from_content(&content);
    let mut provider = FakeDriveExportProvider::new();

    provider
        .create_file(DriveCreateExportRequest {
            operation_id: "operation-replay".to_owned(),
            parent_id: "root".to_owned(),
            name: "note.md".to_owned(),
            mime_type: "text/markdown".to_owned(),
            content_sha256: hash,
            content,
        })
        .expect("first mutation");

    let error = provider
        .create_file(DriveCreateExportRequest {
            operation_id: "operation-replay".to_owned(),
            parent_id: "root".to_owned(),
            name: "note.md".to_owned(),
            mime_type: "text/markdown".to_owned(),
            content_sha256: hash,
            content: b"corrupted".to_vec(),
        })
        .expect_err("invalid replay body");

    assert_eq!(error.category(), DriveExportErrorCategory::InvalidRequest);
    assert_eq!(provider.mutation_count(), 1);
}

#[test]
fn debug_output_redacts_operation_ids_and_revision_tokens() {
    let (change, _source) = upsert_change(
        "operation-secret",
        "Notes/secret.md",
        "revision-10",
        b"secret",
        "other-adapter",
    );
    let create = DriveCreateExportRequest {
        operation_id: "create-secret".to_owned(),
        parent_id: "root".to_owned(),
        name: "secret.md".to_owned(),
        mime_type: "text/markdown".to_owned(),
        content_sha256: ContentSha256::from_content(b"secret"),
        content: b"secret".to_vec(),
    };
    let update = DriveUpdateExportRequest {
        operation_id: "update-secret".to_owned(),
        file_id: "drive-file".to_owned(),
        expected_revision_token: Some("provider-secret".to_owned()),
        mime_type: "text/markdown".to_owned(),
        content_sha256: ContentSha256::from_content(b"secret"),
        content: b"secret".to_vec(),
    };
    let trash = DriveTrashExportRequest {
        operation_id: "trash-secret".to_owned(),
        file_id: "drive-file".to_owned(),
        expected_revision_token: Some("provider-secret".to_owned()),
    };
    let receipt = DriveExportReceipt {
        provider_id: "drive-file".to_owned(),
        revision_token: "receipt-secret".to_owned(),
    };

    let debug = format!("{change:?} {create:?} {update:?} {trash:?} {receipt:?}");

    for secret in [
        "operation-secret",
        "create-secret",
        "update-secret",
        "trash-secret",
        "provider-secret",
        "receipt-secret",
    ] {
        assert!(!debug.contains(secret));
    }
    assert!(debug.contains("redacted-idempotency-key"));
}
