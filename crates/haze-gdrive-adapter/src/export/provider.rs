use crate::hash::ContentSha256;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriveExportErrorCategory {
    Auth,
    Conflict,
    RateLimit,
    Unavailable,
    NotFound,
    InvalidRequest,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriveExportError {
    operation: &'static str,
    category: DriveExportErrorCategory,
    safe_detail: &'static str,
}

impl DriveExportError {
    pub const fn new(
        operation: &'static str,
        category: DriveExportErrorCategory,
        safe_detail: &'static str,
    ) -> Self {
        Self {
            operation,
            category,
            safe_detail,
        }
    }

    pub const fn from_raw_payload(
        operation: &'static str,
        category: DriveExportErrorCategory,
        _raw_payload: &str,
    ) -> Self {
        Self::new(operation, category, "provider error payload redacted")
    }

    pub const fn operation(&self) -> &'static str {
        self.operation
    }

    pub const fn category(&self) -> DriveExportErrorCategory {
        self.category
    }
}

impl fmt::Display for DriveExportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Drive export operation {} failed as {:?}: {}",
            self.operation, self.category, self.safe_detail
        )
    }
}

impl Error for DriveExportError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportRetryDisposition {
    RetryAfter(Duration),
    Reauthenticate,
    DoNotRetry,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportRetryPolicy {
    delays: [Duration; 4],
}

impl ExportRetryPolicy {
    pub const fn new(delays: [Duration; 4]) -> Self {
        Self { delays }
    }

    pub fn classify_provider(
        &self,
        category: DriveExportErrorCategory,
        attempt: usize,
    ) -> ExportRetryDisposition {
        match category {
            DriveExportErrorCategory::RateLimit
            | DriveExportErrorCategory::Unavailable
            | DriveExportErrorCategory::Internal => {
                let index = attempt.min(self.delays.len() - 1);
                ExportRetryDisposition::RetryAfter(self.delays[index])
            }
            DriveExportErrorCategory::Auth => ExportRetryDisposition::Reauthenticate,
            DriveExportErrorCategory::Conflict
            | DriveExportErrorCategory::NotFound
            | DriveExportErrorCategory::InvalidRequest => ExportRetryDisposition::DoNotRetry,
        }
    }
}

impl Default for ExportRetryPolicy {
    fn default() -> Self {
        Self::new([
            Duration::from_secs(5),
            Duration::from_secs(15),
            Duration::from_secs(60),
            Duration::from_secs(300),
        ])
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct DriveCreateExportRequest {
    pub operation_id: String,
    pub parent_id: String,
    pub name: String,
    pub mime_type: String,
    pub content_sha256: ContentSha256,
    pub content: Vec<u8>,
}

impl fmt::Debug for DriveCreateExportRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DriveCreateExportRequest")
            .field("operation_id", &self.operation_id)
            .field("parent_id", &self.parent_id)
            .field("name", &self.name)
            .field("mime_type", &self.mime_type)
            .field("content_sha256", &self.content_sha256)
            .field("content", &"<redacted-file-bytes>")
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct DriveUpdateExportRequest {
    pub operation_id: String,
    pub file_id: String,
    pub expected_revision_token: Option<String>,
    pub mime_type: String,
    pub content_sha256: ContentSha256,
    pub content: Vec<u8>,
}

impl fmt::Debug for DriveUpdateExportRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DriveUpdateExportRequest")
            .field("operation_id", &self.operation_id)
            .field("file_id", &self.file_id)
            .field("has_expected_revision_token", &self.expected_revision_token.is_some())
            .field("mime_type", &self.mime_type)
            .field("content_sha256", &self.content_sha256)
            .field("content", &"<redacted-file-bytes>")
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriveTrashExportRequest {
    pub operation_id: String,
    pub file_id: String,
    pub expected_revision_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriveExportReceipt {
    pub provider_id: String,
    pub revision_token: String,
}

pub trait DriveExportProvider {
    fn create_file(
        &mut self,
        request: DriveCreateExportRequest,
    ) -> Result<DriveExportReceipt, DriveExportError>;

    fn update_file(
        &mut self,
        request: DriveUpdateExportRequest,
    ) -> Result<DriveExportReceipt, DriveExportError>;

    fn trash_file(
        &mut self,
        request: DriveTrashExportRequest,
    ) -> Result<DriveExportReceipt, DriveExportError>;
}

#[derive(Clone, PartialEq, Eq)]
struct FakeExportFile {
    parent_id: String,
    name: String,
    mime_type: String,
    content_sha256: ContentSha256,
    content: Vec<u8>,
    revision_token: String,
    trashed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum MutationFingerprint {
    Create {
        parent_id: String,
        name: String,
        content_sha256: ContentSha256,
    },
    Update {
        file_id: String,
        content_sha256: ContentSha256,
    },
    Trash {
        file_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AppliedMutation {
    fingerprint: MutationFingerprint,
    receipt: DriveExportReceipt,
}

#[derive(Clone, Default)]
pub struct FakeDriveExportProvider {
    files_by_id: BTreeMap<String, FakeExportFile>,
    applied_by_operation_id: BTreeMap<String, AppliedMutation>,
    errors_by_operation: BTreeMap<&'static str, DriveExportError>,
    next_file_number: u64,
    next_revision_number: u64,
    mutation_count: usize,
}

impl FakeDriveExportProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_file(
        mut self,
        file_id: impl Into<String>,
        parent_id: impl Into<String>,
        name: impl Into<String>,
        mime_type: impl Into<String>,
        content: impl Into<Vec<u8>>,
        revision_token: impl Into<String>,
    ) -> Self {
        let file_id = file_id.into();
        let content = content.into();
        self.files_by_id.insert(
            file_id,
            FakeExportFile {
                parent_id: parent_id.into(),
                name: name.into(),
                mime_type: mime_type.into(),
                content_sha256: ContentSha256::from_content(&content),
                content,
                revision_token: revision_token.into(),
                trashed: false,
            },
        );
        self
    }

    pub fn with_error(mut self, operation: &'static str, error: DriveExportError) -> Self {
        self.errors_by_operation.insert(operation, error);
        self
    }

    pub fn clear_error(&mut self, operation: &'static str) {
        self.errors_by_operation.remove(operation);
    }

    pub fn content(&self, file_id: &str) -> Option<&[u8]> {
        self.files_by_id
            .get(file_id)
            .map(|file| file.content.as_slice())
    }

    pub fn is_trashed(&self, file_id: &str) -> Option<bool> {
        self.files_by_id.get(file_id).map(|file| file.trashed)
    }

    pub fn revision_token(&self, file_id: &str) -> Option<&str> {
        self.files_by_id
            .get(file_id)
            .map(|file| file.revision_token.as_str())
    }

    pub const fn mutation_count(&self) -> usize {
        self.mutation_count
    }

    fn configured_error(&self, operation: &'static str) -> Option<DriveExportError> {
        self.errors_by_operation.get(operation).cloned()
    }

    fn replay_or_conflict(
        &self,
        operation_id: &str,
        fingerprint: &MutationFingerprint,
    ) -> Result<Option<DriveExportReceipt>, DriveExportError> {
        let Some(applied) = self.applied_by_operation_id.get(operation_id) else {
            return Ok(None);
        };
        if &applied.fingerprint == fingerprint {
            return Ok(Some(applied.receipt.clone()));
        }
        Err(provider_conflict("idempotent_replay"))
    }

    fn next_revision_token(&mut self) -> String {
        self.next_revision_number = self.next_revision_number.saturating_add(1);
        format!("fake-revision-{}", self.next_revision_number)
    }

    fn record_mutation(
        &mut self,
        operation_id: String,
        fingerprint: MutationFingerprint,
        receipt: DriveExportReceipt,
    ) -> DriveExportReceipt {
        self.mutation_count = self.mutation_count.saturating_add(1);
        self.applied_by_operation_id.insert(
            operation_id,
            AppliedMutation {
                fingerprint,
                receipt: receipt.clone(),
            },
        );
        receipt
    }
}

impl fmt::Debug for FakeDriveExportProvider {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FakeDriveExportProvider")
            .field("file_count", &self.files_by_id.len())
            .field("applied_operation_count", &self.applied_by_operation_id.len())
            .field("configured_error_count", &self.errors_by_operation.len())
            .field("mutation_count", &self.mutation_count)
            .finish()
    }
}

impl DriveExportProvider for FakeDriveExportProvider {
    fn create_file(
        &mut self,
        request: DriveCreateExportRequest,
    ) -> Result<DriveExportReceipt, DriveExportError> {
        if let Some(error) = self.configured_error("create_file") {
            return Err(error);
        }
        let fingerprint = MutationFingerprint::Create {
            parent_id: request.parent_id.clone(),
            name: request.name.clone(),
            content_sha256: request.content_sha256,
        };
        if let Some(receipt) = self.replay_or_conflict(&request.operation_id, &fingerprint)? {
            return Ok(receipt);
        }
        if !request.content_sha256.verifies(&request.content) {
            return Err(DriveExportError::new(
                "create_file",
                DriveExportErrorCategory::InvalidRequest,
                "verified export content hash changed before provider mutation",
            ));
        }

        self.next_file_number = self.next_file_number.saturating_add(1);
        let provider_id = format!("fake-export-{}", self.next_file_number);
        let revision_token = self.next_revision_token();
        self.files_by_id.insert(
            provider_id.clone(),
            FakeExportFile {
                parent_id: request.parent_id,
                name: request.name,
                mime_type: request.mime_type,
                content_sha256: request.content_sha256,
                content: request.content,
                revision_token: revision_token.clone(),
                trashed: false,
            },
        );
        let receipt = DriveExportReceipt {
            provider_id,
            revision_token,
        };
        Ok(self.record_mutation(request.operation_id, fingerprint, receipt))
    }

    fn update_file(
        &mut self,
        request: DriveUpdateExportRequest,
    ) -> Result<DriveExportReceipt, DriveExportError> {
        if let Some(error) = self.configured_error("update_file") {
            return Err(error);
        }
        let fingerprint = MutationFingerprint::Update {
            file_id: request.file_id.clone(),
            content_sha256: request.content_sha256,
        };
        if let Some(receipt) = self.replay_or_conflict(&request.operation_id, &fingerprint)? {
            return Ok(receipt);
        }
        if !request.content_sha256.verifies(&request.content) {
            return Err(DriveExportError::new(
                "update_file",
                DriveExportErrorCategory::InvalidRequest,
                "verified export content hash changed before provider mutation",
            ));
        }

        let current = self
            .files_by_id
            .get(&request.file_id)
            .ok_or_else(|| provider_not_found("update_file"))?;
        if current.trashed
            || request
                .expected_revision_token
                .as_deref()
                .is_some_and(|expected| expected != current.revision_token)
        {
            return Err(provider_conflict("update_file"));
        }

        let revision_token = self.next_revision_token();
        let file = self
            .files_by_id
            .get_mut(&request.file_id)
            .expect("file existence checked");
        file.mime_type = request.mime_type;
        file.content_sha256 = request.content_sha256;
        file.content = request.content;
        file.revision_token = revision_token.clone();
        let receipt = DriveExportReceipt {
            provider_id: request.file_id,
            revision_token,
        };
        Ok(self.record_mutation(request.operation_id, fingerprint, receipt))
    }

    fn trash_file(
        &mut self,
        request: DriveTrashExportRequest,
    ) -> Result<DriveExportReceipt, DriveExportError> {
        if let Some(error) = self.configured_error("trash_file") {
            return Err(error);
        }
        let fingerprint = MutationFingerprint::Trash {
            file_id: request.file_id.clone(),
        };
        if let Some(receipt) = self.replay_or_conflict(&request.operation_id, &fingerprint)? {
            return Ok(receipt);
        }

        let current = self
            .files_by_id
            .get(&request.file_id)
            .ok_or_else(|| provider_not_found("trash_file"))?;
        if request
            .expected_revision_token
            .as_deref()
            .is_some_and(|expected| expected != current.revision_token)
        {
            return Err(provider_conflict("trash_file"));
        }

        let revision_token = self.next_revision_token();
        let file = self
            .files_by_id
            .get_mut(&request.file_id)
            .expect("file existence checked");
        file.trashed = true;
        file.revision_token = revision_token.clone();
        let receipt = DriveExportReceipt {
            provider_id: request.file_id,
            revision_token,
        };
        Ok(self.record_mutation(request.operation_id, fingerprint, receipt))
    }
}

fn provider_conflict(operation: &'static str) -> DriveExportError {
    DriveExportError::new(
        operation,
        DriveExportErrorCategory::Conflict,
        "provider precondition or idempotency conflict",
    )
}

fn provider_not_found(operation: &'static str) -> DriveExportError {
    DriveExportError::new(
        operation,
        DriveExportErrorCategory::NotFound,
        "provider item not found",
    )
}
