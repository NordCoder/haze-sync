//! Fake-first Google Drive provider boundary and safe metadata normalization.
//!
//! This module owns provider-specific input DTOs at the adapter boundary and
//! normalizes them into safe adapter facts. It intentionally contains no real
//! Google SDK wiring and performs no provider side effects unless a caller uses a
//! concrete provider implementation.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

pub const MIME_GOOGLE_FOLDER: &str = "application/vnd.google-apps.folder";
pub const MIME_GOOGLE_DOC: &str = "application/vnd.google-apps.document";
pub const MIME_GOOGLE_SHEET: &str = "application/vnd.google-apps.spreadsheet";
pub const MIME_GOOGLE_SLIDE: &str = "application/vnd.google-apps.presentation";
pub const MIME_GOOGLE_SHORTCUT: &str = "application/vnd.google-apps.shortcut";
pub const MIME_TEXT_MARKDOWN: &str = "text/markdown";
pub const MIME_TEXT_PLAIN: &str = "text/plain";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriveEntryKind {
    File,
    Folder,
    GoogleDoc,
    GoogleSheet,
    GoogleSlide,
    Shortcut,
    SharedDrive,
    Unknown,
}

impl DriveEntryKind {
    pub fn from_mime_type(mime_type: Option<&str>) -> Self {
        match mime_type {
            Some(MIME_GOOGLE_FOLDER) => Self::Folder,
            Some(MIME_GOOGLE_DOC) => Self::GoogleDoc,
            Some(MIME_GOOGLE_SHEET) => Self::GoogleSheet,
            Some(MIME_GOOGLE_SLIDE) => Self::GoogleSlide,
            Some(MIME_GOOGLE_SHORTCUT) => Self::Shortcut,
            Some(_) => Self::File,
            None => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportedFileType {
    Markdown,
    PlainText,
    Binary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsupportedEntryReason {
    MissingName,
    Folder,
    GoogleWorkspaceDocument,
    Shortcut,
    SharedDrive,
    UnknownEntryType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriveEntryClassification {
    Supported(SupportedFileType),
    Unsupported(UnsupportedEntryReason),
}

impl DriveEntryClassification {
    pub const fn is_supported(self) -> bool {
        matches!(self, Self::Supported(_))
    }

    pub const fn is_unsupported(self) -> bool {
        matches!(self, Self::Unsupported(_))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriveMetadata {
    pub id: String,
    pub name: String,
    pub mime_type: Option<String>,
    pub kind: DriveEntryKind,
    pub parent_ids: Vec<String>,
    pub modified_time: Option<String>,
    pub size_bytes: Option<u64>,
    pub md5_checksum: Option<String>,
}

impl DriveMetadata {
    pub fn new_file(
        id: impl Into<String>,
        name: impl Into<String>,
        mime_type: impl Into<String>,
    ) -> Self {
        let mime_type = mime_type.into();
        Self {
            id: id.into(),
            name: name.into(),
            kind: DriveEntryKind::from_mime_type(Some(&mime_type)),
            mime_type: Some(mime_type),
            parent_ids: Vec::new(),
            modified_time: None,
            size_bytes: None,
            md5_checksum: None,
        }
    }

    pub fn new_special(
        id: impl Into<String>,
        name: impl Into<String>,
        mime_type: impl Into<String>,
    ) -> Self {
        Self::new_file(id, name, mime_type)
    }

    pub fn new_shared_drive(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            mime_type: None,
            kind: DriveEntryKind::SharedDrive,
            parent_ids: Vec::new(),
            modified_time: None,
            size_bytes: None,
            md5_checksum: None,
        }
    }

    pub fn with_parent(mut self, parent_id: impl Into<String>) -> Self {
        self.parent_ids.push(parent_id.into());
        self
    }

    pub fn with_size_bytes(mut self, size_bytes: u64) -> Self {
        self.size_bytes = Some(size_bytes);
        self
    }

    pub fn with_md5_checksum(mut self, md5_checksum: impl Into<String>) -> Self {
        self.md5_checksum = Some(md5_checksum.into());
        self
    }

    pub fn with_modified_time(mut self, modified_time: impl Into<String>) -> Self {
        self.modified_time = Some(modified_time.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedDriveEntry {
    pub provider_id: String,
    pub name: String,
    pub parent_ids: Vec<String>,
    pub classification: DriveEntryClassification,
    pub modified_time: Option<String>,
    pub size_bytes: Option<u64>,
    pub md5_checksum: Option<String>,
}

impl NormalizedDriveEntry {
    pub const fn is_supported(&self) -> bool {
        self.classification.is_supported()
    }

    pub const fn is_unsupported(&self) -> bool {
        self.classification.is_unsupported()
    }
}

pub fn normalize_drive_metadata(metadata: &DriveMetadata) -> NormalizedDriveEntry {
    NormalizedDriveEntry {
        provider_id: metadata.id.clone(),
        name: metadata.name.clone(),
        parent_ids: metadata.parent_ids.clone(),
        classification: classify_drive_metadata(metadata),
        modified_time: metadata.modified_time.clone(),
        size_bytes: metadata.size_bytes,
        md5_checksum: metadata.md5_checksum.clone(),
    }
}

pub fn classify_drive_metadata(metadata: &DriveMetadata) -> DriveEntryClassification {
    if metadata.name.trim().is_empty() {
        return DriveEntryClassification::Unsupported(UnsupportedEntryReason::MissingName);
    }

    match metadata.kind {
        DriveEntryKind::File => DriveEntryClassification::Supported(classify_file_type(metadata)),
        DriveEntryKind::Folder => {
            DriveEntryClassification::Unsupported(UnsupportedEntryReason::Folder)
        }
        DriveEntryKind::GoogleDoc | DriveEntryKind::GoogleSheet | DriveEntryKind::GoogleSlide => {
            DriveEntryClassification::Unsupported(UnsupportedEntryReason::GoogleWorkspaceDocument)
        }
        DriveEntryKind::Shortcut => {
            DriveEntryClassification::Unsupported(UnsupportedEntryReason::Shortcut)
        }
        DriveEntryKind::SharedDrive => {
            DriveEntryClassification::Unsupported(UnsupportedEntryReason::SharedDrive)
        }
        DriveEntryKind::Unknown => {
            DriveEntryClassification::Unsupported(UnsupportedEntryReason::UnknownEntryType)
        }
    }
}

fn classify_file_type(metadata: &DriveMetadata) -> SupportedFileType {
    let lower_name = metadata.name.to_ascii_lowercase();
    if lower_name.ends_with(".md") || lower_name.ends_with(".markdown") {
        return SupportedFileType::Markdown;
    }

    match metadata.mime_type.as_deref() {
        Some(MIME_TEXT_MARKDOWN) => SupportedFileType::Markdown,
        Some(MIME_TEXT_PLAIN) => SupportedFileType::PlainText,
        _ => SupportedFileType::Binary,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderErrorCategory {
    Auth,
    RateLimit,
    ProviderUnavailable,
    NotFound,
    Unsupported,
    InvalidRequest,
    Internal,
}

impl fmt::Display for ProviderErrorCategory {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Auth => "auth",
            Self::RateLimit => "rate_limit",
            Self::ProviderUnavailable => "provider_unavailable",
            Self::NotFound => "not_found",
            Self::Unsupported => "unsupported",
            Self::InvalidRequest => "invalid_request",
            Self::Internal => "internal",
        };
        formatter.write_str(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderError {
    operation: &'static str,
    category: ProviderErrorCategory,
    safe_detail: &'static str,
}

impl ProviderError {
    pub const fn new(
        operation: &'static str,
        category: ProviderErrorCategory,
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
        category: ProviderErrorCategory,
        _raw_payload: &str,
    ) -> Self {
        Self::new(operation, category, "provider error payload redacted")
    }

    pub const fn not_found(operation: &'static str) -> Self {
        Self::new(operation, ProviderErrorCategory::NotFound, "provider item not found")
    }

    pub const fn unsupported(operation: &'static str) -> Self {
        Self::new(
            operation,
            ProviderErrorCategory::Unsupported,
            "provider operation is not supported by this implementation",
        )
    }

    pub const fn operation(&self) -> &'static str {
        self.operation
    }

    pub const fn category(&self) -> ProviderErrorCategory {
        self.category
    }

    pub const fn safe_detail(&self) -> &'static str {
        self.safe_detail
    }
}

impl fmt::Display for ProviderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "provider operation {} failed as {}: {}",
            self.operation, self.category, self.safe_detail
        )
    }
}

impl Error for ProviderError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriveUploadRequest {
    pub parent_id: String,
    pub name: String,
    pub mime_type: String,
    pub content: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriveUpdateRequest {
    pub file_id: String,
    pub mime_type: String,
    pub content: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriveMutationOutcome {
    pub provider_id: String,
    pub revision_token: Option<String>,
}

pub trait DriveProvider {
    fn list_children(&self, folder_id: &str) -> Result<Vec<DriveMetadata>, ProviderError>;

    fn get_metadata(&self, file_id: &str) -> Result<DriveMetadata, ProviderError>;

    fn download_file(&self, file_id: &str) -> Result<Vec<u8>, ProviderError>;

    fn upload_file(
        &mut self,
        request: DriveUploadRequest,
    ) -> Result<DriveMutationOutcome, ProviderError>;

    fn update_file(
        &mut self,
        request: DriveUpdateRequest,
    ) -> Result<DriveMutationOutcome, ProviderError>;

    fn trash_file(&mut self, file_id: &str) -> Result<DriveMutationOutcome, ProviderError>;

    fn delete_file(&mut self, file_id: &str) -> Result<DriveMutationOutcome, ProviderError>;
}

#[derive(Debug, Clone, Default)]
pub struct FakeDriveProvider {
    metadata_by_id: BTreeMap<String, DriveMetadata>,
    children_by_parent_id: BTreeMap<String, Vec<String>>,
    content_by_id: BTreeMap<String, Vec<u8>>,
    operation_errors: BTreeMap<&'static str, ProviderError>,
    next_upload_number: u64,
}

impl FakeDriveProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_metadata(mut self, metadata: DriveMetadata) -> Self {
        self.metadata_by_id.insert(metadata.id.clone(), metadata);
        self
    }

    pub fn with_child(mut self, parent_id: impl Into<String>, metadata: DriveMetadata) -> Self {
        let parent_id = parent_id.into();
        self.children_by_parent_id
            .entry(parent_id)
            .or_default()
            .push(metadata.id.clone());
        self.metadata_by_id.insert(metadata.id.clone(), metadata);
        self
    }

    pub fn with_content(mut self, file_id: impl Into<String>, content: impl Into<Vec<u8>>) -> Self {
        self.content_by_id.insert(file_id.into(), content.into());
        self
    }

    pub fn with_error(mut self, operation: &'static str, error: ProviderError) -> Self {
        self.operation_errors.insert(operation, error);
        self
    }

    fn configured_error(&self, operation: &'static str) -> Option<ProviderError> {
        self.operation_errors.get(operation).cloned()
    }
}

impl DriveProvider for FakeDriveProvider {
    fn list_children(&self, folder_id: &str) -> Result<Vec<DriveMetadata>, ProviderError> {
        if let Some(error) = self.configured_error("list_children") {
            return Err(error);
        }

        let children = self
            .children_by_parent_id
            .get(folder_id)
            .into_iter()
            .flatten()
            .filter_map(|file_id| self.metadata_by_id.get(file_id))
            .cloned()
            .collect();
        Ok(children)
    }

    fn get_metadata(&self, file_id: &str) -> Result<DriveMetadata, ProviderError> {
        if let Some(error) = self.configured_error("get_metadata") {
            return Err(error);
        }

        self.metadata_by_id
            .get(file_id)
            .cloned()
            .ok_or_else(|| ProviderError::not_found("get_metadata"))
    }

    fn download_file(&self, file_id: &str) -> Result<Vec<u8>, ProviderError> {
        if let Some(error) = self.configured_error("download_file") {
            return Err(error);
        }

        self.content_by_id
            .get(file_id)
            .cloned()
            .ok_or_else(|| ProviderError::not_found("download_file"))
    }

    fn upload_file(
        &mut self,
        request: DriveUploadRequest,
    ) -> Result<DriveMutationOutcome, ProviderError> {
        if let Some(error) = self.configured_error("upload_file") {
            return Err(error);
        }

        self.next_upload_number += 1;
        let provider_id = format!("fake-upload-{}", self.next_upload_number);
        let metadata = DriveMetadata::new_file(
            provider_id.clone(),
            request.name,
            request.mime_type,
        )
        .with_parent(request.parent_id);
        self.metadata_by_id.insert(provider_id.clone(), metadata);
        self.content_by_id.insert(provider_id.clone(), request.content);
        Ok(DriveMutationOutcome {
            provider_id,
            revision_token: None,
        })
    }

    fn update_file(
        &mut self,
        request: DriveUpdateRequest,
    ) -> Result<DriveMutationOutcome, ProviderError> {
        if let Some(error) = self.configured_error("update_file") {
            return Err(error);
        }

        if !self.metadata_by_id.contains_key(&request.file_id) {
            return Err(ProviderError::not_found("update_file"));
        }

        self.content_by_id
            .insert(request.file_id.clone(), request.content);
        Ok(DriveMutationOutcome {
            provider_id: request.file_id,
            revision_token: None,
        })
    }

    fn trash_file(&mut self, file_id: &str) -> Result<DriveMutationOutcome, ProviderError> {
        if let Some(error) = self.configured_error("trash_file") {
            return Err(error);
        }

        if self.metadata_by_id.contains_key(file_id) {
            Ok(DriveMutationOutcome {
                provider_id: file_id.to_owned(),
                revision_token: None,
            })
        } else {
            Err(ProviderError::not_found("trash_file"))
        }
    }

    fn delete_file(&mut self, _file_id: &str) -> Result<DriveMutationOutcome, ProviderError> {
        if let Some(error) = self.configured_error("delete_file") {
            return Err(error);
        }

        Err(ProviderError::unsupported("delete_file"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_markdown_metadata_as_supported_fact() {
        let metadata = DriveMetadata::new_file("drive-file-1", "Notes/Plan.md", MIME_TEXT_PLAIN)
            .with_parent("root")
            .with_size_bytes(42)
            .with_md5_checksum("0123456789abcdef")
            .with_modified_time("2026-07-09T08:00:00Z");

        let fact = normalize_drive_metadata(&metadata);

        assert!(fact.is_supported());
        assert_eq!(fact.provider_id, "drive-file-1");
        assert_eq!(fact.name, "Notes/Plan.md");
        assert_eq!(fact.parent_ids, vec!["root"]);
        assert_eq!(
            fact.classification,
            DriveEntryClassification::Supported(SupportedFileType::Markdown)
        );
        assert_eq!(fact.size_bytes, Some(42));
        assert_eq!(fact.md5_checksum.as_deref(), Some("0123456789abcdef"));
    }

    #[test]
    fn classifies_google_workspace_entries_as_unsupported() {
        let doc = DriveMetadata::new_special("doc-1", "Live Doc", MIME_GOOGLE_DOC);
        let sheet = DriveMetadata::new_special("sheet-1", "Budget", MIME_GOOGLE_SHEET);
        let slide = DriveMetadata::new_special("slide-1", "Deck", MIME_GOOGLE_SLIDE);

        for metadata in [doc, sheet, slide] {
            let fact = normalize_drive_metadata(&metadata);

            assert_eq!(
                fact.classification,
                DriveEntryClassification::Unsupported(
                    UnsupportedEntryReason::GoogleWorkspaceDocument
                )
            );
        }
    }

    #[test]
    fn classifies_shortcuts_and_shared_drives_as_unsupported() {
        let shortcut = DriveMetadata::new_special("shortcut-1", "Alias", MIME_GOOGLE_SHORTCUT);
        let shared_drive = DriveMetadata::new_shared_drive("drive-1", "Team Drive");

        assert_eq!(
            normalize_drive_metadata(&shortcut).classification,
            DriveEntryClassification::Unsupported(UnsupportedEntryReason::Shortcut)
        );
        assert_eq!(
            normalize_drive_metadata(&shared_drive).classification,
            DriveEntryClassification::Unsupported(UnsupportedEntryReason::SharedDrive)
        );
    }

    #[test]
    fn classifies_missing_name_as_unsupported_without_provider_policy() {
        let metadata = DriveMetadata::new_file("file-1", "   ", MIME_TEXT_PLAIN);

        let fact = normalize_drive_metadata(&metadata);

        assert_eq!(
            fact.classification,
            DriveEntryClassification::Unsupported(UnsupportedEntryReason::MissingName)
        );
    }

    #[test]
    fn fake_provider_lists_metadata_and_downloads_content() {
        let metadata = DriveMetadata::new_file("file-1", "note.md", MIME_TEXT_MARKDOWN)
            .with_parent("root");
        let provider = FakeDriveProvider::new()
            .with_child("root", metadata)
            .with_content("file-1", b"hello".to_vec());

        let children = provider.list_children("root").expect("children");
        let content = provider.download_file("file-1").expect("content");

        assert_eq!(children.len(), 1);
        assert_eq!(children[0].id, "file-1");
        assert_eq!(content, b"hello".to_vec());
    }

    #[test]
    fn fake_provider_supports_upload_and_update_without_live_calls() {
        let mut provider = FakeDriveProvider::new();
        let upload = provider
            .upload_file(DriveUploadRequest {
                parent_id: "root".to_owned(),
                name: "created.md".to_owned(),
                mime_type: MIME_TEXT_MARKDOWN.to_owned(),
                content: b"created".to_vec(),
            })
            .expect("upload");

        let update = provider
            .update_file(DriveUpdateRequest {
                file_id: upload.provider_id.clone(),
                mime_type: MIME_TEXT_MARKDOWN.to_owned(),
                content: b"updated".to_vec(),
            })
            .expect("update");

        assert_eq!(update.provider_id, upload.provider_id);
        assert_eq!(
            provider.download_file(&upload.provider_id).expect("content"),
            b"updated".to_vec()
        );
    }

    #[test]
    fn provider_error_redacts_raw_payload_from_display_and_debug() {
        let error = ProviderError::from_raw_payload(
            "list_children",
            ProviderErrorCategory::Auth,
            "Bearer real-token raw-provider-body",
        );

        let displayed = error.to_string();
        let debugged = format!("{error:?}");

        assert!(displayed.contains("payload redacted"));
        assert!(!displayed.contains("real-token"));
        assert!(!displayed.contains("raw-provider-body"));
        assert!(!debugged.contains("real-token"));
        assert!(!debugged.contains("raw-provider-body"));
    }

    #[test]
    fn fake_provider_returns_sanitized_configured_error() {
        let provider = FakeDriveProvider::new().with_error(
            "list_children",
            ProviderError::from_raw_payload(
                "list_children",
                ProviderErrorCategory::RateLimit,
                "quota raw body",
            ),
        );

        let error = provider
            .list_children("root")
            .expect_err("list should fail");

        assert_eq!(error.category(), ProviderErrorCategory::RateLimit);
        assert!(!error.to_string().contains("quota raw body"));
    }
}
