use super::*;
use chrono::TimeZone;
use serde_json::json;

fn cursor_row(external_cursor_json: Value) -> AdapterCursorRow {
    AdapterCursorRow {
        adapter_id: "gdrive-adapter".to_owned(),
        last_core_seq: 7,
        external_cursor_json,
        last_success_at: Some(Utc.timestamp_opt(1_700_000_000, 0).unwrap()),
        updated_at: Utc.timestamp_opt(1_700_000_100, 0).unwrap(),
    }
}

#[test]
fn update_outcome_reports_updated_status() {
    let row = cursor_row(json!({}));

    assert!(AdapterCursorUpdateOutcome::Updated(row.clone()).is_updated());
    assert!(!AdapterCursorUpdateOutcome::RejectedRegression {
        current: row,
        requested_seq: 3,
    }
    .is_updated());
}

#[test]
fn update_request_accepts_internal_json_cursor_metadata() {
    let update = AdapterCursorUpdate {
        adapter_id: AdapterId::parse("gdrive-adapter").unwrap(),
        last_core_seq: 42,
        external_cursor_json: Some(json!({ "page_token": "opaque-test-token" })),
        mark_success: true,
    };

    assert_eq!(update.last_core_seq, 42);
    assert!(update.external_cursor_json.is_some());
}

#[test]
fn cursor_summary_excludes_raw_external_cursor_json() {
    let row = cursor_row(json!({ "page_token": "opaque-test-token" }));
    let summary = AdapterCursorSummary::from(&row);
    let serialized = serde_json::to_value(&summary).unwrap();
    let serialized_text = serialized.to_string();

    assert_eq!(summary.adapter_id, row.adapter_id);
    assert_eq!(summary.last_core_seq, row.last_core_seq);
    assert!(summary.has_external_cursor);
    assert!(serialized.get("external_cursor_json").is_none());
    assert!(!serialized_text.contains("opaque-test-token"));
    assert!(!serialized_text.contains("page_token"));
}

#[test]
fn empty_or_null_cursor_metadata_is_summarized_as_absent() {
    assert!(!AdapterCursorSummary::from(&cursor_row(Value::Null)).has_external_cursor);
    assert!(!AdapterCursorSummary::from(&cursor_row(json!({}))).has_external_cursor);
    assert!(AdapterCursorSummary::from(&cursor_row(json!([]))).has_external_cursor);
}

#[test]
fn cursor_sql_uses_single_atomic_upserts() {
    let initialize = INITIALIZE_CURSOR_SQL.to_ascii_lowercase();
    let update = UPDATE_CURSOR_SQL.to_ascii_lowercase();

    assert!(initialize.contains("on conflict (adapter_id) do update"));
    assert!(initialize.contains("returning adapter_id"));
    assert!(!initialize.contains("with inserted"));

    assert!(update.contains("on conflict (adapter_id) do update"));
    assert!(update.contains("last_core_seq <= excluded.last_core_seq"));
    assert!(update.contains("returning adapter_id"));
    assert!(!update.contains("with ensured"));
}
