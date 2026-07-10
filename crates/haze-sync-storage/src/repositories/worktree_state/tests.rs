use super::*;
use chrono::TimeZone;

fn fixed_time(seconds: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(seconds, 0).unwrap()
}

fn state_row() -> WorktreeStateRow {
    WorktreeStateRow {
        path: "Notes/worktree.md".to_owned(),
        last_applied_revision_id: Some("rev_01JSTORP8".to_owned()),
        last_seen_sha256: Some(format!("sha256:{}", "a".repeat(64))),
        last_seen_mtime: Some(fixed_time(1_700_000_000)),
        dirty: true,
        last_scanned_at: Some(fixed_time(1_700_000_100)),
        last_written_by_adapter: false,
    }
}

#[test]
fn input_uses_validated_common_values_and_typed_timestamps() {
    let path = VaultPath::parse("Notes/worktree.md").unwrap();
    let revision_id = RevisionId::parse("rev_01JSTORP8").unwrap();
    let hash = Sha256::parse(&"a".repeat(64)).unwrap();
    let input = WorktreeStateUpsert {
        path: &path,
        last_applied_revision_id: Some(&revision_id),
        last_seen_sha256: Some(&hash),
        last_seen_mtime: Some(fixed_time(1_700_000_000)),
        dirty: true,
        last_scanned_at: Some(fixed_time(1_700_000_100)),
        last_written_by_adapter: false,
    };

    assert_eq!(input.path.as_str(), "Notes/worktree.md");
    assert_eq!(
        input.last_applied_revision_id.map(RevisionId::as_str),
        Some("rev_01JSTORP8")
    );
    assert_eq!(
        input.last_seen_sha256.map(Sha256::to_string),
        Some(format!("sha256:{}", "a".repeat(64)))
    );
    assert!(input.dirty);
    assert!(!input.last_written_by_adapter);
}

#[test]
fn persisted_state_validation_rejects_noncanonical_or_invalid_values() {
    assert_eq!(
        validate_worktree_state_row(state_row()).unwrap(),
        state_row()
    );

    let mut invalid_path = state_row();
    invalid_path.path = "Notes//worktree.md".to_owned();
    assert_eq!(
        validate_worktree_state_row(invalid_path),
        Err(RepositoryError::InvalidPath)
    );

    let mut invalid_revision = state_row();
    invalid_revision.last_applied_revision_id = Some("revision-without-prefix".to_owned());
    assert_eq!(
        validate_worktree_state_row(invalid_revision),
        Err(RepositoryError::InvalidIdentifier)
    );

    let mut invalid_hash = state_row();
    invalid_hash.last_seen_sha256 = Some("not-a-hash".to_owned());
    assert_eq!(
        validate_worktree_state_row(invalid_hash),
        Err(RepositoryError::InvalidHash)
    );

    let mut noncanonical_hash = state_row();
    noncanonical_hash.last_seen_sha256 = Some("A".repeat(64));
    assert_eq!(
        validate_worktree_state_row(noncanonical_hash),
        Err(RepositoryError::InvalidHash)
    );
}

#[test]
fn upsert_sql_persists_flags_without_interpreting_them() {
    let sql = UPSERT_SQL.to_ascii_lowercase();

    assert!(sql.contains("on conflict (path) do update"));
    assert!(sql.contains("dirty = excluded.dirty"));
    assert!(sql.contains("last_written_by_adapter = excluded.last_written_by_adapter"));
    assert!(!sql.contains("delete from"));
    assert!(!sql.contains("filesystem"));
}
