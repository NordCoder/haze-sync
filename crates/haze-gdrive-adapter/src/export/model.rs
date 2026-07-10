use crate::config::AdapterMode;
use crate::hash::ContentSha256;
use crate::state::{GDriveMapping, SafeTimestamp, VaultPath};
use std::error::Error;
use std::fmt;

pub const DEFAULT_CORE_CHANGE_PAGE_LIMIT: usize = 500;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportModelError {
    EmptyField(&'static str),
    InvalidChangePage,
}

impl fmt::Display for ExportModelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyField(field) => write!(formatter, "export field {field} must not be empty"),
            Self::InvalidChangePage => {
                formatter.write_str("Core export change page is not monotonic")
            }
        }
    }
}

impl Error for ExportModelError {}

#[derive(Clone, PartialEq, Eq)]
pub enum CoreExportChange {
    UpsertFile {
        seq: u64,
        operation_id: String,
        path: VaultPath,
        revision_id: String,
        content_sha256: ContentSha256,
        size_bytes: u64,
        updated_by: String,
    },
    Tombstone {
        seq: u64,
        operation_id: String,
        path: VaultPath,
        tombstone_id: String,
        updated_by: String,
    },
}

impl CoreExportChange {
    pub fn upsert_file(
        seq: u64,
        operation_id: impl Into<String>,
        path: VaultPath,
        revision_id: impl Into<String>,
        content_sha256: ContentSha256,
        size_bytes: u64,
        updated_by: impl Into<String>,
    ) -> Result<Self, ExportModelError> {
        Ok(Self::UpsertFile {
            seq,
            operation_id: required_string("operation_id", operation_id)?,
            path,
            revision_id: required_string("revision_id", revision_id)?,
            content_sha256,
            size_bytes,
            updated_by: required_string("updated_by", updated_by)?,
        })
    }

    pub fn tombstone(
        seq: u64,
        operation_id: impl Into<String>,
        path: VaultPath,
        tombstone_id: impl Into<String>,
        updated_by: impl Into<String>,
    ) -> Result<Self, ExportModelError> {
        Ok(Self::Tombstone {
            seq,
            operation_id: required_string("operation_id", operation_id)?,
            path,
            tombstone_id: required_string("tombstone_id", tombstone_id)?,
            updated_by: required_string("updated_by", updated_by)?,
        })
    }

    pub const fn sequence(&self) -> u64 {
        match self {
            Self::UpsertFile { seq, .. } | Self::Tombstone { seq, .. } => *seq,
        }
    }

    pub fn operation_id(&self) -> &str {
        match self {
            Self::UpsertFile { operation_id, .. } | Self::Tombstone { operation_id, .. } => {
                operation_id
            }
        }
    }

    pub fn path(&self) -> &VaultPath {
        match self {
            Self::UpsertFile { path, .. } | Self::Tombstone { path, .. } => path,
        }
    }

    pub fn updated_by(&self) -> &str {
        match self {
            Self::UpsertFile { updated_by, .. } | Self::Tombstone { updated_by, .. } => updated_by,
        }
    }
}

impl fmt::Debug for CoreExportChange {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UpsertFile {
                seq,
                path,
                revision_id,
                content_sha256,
                size_bytes,
                updated_by,
                ..
            } => formatter
                .debug_struct("UpsertFile")
                .field("seq", seq)
                .field("operation_id", &"<redacted-idempotency-key>")
                .field("path", path)
                .field("revision_id", revision_id)
                .field("content_sha256", content_sha256)
                .field("size_bytes", size_bytes)
                .field("updated_by", updated_by)
                .finish(),
            Self::Tombstone {
                seq,
                path,
                tombstone_id,
                updated_by,
                ..
            } => formatter
                .debug_struct("Tombstone")
                .field("seq", seq)
                .field("operation_id", &"<redacted-idempotency-key>")
                .field("path", path)
                .field("tombstone_id", tombstone_id)
                .field("updated_by", updated_by)
                .finish(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreExportPage {
    pub from_sequence: u64,
    pub next_sequence: u64,
    pub has_more: bool,
    pub changes: Vec<CoreExportChange>,
}

impl CoreExportPage {
    pub fn new(
        from_sequence: u64,
        next_sequence: u64,
        has_more: bool,
        changes: Vec<CoreExportChange>,
    ) -> Result<Self, ExportModelError> {
        let mut previous = None;
        for change in &changes {
            let sequence = change.sequence();
            if sequence < from_sequence
                || previous.is_some_and(|previous| sequence <= previous)
                || next_sequence <= sequence
            {
                return Err(ExportModelError::InvalidChangePage);
            }
            previous = Some(sequence);
        }
        if next_sequence < from_sequence {
            return Err(ExportModelError::InvalidChangePage);
        }
        Ok(Self {
            from_sequence,
            next_sequence,
            has_more,
            changes,
        })
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct CoreFileContent {
    path: VaultPath,
    revision_id: String,
    declared_sha256: ContentSha256,
    declared_size_bytes: u64,
    content: Vec<u8>,
}

impl CoreFileContent {
    pub fn new(
        path: VaultPath,
        revision_id: impl Into<String>,
        declared_sha256: ContentSha256,
        declared_size_bytes: u64,
        content: impl Into<Vec<u8>>,
    ) -> Result<Self, ExportModelError> {
        Ok(Self {
            path,
            revision_id: required_string("revision_id", revision_id)?,
            declared_sha256,
            declared_size_bytes,
            content: content.into(),
        })
    }

    pub fn path(&self) -> &VaultPath {
        &self.path
    }

    pub fn revision_id(&self) -> &str {
        &self.revision_id
    }

    pub const fn declared_sha256(&self) -> ContentSha256 {
        self.declared_sha256
    }

    pub const fn declared_size_bytes(&self) -> u64 {
        self.declared_size_bytes
    }

    pub fn content(&self) -> &[u8] {
        &self.content
    }

    pub fn into_content(self) -> Vec<u8> {
        self.content
    }
}

impl fmt::Debug for CoreFileContent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CoreFileContent")
            .field("path", &self.path)
            .field("revision_id", &self.revision_id)
            .field("declared_sha256", &self.declared_sha256)
            .field("declared_size_bytes", &self.declared_size_bytes)
            .field("content", &"<redacted-file-bytes>")
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriveCreateTarget {
    pub parent_id: String,
    pub name: String,
}

impl DriveCreateTarget {
    pub fn new(
        parent_id: impl Into<String>,
        name: impl Into<String>,
    ) -> Result<Self, ExportModelError> {
        Ok(Self {
            parent_id: required_string("parent_id", parent_id)?,
            name: required_string("name", name)?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportExecution {
    Submit,
    DryRun,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportSkipReason {
    ModeDoesNotExport,
    MappingAlreadyReflectsChange,
    MissingMappingForTombstone,
}

#[derive(Clone, PartialEq, Eq)]
pub struct VerifiedExportSource {
    pub path: VaultPath,
    pub revision_id: String,
    pub content_sha256: ContentSha256,
    pub size_bytes: u64,
    content: Vec<u8>,
}

impl VerifiedExportSource {
    pub fn content(&self) -> &[u8] {
        &self.content
    }

    pub fn into_content(self) -> Vec<u8> {
        self.content
    }

    pub(super) fn new(
        path: VaultPath,
        revision_id: String,
        content_sha256: ContentSha256,
        size_bytes: u64,
        content: Vec<u8>,
    ) -> Self {
        Self {
            path,
            revision_id,
            content_sha256,
            size_bytes,
            content,
        }
    }
}

impl fmt::Debug for VerifiedExportSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VerifiedExportSource")
            .field("path", &self.path)
            .field("revision_id", &self.revision_id)
            .field("content_sha256", &self.content_sha256)
            .field("size_bytes", &self.size_bytes)
            .field("content", &"<redacted-file-bytes>")
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportPlanItem {
    Skip {
        change: CoreExportChange,
        reason: ExportSkipReason,
    },
    Create {
        change: CoreExportChange,
        target: DriveCreateTarget,
        source: VerifiedExportSource,
        execution: ExportExecution,
    },
    Update {
        change: CoreExportChange,
        mapping: GDriveMapping,
        source: VerifiedExportSource,
        execution: ExportExecution,
    },
    Trash {
        change: CoreExportChange,
        mapping: GDriveMapping,
        execution: ExportExecution,
    },
}

impl ExportPlanItem {
    pub const fn execution(&self) -> Option<ExportExecution> {
        match self {
            Self::Skip { .. } => None,
            Self::Create { execution, .. }
            | Self::Update { execution, .. }
            | Self::Trash { execution, .. } => Some(*execution),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportCycleInput {
    pub adapter_id: String,
    pub mode: AdapterMode,
    pub dry_run: bool,
    pub applied_at: SafeTimestamp,
    pub page_limit: usize,
    pub provider_attempt: usize,
}

impl ExportCycleInput {
    pub fn new(
        adapter_id: impl Into<String>,
        mode: AdapterMode,
        dry_run: bool,
        applied_at: SafeTimestamp,
    ) -> Result<Self, ExportModelError> {
        Ok(Self {
            adapter_id: required_string("adapter_id", adapter_id)?,
            mode,
            dry_run,
            applied_at,
            page_limit: DEFAULT_CORE_CHANGE_PAGE_LIMIT,
            provider_attempt: 0,
        })
    }

    pub const fn with_page_limit(mut self, page_limit: usize) -> Self {
        self.page_limit = page_limit;
        self
    }

    pub const fn with_provider_attempt(mut self, provider_attempt: usize) -> Self {
        self.provider_attempt = provider_attempt;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportCycleOutcome {
    pub changes_received: usize,
    pub work_items_planned: usize,
    pub provider_mutations: usize,
    pub mappings_saved: usize,
    pub echoes_recorded: usize,
    pub cursor_saved: bool,
    pub next_sequence: u64,
    pub has_more: bool,
}

fn required_string(
    field: &'static str,
    value: impl Into<String>,
) -> Result<String, ExportModelError> {
    let value = value.into();
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ExportModelError::EmptyField(field));
    }
    Ok(trimmed.to_owned())
}
