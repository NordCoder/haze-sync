use super::*;
use crate::config::{AdapterMode, DeleteSafetyConfig};
use crate::drive::{DriveMetadata, FakeDriveProvider, MIME_TEXT_MARKDOWN};
use crate::scan::{plan_full_scan, DeleteCandidatePlan, FullScanInput, FullScanPlan};
use crate::state::{GDriveMapping, SafeTimestamp, VaultPath};

fn timestamp(value: &str) -> SafeTimestamp {
    SafeTimestamp::new(value).expect("timestamp")
}

fn path(value: &str) -> VaultPath {
    VaultPath::new(value).expect("path")
}

fn mapping(provider_id: &str, path_value: &str, candidate_since: Option<&str>) -> GDriveMapping {
    let name = path_value.rsplit('/').next().expect("name");
    let mut mapping =
        GDriveMapping::new(path(path_value), provider_id, "root", name).expect("mapping");
    mapping.checksum = Some(format!("checksum-{provider_id}"));
    mapping.core_revision = Some(format!("core-revision-{provider_id}"));
    mapping.delete_candidate_since = candidate_since.map(timestamp);
    mapping
}

fn observation(
    provider: &FakeDriveProvider,
    mappings: &[GDriveMapping],
    observed_at: &str,
) -> DeleteScanObservation {
    DeleteScanObservation::from_full_scan_result(
        plan_full_scan(
            provider,
            FullScanInput {
                root_folder_id: "root",
                mode: AdapterMode::ImportOnly,
                dry_run: false,
                observed_at: timestamp(observed_at),
                mappings,
            },
        ),
        mappings,
    )
}

fn reconciliation_input(
    mode: AdapterMode,
    observed_at: &str,
    observation: DeleteScanObservation,
) -> DeleteReconciliationInput {
    DeleteReconciliationInput::new(
        "clean-review",
        mode,
        mode == AdapterMode::DryRun,
        timestamp(observed_at),
        observation,
    )
    .expect("input")
}

fn permissive_safety() -> DeleteSafetyConfig {
    DeleteSafetyConfig::new(100, 100).expect("delete safety")
}

#[test]
fn stale_candidate_timestamp_blocks_before_core_delete() {
    let scan_mappings = vec![mapping(
        "file-1",
        "Notes/missing.md",
        Some("2026-07-11T08:00:00Z"),
    )];
    let scan = observation(
        &FakeDriveProvider::new(),
        &scan_mappings,
        "2026-07-11T09:00:00Z",
    );
    let state_mappings = vec![mapping(
        "file-1",
        "Notes/missing.md",
        Some("2026-07-11T08:30:00Z"),
    )];
    let mut state = InMemoryDeleteCandidateStateStore::new(state_mappings).expect("state");
    let mut core = FakeCoreDeleteGateway::new();

    let outcome = run_delete_reconciliation(
        &mut core,
        &mut state,
        permissive_safety(),
        reconciliation_input(AdapterMode::ImportOnly, "2026-07-11T09:00:00Z", scan),
    )
    .expect("reconciliation");

    assert_eq!(
        outcome.blocked,
        Some(DeleteBlockReason::StateChangedSinceScan)
    );
    assert_eq!(core.guard_evaluation_count(), 0);
    assert_eq!(core.delete_call_count(), 0);
    assert_eq!(state.mapping_count(), 1);
}

#[test]
fn older_observation_cannot_confirm_candidate() {
    let mappings = vec![mapping(
        "file-1",
        "Notes/missing.md",
        Some("2026-07-11T09:00:00Z"),
    )];
    let scan = observation(&FakeDriveProvider::new(), &mappings, "2026-07-11T08:00:00Z");
    let mut state = InMemoryDeleteCandidateStateStore::new(mappings).expect("state");
    let mut core = FakeCoreDeleteGateway::new();

    let outcome = run_delete_reconciliation(
        &mut core,
        &mut state,
        permissive_safety(),
        reconciliation_input(AdapterMode::ImportOnly, "2026-07-11T08:00:00Z", scan),
    )
    .expect("reconciliation");

    assert_eq!(
        outcome.blocked,
        Some(DeleteBlockReason::ScanUnreliable(
            DeleteScanIssue::IncompleteScan
        ))
    );
    assert_eq!(core.guard_evaluation_count(), 0);
    assert_eq!(core.delete_call_count(), 0);
}

#[test]
fn mapping_count_change_blocks_ratio_before_core_guard() {
    let scan_mappings = vec![mapping(
        "file-1",
        "Notes/missing.md",
        Some("2026-07-11T08:00:00Z"),
    )];
    let scan = observation(
        &FakeDriveProvider::new(),
        &scan_mappings,
        "2026-07-11T09:00:00Z",
    );
    let state_mappings = vec![
        scan_mappings[0].clone(),
        mapping("file-2", "Notes/other.md", None),
    ];
    let mut state = InMemoryDeleteCandidateStateStore::new(state_mappings).expect("state");
    let mut core = FakeCoreDeleteGateway::new();

    let outcome = run_delete_reconciliation(
        &mut core,
        &mut state,
        permissive_safety(),
        reconciliation_input(AdapterMode::ImportOnly, "2026-07-11T09:00:00Z", scan),
    )
    .expect("reconciliation");

    assert_eq!(
        outcome.blocked,
        Some(DeleteBlockReason::StateChangedSinceScan)
    );
    assert_eq!(core.guard_evaluation_count(), 0);
    assert_eq!(core.delete_call_count(), 0);
}

#[test]
fn recovery_does_not_clear_replaced_mapping_identity() {
    let scan_mapping = mapping("file-1", "Notes/recovered.md", Some("2026-07-11T08:00:00Z"));
    let metadata = DriveMetadata::new_file("file-1", "recovered.md", MIME_TEXT_MARKDOWN)
        .with_parent("root")
        .with_md5_checksum("checksum-file-1");
    let provider = FakeDriveProvider::new().with_child("root", metadata);
    let scan = observation(
        &provider,
        std::slice::from_ref(&scan_mapping),
        "2026-07-11T09:00:00Z",
    );
    let replacement = mapping("file-2", "Notes/recovered.md", Some("2026-07-11T08:30:00Z"));
    let mut state = InMemoryDeleteCandidateStateStore::new(vec![replacement]).expect("state");
    let mut core = FakeCoreDeleteGateway::new();

    let outcome = run_delete_reconciliation(
        &mut core,
        &mut state,
        permissive_safety(),
        reconciliation_input(AdapterMode::ImportOnly, "2026-07-11T09:00:00Z", scan),
    )
    .expect("reconciliation");

    assert_eq!(
        outcome.blocked,
        Some(DeleteBlockReason::StateChangedSinceScan)
    );
    assert_eq!(state.clear_count(), 0);
    assert_eq!(
        state
            .mapping(&path("Notes/recovered.md"))
            .and_then(|mapping| mapping.delete_candidate_since.as_ref())
            .map(SafeTimestamp::as_str),
        Some("2026-07-11T08:30:00Z")
    );
}

#[test]
fn dry_run_first_absence_is_preview_not_mutation() {
    let mappings = vec![mapping("file-1", "Notes/missing.md", None)];
    let scan = observation(&FakeDriveProvider::new(), &mappings, "2026-07-11T09:00:00Z");
    let mut state = InMemoryDeleteCandidateStateStore::new(mappings).expect("state");
    let mut core = FakeCoreDeleteGateway::new();

    let outcome = run_delete_reconciliation(
        &mut core,
        &mut state,
        permissive_safety(),
        reconciliation_input(AdapterMode::DryRun, "2026-07-11T09:00:00Z", scan),
    )
    .expect("reconciliation");

    assert_eq!(outcome.candidates_marked, 0);
    assert_eq!(outcome.candidate_mark_previews, 1);
    assert_eq!(state.mark_count(), 0);
    assert_eq!(
        state
            .mapping(&path("Notes/missing.md"))
            .and_then(|mapping| mapping.delete_candidate_since.as_ref()),
        None
    );
}

#[test]
fn dry_run_recovery_is_preview_not_mutation() {
    let existing = mapping("file-1", "Notes/recovered.md", Some("2026-07-11T08:00:00Z"));
    let metadata = DriveMetadata::new_file("file-1", "recovered.md", MIME_TEXT_MARKDOWN)
        .with_parent("root")
        .with_md5_checksum("checksum-file-1");
    let scan = observation(
        &FakeDriveProvider::new().with_child("root", metadata),
        std::slice::from_ref(&existing),
        "2026-07-11T09:00:00Z",
    );
    let mut state = InMemoryDeleteCandidateStateStore::new(vec![existing]).expect("state");
    let mut core = FakeCoreDeleteGateway::new();

    let outcome = run_delete_reconciliation(
        &mut core,
        &mut state,
        permissive_safety(),
        reconciliation_input(AdapterMode::DryRun, "2026-07-11T09:00:00Z", scan),
    )
    .expect("reconciliation");

    assert_eq!(outcome.candidates_cleared, 0);
    assert_eq!(outcome.candidate_clear_previews, 1);
    assert_eq!(state.clear_count(), 0);
    assert_eq!(
        state
            .mapping(&path("Notes/recovered.md"))
            .and_then(|mapping| mapping.delete_candidate_since.as_ref())
            .map(SafeTimestamp::as_str),
        Some("2026-07-11T08:00:00Z")
    );
}

#[test]
fn scan_snapshot_mismatch_is_rejected_before_reconciliation() {
    let mapping = mapping("file-1", "Notes/missing.md", Some("2026-07-11T08:00:00Z"));
    let plan = FullScanPlan {
        delete_candidates: vec![DeleteCandidatePlan {
            provider_id: "file-1".to_owned(),
            path: path("Notes/missing.md"),
            base_revision_id: mapping.core_revision.clone(),
            detected_at: timestamp("2026-07-11T09:00:00Z"),
            previously_detected_at: Some(timestamp("2026-07-11T07:00:00Z")),
        }],
        ..FullScanPlan::default()
    };

    assert_eq!(
        CompleteDeleteScan::from_full_scan(plan, &[mapping]),
        DeleteScanObservation::Unreliable(DeleteScanIssue::IncompleteScan)
    );
}

#[test]
fn retirement_requires_same_identity_candidate_and_core_revision() {
    let mapping = mapping("file-1", "Notes/missing.md", Some("2026-07-11T08:00:00Z"));
    let mut state = InMemoryDeleteCandidateStateStore::new(vec![mapping]).expect("state");

    assert_eq!(
        state.retire_mapping(
            "file-2",
            &path("Notes/missing.md"),
            &timestamp("2026-07-11T08:00:00Z"),
            Some("core-revision-file-1"),
        ),
        Err(DeleteStateError::MappingIdentityMismatch)
    );
    assert_eq!(
        state.retire_mapping(
            "file-1",
            &path("Notes/missing.md"),
            &timestamp("2026-07-11T07:00:00Z"),
            Some("core-revision-file-1"),
        ),
        Err(DeleteStateError::CandidateTimestampMismatch)
    );
    assert_eq!(
        state.retire_mapping(
            "file-1",
            &path("Notes/missing.md"),
            &timestamp("2026-07-11T08:00:00Z"),
            Some("other-revision"),
        ),
        Err(DeleteStateError::CoreRevisionMismatch)
    );
    assert_eq!(state.mapping_count(), 1);
}
