use super::*;
use crate::drive::{DriveMetadata, MIME_GOOGLE_FOLDER, MIME_TEXT_MARKDOWN};
use crate::state::EchoGuardEntry;

const POLLED_AT: &str = "2026-07-10T12:00:00Z";

fn timestamp(value: &str) -> SafeTimestamp {
    SafeTimestamp::new(value).expect("timestamp")
}

fn synced_cursor(sync_token: &str) -> DriveChangeCursor {
    let mut cursor = DriveChangeCursor::new("initial-token").expect("cursor");
    cursor
        .finish_batch(sync_token, timestamp("2026-07-10T11:00:00Z"))
        .expect("finish cursor");
    cursor
}

fn markdown_change(provider_id: &str, checksum: &str) -> DriveChangeEntry {
    DriveChangeEntry::file(
        DriveMetadata::new_file(provider_id, format!("{provider_id}.md"), MIME_TEXT_MARKDOWN)
            .with_parent("root")
            .with_md5_checksum(checksum),
    )
    .expect("change")
}

fn cycle_input() -> ChangeFeedCycleInput {
    ChangeFeedCycleInput::new(AdapterMode::ImportOnly, false, timestamp(POLLED_AT))
}

#[derive(Default)]
struct RecordingProcessor {
    batches: Vec<ChangeWorkBatch>,
    fail: bool,
}

impl RecordingProcessor {
    fn failing() -> Self {
        Self {
            batches: Vec::new(),
            fail: true,
        }
    }
}

impl ChangeWorkProcessor for RecordingProcessor {
    fn process(&mut self, batch: &ChangeWorkBatch) -> Result<(), ChangeProcessingError> {
        self.batches.push(batch.clone());
        if self.fail {
            return Err(ChangeProcessingError::new("injected processing failure"));
        }
        Ok(())
    }
}

#[test]
fn missing_cursor_runs_full_scan_before_saving_start_token() {
    let provider = FakeDriveChangeFeedProvider::new("start-2").expect("provider");
    let mut store = InMemoryDriveCursorStore::new(None);
    let mut processor = RecordingProcessor::default();

    let outcome = run_change_feed_cycle(
        &provider,
        &mut store,
        &mut processor,
        &EchoGuard::new(),
        &ProviderBackoffPolicy::default(),
        cycle_input(),
    )
    .expect("cycle");

    assert!(outcome.full_scan_used);
    assert_eq!(outcome.pages_polled, 0);
    assert_eq!(store.save_count(), 1);
    assert_eq!(
        store
            .cursor()
            .and_then(|cursor| cursor.sync_token.as_deref()),
        Some("start-2")
    );
    assert_eq!(
        processor.batches,
        vec![ChangeWorkBatch::full_scan(
            FullScanFallbackReason::InitialCursor
        )]
    );
}

#[test]
fn provider_cursor_invalidation_falls_back_to_full_scan() {
    let provider = FakeDriveChangeFeedProvider::new("unused")
        .expect("provider")
        .with_poll(
            "sync-1",
            DriveChangePoll::cursor_invalidated("sync-2").expect("invalidated"),
        );
    let mut store = InMemoryDriveCursorStore::new(Some(synced_cursor("sync-1")));
    let mut processor = RecordingProcessor::default();

    let outcome = run_change_feed_cycle(
        &provider,
        &mut store,
        &mut processor,
        &EchoGuard::new(),
        &ProviderBackoffPolicy::default(),
        cycle_input(),
    )
    .expect("cycle");

    assert!(outcome.full_scan_used);
    assert_eq!(
        store
            .cursor()
            .and_then(|cursor| cursor.sync_token.as_deref()),
        Some("sync-2")
    );
    assert_eq!(
        processor.batches[0],
        ChangeWorkBatch::full_scan(FullScanFallbackReason::ProviderCursorInvalidated)
    );
}

#[test]
fn duplicate_entries_coalesce_and_conflicting_history_supersedes_incremental_work() {
    let duplicate = markdown_change("file-a", "checksum-a");
    let old = markdown_change("file-b", "checksum-old");
    let new = markdown_change("file-b", "checksum-new");
    let forward = vec![
        duplicate.clone(),
        old.clone(),
        duplicate.clone(),
        new.clone(),
    ];
    let reverse = vec![new, duplicate.clone(), old, duplicate];

    let forward_batch =
        classify_drive_changes(forward, AdapterMode::ImportOnly, false, &EchoGuard::new());
    let reverse_batch =
        classify_drive_changes(reverse, AdapterMode::ImportOnly, false, &EchoGuard::new());

    assert_eq!(forward_batch, reverse_batch);
    assert_eq!(forward_batch.items.len(), 1);
    assert!(matches!(
        &forward_batch.items[0],
        ChangeWorkItem::FullScan {
            reason: FullScanFallbackReason::ConflictingChangeHistory,
            provider_id: Some(provider_id),
        } if provider_id == "file-b"
    ));
}

#[test]
fn successful_multi_page_poll_saves_only_final_cursor() {
    let provider = FakeDriveChangeFeedProvider::new("unused")
        .expect("provider")
        .with_poll(
            "sync-1",
            DriveChangePoll::Page(
                DriveChangePage::intermediate(
                    vec![markdown_change("file-b", "checksum-b")],
                    "page-2",
                )
                .expect("page"),
            ),
        )
        .with_poll(
            "page-2",
            DriveChangePoll::Page(
                DriveChangePage::final_page(
                    vec![markdown_change("file-a", "checksum-a")],
                    "sync-2",
                )
                .expect("page"),
            ),
        );
    let mut store = InMemoryDriveCursorStore::new(Some(synced_cursor("sync-1")));
    let mut processor = RecordingProcessor::default();

    let outcome = run_change_feed_cycle(
        &provider,
        &mut store,
        &mut processor,
        &EchoGuard::new(),
        &ProviderBackoffPolicy::default(),
        cycle_input(),
    )
    .expect("cycle");

    assert_eq!(outcome.pages_polled, 2);
    assert_eq!(outcome.entries_received, 2);
    assert_eq!(outcome.work_items_processed, 2);
    assert_eq!(store.save_count(), 1);
    assert_eq!(
        store
            .cursor()
            .and_then(|cursor| cursor.sync_token.as_deref()),
        Some("sync-2")
    );
    assert!(processor.batches[0].items.iter().all(|item| matches!(
        item,
        ChangeWorkItem::Import {
            execution: ImportExecution::Submit,
            ..
        }
    )));
}

#[test]
fn failed_processing_does_not_advance_cursor() {
    let provider = FakeDriveChangeFeedProvider::new("unused")
        .expect("provider")
        .with_poll(
            "sync-1",
            DriveChangePoll::Page(
                DriveChangePage::final_page(
                    vec![markdown_change("file-a", "checksum-a")],
                    "sync-2",
                )
                .expect("page"),
            ),
        );
    let mut store = InMemoryDriveCursorStore::new(Some(synced_cursor("sync-1")));
    let mut processor = RecordingProcessor::failing();

    let error = run_change_feed_cycle(
        &provider,
        &mut store,
        &mut processor,
        &EchoGuard::new(),
        &ProviderBackoffPolicy::default(),
        cycle_input(),
    )
    .expect_err("processing failure");

    assert!(matches!(error, ChangeFeedError::Processing(_)));
    assert_eq!(store.save_count(), 0);
    assert_eq!(
        store
            .cursor()
            .and_then(|cursor| cursor.sync_token.as_deref()),
        Some("sync-1")
    );
}

#[test]
fn cursor_save_failure_leaves_processed_batch_replayable() {
    let provider = FakeDriveChangeFeedProvider::new("unused")
        .expect("provider")
        .with_poll(
            "sync-1",
            DriveChangePoll::Page(
                DriveChangePage::final_page(
                    vec![markdown_change("file-a", "checksum-a")],
                    "sync-2",
                )
                .expect("page"),
            ),
        );
    let mut store = InMemoryDriveCursorStore::new(Some(synced_cursor("sync-1"))).with_save_error(
        CursorStoreError::new("save_cursor", "injected persistence failure"),
    );
    let mut processor = RecordingProcessor::default();

    let error = run_change_feed_cycle(
        &provider,
        &mut store,
        &mut processor,
        &EchoGuard::new(),
        &ProviderBackoffPolicy::default(),
        cycle_input(),
    )
    .expect_err("cursor save failure");

    assert!(matches!(error, ChangeFeedError::CursorStore(_)));
    assert_eq!(processor.batches.len(), 1);
    assert_eq!(store.save_count(), 0);
    assert_eq!(
        store
            .cursor()
            .and_then(|cursor| cursor.sync_token.as_deref()),
        Some("sync-1")
    );
}

#[test]
fn matching_echo_is_classified_as_export_confirmation() {
    let mut echo_guard = EchoGuard::new();
    echo_guard.record_exported_write(EchoGuardEntry {
        drive_file_id: "file-a".to_owned(),
        checksum: Some("checksum-a".to_owned()),
        drive_version: Some("version-2".to_owned()),
        core_revision: Some("revision-2".to_owned()),
        core_sequence: Some(2),
        exported_at: timestamp("2026-07-10T11:30:00Z"),
    });
    let change = markdown_change("file-a", "checksum-a")
        .with_drive_version("version-2")
        .expect("version");

    let batch =
        classify_drive_changes(vec![change], AdapterMode::Bidirectional, false, &echo_guard);

    assert!(matches!(
        &batch.items[0],
        ChangeWorkItem::ConfirmExportEcho { observation }
            if observation.drive_file_id == "file-a"
    ));
}

#[test]
fn full_scan_fallback_supersedes_import_and_echo_work() {
    let mut echo_guard = EchoGuard::new();
    echo_guard.record_exported_write(EchoGuardEntry {
        drive_file_id: "echo".to_owned(),
        checksum: Some("checksum-echo".to_owned()),
        drive_version: Some("version-2".to_owned()),
        core_revision: Some("revision-2".to_owned()),
        core_sequence: Some(2),
        exported_at: timestamp("2026-07-10T11:30:00Z"),
    });
    let echo = markdown_change("echo", "checksum-echo")
        .with_drive_version("version-2")
        .expect("version");
    let import = markdown_change("import", "checksum-import");
    let removed = DriveChangeEntry::removed("removed").expect("removed");

    let batch = classify_drive_changes(
        vec![echo, import, removed],
        AdapterMode::Bidirectional,
        false,
        &echo_guard,
    );

    assert_eq!(batch.items.len(), 1);
    assert!(matches!(
        &batch.items[0],
        ChangeWorkItem::FullScan {
            reason: FullScanFallbackReason::RemovedEntry,
            provider_id: Some(provider_id),
        } if provider_id == "removed"
    ));
}

#[test]
fn removed_folder_and_missing_metadata_use_full_scan_work() {
    let removed = DriveChangeEntry::removed("removed").expect("removed");
    let folder = DriveChangeEntry::file(
        DriveMetadata::new_special("folder", "Notes", MIME_GOOGLE_FOLDER).with_parent("root"),
    )
    .expect("folder");
    let missing = DriveChangeEntry::metadata_missing("missing").expect("missing");

    let batch = classify_drive_changes(
        vec![removed, folder, missing],
        AdapterMode::ImportOnly,
        false,
        &EchoGuard::new(),
    );

    assert_eq!(batch.items.len(), 3);
    assert!(batch.requires_full_scan());
    assert!(batch.items.iter().any(|item| matches!(
        item,
        ChangeWorkItem::FullScan {
            reason: FullScanFallbackReason::RemovedEntry,
            ..
        }
    )));
    assert!(batch.items.iter().any(|item| matches!(
        item,
        ChangeWorkItem::FullScan {
            reason: FullScanFallbackReason::FolderChanged,
            ..
        }
    )));
    assert!(batch.items.iter().any(|item| matches!(
        item,
        ChangeWorkItem::FullScan {
            reason: FullScanFallbackReason::MissingMetadata,
            ..
        }
    )));
}

#[test]
fn non_import_mode_keeps_change_without_download_or_submit_work() {
    let batch = classify_drive_changes(
        vec![markdown_change("file-a", "checksum-a")],
        AdapterMode::ExportOnly,
        false,
        &EchoGuard::new(),
    );

    assert_eq!(
        batch.items,
        vec![ChangeWorkItem::SkipImportByMode {
            provider_id: "file-a".to_owned(),
            mode: AdapterMode::ExportOnly,
        }]
    );
}

#[test]
fn rate_limit_error_uses_bounded_exponential_backoff() {
    let provider = FakeDriveChangeFeedProvider::new("unused")
        .expect("provider")
        .with_error(
            "list_changes",
            ProviderError::new(
                "list_changes",
                ProviderErrorCategory::RateLimit,
                "provider quota limited",
            ),
        );
    let mut store = InMemoryDriveCursorStore::new(Some(synced_cursor("sync-1")));
    let mut processor = RecordingProcessor::default();

    let error = run_change_feed_cycle(
        &provider,
        &mut store,
        &mut processor,
        &EchoGuard::new(),
        &ProviderBackoffPolicy::default(),
        cycle_input().with_provider_attempt(2),
    )
    .expect_err("rate limit");

    assert!(matches!(
        error,
        ChangeFeedError::Provider {
            retry: RetryDisposition::RetryAfter(delay),
            ..
        } if delay == Duration::from_secs(60)
    ));
    assert_eq!(store.save_count(), 0);
}

#[test]
fn debouncer_coalesces_burst_triggers_into_one_due_poll() {
    let mut debouncer = ChangePollDebouncer::new(Duration::from_millis(250));
    debouncer.record_trigger(ChangePollTrigger::PollTick, 1_000);
    debouncer.record_trigger(ChangePollTrigger::ProviderSignal, 1_100);
    debouncer.record_trigger(ChangePollTrigger::ProviderSignal, 1_050);

    assert!(debouncer.take_due(1_349).is_none());
    let due = debouncer.take_due(1_350).expect("due poll");

    assert_eq!(due.trigger_count, 3);
    assert_eq!(due.first_trigger_millis, 1_000);
    assert_eq!(due.last_trigger_millis, 1_100);
    assert_eq!(
        due.trigger_kinds,
        BTreeSet::from([
            ChangePollTrigger::PollTick,
            ChangePollTrigger::ProviderSignal,
        ])
    );
    assert!(!debouncer.has_pending());
}
