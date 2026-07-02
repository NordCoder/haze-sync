use chrono::{TimeZone, Utc};
use haze_sync_common::{AdapterId, ContentHash, RevisionId, VaultPath};
use haze_sync_core::conflict_service::{
    ConflictPathRequest, ConflictPolicy, ConflictPolicyError, ConflictRecordStatus,
    CurrentRevision, IncomingConflictCandidate,
};
use haze_sync_core::policy_engine::{
    apply_conflict_policy, ConflictPolicyOutcome, ConflictPolicyRequest,
};

fn timestamp() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 7, 1, 22, 0, 0)
        .single()
        .expect("timestamp should be valid")
}

fn hash(repeated_hex: char) -> ContentHash {
    let hex = repeated_hex.to_string().repeat(64);
    ContentHash::parse(&hex).expect("test hash should parse")
}

fn revision_id(value: &str) -> RevisionId {
    RevisionId::parse(value).expect("test revision id should parse")
}

fn adapter_id(value: &str) -> AdapterId {
    AdapterId::parse(value).expect("test adapter id should parse")
}

fn vault_path(value: &str) -> VaultPath {
    VaultPath::parse(value).expect("test vault path should parse")
}

fn current_revision(path: &str) -> CurrentRevision {
    CurrentRevision::new(revision_id("rev_124"), vault_path(path), hash('0'), 12)
}

fn incoming_candidate(path: &str) -> IncomingConflictCandidate {
    IncomingConflictCandidate::new(
        vault_path(path),
        Some(revision_id("rev_123")),
        adapter_id("iphone-anna"),
        hash('1'),
        34,
    )
}

fn apply_policy(path: &str, policy: ConflictPolicy) -> ConflictPolicyOutcome {
    let request = ConflictPolicyRequest::new(
        policy,
        current_revision(path),
        incoming_candidate(path),
        timestamp(),
    )
    .expect("policy request should be valid");
    apply_conflict_policy(request).expect("policy should apply")
}

#[test]
fn preserve_both_generates_deterministic_safe_conflict_path() {
    let outcome = apply_policy("Projects/Haze/plan.md", ConflictPolicy::PreserveBoth);

    match &outcome {
        ConflictPolicyOutcome::PreserveBoth(preserved) => {
            assert_eq!(preserved.current_revision.revision_id.as_str(), "rev_124");
            assert_eq!(
                preserved.incoming_backup.conflict_path.as_str(),
                "_haze_conflicts/open/Projects/Haze/plan.conflict.iphone-anna.2026-07-01-220000.md"
            );
            assert_eq!(
                preserved.conflict_record.policy_applied,
                ConflictPolicy::PreserveBoth
            );
            assert_eq!(preserved.conflict_record.status, ConflictRecordStatus::Open);
        }
        ConflictPolicyOutcome::CurrentWinsWithIncomingBackup(_) => {
            panic!("expected preserve_both outcome")
        }
    }

    assert_eq!(
        outcome.incoming_backup().conflict_path.as_str(),
        "_haze_conflicts/open/Projects/Haze/plan.conflict.iphone-anna.2026-07-01-220000.md"
    );
}

#[test]
fn current_wins_with_incoming_backup_returns_current_plus_backup_plan() {
    let outcome = apply_policy(
        "Projects/Haze/plan.md",
        ConflictPolicy::CurrentWinsWithIncomingBackup,
    );

    match &outcome {
        ConflictPolicyOutcome::CurrentWinsWithIncomingBackup(current_wins) => {
            assert_eq!(
                current_wins.current_revision.revision_id.as_str(),
                "rev_124"
            );
            assert_eq!(current_wins.current_revision.content_hash, hash('0'));
            assert_eq!(current_wins.incoming_backup.content_hash, hash('1'));
            assert_eq!(
                current_wins.incoming_backup.conflict_path.as_str(),
                "_haze_conflicts/open/Projects/Haze/plan.conflict.iphone-anna.2026-07-01-220000.md"
            );
            assert_eq!(
                current_wins.conflict_record.policy_applied,
                ConflictPolicy::CurrentWinsWithIncomingBackup
            );
        }
        ConflictPolicyOutcome::PreserveBoth(_) => {
            panic!("expected current_wins_with_incoming_backup outcome")
        }
    }
}

#[test]
fn nested_original_directories_are_preserved_under_conflict_open_area() {
    let request = ConflictPolicyRequest::new(
        ConflictPolicy::PreserveBoth,
        CurrentRevision::new(
            revision_id("rev_124"),
            vault_path("A/B/C/report.txt"),
            hash('0'),
            12,
        ),
        IncomingConflictCandidate::new(
            vault_path("A/B/C/report.txt"),
            Some(revision_id("rev_123")),
            adapter_id("worktree-adapter"),
            hash('1'),
            34,
        ),
        timestamp(),
    )
    .expect("policy request should be valid");

    let outcome = apply_conflict_policy(request).expect("policy should apply");

    assert_eq!(
        outcome.incoming_backup().conflict_path.as_str(),
        "_haze_conflicts/open/A/B/C/report.conflict.worktree-adapter.2026-07-01-220000.txt"
    );
}

#[test]
fn extensionless_files_are_handled_safely() {
    let outcome = apply_policy("Notes/README", ConflictPolicy::PreserveBoth);

    assert_eq!(
        outcome.incoming_backup().conflict_path.as_str(),
        "_haze_conflicts/open/Notes/README.conflict.iphone-anna.2026-07-01-220000"
    );
}

#[test]
fn conflict_area_input_is_rejected_to_prevent_recursive_conflict_explosion() {
    let request = ConflictPolicyRequest::new(
        ConflictPolicy::PreserveBoth,
        current_revision("_haze_conflicts/open/Notes/plan.md"),
        incoming_candidate("_haze_conflicts/open/Notes/plan.md"),
        timestamp(),
    );

    assert_eq!(
        request.expect_err("conflict area input should be rejected"),
        ConflictPolicyError::RecursiveConflictPath
    );
}

#[test]
fn unsafe_paths_and_adapters_are_rejected() {
    assert!(matches!(
        ConflictPathRequest::parse("../bad.md", "iphone-anna", timestamp()),
        Err(ConflictPolicyError::InvalidPath { .. })
    ));
    assert!(matches!(
        ConflictPathRequest::parse("/bad.md", "iphone-anna", timestamp()),
        Err(ConflictPolicyError::InvalidPath { .. })
    ));
    assert_eq!(
        ConflictPathRequest::parse("Notes/a.md", "bad/adapter", timestamp())
            .expect_err("adapter path separator should be rejected"),
        ConflictPolicyError::UnsafeAdapterId
    );
    assert_eq!(
        ConflictPathRequest::new(
            vault_path("Notes/a.md"),
            adapter_id("bad.adapter"),
            timestamp()
        )
        .expect_err("dot adapter ids are unsafe in conflict filenames"),
        ConflictPolicyError::UnsafeAdapterId
    );
}

#[test]
fn conflict_record_serialization_is_deterministic() {
    let outcome = apply_policy("Projects/Haze/plan.md", ConflictPolicy::PreserveBoth);
    let json = serde_json::to_string(outcome.conflict_record()).expect("record should serialize");

    assert_eq!(
        json,
        concat!(
            "{",
            "\"original_path\":\"Projects/Haze/plan.md\",",
            "\"conflict_path\":\"_haze_conflicts/open/Projects/Haze/plan.conflict.iphone-anna.2026-07-01-220000.md\",",
            "\"base_revision_id\":\"rev_123\",",
            "\"current_revision_id\":\"rev_124\",",
            "\"incoming_content_hash\":\"sha256:1111111111111111111111111111111111111111111111111111111111111111\",",
            "\"incoming_size_bytes\":34,",
            "\"adapter_id\":\"iphone-anna\",",
            "\"policy_applied\":\"preserve_both\",",
            "\"status\":\"open\",",
            "\"created_at\":\"2026-07-01T22:00:00Z\"",
            "}"
        )
    );
}
