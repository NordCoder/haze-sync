//! Passive database row models for the Haze Sync storage schema.
//!
//! These structs intentionally contain no repository methods, SQL query macros,
//! connection pools, transactions, or Core policy behavior. Future phases will
//! implement apply, conflict, delete, cursor, and idempotency logic on top of the
//! schema represented here.
//!
//! Row models mirror persisted database fields and are internal storage/service
//! values, not public API DTOs. Some fields intentionally contain token hashes,
//! provider metadata, cursor snapshots, idempotency material, or object-store
//! metadata; public API, admin, status, and CLI surfaces must sanitize or map
//! rows before rendering them.

use crate::schema::table_names;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Internal row field that must not be rendered directly in public output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SensitiveRowField {
    pub table_name: &'static str,
    pub field_name: &'static str,
    pub reason: &'static str,
}

/// Persisted row fields that require explicit API/Server/CLI sanitization before
/// any public rendering.
///
/// This list is audit metadata for storage consumers. It does not remove fields
/// from row models because repositories still need to persist and load them.
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
        table_name: table_names::AUDIT_EVENTS,
        field_name: "metadata",
        reason: "structured operational metadata; public output requires redaction discipline",
    },
];

/// Row from `sync_adapters`.
///
/// Stores registered adapter identity and token hash metadata. Authorization and
/// role policy are implemented in future API/Core phases.
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

/// Row from `content_blobs`.
///
/// Represents immutable content-addressed blob metadata. Actual object-store
/// writes and verification are implemented in future content-store phases.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ContentBlobRow {
    pub sha256: String,
    pub size_bytes: i64,
    pub object_store_path: String,
    pub created_at: DateTime<Utc>,
}

/// Row from `sync_objects`.
///
/// Tracks the logical vault object at a current path. Current-revision mutation
/// and delete policy are implemented in future Core phases.
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

/// Row from `file_revisions`.
///
/// Immutable file revision metadata. Revision acceptance and conflict handling
/// are implemented in future Core phases.
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

/// Row from `operation_log`.
///
/// Append-only operation record consumed by adapters. Appending and changes-feed
/// behavior are implemented in future phases.
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

/// Row from `tombstones`.
///
/// Safe delete marker with retention metadata. Delete guards, restoration, and
/// physical cleanup are implemented in future phases.
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

/// Row from `conflicts`.
///
/// Records both current and incoming revisions for a preserved conflict. Conflict
/// resolution behavior is implemented in future Core/API phases.
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

/// Row from `adapter_cursors`.
///
/// Tracks adapter progress. Cursor advancement after successful processing is
/// implemented in future adapter/Core phases.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct AdapterCursorRow {
    pub adapter_id: String,
    pub last_core_seq: i64,
    pub external_cursor_json: Value,
    pub last_success_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

/// Row from `idempotency_records`.
///
/// Stores retry-safety request hashes and safe public response snapshots.
/// Idempotency comparison and replay are implemented in future Core phases.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct IdempotencyRecordRow {
    pub adapter_id: String,
    pub idempotency_key: String,
    pub request_hash: String,
    pub response_json: Value,
    pub created_at: DateTime<Utc>,
}

/// Row from `gdrive_mapping`.
///
/// Stores Google Drive mapping metadata for a vault path. Drive API access,
/// echo guard behavior, and delete-candidate policy are implemented in future
/// adapter phases.
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

/// Row from `worktree_state`.
///
/// Tracks materialized worktree state. Scanner, watcher, atomic writer, and
/// repair behavior are implemented in future worktree phases.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct WorktreeStateRow {
    pub path: String,
    pub last_applied_revision_id: Option<String>,
    pub last_seen_sha256: Option<String>,
    pub last_seen_mtime: Option<DateTime<Utc>>,
    pub dirty: bool,
    pub last_scanned_at: Option<DateTime<Utc>>,
    pub last_written_by_adapter: bool,
}

/// Row from `audit_events`.
///
/// Stores safe structured audit metadata. Audit event production and redaction
/// policy are implemented in future phases.
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
        assert_eq!(serialized["display_name"], "Test Adapter");
        assert_eq!(serialized["role"], "worktree_adapter");
        assert_eq!(serialized["token_hash"], "token-hash");
        assert!(serialized["enabled"]
            .as_bool()
            .expect("enabled should serialize as bool"));
        assert!(serialized["last_seen_at"].is_null());

        let roundtrip: SyncAdapterRow =
            serde_json::from_value(serialized).expect("row should deserialize");
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

        let serialized = serde_json::to_value(&row).expect("row should serialize");

        assert_eq!(serialized["adapter_id"], "gdrive");
        assert_eq!(serialized["last_core_seq"], 42);
        assert_eq!(
            serialized["external_cursor_json"]["page_token"],
            "opaque-test-token"
        );
        assert!(serialized["last_success_at"].is_null());

        let roundtrip: AdapterCursorRow =
            serde_json::from_value(serialized).expect("row should deserialize");
        assert_eq!(roundtrip, row);
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

        let serialized = serde_json::to_value(&row).expect("row should serialize");

        assert_eq!(serialized["audit_id"], "audit-1");
        assert_eq!(serialized["actor_adapter_id"], "adapter-1");
        assert_eq!(serialized["event_type"], "file.put");
        assert_eq!(serialized["path"], "Notes/a.md");
        assert_eq!(serialized["revision_id"], "rev-1");
        assert_eq!(serialized["metadata"]["status"], "accepted");

        let roundtrip: AuditEventRow =
            serde_json::from_value(serialized).expect("row should deserialize");
        assert_eq!(roundtrip, row);
    }

    fn fixed_time() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-07-06T00:00:00Z")
            .expect("test timestamp should parse")
            .with_timezone(&Utc)
    }
}
