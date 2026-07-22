use super::*;
use chrono::TimeZone;

fn fixed_time(seconds: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(seconds, 0).unwrap()
}

fn mapping_row() -> GDriveMappingRow {
    GDriveMappingRow {
        path: "Notes/roadmap.md".to_owned(),
        drive_file_id: Some("drive-file-1".to_owned()),
        drive_parent_id: Some("drive-parent-1".to_owned()),
        drive_name: Some("roadmap.md".to_owned()),
        mime_type: Some("text/markdown".to_owned()),
        md5_checksum: Some("a".repeat(MD5_HEX_LEN)),
        head_revision_id: Some("drive-head-1".to_owned()),
        drive_version: Some("42".to_owned()),
        drive_modified_time: Some(fixed_time(1_700_000_000)),
        core_revision_id: Some("rev_01JSTORP8".to_owned()),
        core_seq: Some(42),
        last_imported_at: Some(fixed_time(1_700_000_010)),
        last_exported_at: Some(fixed_time(1_700_000_020)),
        last_seen_at: Some(fixed_time(1_700_000_030)),
        delete_candidate_at: None,
    }
}

#[test]
fn accepts_storage_safe_mapping_facts() {
    let path = VaultPath::parse("Notes/roadmap.md").unwrap();
    let revision_id = RevisionId::parse("rev_01JSTORP8").unwrap();
    let checksum = "a".repeat(MD5_HEX_LEN);
    let input = GDriveMappingUpsert {
        path: &path,
        drive_file_id: Some("drive-file-1"),
        drive_parent_id: Some("drive-parent-1"),
        drive_name: Some("Roadmap 2026.md"),
        mime_type: Some("text/markdown"),
        md5_checksum: Some(&checksum),
        head_revision_id: Some("drive-head-1"),
        drive_version: Some("42"),
        drive_modified_time: Some(fixed_time(1_700_000_000)),
        core_revision_id: Some(&revision_id),
        core_seq: Some(42),
        last_imported_at: None,
        last_exported_at: None,
        last_seen_at: Some(fixed_time(1_700_000_010)),
        delete_candidate_at: None,
    };

    assert_eq!(validate_gdrive_mapping_input(&input), Ok(()));
}

#[test]
fn rejects_invalid_provider_identifiers_hashes_and_sequences() {
    let path = VaultPath::parse("Notes/roadmap.md").unwrap();
    let invalid_identifier = GDriveMappingUpsert {
        path: &path,
        drive_file_id: Some("drive file with spaces"),
        drive_parent_id: None,
        drive_name: None,
        mime_type: None,
        md5_checksum: None,
        head_revision_id: None,
        drive_version: None,
        drive_modified_time: None,
        core_revision_id: None,
        core_seq: None,
        last_imported_at: None,
        last_exported_at: None,
        last_seen_at: None,
        delete_candidate_at: None,
    };
    assert_eq!(
        validate_gdrive_mapping_input(&invalid_identifier),
        Err(RepositoryError::InvalidIdentifier)
    );

    let invalid_hash = GDriveMappingUpsert {
        drive_file_id: None,
        md5_checksum: Some("not-md5"),
        ..invalid_identifier
    };
    assert_eq!(
        validate_gdrive_mapping_input(&invalid_hash),
        Err(RepositoryError::InvalidHash)
    );

    let invalid_sequence = GDriveMappingUpsert {
        md5_checksum: None,
        core_seq: Some(-1),
        ..invalid_hash
    };
    assert_eq!(
        validate_gdrive_mapping_input(&invalid_sequence),
        Err(RepositoryError::InvalidSequence)
    );
}

#[test]
fn persisted_mapping_validation_rejects_noncanonical_or_invalid_values() {
    assert_eq!(
        validate_gdrive_mapping_row(mapping_row()).unwrap(),
        mapping_row()
    );

    let mut invalid_path = mapping_row();
    invalid_path.path = "Notes//roadmap.md".to_owned();
    assert_eq!(
        validate_gdrive_mapping_row(invalid_path),
        Err(RepositoryError::InvalidPath)
    );

    let mut invalid_revision = mapping_row();
    invalid_revision.core_revision_id = Some("revision-without-prefix".to_owned());
    assert_eq!(
        validate_gdrive_mapping_row(invalid_revision),
        Err(RepositoryError::InvalidIdentifier)
    );

    let mut invalid_hash = mapping_row();
    invalid_hash.md5_checksum = Some("z".repeat(MD5_HEX_LEN));
    assert_eq!(
        validate_gdrive_mapping_row(invalid_hash),
        Err(RepositoryError::InvalidHash)
    );
}

#[test]
fn upsert_sql_replaces_facts_by_path_without_encoding_adapter_policy() {
    let sql = UPSERT_SQL.to_ascii_lowercase();

    assert!(sql.contains("on conflict (path) do update"));
    assert!(sql.contains("drive_file_id = excluded.drive_file_id"));
    assert!(sql.contains("delete_candidate_at = excluded.delete_candidate_at"));
    assert!(!sql.contains("delete from"));
    assert!(!sql.contains("http"));
}
