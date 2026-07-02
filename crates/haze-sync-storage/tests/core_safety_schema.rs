use chrono::Utc;
use haze_sync_storage::models::{ConflictRow, TombstoneRow};

#[test]
fn tombstone_row_is_delete_metadata_not_embedded_content() {
    let now = Utc::now();
    let row = TombstoneRow {
        tombstone_id: "tmb_01JSAFE".to_owned(),
        path: "Projects/Haze/old.md".to_owned(),
        deleted_revision_id: Some("rev_deleted".to_owned()),
        deleted_by: "iphone-anna".to_owned(),
        deleted_at: now,
        retention_until: now,
        restored_at: None,
    };

    let serialized = serde_json::to_string(&row).expect("tombstone row should serialize");

    assert!(serialized.contains("tmb_01JSAFE"));
    assert!(serialized.contains("retention_until"));
    assert!(!serialized.contains("file_bytes"));
    assert!(!serialized.contains("content"));
    assert!(!serialized.contains("hard_delete"));
}

#[test]
fn conflict_row_keeps_open_materialized_path_under_conflicts_open() {
    let now = Utc::now();
    let row = ConflictRow {
        conflict_id: "conf_01JSAFE".to_owned(),
        original_path: "Projects/Haze/plan.md".to_owned(),
        base_revision_id: Some("rev_base".to_owned()),
        current_revision_id: "rev_current".to_owned(),
        incoming_revision_id: "rev_incoming".to_owned(),
        incoming_adapter_id: "iphone-anna".to_owned(),
        policy_applied: "preserve_both".to_owned(),
        materialized_path:
            "_haze_conflicts/open/Projects/Haze/plan.conflict.iphone-anna.2026-07-01-2200.md"
                .to_owned(),
        status: "open".to_owned(),
        created_at: now,
        resolved_at: None,
        resolved_by: None,
    };

    assert!(row.materialized_path.starts_with("_haze_conflicts/open/"));
    assert!(!row
        .materialized_path
        .contains("/_haze_conflicts/open/_haze_conflicts/"));

    let serialized = serde_json::to_string(&row).expect("conflict row should serialize");
    assert!(serialized.contains("preserve_both"));
    assert!(!serialized.contains("DATABASE_URL"));
    assert!(!serialized.contains("Authorization"));
    assert!(!serialized.contains("/srv/"));
}
