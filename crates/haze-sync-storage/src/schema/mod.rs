//! Schema metadata for Haze Sync storage tables.
//!
//! This module exposes stable table-name constants for future repository code.
//! It does not implement SQL execution, migrations, connections, transactions,
//! or Core policy behavior.

/// Storage table names owned by the initial Haze Sync metadata schema.
pub mod table_names {
    /// Registered adapter identities and token hashes.
    pub const SYNC_ADAPTERS: &str = "sync_adapters";
    /// Immutable content-addressed blob metadata keyed by SHA-256.
    pub const CONTENT_BLOBS: &str = "content_blobs";
    /// Logical vault objects keyed by stable object id with a unique current path.
    pub const SYNC_OBJECTS: &str = "sync_objects";
    /// Immutable file revision rows.
    pub const FILE_REVISIONS: &str = "file_revisions";
    /// Append-only operation feed consumed by adapters.
    pub const OPERATION_LOG: &str = "operation_log";
    /// Tombstone records used for retained deletes.
    pub const TOMBSTONES: &str = "tombstones";
    /// Preserved conflict records containing both versions.
    pub const CONFLICTS: &str = "conflicts";
    /// Per-adapter progress and external cursor metadata.
    pub const ADAPTER_CURSORS: &str = "adapter_cursors";
    /// Retry-safety records keyed by adapter and idempotency key.
    pub const IDEMPOTENCY_RECORDS: &str = "idempotency_records";
    /// Google Drive file mapping and echo/delete-candidate metadata.
    pub const GDRIVE_MAPPING: &str = "gdrive_mapping";
    /// Built-in worktree materialization state.
    pub const WORKTREE_STATE: &str = "worktree_state";
    /// Safe audit event metadata.
    pub const AUDIT_EVENTS: &str = "audit_events";
}

/// Ordered migration filenames for the initial storage schema.
///
/// Future migration runners are intentionally out of scope for this phase.
pub const INITIAL_MIGRATIONS: &[&str] = &[
    "0001_sync_adapters.sql",
    "0002_content_blobs.sql",
    "0003_sync_objects_file_revisions.sql",
    "0004_operation_log.sql",
    "0005_tombstones_conflicts.sql",
    "0006_cursors_idempotency.sql",
    "0007_gdrive_mapping.sql",
    "0008_worktree_state.sql",
    "0009_audit_events.sql",
];
