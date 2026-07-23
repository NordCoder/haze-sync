//! Passive database row models for the Haze Sync storage schema.
//!
//! These structs intentionally contain no repository methods, connection pools,
//! transactions, or Core policy behavior. Row models are internal storage/service
//! values, not public API DTOs.

use crate::schema::table_names;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

/// Internal row field that must not be rendered directly in public output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SensitiveRowField {
    pub table_name: &'static str,
    pub field_name: &'static str,
    pub reason: &'static str,
}

/// Persisted row fields that require explicit API/Server/CLI sanitization before
/// any public rendering.
pub const SENSITIVE_ROW_FIELDS: &[SensitiveRowField] = &[
    SensitiveRowField {
        table_name: table_names::SYNC_ADAPTERS,
        field_name: "token_hash",
        reason: "server-side token hash; never expose in public output",
    },
    SensitiveRowField {
        table_name: table_names::CONTENT_BLOBS,
        field_name: "object_store_path",
        reason: "internal object-store metadata; do not expose storage layout or local paths",
    },
    SensitiveRowField {
        table_name: table_names::ADAPTER_CURSORS,
        field_name: "external_cursor_json",
        reason: "raw adapter/provider cursor metadata; expose only sanitized summaries",
    },
    SensitiveRowField {
        table_name: table_names::IDEMPOTENCY_RECORDS,
        field_name: "idempotency_key",
        reason: "raw retry key material; do not return or log publicly",
    },
    SensitiveRowField {
        table_name: table_names::IDEMPOTENCY_RECORDS,
        field_name: "request_hash",
        reason: "request fingerprint used for replay checks; not a public contract field",
    },
    SensitiveRowField {
        table_name: table_names::IDEMPOTENCY_RECORDS,
        field_name: "response_json",
        reason: "stored response snapshot; expose only through API-owned sanitized DTOs",
    },
    SensitiveRowField {
        table_name: table_names::GDRIVE_MAPPING,
        field_name: "drive_file_id",
        reason: "provider object identifier; expose only through adapter/API-approved views",
    },
    SensitiveRowField {
        table_name: table_names::GDRIVE_MAPPING,
        field_name: "drive_parent_id",
        reason: "provider parent identifier; expose only through adapter/API-approved views",
    },
    SensitiveRowField {
        table_name: table_names::WORKTREE_INSTANCES,
        field_name: "root_fingerprint",
        reason: "non-public normalized-root fingerprint; never render in status or logs",
    },
    SensitiveRowField {
        table_name: table_names::AUDIT_EVENTS,
        field_name: "metadata",
        reason: "structured operational metadata; public output requires redaction discipline",
    },
];

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SyncAdapterRow {
    pub adapter_id: String,
    pub display_name: String,
    pub role: String,
    pub token_hash: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ContentBlobRow {
    pub sha256: String,
    pub size_bytes: i64,
    pub object_store_path: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SyncObjectRow {
    pub object_id: String,
    pub path: String,
    pub kind: String,
    pub current_revision_id: Option<String>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
    pub updated_by: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct FileRevisionRow {
    pub revision_id: String,
    pub object_id: String,
    pub path: String,
    pub parent_revision_id: Option<String>,
    pub content_sha256: String,
    pub size_bytes: i64,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct OperationLogRow {
    pub seq: i64,
    pub op_id: String,
    pub adapter_id: String,
    pub kind: String,
    pub path: String,
    pub revision_id: Option<String>,
    pub tombstone_id: Option<String>,
    pub conflict_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TombstoneRow {
    pub tombstone_id: String,
    pub path: String,
    pub deleted_revision_id: Option<String>,
    pub deleted_by: String,
    pub deleted_at: DateTime<Utc>,
    pub retention_until: DateTime<Utc>,
    pub restored_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ConflictRow {
    pub conflict_id: String,
    pub original_path: String,
    pub base_revision_id: Option<String>,
    pub current_revision_id: String,
    pub incoming_revision_id: String,
    pub incoming_adapter_id: String,
    pub policy_applied: String,
    pub materialized_path: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolved_by: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct AdapterCursorRow {
    pub adapter_id: String,
    pub last_core_seq: i64,
    pub external_cursor_json: Value,
    pub last_success_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct IdempotencyRecordRow {
    pub adapter_id: String,
    pub idempotency_key: String,
    pub request_hash: String,
    pub response_json: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct GDriveMappingRow {
    pub path: String,
    pub drive_file_id: Option<String>,
    pub drive_parent_id: Option<String>,
    pub drive_name: Option<String>,
    pub mime_type: Option<String>,
    pub md5_checksum: Option<String>,
    pub head_revision_id: Option<String>,
    pub drive_version: Option<String>,
    pub drive_modified_time: Option<DateTime<Utc>>,
    pub core_revision_id: Option<String>,
    pub core_seq: Option<i64>,
    pub last_imported_at: Option<DateTime<Utc>>,
    pub last_exported_at: Option<DateTime<Utc>>,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub delete_candidate_at: Option<DateTime<Utc>>,
}

/// Row from `worktree_instances`.
///
/// `root_fingerprint` is intentionally excluded from derived `Debug` and serde
/// surfaces. It is an internal binding fact, not status output.
#[derive(Clone, PartialEq)]
pub struct WorktreeInstanceRow {
    pub adapter_id: String,
    pub root_fingerprint: String,
    pub state_format_version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl fmt::Debug for WorktreeInstanceRow {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WorktreeInstanceRow")
            .field("adapter_id", &self.adapter_id)
            .field("root_fingerprint", &"[REDACTED]")
            .field("state_format_version", &self.state_format_version)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

/// Row from the versioned per-instance `worktree_state` table.
#[derive(Clone, Debug, PartialEq)]
pub struct WorktreeStateRow {
    pub adapter_id: String,
    pub path: String,
    pub state_kind: String,
    pub state_format_version: i32,
    pub last_applied_revision_id: String,
    pub content_sha256: Option<String>,
    pub observation_schema_version: Option<i32>,
    pub observed_size_bytes: Option<i64>,
    pub observed_mtime: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct AuditEventRow {
    pub audit_id: String,
    pub actor_adapter_id: Option<String>,
    pub event_type: String,
    pub path: Option<String>,
    pub revision_id: Option<String>,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sensitive_field_metadata_covers_high_risk_persisted_values() {
        for (table_name, field_name) in [
            (table_names::SYNC_ADAPTERS, "token_hash"),
            (table_names::CONTENT_BLOBS, "object_store_path"),
            (table_names::ADAPTER_CURSORS, "external_cursor_json"),
            (table_names::IDEMPOTENCY_RECORDS, "idempotency_key"),
            (table_names::IDEMPOTENCY_RECORDS, "request_hash"),
            (table_names::IDEMPOTENCY_RECORDS, "response_json"),
            (table_names::GDRIVE_MAPPING, "drive_file_id"),
            (table_names::GDRIVE_MAPPING, "drive_parent_id"),
            (table_names::WORKTREE_INSTANCES, "root_fingerprint"),
            (table_names::AUDIT_EVENTS, "metadata"),
        ] {
            assert!(
                SENSITIVE_ROW_FIELDS
                    .iter()
                    .any(|field| field.table_name == table_name && field.field_name == field_name),
                "missing sensitivity metadata for {table_name}.{field_name}",
            );
        }
    }

    #[test]
    fn sync_adapter_row_serializes_database_field_names() {
        let row = SyncAdapterRow {
            adapter_id: "adapter-1".to_owned(),
            display_name: "Test Adapter".to_owned(),
            role: "worktree_adapter".to_owned(),
            token_hash: "token-hash".to_owned(),
            enabled: true,
            created_at: fixed_time(),
            last_seen_at: None,
        };

        let serialized = serde_json::to_value(&row).expect("row should serialize");
        assert_eq!(serialized["adapter_id"], "adapter-1");
        assert_eq!(serialized["token_hash"], "token-hash");
        assert!(serialized["last_seen_at"].is_null());
        let roundtrip: SyncAdapterRow = serde_json::from_value(serialized).unwrap();
        assert_eq!(roundtrip, row);
    }

    #[test]
    fn adapter_cursor_row_preserves_json_cursor_and_optional_timestamp() {
        let row = AdapterCursorRow {
            adapter_id: "gdrive".to_owned(),
            last_core_seq: 42,
            external_cursor_json: serde_json::json!({ "page_token": "opaque-test-token" }),
            last_success_at: None,
            updated_at: fixed_time(),
        };
        let serialized = serde_json::to_value(&row).unwrap();
        let roundtrip: AdapterCursorRow = serde_json::from_value(serialized).unwrap();
        assert_eq!(roundtrip, row);
    }

    #[test]
    fn worktree_instance_debug_redacts_root_fingerprint() {
        let row = WorktreeInstanceRow {
            adapter_id: "worktree".to_owned(),
            root_fingerprint: format!("sha256:{}", "a".repeat(64)),
            state_format_version: 1,
            created_at: fixed_time(),
            updated_at: fixed_time(),
        };
        let rendered = format!("{row:?}");

        assert!(rendered.contains("[REDACTED]"));
        assert!(!rendered.contains(&"a".repeat(64)));
        assert!(!rendered.contains("/srv/"));
    }

    #[test]
    fn audit_event_row_preserves_structured_metadata() {
        let row = AuditEventRow {
            audit_id: "audit-1".to_owned(),
            actor_adapter_id: Some("adapter-1".to_owned()),
            event_type: "file.put".to_owned(),
            path: Some("Notes/a.md".to_owned()),
            revision_id: Some("rev-1".to_owned()),
            metadata: serde_json::json!({ "status": "accepted" }),
            created_at: fixed_time(),
        };
        let serialized = serde_json::to_value(&row).unwrap();
        let roundtrip: AuditEventRow = serde_json::from_value(serialized).unwrap();
        assert_eq!(roundtrip, row);
    }

    fn fixed_time() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-07-06T00:00:00Z")
            .expect("test timestamp should parse")
            .with_timezone(&Utc)
    }
}
