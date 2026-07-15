/// Internal row from `gdrive_adapter_state`.
///
/// The opaque Drive cursor is deliberately excluded from Debug/serde surfaces.
#[derive(Clone, PartialEq)]
pub struct GDriveAdapterStateRow {
    pub adapter_id: String,
    pub state_format_version: i32,
    pub state_version: i64,
    pub drive_cursor: Option<String>,
    pub drive_cursor_generation: i64,
    pub core_export_seq: i64,
    pub last_import_operation_id: Option<String>,
    pub last_export_operation_id: Option<String>,
    pub last_provider_mutation_operation_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl fmt::Debug for GDriveAdapterStateRow {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GDriveAdapterStateRow")
            .field("adapter_id", &self.adapter_id)
            .field("state_format_version", &self.state_format_version)
            .field("state_version", &self.state_version)
            .field("drive_cursor", &self.drive_cursor.as_ref().map(|_| "[REDACTED]"))
            .field("drive_cursor_generation", &self.drive_cursor_generation)
            .field("core_export_seq", &self.core_export_seq)
            .field("has_last_import_operation", &self.last_import_operation_id.is_some())
            .field("has_last_export_operation", &self.last_export_operation_id.is_some())
            .field(
                "has_last_provider_mutation_operation",
                &self.last_provider_mutation_operation_id.is_some(),
            )
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

/// Internal adapter-scoped Drive/Core mapping, echo, and delete-candidate row.
#[derive(Clone, PartialEq)]
pub struct GDriveDurableItemRow {
    pub adapter_id: String,
    pub path: String,
    pub drive_file_id: Option<String>,
    pub drive_parent_id: Option<String>,
    pub drive_name: Option<String>,
    pub mime_type: Option<String>,
    pub md5_checksum: Option<String>,
    pub head_revision_id: Option<String>,
    pub drive_version: Option<String>,
    pub drive_modified_time: Option<DateTime<Utc>>,
    pub core_object_id: Option<String>,
    pub core_revision_id: Option<String>,
    pub core_seq: Option<i64>,
    pub echo_state: String,
    pub echo_operation_id: Option<String>,
    pub echo_provider_version: Option<String>,
    pub delete_candidate_first_seen_at: Option<DateTime<Utc>>,
    pub delete_candidate_last_seen_at: Option<DateTime<Utc>>,
    pub delete_candidate_generation: Option<i64>,
    pub delete_candidate_blocked: bool,
    pub delete_confirmation_audit_id: Option<String>,
    pub last_imported_at: Option<DateTime<Utc>>,
    pub last_exported_at: Option<DateTime<Utc>>,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl fmt::Debug for GDriveDurableItemRow {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GDriveDurableItemRow")
            .field("adapter_id", &self.adapter_id)
            .field("path", &self.path)
            .field("has_drive_file_id", &self.drive_file_id.is_some())
            .field("has_drive_parent_id", &self.drive_parent_id.is_some())
            .field("has_drive_name", &self.drive_name.is_some())
            .field("has_mime_type", &self.mime_type.is_some())
            .field("has_md5_checksum", &self.md5_checksum.is_some())
            .field("has_head_revision_id", &self.head_revision_id.is_some())
            .field("has_drive_version", &self.drive_version.is_some())
            .field("core_object_id", &self.core_object_id)
            .field("core_revision_id", &self.core_revision_id)
            .field("core_seq", &self.core_seq)
            .field("echo_state", &self.echo_state)
            .field("has_echo_operation", &self.echo_operation_id.is_some())
            .field("has_echo_provider_version", &self.echo_provider_version.is_some())
            .field("delete_candidate_generation", &self.delete_candidate_generation)
            .field("delete_candidate_blocked", &self.delete_candidate_blocked)
            .field(
                "has_delete_confirmation_audit",
                &self.delete_confirmation_audit_id.is_some(),
            )
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

/// Retry-safe typed operation outcome. Fingerprints and provider identifiers are
/// intentionally redacted from Debug output.
#[derive(Clone, PartialEq)]
pub struct GDriveOperationRow {
    pub adapter_id: String,
    pub operation_id: String,
    pub operation_kind: String,
    pub facts_hash: String,
    pub outcome_kind: String,
    pub committed_state_version: i64,
    pub mapping_path: Option<String>,
    pub core_seq: Option<i64>,
    pub drive_version: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl fmt::Debug for GDriveOperationRow {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GDriveOperationRow")
            .field("adapter_id", &self.adapter_id)
            .field("operation_id", &"[REDACTED]")
            .field("operation_kind", &self.operation_kind)
            .field("facts_hash", &"[REDACTED]")
            .field("outcome_kind", &self.outcome_kind)
            .field("committed_state_version", &self.committed_state_version)
            .field("mapping_path", &self.mapping_path)
            .field("core_seq", &self.core_seq)
            .field("has_drive_version", &self.drive_version.is_some())
            .field("created_at", &self.created_at)
            .finish()
    }
}

const STATE_COLUMNS: &str = "adapter_id, state_format_version, state_version, drive_cursor, \
     drive_cursor_generation, core_export_seq, last_import_operation_id, \
     last_export_operation_id, last_provider_mutation_operation_id, created_at, updated_at";
const ITEM_COLUMNS: &str = "adapter_id, path, drive_file_id, drive_parent_id, drive_name, \
     mime_type, md5_checksum, head_revision_id, drive_version, drive_modified_time, \
     core_object_id, core_revision_id, core_seq, echo_state, echo_operation_id, \
     echo_provider_version, delete_candidate_first_seen_at, delete_candidate_last_seen_at, \
     delete_candidate_generation, delete_candidate_blocked, delete_confirmation_audit_id, \
     last_imported_at, last_exported_at, last_seen_at, created_at, updated_at";

const INITIALIZE_STATE_SQL: &str = "insert into gdrive_adapter_state (adapter_id, state_format_version) \
     values ($1, $2) on conflict (adapter_id) do update \
     set adapter_id = gdrive_adapter_state.adapter_id \
     returning adapter_id, state_format_version, state_version, drive_cursor, \
         drive_cursor_generation, core_export_seq, last_import_operation_id, \
         last_export_operation_id, last_provider_mutation_operation_id, created_at, updated_at";
const LOCK_STATE_SQL: &str = "select adapter_id, state_format_version, state_version, drive_cursor, \
     drive_cursor_generation, core_export_seq, last_import_operation_id, \
     last_export_operation_id, last_provider_mutation_operation_id, created_at, updated_at \
     from gdrive_adapter_state where adapter_id = $1 for update";
const SELECT_OPERATION_SQL: &str = "select adapter_id, operation_id, operation_kind, facts_hash, \
     outcome_kind, committed_state_version, mapping_path, core_seq, drive_version, created_at \
     from gdrive_operations where adapter_id = $1 and operation_id = $2";
const UPSERT_ITEM_SQL: &str = "insert into gdrive_durable_items ( \
     adapter_id, path, drive_file_id, drive_parent_id, drive_name, mime_type, md5_checksum, \
     head_revision_id, drive_version, drive_modified_time, core_object_id, core_revision_id, \
     core_seq, echo_state, echo_operation_id, echo_provider_version, \
     delete_candidate_first_seen_at, delete_candidate_last_seen_at, \
     delete_candidate_generation, delete_candidate_blocked, delete_confirmation_audit_id, \
     last_imported_at, last_exported_at, last_seen_at \
 ) values ( \
     $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, \
     $17, $18, $19, $20, $21, $22, $23, $24 \
 ) on conflict (adapter_id, path) do update set \
     drive_file_id = excluded.drive_file_id, drive_parent_id = excluded.drive_parent_id, \
     drive_name = excluded.drive_name, mime_type = excluded.mime_type, \
     md5_checksum = excluded.md5_checksum, head_revision_id = excluded.head_revision_id, \
     drive_version = excluded.drive_version, drive_modified_time = excluded.drive_modified_time, \
     core_object_id = excluded.core_object_id, core_revision_id = excluded.core_revision_id, \
     core_seq = excluded.core_seq, echo_state = excluded.echo_state, \
     echo_operation_id = excluded.echo_operation_id, \
     echo_provider_version = excluded.echo_provider_version, \
     delete_candidate_first_seen_at = excluded.delete_candidate_first_seen_at, \
     delete_candidate_last_seen_at = excluded.delete_candidate_last_seen_at, \
     delete_candidate_generation = excluded.delete_candidate_generation, \
     delete_candidate_blocked = excluded.delete_candidate_blocked, \
     delete_confirmation_audit_id = excluded.delete_confirmation_audit_id, \
     last_imported_at = excluded.last_imported_at, last_exported_at = excluded.last_exported_at, \
     last_seen_at = excluded.last_seen_at, updated_at = now() \
 returning adapter_id, path, drive_file_id, drive_parent_id, drive_name, mime_type, \
     md5_checksum, head_revision_id, drive_version, drive_modified_time, core_object_id, \
     core_revision_id, core_seq, echo_state, echo_operation_id, echo_provider_version, \
     delete_candidate_first_seen_at, delete_candidate_last_seen_at, \
     delete_candidate_generation, delete_candidate_blocked, delete_confirmation_audit_id, \
     last_imported_at, last_exported_at, last_seen_at, created_at, updated_at";
const UPDATE_STATE_SQL: &str = "update gdrive_adapter_state set \
     state_version = $3, \
     drive_cursor = coalesce($4, drive_cursor), \
     drive_cursor_generation = coalesce($5, drive_cursor_generation), \
     core_export_seq = coalesce($6, core_export_seq), \
     last_import_operation_id = case when $7 = 'import' then $8 else last_import_operation_id end, \
     last_export_operation_id = case when $7 = 'export' then $8 else last_export_operation_id end, \
     last_provider_mutation_operation_id = case when $7 = 'provider_mutation' then $8 else last_provider_mutation_operation_id end, \
     updated_at = now() \
     where adapter_id = $1 and state_version = $2 \
     returning adapter_id, state_format_version, state_version, drive_cursor, \
         drive_cursor_generation, core_export_seq, last_import_operation_id, \
         last_export_operation_id, last_provider_mutation_operation_id, created_at, updated_at";
const INSERT_OPERATION_SQL: &str = "insert into gdrive_operations ( \
     adapter_id, operation_id, operation_kind, facts_hash, outcome_kind, \
     committed_state_version, mapping_path, core_seq, drive_version \
 ) values ($1, $2, $3, $4, 'committed', $5, $6, $7, $8) \
 returning adapter_id, operation_id, operation_kind, facts_hash, outcome_kind, \
     committed_state_version, mapping_path, core_seq, drive_version, created_at";

#[derive(Clone, Eq, PartialEq)]
pub struct GDriveCursor(String);

impl GDriveCursor {
    pub fn parse(value: impl Into<String>) -> RepositoryResult<Self> {
        let value = value.into();
        if value.is_empty()
            || value.len() > MAX_CURSOR_LEN
            || value.chars().any(char::is_control)
        {
            return Err(RepositoryError::InvalidProviderMetadata);
        }
        Ok(Self(value))
    }

    #[must_use]
    fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for GDriveCursor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("GDriveCursor([REDACTED])")
    }
}

impl fmt::Display for GDriveCursor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("[REDACTED]")
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct GDriveOperationFingerprint(String);

impl GDriveOperationFingerprint {
    pub fn parse(value: impl Into<String>) -> RepositoryResult<Self> {
        let value = value.into();
        if value.len() != SHA256_HEX_LEN || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(RepositoryError::InvalidHash);
        }
        Ok(Self(value.to_ascii_lowercase()))
    }

    #[must_use]
    fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for GDriveOperationFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("GDriveOperationFingerprint([REDACTED])")
    }
}

impl fmt::Display for GDriveOperationFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("[REDACTED]")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GDriveEchoState {
    None,
    Pending,
    Confirmed,
}

impl GDriveEchoState {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Pending => "pending",
            Self::Confirmed => "confirmed",
        }
    }

    fn parse(value: &str) -> RepositoryResult<Self> {
        match value {
            "none" => Ok(Self::None),
            "pending" => Ok(Self::Pending),
            "confirmed" => Ok(Self::Confirmed),
            _ => Err(RepositoryError::InvalidProviderMetadata),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GDriveOperationKind {
    Import,
    Export,
    ProviderMutation,
    CursorCheckpoint,
    DeleteCandidate,
}

impl GDriveOperationKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Import => "import",
            Self::Export => "export",
            Self::ProviderMutation => "provider_mutation",
            Self::CursorCheckpoint => "cursor_checkpoint",
            Self::DeleteCandidate => "delete_candidate",
        }
    }

    fn parse(value: &str) -> RepositoryResult<Self> {
        match value {
            "import" => Ok(Self::Import),
            "export" => Ok(Self::Export),
            "provider_mutation" => Ok(Self::ProviderMutation),
            "cursor_checkpoint" => Ok(Self::CursorCheckpoint),
            "delete_candidate" => Ok(Self::DeleteCandidate),
            _ => Err(RepositoryError::InvalidOperationKind),
        }
    }
}

#[derive(Clone)]
pub struct GDriveCursorAdvance<'a> {
    pub cursor: &'a GDriveCursor,
    pub expected_generation: i64,
    pub next_generation: i64,
}

#[derive(Clone)]
pub struct GDriveItemUpsert<'a> {
    pub path: &'a VaultPath,
    pub drive_file_id: Option<&'a str>,
    pub drive_parent_id: Option<&'a str>,
    pub drive_name: Option<&'a str>,
    pub mime_type: Option<&'a str>,
    pub md5_checksum: Option<&'a str>,
    pub head_revision_id: Option<&'a str>,
    pub drive_version: Option<&'a str>,
    pub drive_modified_time: Option<DateTime<Utc>>,
    pub core_object_id: Option<&'a str>,
    pub core_revision_id: Option<&'a RevisionId>,
    pub core_seq: Option<i64>,
    pub echo_state: GDriveEchoState,
    pub echo_operation_id: Option<&'a OperationId>,
    pub echo_provider_version: Option<&'a str>,
    pub delete_candidate_first_seen_at: Option<DateTime<Utc>>,
    pub delete_candidate_last_seen_at: Option<DateTime<Utc>>,
    pub delete_candidate_generation: Option<i64>,
    pub delete_candidate_blocked: bool,
    pub delete_confirmation_audit_id: Option<&'a str>,
    pub last_imported_at: Option<DateTime<Utc>>,
    pub last_exported_at: Option<DateTime<Utc>>,
    pub last_seen_at: Option<DateTime<Utc>>,
}

#[derive(Clone)]
pub struct GDriveOperationInput<'a> {
    pub operation_id: &'a OperationId,
    pub kind: GDriveOperationKind,
    pub facts_fingerprint: &'a GDriveOperationFingerprint,
    pub mapping_path: Option<&'a VaultPath>,
    pub core_seq: Option<i64>,
    pub drive_version: Option<&'a str>,
}

#[derive(Clone)]
pub struct GDriveStateCommit<'a> {
    pub adapter_id: &'a AdapterId,
    pub expected_state_version: i64,
    pub cursor_advance: Option<GDriveCursorAdvance<'a>>,
    pub core_export_seq: Option<i64>,
    pub item: Option<GDriveItemUpsert<'a>>,
    pub operation: GDriveOperationInput<'a>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GDriveStateSnapshotPage {
    pub state: GDriveAdapterStateRow,
    pub items: Vec<GDriveDurableItemRow>,
    pub next_after_path: Option<VaultPath>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GDriveCommitOutcome {
    Committed {
        state: GDriveAdapterStateRow,
        operation: GDriveOperationRow,
    },
    Replayed {
        operation: GDriveOperationRow,
    },
}
