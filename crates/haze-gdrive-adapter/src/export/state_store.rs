use super::model::DriveCreateTarget;
use crate::state::{CoreChangeCursor, GDriveMapping, VaultPath};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportStateError {
    operation: &'static str,
    safe_detail: &'static str,
}

impl ExportStateError {
    pub const fn new(operation: &'static str, safe_detail: &'static str) -> Self {
        Self {
            operation,
            safe_detail,
        }
    }

    pub const fn operation(&self) -> &'static str {
        self.operation
    }
}

impl fmt::Display for ExportStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "export state operation {} failed: {}",
            self.operation, self.safe_detail
        )
    }
}

impl Error for ExportStateError {}

pub trait ExportStateStore {
    fn load_cursor(&self) -> Result<CoreChangeCursor, ExportStateError>;

    fn load_mapping(&self, path: &VaultPath) -> Result<Option<GDriveMapping>, ExportStateError>;

    fn resolve_create_target(
        &self,
        path: &VaultPath,
    ) -> Result<DriveCreateTarget, ExportStateError>;

    fn save_mapping(&mut self, mapping: GDriveMapping) -> Result<(), ExportStateError>;

    fn save_cursor(&mut self, cursor: &CoreChangeCursor) -> Result<(), ExportStateError>;
}

#[derive(Clone, PartialEq, Eq)]
pub struct InMemoryExportStateStore {
    cursor: CoreChangeCursor,
    root_parent_id: String,
    folder_ids_by_path: BTreeMap<String, String>,
    mappings_by_path: BTreeMap<String, GDriveMapping>,
    load_error: Option<ExportStateError>,
    target_error: Option<ExportStateError>,
    save_mapping_error: Option<ExportStateError>,
    save_cursor_error: Option<ExportStateError>,
    mapping_save_count: usize,
    cursor_save_count: usize,
}

impl InMemoryExportStateStore {
    pub fn new(cursor: CoreChangeCursor, root_parent_id: impl Into<String>) -> Self {
        Self {
            cursor,
            root_parent_id: root_parent_id.into(),
            folder_ids_by_path: BTreeMap::new(),
            mappings_by_path: BTreeMap::new(),
            load_error: None,
            target_error: None,
            save_mapping_error: None,
            save_cursor_error: None,
            mapping_save_count: 0,
            cursor_save_count: 0,
        }
    }

    pub fn with_folder(
        mut self,
        vault_folder_path: impl Into<String>,
        provider_folder_id: impl Into<String>,
    ) -> Self {
        self.folder_ids_by_path
            .insert(vault_folder_path.into(), provider_folder_id.into());
        self
    }

    pub fn with_mapping(mut self, mapping: GDriveMapping) -> Self {
        self.mappings_by_path
            .insert(mapping.vault_path.as_str().to_owned(), mapping);
        self
    }

    pub fn with_load_error(mut self, error: ExportStateError) -> Self {
        self.load_error = Some(error);
        self
    }

    pub fn with_target_error(mut self, error: ExportStateError) -> Self {
        self.target_error = Some(error);
        self
    }

    pub fn with_save_mapping_error(mut self, error: ExportStateError) -> Self {
        self.save_mapping_error = Some(error);
        self
    }

    pub fn with_save_cursor_error(mut self, error: ExportStateError) -> Self {
        self.save_cursor_error = Some(error);
        self
    }

    pub fn clear_save_mapping_error(&mut self) {
        self.save_mapping_error = None;
    }

    pub fn clear_save_cursor_error(&mut self) {
        self.save_cursor_error = None;
    }

    pub fn cursor(&self) -> &CoreChangeCursor {
        &self.cursor
    }

    pub fn mapping(&self, path: &str) -> Option<&GDriveMapping> {
        self.mappings_by_path.get(path)
    }

    pub const fn mapping_save_count(&self) -> usize {
        self.mapping_save_count
    }

    pub const fn cursor_save_count(&self) -> usize {
        self.cursor_save_count
    }
}

impl fmt::Debug for InMemoryExportStateStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InMemoryExportStateStore")
            .field("cursor", &self.cursor)
            .field("folder_mapping_count", &self.folder_ids_by_path.len())
            .field("file_mapping_count", &self.mappings_by_path.len())
            .field("mapping_save_count", &self.mapping_save_count)
            .field("cursor_save_count", &self.cursor_save_count)
            .finish()
    }
}

impl ExportStateStore for InMemoryExportStateStore {
    fn load_cursor(&self) -> Result<CoreChangeCursor, ExportStateError> {
        if let Some(error) = &self.load_error {
            return Err(error.clone());
        }
        Ok(self.cursor.clone())
    }

    fn load_mapping(&self, path: &VaultPath) -> Result<Option<GDriveMapping>, ExportStateError> {
        if let Some(error) = &self.load_error {
            return Err(error.clone());
        }
        Ok(self.mappings_by_path.get(path.as_str()).cloned())
    }

    fn resolve_create_target(
        &self,
        path: &VaultPath,
    ) -> Result<DriveCreateTarget, ExportStateError> {
        if let Some(error) = &self.target_error {
            return Err(error.clone());
        }
        let mut segments = path.as_str().rsplitn(2, '/');
        let name = segments.next().expect("validated VaultPath has a segment");
        let parent_path = segments.next();
        let parent_id = match parent_path {
            None => self.root_parent_id.as_str(),
            Some(parent_path) => self
                .folder_ids_by_path
                .get(parent_path)
                .map(String::as_str)
                .ok_or_else(|| {
                    ExportStateError::new(
                        "resolve_create_target",
                        "provider folder mapping is unavailable",
                    )
                })?,
        };
        DriveCreateTarget::new(parent_id, name).map_err(|_| {
            ExportStateError::new(
                "resolve_create_target",
                "resolved provider target is invalid",
            )
        })
    }

    fn save_mapping(&mut self, mapping: GDriveMapping) -> Result<(), ExportStateError> {
        if let Some(error) = &self.save_mapping_error {
            return Err(error.clone());
        }
        self.mappings_by_path
            .insert(mapping.vault_path.as_str().to_owned(), mapping);
        self.mapping_save_count = self.mapping_save_count.saturating_add(1);
        Ok(())
    }

    fn save_cursor(&mut self, cursor: &CoreChangeCursor) -> Result<(), ExportStateError> {
        if let Some(error) = &self.save_cursor_error {
            return Err(error.clone());
        }
        self.cursor = cursor.clone();
        self.cursor_save_count = self.cursor_save_count.saturating_add(1);
        Ok(())
    }
}
