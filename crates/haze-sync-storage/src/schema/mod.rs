//! Stable names and migration metadata for Haze Sync storage tables.
//!
//! This module exposes stable table-name constants for repository code. It does
//! not implement SQL execution, connections, transactions, or Core policy.

/// Storage table names owned by the Haze Sync metadata schema.
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
    /// Historical path-scoped Google Drive mapping facts.
    pub const GDRIVE_MAPPING: &str = "gdrive_mapping";
    /// Versioned adapter-scoped Google Drive progress facts.
    pub const GDRIVE_ADAPTER_STATE: &str = "gdrive_adapter_state";
    /// Adapter-scoped Google Drive mapping, echo, and delete-candidate facts.
    pub const GDRIVE_DURABLE_ITEMS: &str = "gdrive_durable_items";
    /// Retry-safe Google Drive operation outcomes.
    pub const GDRIVE_OPERATIONS: &str = "gdrive_operations";
    /// Versioned Worktree runtime-instance bindings.
    pub const WORKTREE_INSTANCES: &str = "worktree_instances";
    /// Per-instance Worktree last-applied path state.
    pub const WORKTREE_STATE: &str = "worktree_state";
    /// Safe audit event metadata.
    pub const AUDIT_EVENTS: &str = "audit_events";

    /// Ordered table list for the accepted current storage schema.
    pub const ALL: &[&str] = &[
        SYNC_ADAPTERS,
        CONTENT_BLOBS,
        SYNC_OBJECTS,
        FILE_REVISIONS,
        OPERATION_LOG,
        TOMBSTONES,
        CONFLICTS,
        ADAPTER_CURSORS,
        IDEMPOTENCY_RECORDS,
        GDRIVE_MAPPING,
        GDRIVE_ADAPTER_STATE,
        GDRIVE_DURABLE_ITEMS,
        GDRIVE_OPERATIONS,
        WORKTREE_INSTANCES,
        WORKTREE_STATE,
        AUDIT_EVENTS,
    ];

    /// Exact accepted pre-STOR-GDA-P1 table set migrated by 0011.
    pub const PRE_STOR_GDA_P11: &[&str] = &[
        SYNC_ADAPTERS,
        CONTENT_BLOBS,
        SYNC_OBJECTS,
        FILE_REVISIONS,
        OPERATION_LOG,
        TOMBSTONES,
        CONFLICTS,
        ADAPTER_CURSORS,
        IDEMPOTENCY_RECORDS,
        GDRIVE_MAPPING,
        WORKTREE_INSTANCES,
        WORKTREE_STATE,
        AUDIT_EVENTS,
    ];

    /// Exact pre-STOR-P10 table set accepted for deterministic migration.
    pub const PRE_STOR_P10: &[&str] = &[
        SYNC_ADAPTERS,
        CONTENT_BLOBS,
        SYNC_OBJECTS,
        FILE_REVISIONS,
        OPERATION_LOG,
        TOMBSTONES,
        CONFLICTS,
        ADAPTER_CURSORS,
        IDEMPOTENCY_RECORDS,
        GDRIVE_MAPPING,
        WORKTREE_STATE,
        AUDIT_EVENTS,
    ];
}

/// Ordered migration filenames for the current storage schema.
///
/// The historical name is retained as a public compatibility surface.
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
    "0010_worktree_durable_state.sql",
    "0011_gdrive_durable_state.sql",
];

#[cfg(test)]
mod tests {
    use super::{table_names, INITIAL_MIGRATIONS};

    const MIGRATION_CONTENTS: &[(&str, &str)] = &[
        (
            "0001_sync_adapters.sql",
            include_str!("../../../../migrations/0001_sync_adapters.sql"),
        ),
        (
            "0002_content_blobs.sql",
            include_str!("../../../../migrations/0002_content_blobs.sql"),
        ),
        (
            "0003_sync_objects_file_revisions.sql",
            include_str!("../../../../migrations/0003_sync_objects_file_revisions.sql"),
        ),
        (
            "0004_operation_log.sql",
            include_str!("../../../../migrations/0004_operation_log.sql"),
        ),
        (
            "0005_tombstones_conflicts.sql",
            include_str!("../../../../migrations/0005_tombstones_conflicts.sql"),
        ),
        (
            "0006_cursors_idempotency.sql",
            include_str!("../../../../migrations/0006_cursors_idempotency.sql"),
        ),
        (
            "0007_gdrive_mapping.sql",
            include_str!("../../../../migrations/0007_gdrive_mapping.sql"),
        ),
        (
            "0008_worktree_state.sql",
            include_str!("../../../../migrations/0008_worktree_state.sql"),
        ),
        (
            "0009_audit_events.sql",
            include_str!("../../../../migrations/0009_audit_events.sql"),
        ),
        (
            "0010_worktree_durable_state.sql",
            include_str!("../../../../migrations/0010_worktree_durable_state.sql"),
        ),
        (
            "0011_gdrive_durable_state.sql",
            include_str!("../../../../migrations/0011_gdrive_durable_state.sql"),
        ),
    ];

    #[test]
    fn migration_metadata_matches_actual_files() {
        assert_eq!(INITIAL_MIGRATIONS.len(), MIGRATION_CONTENTS.len());
        for (metadata_name, (actual_name, contents)) in
            INITIAL_MIGRATIONS.iter().zip(MIGRATION_CONTENTS)
        {
            assert_eq!(metadata_name, actual_name);
            assert!(contents.contains("create table") || contents.contains("alter table"));
        }
    }

    #[test]
    fn migration_metadata_is_strictly_ordered() {
        for pair in INITIAL_MIGRATIONS.windows(2) {
            let previous = migration_prefix(pair[0]);
            let next = migration_prefix(pair[1]);
            assert!(
                previous < next,
                "migration filenames must be strictly ordered"
            );
        }
    }

    #[test]
    fn table_name_metadata_matches_final_storage_schema() {
        let mut expected = table_names::ALL.to_vec();
        let mut actual = final_created_tables();
        expected.sort_unstable();
        actual.sort_unstable();
        assert_eq!(expected, actual);
    }

    #[test]
    fn accepted_pre_gdrive_table_set_excludes_only_0011_tables() {
        for table in [
            table_names::GDRIVE_ADAPTER_STATE,
            table_names::GDRIVE_DURABLE_ITEMS,
            table_names::GDRIVE_OPERATIONS,
        ] {
            assert!(!table_names::PRE_STOR_GDA_P11.contains(&table));
            assert!(table_names::ALL.contains(&table));
        }
    }

    fn final_created_tables() -> Vec<&'static str> {
        let mut tables = Vec::new();
        for (_, contents) in MIGRATION_CONTENTS {
            for line in contents.lines().map(str::trim) {
                if let Some(table_name) = line
                    .strip_prefix("drop table ")
                    .and_then(|rest| rest.split_whitespace().next())
                {
                    tables.retain(|candidate| *candidate != table_name.trim_end_matches(';'));
                }
                if let Some(table_name) = line
                    .strip_prefix("create table ")
                    .and_then(|rest| rest.split_whitespace().next())
                {
                    let table_name = table_name.trim_end_matches(';');
                    if !tables.contains(&table_name) {
                        tables.push(table_name);
                    }
                }
            }
        }
        tables
    }

    fn migration_prefix(filename: &str) -> u32 {
        filename
            .split_once('_')
            .and_then(|(prefix, _)| prefix.parse().ok())
            .expect("migration filename must start with a numeric prefix")
    }
}
