use super::*;
use crate::config::{AdapterMode, DeleteSafetyConfig};
use crate::drive::{
    DriveMetadata, FakeDriveProvider, ProviderError, ProviderErrorCategory, MIME_GOOGLE_FOLDER,
    MIME_TEXT_MARKDOWN,
};
use crate::scan::{
    plan_full_scan, FullScanInput, FullScanPlan, ScanSkipReason, SkippedScanEntry,
};
use crate::state::{GDriveMapping, SafeTimestamp, VaultPath};

fn timestamp(value: &str) -> SafeTimestamp {
    SafeTimestamp::new(value).expect("timestamp")
}

fn path(value: &str) -> VaultPath {
    VaultPath::new(value).expect("path")
}

fn mapping(
    provider_id: &str,
    path_value: &str,
    parent_id: &str,
    name: &str,
    candidate_since: Option<&str>,
) -> GDriveMapping {
    let mut mapping = GDriveMapping::new(path(path_value), provider_id, parent_id, name)
        .expect("mapping");
    mapping.checksum = Some(format!("checksum-{provider_id}"));
    mapping.core_revision = Some(format!("core-revision-{provider_id}"));
    mapping.core_sequence = Some(10);
    mapping.delete_candidate_since = candidate_since.map(timestamp);
    mapping
}

fn scan_observation(
    provider: &FakeDriveProvider,
    mappings: &[GDriveMapping],
    observed_at: &str,
) -> DeleteScanObservation {
    let result = plan_full_scan(
        provider,
        FullScanInput {
            root_folder_id: "root",
            mode: AdapterMode::ImportOnly,
            dry_run: false,
            observed_at: timestamp(observed_at),
            mappings,
        },
    );
    DeleteScanObservation::from_full_scan_result(result, mappings)
}

fn input(
    run_id: &str,
    observed_at: &str,
    observation: DeleteScanObservation,
) -> DeleteReconciliationInput {
    DeleteReconciliationInput::new(
        run_id,
        AdapterMode::ImportOnly,
        false,
        timestamp(observed_at),
        observation,
    )
    .expect("input")
}

fn permissive_delete_safety() -> DeleteSafetyConfig {
    DeleteSafetyConfig::new(100, 100).expect("delete safety")
}

#[test]
fn first_absence_only_marks_candidate() {
    let mappings = vec![mapping(
        "file-1",
        "Notes/missing.md",
        "root",
        "missing.md",
        None,
    )];
    let provider = FakeDriveProvider::new();
    let observation = scan_observation(&provider, &mappings, "2026-07-11T08:00:00Z");
    let mut state = InMemoryDeleteCandidateStateStore::new(mappings).expect("state");
    let mut core = FakeCoreDeleteGateway::new();

    let outcome = run_delete_reconciliation(
        &mut core,
        &mut state,
        permissive_delete_safety(),
        input("scan-1", "2026-07-11T08:00:00Z", observation),
    )
    .expect("reconciliation");

    assert_eq!(outcome.candidates_marked, 1);
    assert_eq!(outcome.confirmed_candidates, 0);
    assert_eq!(outcome.delete_submissions, 0);
    assert_eq!(core.guard_evaluation_count(), 0);
    assert_eq!(core.delete_call_count(), 0);
    assert_eq!(
        state
            .mapping(&path("Notes/missing.md"))
            .and_then(|mapping| mapping.delete_candidate_since.as_ref())
            .map(SafeTimestamp::as_str),
        Some("2026-07-11T08:00:00Z")
    );
}

#[test]
fn repeated_absence_submits_core_delete_and_retires_mapping() {
    let mappings = vec![mapping(
        "file-1",
        "Notes/missing.md",
        "root",
        "missing.md",
        None,
    )];
    let provider = FakeDriveProvider::new();
    let mut state = InMemoryDeleteCandidateStateStore::new(mappings).expect("state");
    let mut core = FakeCoreDeleteGateway::new();

    let first_mappings = state.mappings();
    let first_observation = scan_observation(
        &provider,
        &first_mappings,
        "2026-07-11T08:00:00Z",
    );
    run_delete_reconciliation(
        &mut core,
        &mut state,
        permissive_delete_safety(),
        input("scan-1", "2026-07-11T08:00:00Z", first_observation),
    )
    .expect("first reconciliation");

    let second_mappings = state.mappings();
    let second_observation = scan_observation(
        &provider,
        &second_mappings,
        "2026-07-11T09:00:00Z",
    );
    let outcome = run_delete_reconciliation(
        &mut core,
        &mut state,
        permissive_delete_safety(),
        input("scan-2", "2026-07-11T09:00:00Z", second_observation),
    )
    .expect("second reconciliation");

    assert_eq!(outcome.confirmed_candidates, 1);
    assert_eq!(outcome.delete_submissions, 1);
    assert_eq!(outcome.mappings_retired, 1);
    assert_eq!(core.guard_evaluation_count(), 1);
    assert_eq!(core.delete_call_count(), 1);
    assert_eq!(state.mapping_count(), 0);
}

#[test]
fn recovered_file_clears_candidate_without_core_delete() {
    let existing_mapping = mapping(
        "file-1",
        "Notes/recovered.md",
        "root",
        "recovered.md",
        Some("2026-07-11T08:00:00Z"),
    );
    let metadata = DriveMetadata::new_file("file-1", "recovered.md", MIME_TEXT_MARKDOWN)
        .with_parent("root")
        .with_md5_checksum("checksum-file-1");
    let provider = FakeDriveProvider::new().with_child("root", metadata);
    let mappings = vec![existing_mapping];
    let observation = scan_observation(&provider, &mappings, "2026-07-11T09:00:00Z");
    let mut state = InMemoryDeleteCandidateStateStore::new(mappings).expect("state");
    let mut core = FakeCoreDeleteGateway::new();

    let outcome = run_delete_reconciliation(
        &mut core,
        &mut state,
        permissive_delete_safety(),
        input("scan-recovery", "2026-07-11T09:00:00Z", observation),
    )
    .expect("reconciliation");

    assert_eq!(outcome.candidates_cleared, 1);
    assert_eq!(outcome.delete_submissions, 0);
    assert_eq!(core.guard_evaluation_count(), 0);
    assert_eq!(
        state
            .mapping(&path("Notes/recovered.md"))
            .and_then(|mapping| mapping.delete_candidate_since.as_ref()),
        None
    );
}

#[test]
fn folder_movement_is_not_treated_as_disappearance() {
    let existing_mapping = mapping(
        "file-1",
        "Old/note.md",
        "old-folder",
        "note.md",
        Some("2026-07-11T08:00:00Z"),
    );
    let moved_folder = DriveMetadata::new_special("new-folder", "New", MIME_GOOGLE_FOLDER)
        .with_parent("root");
    let moved_file = DriveMetadata::new_file("file-1", "note.md", MIME_TEXT_MARKDOWN)
        .with_parent("new-folder")
        .with_md5_checksum("checksum-file-1");
    let provider = FakeDriveProvider::new()
        .with_child("root", moved_folder)
        .with_child("new-folder", moved_file);
    let mappings = vec![existing_mapping];
    let observation = scan_observation(&provider, &mappings, "2026-07-11T09:00:00Z");
    let mut state = InMemoryDeleteCandidateStateStore::new(mappings).expect("state");
    let mut core = FakeCoreDeleteGateway::new();

    let outcome = run_delete_reconciliation(
        &mut core,
        &mut state,
        permissive_delete_safety(),
        input("scan-move", "2026-07-11T09:00:00Z", observation),
    )
    .expect("reconciliation");

    assert_eq!(outcome.confirmed_candidates, 0);
    assert_eq!(outcome.delete_submissions, 0);
    assert_eq!(outcome.candidates_cleared, 1);
    assert!(outcome.notices.iter().any(|notice| matches!(
        notice,
        DeleteSafetyNotice::ProviderIdentityMoved(moved) if moved.provider_id == "file-1"
    )));
    assert_eq!(
        state
            .mapping(&path("Old/note.md"))
            .and_then(|mapping| mapping.delete_candidate_since.as_ref()),
        None
    );
}

#[test]
fn auth_scope_loss_blocks_without_candidate_mutation() {
    let mappings = vec![mapping(
        "file-1",
        "Notes/private.md",
        "root",
        "private.md",
        None,
    )];
    let provider = FakeDriveProvider::new().with_error(
        "list_children",
        ProviderError::new(
            "list_children",
            ProviderErrorCategory::Auth,
            "provider authorization unavailable",
        ),
    );
    let observation = scan_observation(&provider, &mappings, "2026-07-11T09:00:00Z");
    let mut state = InMemoryDeleteCandidateStateStore::new(mappings).expect("state");
    let mut core = FakeCoreDeleteGateway::new();

    let outcome = run_delete_reconciliation(
        &mut core,
        &mut state,
        permissive_delete_safety(),
        input("scan-auth", "2026-07-11T09:00:00Z", observation),
    )
    .expect("reconciliation");

    assert_eq!(
        outcome.blocked,
        Some(DeleteBlockReason::ScanUnreliable(
            DeleteScanIssue::PermissionLoss
        ))
    );
    assert_eq!(state.mark_count(), 0);
    assert_eq!(state.clear_count(), 0);
    assert_eq!(core.guard_evaluation_count(), 0);
    assert_eq!(core.delete_call_count(), 0);
}

#[test]
fn provider_failure_and_incomplete_scan_are_distinct_safe_blocks() {
    let mappings = vec![mapping(
        "file-1",
        "Notes/note.md",
        "root",
        "note.md",
        None,
    )];
    let provider = FakeDriveProvider::new().with_error(
        "list_children",
        ProviderError::new(
            "list_children",
            ProviderErrorCategory::ProviderUnavailable,
            "provider temporarily unavailable",
        ),
    );
    let provider_failure = scan_observation(&provider, &mappings, "2026-07-11T09:00:00Z");
    assert_eq!(
        provider_failure,
        DeleteScanObservation::Unreliable(DeleteScanIssue::ProviderFailure)
    );

    let incomplete_plan = FullScanPlan {
        unsupported: vec![SkippedScanEntry {
            provider_id: "folder-cycle".to_owned(),
            path: Some(path("Cycle")),
            reason: ScanSkipReason::FolderCycle,
        }],
        ..FullScanPlan::default()
    };
    let incomplete = CompleteDeleteScan::from_full_scan(incomplete_plan, &mappings);
    assert_eq!(
        incomplete,
        DeleteScanObservation::Unreliable(DeleteScanIssue::IncompleteScan)
    );
}

#[test]
fn repeated_folder_disappearance_is_blocked_as_mass_delete() {
    let mappings = (0..12)
        .map(|index| {
            mapping(
                &format!("file-{index}"),
                &format!("Folder/note-{index}.md"),
                "folder",
                &format!("note-{index}.md"),
                None,
            )
        })
        .collect::<Vec<_>>();
    let provider = FakeDriveProvider::new();
    let mut state = InMemoryDeleteCandidateStateStore::new(mappings).expect("state");
    let mut core = FakeCoreDeleteGateway::new();
    let delete_safety = DeleteSafetyConfig::new(5, 100).expect("delete safety");

    let first_mappings = state.mappings();
    let first_observation = scan_observation(
        &provider,
        &first_mappings,
        "2026-07-11T08:00:00Z",
    );
    let first = run_delete_reconciliation(
        &mut core,
        &mut state,
        delete_safety,
        input("folder-scan-1", "2026-07-11T08:00:00Z", first_observation),
    )
    .expect("first reconciliation");
    assert_eq!(first.candidates_marked, 12);
    assert_eq!(first.delete_submissions, 0);

    let second_mappings = state.mappings();
    let second_observation = scan_observation(
        &provider,
        &second_mappings,
        "2026-07-11T09:00:00Z",
    );
    let second = run_delete_reconciliation(
        &mut core,
        &mut state,
        delete_safety,
        input(
            "folder-scan-2",
            "2026-07-11T09:00:00Z",
            second_observation,
        ),
    )
    .expect("second reconciliation");

    assert_eq!(second.confirmed_candidates, 12);
    assert_eq!(
        second.blocked,
        Some(DeleteBlockReason::AdapterDeleteCountExceeded {
            proposed_delete_count: 12,
            max_deletes_per_run: 5,
        })
    );
    assert_eq!(second.delete_submissions, 0);
    assert_eq!(core.guard_evaluation_count(), 0);
    assert_eq!(core.delete_call_count(), 0);
    assert_eq!(state.mapping_count(), 12);
}

#[test]
fn high_delete_ratio_blocks_even_below_count_threshold() {
    let mappings = (0..10)
        .map(|index| {
            mapping(
                &format!("file-{index}"),
                &format!("Notes/note-{index}.md"),
                "root",
                &format!("note-{index}.md"),
                if index == 0 {
                    Some("2026-07-11T08:00:00Z")
                } else {
                    None
                },
            )
        })
        .collect::<Vec<_>>();
    let mut provider = FakeDriveProvider::new();
    for index in 1..10 {
        let metadata = DriveMetadata::new_file(
            format!("file-{index}"),
            format!("note-{index}.md"),
            MIME_TEXT_MARKDOWN,
        )
        .with_parent("root")
        .with_md5_checksum(format!("checksum-file-{index}"));
        provider = provider.with_child("root", metadata);
    }
    let observation = scan_observation(&provider, &mappings, "2026-07-11T09:00:00Z");
    let mut state = InMemoryDeleteCandidateStateStore::new(mappings).expect("state");
    let mut core = FakeCoreDeleteGateway::new();

    let outcome = run_delete_reconciliation(
        &mut core,
        &mut state,
        DeleteSafetyConfig::new(10, 5).expect("delete safety"),
        input("ratio-scan", "2026-07-11T09:00:00Z", observation),
    )
    .expect("reconciliation");

    assert_eq!(
        outcome.blocked,
        Some(DeleteBlockReason::AdapterDeleteRatioExceeded {
            proposed_delete_count: 1,
            total_files_before_run: 10,
            max_delete_ratio_percent: 5,
        })
    );
    assert_eq!(core.guard_evaluation_count(), 0);
    assert_eq!(core.delete_call_count(), 0);
}

#[test]
fn core_manual_unlock_requirement_stays_unavailable() {
    let mappings = vec![mapping(
        "file-1",
        "Notes/missing.md",
        "root",
        "missing.md",
        Some("2026-07-11T08:00:00Z"),
    )];
    let provider = FakeDriveProvider::new();
    let observation = scan_observation(&provider, &mappings, "2026-07-11T09:00:00Z");
    let decision = CoreDeleteGuardDecision::BlockedRequiresManualUnlock {
        proposed_delete_count: 1,
        total_files_before_run: 1,
        reason: CoreDeleteGuardBlockReason::DeleteRatio,
    };
    let mut core = FakeCoreDeleteGateway::new().with_guard_decision(decision.clone());
    let mut state = InMemoryDeleteCandidateStateStore::new(mappings).expect("state");

    let outcome = run_delete_reconciliation(
        &mut core,
        &mut state,
        permissive_delete_safety(),
        input("manual-unlock", "2026-07-11T09:00:00Z", observation),
    )
    .expect("reconciliation");

    assert_eq!(
        outcome.blocked,
        Some(DeleteBlockReason::CoreGuardBlocked(decision))
    );
    assert_eq!(
        outcome.manual_unlock,
        ManualDeleteUnlockAvailability::Unavailable
    );
    assert_eq!(core.delete_call_count(), 0);
    assert_eq!(state.mapping_count(), 1);
}

#[test]
fn dry_run_previews_confirmed_delete_without_mutating_state_or_core() {
    let mappings = vec![mapping(
        "file-1",
        "Notes/missing.md",
        "root",
        "missing.md",
        Some("2026-07-11T08:00:00Z"),
    )];
    let provider = FakeDriveProvider::new();
    let observation = scan_observation(&provider, &mappings, "2026-07-11T09:00:00Z");
    let mut core = FakeCoreDeleteGateway::new();
    let mut state = InMemoryDeleteCandidateStateStore::new(mappings).expect("state");
    let input = DeleteReconciliationInput::new(
        "dry-run",
        AdapterMode::DryRun,
        true,
        timestamp("2026-07-11T09:00:00Z"),
        observation,
    )
    .expect("input");

    let outcome = run_delete_reconciliation(
        &mut core,
        &mut state,
        permissive_delete_safety(),
        input,
    )
    .expect("reconciliation");

    assert_eq!(outcome.execution, Some(DeleteExecution::DryRun));
    assert_eq!(outcome.delete_previews, 1);
    assert_eq!(outcome.delete_submissions, 0);
    assert_eq!(core.guard_evaluation_count(), 1);
    assert_eq!(core.delete_call_count(), 0);
    assert_eq!(state.retire_count(), 0);
    assert_eq!(state.mapping_count(), 1);
}

#[test]
fn unsafe_core_rejection_stops_remaining_delete_submissions() {
    let mappings = vec![
        mapping(
            "file-a",
            "Notes/a.md",
            "root",
            "a.md",
            Some("2026-07-11T08:00:00Z"),
        ),
        mapping(
            "file-b",
            "Notes/b.md",
            "root",
            "b.md",
            Some("2026-07-11T08:00:00Z"),
        ),
    ];
    let provider = FakeDriveProvider::new();
    let observation = scan_observation(&provider, &mappings, "2026-07-11T09:00:00Z");
    let mut core = FakeCoreDeleteGateway::new().with_response(
        path("Notes/a.md"),
        CoreDeleteResponse::Rejected {
            reason: CoreDeleteRejectedReason::UnsafeDelete,
        },
    );
    let mut state = InMemoryDeleteCandidateStateStore::new(mappings).expect("state");

    let outcome = run_delete_reconciliation(
        &mut core,
        &mut state,
        permissive_delete_safety(),
        input("core-reject", "2026-07-11T09:00:00Z", observation),
    )
    .expect("reconciliation");

    assert_eq!(
        outcome.blocked,
        Some(DeleteBlockReason::CoreRejectedUnsafeDelete)
    );
    assert_eq!(core.delete_call_count(), 1);
    assert_eq!(state.mapping_count(), 2);
}

#[test]
fn debug_output_redacts_delete_idempotency_key() {
    let request = CoreDeleteRequest {
        operation_id: "secret-delete-operation".to_owned(),
        path: path("Notes/missing.md"),
        base_revision_id: Some("revision-1".to_owned()),
    };

    let debug = format!("{request:?}");

    assert!(!debug.contains("secret-delete-operation"));
    assert!(debug.contains("redacted-idempotency-key"));
}
