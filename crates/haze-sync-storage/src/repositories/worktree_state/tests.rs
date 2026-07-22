use super::*;
use chrono::TimeZone;

fn fixed_time(seconds: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(seconds, 0).unwrap()
}

fn present_row() -> WorktreeStateRow {
    WorktreeStateRow {
        adapter_id: "worktree".to_owned(),
        path: "Notes/worktree.md".to_owned(),
        state_kind: WorktreeStateKind::Present.as_str().to_owned(),
        state_format_version: WORKTREE_STATE_FORMAT_VERSION,
        last_applied_revision_id: "rev_01JSTORP10".to_owned(),
        content_sha256: Some(format!("sha256:{}", "a".repeat(64))),
        observation_schema_version: Some(WORKTREE_RECONCILIATION_FORMAT_VERSION),
        observed_size_bytes: Some(42),
        observed_mtime: Some(fixed_time(1_700_000_000)),
        created_at: fixed_time(1_700_000_100),
        updated_at: fixed_time(1_700_000_200),
    }
}

#[test]
fn instance_binding_uses_typed_fingerprint_and_redacted_debug() {
    let binding = WorktreeInstanceBinding::new(
        AdapterId::parse("worktree").unwrap(),
        Sha256::parse(&"a".repeat(64)).unwrap(),
    );
    let rendered = format!("{binding:?}");

    assert_eq!(binding.adapter_id().as_str(), "worktree");
    assert_eq!(
        binding.state_format_version(),
        WORKTREE_STATE_FORMAT_VERSION
    );
    assert!(rendered.contains("[REDACTED]"));
    assert!(!rendered.contains(&"a".repeat(64)));
    assert!(!rendered.contains("/srv/"));
}

#[test]
fn unsupported_state_and_observation_versions_fail_closed() {
    assert_eq!(
        WorktreeInstanceBinding::try_with_version(
            AdapterId::parse("worktree").unwrap(),
            Sha256::parse(&"a".repeat(64)).unwrap(),
            2,
        ),
        Err(RepositoryError::UnsupportedWorktreeStateVersion)
    );
    assert_eq!(
        WorktreeReconciliationObservation::try_with_version(2, 42, None),
        Err(RepositoryError::UnsupportedWorktreeStateVersion)
    );
    assert_eq!(
        WorktreeReconciliationObservation::try_with_version(
            WORKTREE_RECONCILIATION_FORMAT_VERSION,
            i64::MAX as u64 + 1,
            None,
        ),
        Err(RepositoryError::InvalidSizeBytes)
    );
}

#[test]
fn persisted_present_and_tombstoned_rows_enforce_kind_hash_invariants() {
    assert_eq!(
        validate_worktree_state_row(present_row()).unwrap(),
        present_row()
    );

    let mut tombstoned = present_row();
    tombstoned.state_kind = WorktreeStateKind::Tombstoned.as_str().to_owned();
    tombstoned.content_sha256 = None;
    tombstoned.observation_schema_version = None;
    tombstoned.observed_size_bytes = None;
    tombstoned.observed_mtime = None;
    assert_eq!(
        validate_worktree_state_row(tombstoned.clone()).unwrap(),
        tombstoned
    );

    let mut missing_present_hash = present_row();
    missing_present_hash.content_sha256 = None;
    assert_eq!(
        validate_worktree_state_row(missing_present_hash),
        Err(RepositoryError::InvalidWorktreeStateKind)
    );

    let mut tombstone_with_hash = present_row();
    tombstone_with_hash.state_kind = WorktreeStateKind::Tombstoned.as_str().to_owned();
    assert_eq!(
        validate_worktree_state_row(tombstone_with_hash),
        Err(RepositoryError::InvalidWorktreeStateKind)
    );
}

#[test]
fn persisted_rows_reject_invalid_identity_path_version_and_observation() {
    let mut invalid_adapter = present_row();
    invalid_adapter.adapter_id = "bad/adapter".to_owned();
    assert_eq!(
        validate_worktree_state_row(invalid_adapter),
        Err(RepositoryError::InvalidIdentifier)
    );

    let mut invalid_path = present_row();
    invalid_path.path = "Notes//worktree.md".to_owned();
    assert_eq!(
        validate_worktree_state_row(invalid_path),
        Err(RepositoryError::InvalidPath)
    );

    let mut invalid_version = present_row();
    invalid_version.state_format_version = 2;
    assert_eq!(
        validate_worktree_state_row(invalid_version),
        Err(RepositoryError::UnsupportedWorktreeStateVersion)
    );

    let mut incomplete_observation = present_row();
    incomplete_observation.observed_size_bytes = None;
    assert_eq!(
        validate_worktree_state_row(incomplete_observation),
        Err(RepositoryError::InvalidWorktreeObservation)
    );

    let mut unknown_observation = present_row();
    unknown_observation.observation_schema_version = Some(2);
    assert_eq!(
        validate_worktree_state_row(unknown_observation),
        Err(RepositoryError::UnsupportedWorktreeStateVersion)
    );
}

#[test]
fn repository_sql_is_instance_scoped_bounded_and_non_destructive() {
    let bind = BIND_INSTANCE_SQL.to_ascii_lowercase();
    let upsert = UPSERT_STATE_SQL.to_ascii_lowercase();
    let observation = UPDATE_OBSERVATION_SQL.to_ascii_lowercase();

    assert!(bind.contains("on conflict (adapter_id) do update"));
    assert!(upsert.contains("on conflict (adapter_id, path) do update"));
    assert!(observation.contains("where adapter_id = $1 and path = $2"));
    assert!(observation.contains("last_applied_revision_id = $3"));
    assert!(observation.contains("content_sha256 = $4"));

    for sql in [bind, upsert, observation] {
        assert!(!sql.contains("delete from"));
        assert!(!sql.contains("drop table"));
        assert!(!sql.contains("filesystem"));
    }
}
