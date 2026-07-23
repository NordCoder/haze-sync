const TEST_SETUP_ADVISORY_LOCK_KEY: i64 = 7_252_953_924_883_537_409;

const PRE_STOR_P10_WORKTREE_STATE_COLUMNS: &[&str] = &[
    "path",
    "last_applied_revision_id",
    "last_seen_sha256",
    "last_seen_mtime",
    "dirty",
    "last_scanned_at",
    "last_written_by_adapter",
];
const WORKTREE_INSTANCE_COLUMNS: &[&str] = &[
    "adapter_id",
    "root_fingerprint",
    "state_format_version",
    "created_at",
    "updated_at",
];
const WORKTREE_STATE_COLUMNS: &[&str] = &[
    "adapter_id",
    "path",
    "state_kind",
    "state_format_version",
    "last_applied_revision_id",
    "content_sha256",
    "observation_schema_version",
    "observed_size_bytes",
    "observed_mtime",
    "created_at",
    "updated_at",
];
const GDRIVE_ADAPTER_STATE_COLUMNS: &[&str] = &[
    "adapter_id",
    "state_format_version",
    "state_version",
    "drive_cursor",
    "drive_cursor_generation",
    "core_export_seq",
    "last_import_operation_id",
    "last_export_operation_id",
    "last_provider_mutation_operation_id",
    "created_at",
    "updated_at",
];
const GDRIVE_DURABLE_ITEM_COLUMNS: &[&str] = &[
    "adapter_id",
    "path",
    "drive_file_id",
    "drive_parent_id",
    "drive_name",
    "mime_type",
    "md5_checksum",
    "head_revision_id",
    "drive_version",
    "drive_modified_time",
    "core_object_id",
    "core_revision_id",
    "core_seq",
    "echo_state",
    "echo_operation_id",
    "echo_provider_version",
    "delete_candidate_first_seen_at",
    "delete_candidate_last_seen_at",
    "delete_candidate_generation",
    "delete_candidate_blocked",
    "delete_confirmation_audit_id",
    "last_imported_at",
    "last_exported_at",
    "last_seen_at",
    "created_at",
    "updated_at",
];
const GDRIVE_OPERATION_COLUMNS: &[&str] = &[
    "adapter_id",
    "operation_id",
    "operation_kind",
    "facts_hash",
    "outcome_kind",
    "committed_state_version",
    "mapping_path",
    "core_seq",
    "drive_version",
    "created_at",
];

const CURSOR_NONNEGATIVE_CONSTRAINT: &str = "adapter_cursors_last_core_seq_nonnegative";
const GDRIVE_STATE_VERSION_CONSTRAINT: &str = "gdrive_adapter_state_version_nonnegative";
const GDRIVE_CURSOR_GENERATION_CONSTRAINT: &str =
    "gdrive_adapter_cursor_generation_nonnegative";
const GDRIVE_EXPORT_CHECKPOINT_CONSTRAINT: &str =
    "gdrive_adapter_core_export_seq_nonnegative";
const GDRIVE_ECHO_CONSTRAINT: &str = "gdrive_durable_items_echo_consistent";
const GDRIVE_DELETE_CANDIDATE_CONSTRAINT: &str =
    "gdrive_durable_items_delete_candidate_consistent";

type PostgresTestResult<T> = Result<T, TestSupportError>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TestMigration {
    pub name: &'static str,
    pub sql: &'static str,
}

macro_rules! migration {
    ($name:literal) => {
        TestMigration {
            name: $name,
            sql: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../migrations/", $name)),
        }
    };
}

pub const STORAGE_TEST_MIGRATIONS: &[TestMigration] = &[
    migration!("0001_sync_adapters.sql"),
    migration!("0002_content_blobs.sql"),
    migration!("0003_sync_objects_file_revisions.sql"),
    migration!("0004_operation_log.sql"),
    migration!("0005_tombstones_conflicts.sql"),
    migration!("0006_cursors_idempotency.sql"),
    migration!("0007_gdrive_mapping.sql"),
    migration!("0008_worktree_state.sql"),
    migration!("0009_audit_events.sql"),
    migration!("0010_worktree_durable_state.sql"),
    migration!("0011_gdrive_durable_state.sql"),
    migration!("0012_operational_control_storage.sql"),
    migration!("0013_operational_jobs_audit.sql"),
    migration!("0014_gdrive_runtime_authority.sql"),
    migration!("0015_qa_contract_corrections.sql"),
];
