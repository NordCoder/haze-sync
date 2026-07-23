use super::*;
use crate::config::AdapterMode;
use crate::drive::MIME_TEXT_MARKDOWN;
use crate::hash::ContentSha256;
use crate::state::{
    CoreChangeCursor, DriveEchoObservation, EchoDecision, EchoGuard, GDriveMapping, SafeTimestamp,
    VaultPath,
};
use std::time::Duration;

const APPLIED_AT: &str = "2026-07-10T17:00:00Z";

fn timestamp(value: &str) -> SafeTimestamp {
    SafeTimestamp::new(value).expect("timestamp")
}

fn path(value: &str) -> VaultPath {
    VaultPath::new(value).expect("path")
}

fn upsert(
    seq: u64,
    operation_id: &str,
    path_value: &str,
    revision_id: &str,
    content: &[u8],
) -> (CoreExportChange, CoreFileContent) {
    let vault_path = path(path_value);
    let hash = ContentSha256::from_content(content);
    let change = CoreExportChange::upsert_file(
        seq,
        operation_id,
        vault_path.clone(),
        revision_id,
        hash,
        content.len() as u64,
        "core-adapter",
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

fn tombstone(seq: u64, operation_id: &str, path_value: &str) -> CoreExportChange {
    CoreExportChange::tombstone(
        seq,
        operation_id,
        path(path_value),
        format!("tombstone-{seq}"),
        "core-adapter",
    )
    .expect("tombstone")
}

fn page(from_sequence: u64, next_sequence: u64, changes: Vec<CoreExportChange>) -> CoreExportPage {
    CoreExportPage::new(from_sequence, next_sequence, false, changes).expect("page")
}

fn input(mode: AdapterMode, dry_run: bool) -> ExportCycleInput {
    ExportCycleInput::new("gdrive-adapter", mode, dry_run, timestamp(APPLIED_AT)).expect("input")
}

fn existing_mapping(path_value: &str, provider_id: &str, revision_token: &str) -> GDriveMapping {
    let mut mapping = GDriveMapping::new(
        path(path_value),
        provider_id,
        "folder-notes",
        path_value.rsplit('/').next().expect("name"),
    )
    .expect("mapping");
    mapping.drive_version = Some(revision_token.to_owned());
    mapping.core_revision = Some("revision-old".to_owned());
    mapping.core_sequence = Some(9);
    mapping
}

#[test]
fn create_export_verifies_source_then_saves_mapping_echo_and_cursor() {
    let (change, source) = upsert(
        10,
        "operation-create",
        "Notes/new.md",
        "revision-10",
        b"new",
    );
    let core = FakeCoreExportClient::new()
        .with_page(page(10, 11, vec![change]))
        .with_content(source);
    let mut provider = FakeDriveExportProvider::new();
    let mut state = InMemoryExportStateStore::new(CoreChangeCursor::new(10), "root")
        .with_folder("Notes", "folder-notes");
    let mut echo_guard = EchoGuard::new();

    let outcome = run_export_cycle(
        &core,
        &mut provider,
        &mut state,
        &mut echo_guard,
        &ExportRetryPolicy::default(),
        input(AdapterMode::ExportOnly, false),
    )
    .expect("export");

    assert_eq!(outcome.provider_mutations, 1);
    assert_eq!(outcome.mappings_saved, 1);
    assert_eq!(outcome.echoes_recorded, 1);
    assert!(outcome.cursor_saved);
    assert_eq!(state.cursor().next_sequence, 11);
    assert_eq!(state.mapping_save_count(), 1);
    assert_eq!(state.cursor_save_count(), 1);

    let mapping = state.mapping("Notes/new.md").expect("mapping");
    assert_eq!(mapping.drive_file_id, "fake-export-1");
    assert_eq!(mapping.parent_id, "folder-notes");
    assert_eq!(mapping.core_revision.as_deref(), Some("revision-10"));
    assert_eq!(mapping.core_sequence, Some(10));
    assert_eq!(provider.content("fake-export-1"), Some(b"new".as_slice()));

    let observation = DriveEchoObservation::new(mapping.drive_file_id.clone())
        .expect("observation")
        .with_drive_version(mapping.drive_version.clone().expect("version"))
        .expect("version");
    assert_eq!(
        echo_guard.decision_for(&observation),
        EchoDecision::SuppressAdapterEcho
    );
}

#[test]
fn update_then_tombstone_uses_confirmed_mapping_order_and_trash() {
    let (upsert_change, source) = upsert(
        10,
        "operation-update",
        "Notes/existing.md",
        "revision-10",
        b"updated",
    );
    let delete_change = tombstone(11, "operation-trash", "Notes/existing.md");
    let core = FakeCoreExportClient::new()
        .with_page(page(10, 12, vec![upsert_change, delete_change]))
        .with_content(source);
    let mut provider = FakeDriveExportProvider::new().with_file(
        "drive-existing",
        "folder-notes",
        "existing.md",
        MIME_TEXT_MARKDOWN,
        b"old".to_vec(),
        "provider-v1",
    );
    let mapping = existing_mapping("Notes/existing.md", "drive-existing", "provider-v1");
    let mut state = InMemoryExportStateStore::new(CoreChangeCursor::new(10), "root")
        .with_folder("Notes", "folder-notes")
        .with_mapping(mapping);
    let mut echo_guard = EchoGuard::new();

    let outcome = run_export_cycle(
        &core,
        &mut provider,
        &mut state,
        &mut echo_guard,
        &ExportRetryPolicy::default(),
        input(AdapterMode::Bidirectional, false),
    )
    .expect("export");

    assert_eq!(outcome.provider_mutations, 2);
    assert_eq!(state.mapping_save_count(), 2);
    assert_eq!(
        provider.content("drive-existing"),
        Some(b"updated".as_slice())
    );
    assert_eq!(provider.is_trashed("drive-existing"), Some(true));
    let mapping = state.mapping("Notes/existing.md").expect("mapping");
    assert_eq!(mapping.core_revision, None);
    assert_eq!(mapping.core_sequence, Some(11));
    assert_eq!(state.cursor().next_sequence, 12);
    assert_eq!(echo_guard.len(), 1);
}

#[test]
fn dry_run_verifies_content_without_mutating_or_consuming_cursor() {
    let (change, source) = upsert(
        10,
        "operation-dry-run",
        "Notes/dry.md",
        "revision-10",
        b"dry",
    );
    let core = FakeCoreExportClient::new()
        .with_page(page(10, 11, vec![change]))
        .with_content(source);
    let mut provider = FakeDriveExportProvider::new();
    let mut state = InMemoryExportStateStore::new(CoreChangeCursor::new(10), "root")
        .with_folder("Notes", "folder-notes");
    let mut echo_guard = EchoGuard::new();

    let outcome = run_export_cycle(
        &core,
        &mut provider,
        &mut state,
        &mut echo_guard,
        &ExportRetryPolicy::default(),
        input(AdapterMode::ExportOnly, true),
    )
    .expect("dry run");

    assert_eq!(outcome.work_items_planned, 1);
    assert_eq!(provider.mutation_count(), 0);
    assert_eq!(state.mapping_save_count(), 0);
    assert_eq!(state.cursor_save_count(), 0);
    assert_eq!(state.cursor().next_sequence, 10);
    assert!(echo_guard.is_empty());
    assert!(!outcome.cursor_saved);
}

#[test]
fn non_exporting_mode_skips_without_downloading_or_consuming_cursor() {
    let (change, _source) = upsert(
        10,
        "operation-skip",
        "Notes/skip.md",
        "revision-10",
        b"skip",
    );
    let core = FakeCoreExportClient::new().with_page(page(10, 11, vec![change]));
    let mut provider = FakeDriveExportProvider::new();
    let mut state = InMemoryExportStateStore::new(CoreChangeCursor::new(10), "root")
        .with_folder("Notes", "folder-notes");
    let mut echo_guard = EchoGuard::new();

    let outcome = run_export_cycle(
        &core,
        &mut provider,
        &mut state,
        &mut echo_guard,
        &ExportRetryPolicy::default(),
        input(AdapterMode::ImportOnly, false),
    )
    .expect("mode skip");

    assert_eq!(outcome.work_items_planned, 1);
    assert_eq!(provider.mutation_count(), 0);
    assert_eq!(state.cursor().next_sequence, 10);
    assert!(!outcome.cursor_saved);
}

#[test]
fn source_hash_mismatch_stops_before_provider_and_state_mutation() {
    let expected = b"abc";
    let vault_path = path("Notes/hash.md");
    let expected_hash = ContentSha256::from_content(expected);
    let change = CoreExportChange::upsert_file(
        10,
        "operation-hash",
        vault_path.clone(),
        "revision-10",
        expected_hash,
        3,
        "core-adapter",
    )
    .expect("change");
    let source = CoreFileContent::new(vault_path, "revision-10", expected_hash, 3, b"abd".to_vec())
        .expect("source");
    let core = FakeCoreExportClient::new()
        .with_page(page(10, 11, vec![change]))
        .with_content(source);
    let mut provider = FakeDriveExportProvider::new();
    let mut state = InMemoryExportStateStore::new(CoreChangeCursor::new(10), "root")
        .with_folder("Notes", "folder-notes");
    let mut echo_guard = EchoGuard::new();

    let error = run_export_cycle(
        &core,
        &mut provider,
        &mut state,
        &mut echo_guard,
        &ExportRetryPolicy::default(),
        input(AdapterMode::ExportOnly, false),
    )
    .expect_err("hash mismatch");

    assert_eq!(error, ExportError::SourceContentHashMismatch);
    assert_eq!(provider.mutation_count(), 0);
    assert_eq!(state.mapping_save_count(), 0);
    assert_eq!(state.cursor_save_count(), 0);
    assert!(echo_guard.is_empty());
}

#[test]
fn provider_conflict_is_non_retryable_and_preserves_mapping_order() {
    let (change, source) = upsert(
        10,
        "operation-conflict",
        "Notes/conflict.md",
        "revision-10",
        b"new",
    );
    let core = FakeCoreExportClient::new()
        .with_page(page(10, 11, vec![change]))
        .with_content(source);
    let mut provider = FakeDriveExportProvider::new().with_file(
        "drive-conflict",
        "folder-notes",
        "conflict.md",
        MIME_TEXT_MARKDOWN,
        b"old".to_vec(),
        "provider-current",
    );
    let mapping = existing_mapping("Notes/conflict.md", "drive-conflict", "provider-stale");
    let mut state =
        InMemoryExportStateStore::new(CoreChangeCursor::new(10), "root").with_mapping(mapping);
    let mut echo_guard = EchoGuard::new();

    let error = run_export_cycle(
        &core,
        &mut provider,
        &mut state,
        &mut echo_guard,
        &ExportRetryPolicy::default(),
        input(AdapterMode::ExportOnly, false),
    )
    .expect_err("provider conflict");

    assert!(matches!(
        error,
        ExportError::Provider {
            error,
            retry: ExportRetryDisposition::DoNotRetry,
        } if error.category() == DriveExportErrorCategory::Conflict
    ));
    assert_eq!(provider.mutation_count(), 0);
    assert_eq!(state.mapping_save_count(), 0);
    assert_eq!(state.cursor_save_count(), 0);
    assert!(echo_guard.is_empty());
}

#[test]
fn provider_rate_limit_uses_bounded_retry_and_does_not_advance_state() {
    let (change, source) = upsert(
        10,
        "operation-rate-limit",
        "Notes/rate.md",
        "revision-10",
        b"new",
    );
    let core = FakeCoreExportClient::new()
        .with_page(page(10, 11, vec![change]))
        .with_content(source);
    let mut provider = FakeDriveExportProvider::new()
        .with_file(
            "drive-rate",
            "folder-notes",
            "rate.md",
            MIME_TEXT_MARKDOWN,
            b"old".to_vec(),
            "provider-v1",
        )
        .with_error(
            "update_file",
            DriveExportError::new(
                "update_file",
                DriveExportErrorCategory::RateLimit,
                "provider quota limited",
            ),
        );
    let mapping = existing_mapping("Notes/rate.md", "drive-rate", "provider-v1");
    let mut state =
        InMemoryExportStateStore::new(CoreChangeCursor::new(10), "root").with_mapping(mapping);
    let mut echo_guard = EchoGuard::new();

    let error = run_export_cycle(
        &core,
        &mut provider,
        &mut state,
        &mut echo_guard,
        &ExportRetryPolicy::default(),
        input(AdapterMode::ExportOnly, false).with_provider_attempt(2),
    )
    .expect_err("rate limit");

    assert!(matches!(
        error,
        ExportError::Provider {
            retry: ExportRetryDisposition::RetryAfter(delay),
            ..
        } if delay == Duration::from_secs(60)
    ));
    assert_eq!(state.mapping_save_count(), 0);
    assert_eq!(state.cursor_save_count(), 0);
}

#[test]
fn mapping_save_failure_replays_provider_operation_without_duplicate_create() {
    let (change, source) = upsert(
        10,
        "operation-replay-create",
        "Notes/replay.md",
        "revision-10",
        b"replay",
    );
    let core = FakeCoreExportClient::new()
        .with_page(page(10, 11, vec![change]))
        .with_content(source);
    let mut provider = FakeDriveExportProvider::new();
    let mut state = InMemoryExportStateStore::new(CoreChangeCursor::new(10), "root")
        .with_folder("Notes", "folder-notes")
        .with_save_mapping_error(ExportStateError::new(
            "save_mapping",
            "injected mapping persistence failure",
        ));
    let mut echo_guard = EchoGuard::new();

    let first_error = run_export_cycle(
        &core,
        &mut provider,
        &mut state,
        &mut echo_guard,
        &ExportRetryPolicy::default(),
        input(AdapterMode::ExportOnly, false),
    )
    .expect_err("mapping save failure");

    assert!(matches!(first_error, ExportError::StateStore(_)));
    assert_eq!(provider.mutation_count(), 1);
    assert!(state.mapping("Notes/replay.md").is_none());
    assert_eq!(state.cursor().next_sequence, 10);
    assert!(echo_guard.is_empty());

    state.clear_save_mapping_error();
    let outcome = run_export_cycle(
        &core,
        &mut provider,
        &mut state,
        &mut echo_guard,
        &ExportRetryPolicy::default(),
        input(AdapterMode::ExportOnly, false),
    )
    .expect("replay");

    assert_eq!(provider.mutation_count(), 1);
    assert_eq!(outcome.provider_mutations, 1);
    assert!(state.mapping("Notes/replay.md").is_some());
    assert_eq!(state.cursor().next_sequence, 11);
    assert_eq!(echo_guard.len(), 1);
}

#[test]
fn cursor_save_failure_replays_page_without_repeating_confirmed_mutation() {
    let (change, source) = upsert(
        10,
        "operation-replay-cursor",
        "Notes/cursor.md",
        "revision-10",
        b"cursor",
    );
    let core = FakeCoreExportClient::new()
        .with_page(page(10, 11, vec![change]))
        .with_content(source);
    let mut provider = FakeDriveExportProvider::new();
    let mut state = InMemoryExportStateStore::new(CoreChangeCursor::new(10), "root")
        .with_folder("Notes", "folder-notes")
        .with_save_cursor_error(ExportStateError::new(
            "save_cursor",
            "injected cursor persistence failure",
        ));
    let mut echo_guard = EchoGuard::new();

    let first_error = run_export_cycle(
        &core,
        &mut provider,
        &mut state,
        &mut echo_guard,
        &ExportRetryPolicy::default(),
        input(AdapterMode::ExportOnly, false),
    )
    .expect_err("cursor save failure");

    assert!(matches!(first_error, ExportError::StateStore(_)));
    assert_eq!(provider.mutation_count(), 1);
    assert_eq!(state.mapping_save_count(), 1);
    assert_eq!(state.cursor().next_sequence, 10);
    assert_eq!(echo_guard.len(), 1);

    state.clear_save_cursor_error();
    let outcome = run_export_cycle(
        &core,
        &mut provider,
        &mut state,
        &mut echo_guard,
        &ExportRetryPolicy::default(),
        input(AdapterMode::ExportOnly, false),
    )
    .expect("cursor replay");

    assert_eq!(provider.mutation_count(), 1);
    assert_eq!(outcome.provider_mutations, 0);
    assert_eq!(state.mapping_save_count(), 1);
    assert_eq!(state.cursor().next_sequence, 11);
    assert!(outcome.cursor_saved);
}

#[test]
fn tombstone_without_mapping_is_safe_noop_that_can_advance_submit_cursor() {
    let core = FakeCoreExportClient::new().with_page(page(
        10,
        11,
        vec![tombstone(10, "operation-missing-trash", "Notes/missing.md")],
    ));
    let mut provider = FakeDriveExportProvider::new();
    let mut state = InMemoryExportStateStore::new(CoreChangeCursor::new(10), "root");
    let mut echo_guard = EchoGuard::new();

    let outcome = run_export_cycle(
        &core,
        &mut provider,
        &mut state,
        &mut echo_guard,
        &ExportRetryPolicy::default(),
        input(AdapterMode::ExportOnly, false),
    )
    .expect("missing mapping tombstone");

    assert_eq!(provider.mutation_count(), 0);
    assert_eq!(state.mapping_save_count(), 0);
    assert_eq!(state.cursor().next_sequence, 11);
    assert!(outcome.cursor_saved);
}

#[test]
fn debug_output_redacts_core_and_provider_file_bytes() {
    let content = CoreFileContent::new(
        path("Notes/secret.md"),
        "revision-secret",
        ContentSha256::from_content(b"secret-bytes"),
        12,
        b"secret-bytes".to_vec(),
    )
    .expect("content");
    let request = DriveCreateExportRequest {
        operation_id: "operation-secret".to_owned(),
        parent_id: "root".to_owned(),
        name: "secret.md".to_owned(),
        mime_type: MIME_TEXT_MARKDOWN.to_owned(),
        content_sha256: ContentSha256::from_content(b"secret-bytes"),
        content: b"secret-bytes".to_vec(),
    };

    let content_debug = format!("{content:?}");
    let request_debug = format!("{request:?}");

    assert!(!content_debug.contains("secret-bytes"));
    assert!(!request_debug.contains("secret-bytes"));
    assert!(content_debug.contains("redacted-file-bytes"));
    assert!(request_debug.contains("redacted-file-bytes"));
}
