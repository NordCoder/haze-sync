//! Passive database row models for the Haze Sync storage schema.
//!
//! These structs intentionally contain no repository methods, SQL query macros,
//! connection pools, transactions, or Core policy behavior. Future phases will
//! implement apply, conflict, delete, cursor, and idempotency logic on top of the
//! schema represented here.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
