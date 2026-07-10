use crate::state::{GDriveMapping, SafeTimestamp, VaultPath};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteStateError {
    MappingNotFound,
    MappingIdentityMismatch,
    DuplicateMappingPath,
    DuplicateMappingProviderId,
    InjectedFailure,
}

impl fmt::Display for DeleteStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MappingNotFound => "delete candidate mapping was not found",
            Self::MappingIdentityMismatch => "delete candidate mapping identity does not match",
            Self::DuplicateMappingPath => "delete candidate state has duplicate mapping path",
            Self::DuplicateMappingProviderId => {
                "delete candidate state has duplicate provider identity"
            }
            Self::InjectedFailure => "delete candidate state operation failed",
        })
    }
}

impl Error for DeleteStateError {}

pub trait DeleteCandidateStateStore {
    fn mark_candidate(
        &mut self,
        provider_id: &str,
        path: &VaultPath,
        detected_at: &SafeTimestamp,
    ) -> Result<(), DeleteStateError>;

    fn clear_candidate(&mut self, path: &VaultPath) -> Result<(), DeleteStateError>;

    fn retire_mapping(&mut self, path: &VaultPath) -> Result<(), DeleteStateError>;
}

#[derive(Clone, Default, PartialEq, Eq)]
pub struct InMemoryDeleteCandidateStateStore {
    mappings_by_path: BTreeMap<VaultPath, GDriveMapping>,
    fail_mark: bool,
    fail_clear: bool,
    fail_retire: bool,
    mark_count: usize,
    clear_count: usize,
    retire_count: usize,
}

impl InMemoryDeleteCandidateStateStore {
    pub fn new(mappings: Vec<GDriveMapping>) -> Result<Self, DeleteStateError> {
        let mut mappings_by_path = BTreeMap::new();
        let mut provider_ids = BTreeSet::new();
        for mapping in mappings {
            if !provider_ids.insert(mapping.drive_file_id.clone()) {
                return Err(DeleteStateError::DuplicateMappingProviderId);
            }
            if mappings_by_path
                .insert(mapping.vault_path.clone(), mapping)
                .is_some()
            {
                return Err(DeleteStateError::DuplicateMappingPath);
            }
        }
        Ok(Self {
            mappings_by_path,
            ..Self::default()
        })
    }

    #[must_use]
    pub fn with_mark_failure(mut self) -> Self {
        self.fail_mark = true;
        self
    }

    #[must_use]
    pub fn with_clear_failure(mut self) -> Self {
        self.fail_clear = true;
        self
    }

    #[must_use]
    pub fn with_retire_failure(mut self) -> Self {
        self.fail_retire = true;
        self
    }

    #[must_use]
    pub fn mapping(&self, path: &VaultPath) -> Option<&GDriveMapping> {
        self.mappings_by_path.get(path)
    }

    #[must_use]
    pub fn mappings(&self) -> Vec<GDriveMapping> {
        self.mappings_by_path.values().cloned().collect()
    }

    #[must_use]
    pub fn mapping_count(&self) -> usize {
        self.mappings_by_path.len()
    }

    #[must_use]
    pub const fn mark_count(&self) -> usize {
        self.mark_count
    }

    #[must_use]
    pub const fn clear_count(&self) -> usize {
        self.clear_count
    }

    #[must_use]
    pub const fn retire_count(&self) -> usize {
        self.retire_count
    }
}

impl fmt::Debug for InMemoryDeleteCandidateStateStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InMemoryDeleteCandidateStateStore")
            .field("mapping_count", &self.mappings_by_path.len())
            .field("mark_count", &self.mark_count)
            .field("clear_count", &self.clear_count)
            .field("retire_count", &self.retire_count)
            .finish()
    }
}

impl DeleteCandidateStateStore for InMemoryDeleteCandidateStateStore {
    fn mark_candidate(
        &mut self,
        provider_id: &str,
        path: &VaultPath,
        detected_at: &SafeTimestamp,
    ) -> Result<(), DeleteStateError> {
        if self.fail_mark {
            return Err(DeleteStateError::InjectedFailure);
        }
        let mapping = self
            .mappings_by_path
            .get_mut(path)
            .ok_or(DeleteStateError::MappingNotFound)?;
        if mapping.drive_file_id != provider_id {
            return Err(DeleteStateError::MappingIdentityMismatch);
        }
        if mapping.delete_candidate_since.is_none() {
            mapping.mark_delete_candidate(detected_at.clone());
        }
        self.mark_count = self.mark_count.saturating_add(1);
        Ok(())
    }

    fn clear_candidate(&mut self, path: &VaultPath) -> Result<(), DeleteStateError> {
        if self.fail_clear {
            return Err(DeleteStateError::InjectedFailure);
        }
        let mapping = self
            .mappings_by_path
            .get_mut(path)
            .ok_or(DeleteStateError::MappingNotFound)?;
        mapping.clear_delete_candidate();
        self.clear_count = self.clear_count.saturating_add(1);
        Ok(())
    }

    fn retire_mapping(&mut self, path: &VaultPath) -> Result<(), DeleteStateError> {
        if self.fail_retire {
            return Err(DeleteStateError::InjectedFailure);
        }
        self.mappings_by_path
            .remove(path)
            .ok_or(DeleteStateError::MappingNotFound)?;
        self.retire_count = self.retire_count.saturating_add(1);
        Ok(())
    }
}
