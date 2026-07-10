use super::*;
use crate::test_support::connect_test_database_from_env;
use chrono::TimeZone;

#[tokio::test]
async fn gdrive_mapping_roundtrips_in_caller_owned_transaction() {
    let Some(context) = connect_test_database_from_env().await.unwrap() else {
        return;
    };
    context.apply_migrations().await.unwrap();

    let path = VaultPath::parse(&context.namespace().vault_path("gdrive-map.md")).unwrap();
    let drive_file_id = context.namespace().child_id("drive-file");
    let drive_parent_id = context.namespace().child_id("drive-parent");
    let checksum = "a".repeat(MD5_HEX_LEN);
    let modified_at = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    let seen_at = Utc.timestamp_opt(1_700_000_100, 0).unwrap();
    let mut transaction = context.pool().begin().await.unwrap();

    let inserted = upsert_gdrive_mapping(
        &mut *transaction,
        GDriveMappingUpsert {
            path: &path,
            drive_file_id: Some(&drive_file_id),
            drive_parent_id: Some(&drive_parent_id),
            drive_name: Some("gdrive-map.md"),
            mime_type: Some("text/markdown"),
            md5_checksum: Some(&checksum),
            head_revision_id: Some("drive-head-1"),
            drive_version: Some("1"),
            drive_modified_time: Some(modified_at),
            core_revision_id: None,
            core_seq: Some(7),
            last_imported_at: Some(seen_at),
            last_exported_at: None,
            last_seen_at: Some(seen_at),
            delete_candidate_at: None,
        },
    )
    .await
    .unwrap();

    assert_eq!(inserted.path, path.as_str());
    assert_eq!(
        inserted.drive_file_id.as_deref(),
        Some(drive_file_id.as_str())
    );
    assert_eq!(inserted.core_seq, Some(7));
    assert_eq!(
        get_gdrive_mapping_by_path(&mut *transaction, &path)
            .await
            .unwrap(),
        Some(inserted.clone())
    );
    assert_eq!(
        get_gdrive_mapping_by_drive_file_id(&mut *transaction, &drive_file_id)
            .await
            .unwrap(),
        Some(inserted)
    );

    let delete_candidate_at = Utc.timestamp_opt(1_700_000_200, 0).unwrap();
    let updated = upsert_gdrive_mapping(
        &mut *transaction,
        GDriveMappingUpsert {
            path: &path,
            drive_file_id: Some(&drive_file_id),
            drive_parent_id: Some(&drive_parent_id),
            drive_name: Some("gdrive-map.md"),
            mime_type: Some("text/markdown"),
            md5_checksum: Some(&checksum),
            head_revision_id: Some("drive-head-2"),
            drive_version: Some("2"),
            drive_modified_time: Some(modified_at),
            core_revision_id: None,
            core_seq: Some(8),
            last_imported_at: Some(seen_at),
            last_exported_at: Some(seen_at),
            last_seen_at: Some(seen_at),
            delete_candidate_at: Some(delete_candidate_at),
        },
    )
    .await
    .unwrap();

    assert_eq!(updated.drive_version.as_deref(), Some("2"));
    assert_eq!(updated.core_seq, Some(8));
    assert_eq!(updated.delete_candidate_at, Some(delete_candidate_at));

    transaction.rollback().await.unwrap();
}
