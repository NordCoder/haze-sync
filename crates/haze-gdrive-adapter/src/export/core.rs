use super::model::{CoreExportPage, CoreFileContent};
use crate::state::VaultPath;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreClientErrorCategory {
    Auth,
    RateLimit,
    Unavailable,
    NotFound,
    InvalidResponse,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreClientError {
    operation: &'static str,
    category: CoreClientErrorCategory,
    safe_detail: &'static str,
}

impl CoreClientError {
    pub const fn new(
        operation: &'static str,
        category: CoreClientErrorCategory,
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
        category: CoreClientErrorCategory,
        _raw_payload: &str,
    ) -> Self {
        Self::new(operation, category, "Core/API error payload redacted")
    }

    pub const fn operation(&self) -> &'static str {
        self.operation
    }

    pub const fn category(&self) -> CoreClientErrorCategory {
        self.category
    }
}

impl fmt::Display for CoreClientError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Core/API operation {} failed as {:?}: {}",
            self.operation, self.category, self.safe_detail
        )
    }
}

impl Error for CoreClientError {}

pub trait CoreExportClient {
    fn list_changes(
        &self,
        from_sequence: u64,
        limit: usize,
    ) -> Result<CoreExportPage, CoreClientError>;

    fn download_revision(
        &self,
        path: &VaultPath,
        revision_id: &str,
    ) -> Result<CoreFileContent, CoreClientError>;
}

#[derive(Clone, Default)]
pub struct FakeCoreExportClient {
    pages_by_sequence: BTreeMap<u64, CoreExportPage>,
    content_by_revision: BTreeMap<(String, String), CoreFileContent>,
    errors_by_operation: BTreeMap<&'static str, CoreClientError>,
}

impl FakeCoreExportClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_page(mut self, page: CoreExportPage) -> Self {
        self.pages_by_sequence.insert(page.from_sequence, page);
        self
    }

    pub fn with_content(mut self, content: CoreFileContent) -> Self {
        self.content_by_revision.insert(
            (
                content.path().as_str().to_owned(),
                content.revision_id().to_owned(),
            ),
            content,
        );
        self
    }

    pub fn with_error(mut self, operation: &'static str, error: CoreClientError) -> Self {
        self.errors_by_operation.insert(operation, error);
        self
    }

    fn configured_error(&self, operation: &'static str) -> Option<CoreClientError> {
        self.errors_by_operation.get(operation).cloned()
    }
}

impl fmt::Debug for FakeCoreExportClient {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FakeCoreExportClient")
            .field("configured_pages", &self.pages_by_sequence.len())
            .field("configured_revisions", &self.content_by_revision.len())
            .field("configured_errors", &self.errors_by_operation.len())
            .finish()
    }
}

impl CoreExportClient for FakeCoreExportClient {
    fn list_changes(
        &self,
        from_sequence: u64,
        _limit: usize,
    ) -> Result<CoreExportPage, CoreClientError> {
        if let Some(error) = self.configured_error("list_changes") {
            return Err(error);
        }
        self.pages_by_sequence
            .get(&from_sequence)
            .cloned()
            .ok_or_else(|| {
                CoreClientError::new(
                    "list_changes",
                    CoreClientErrorCategory::NotFound,
                    "fake Core change page not configured",
                )
            })
    }

    fn download_revision(
        &self,
        path: &VaultPath,
        revision_id: &str,
    ) -> Result<CoreFileContent, CoreClientError> {
        if let Some(error) = self.configured_error("download_revision") {
            return Err(error);
        }
        self.content_by_revision
            .get(&(path.as_str().to_owned(), revision_id.to_owned()))
            .cloned()
            .ok_or_else(|| {
                CoreClientError::new(
                    "download_revision",
                    CoreClientErrorCategory::NotFound,
                    "fake Core revision not configured",
                )
            })
    }
}
