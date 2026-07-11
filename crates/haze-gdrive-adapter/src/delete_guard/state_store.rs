use crate::state::{GDriveMapping, SafeTimestamp, VaultPath};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteStateError {
    MappingNotFound,
    MappingIdentityMismatch,
    CandidateTimestampMismatch,
    CoreRevisionMismatch,
    DuplicateMappingPath,
    DuplicateMappingProviderId,
    InjectedFailure,
}

impl DeleteStateError {
    #[must_use]
    pub const fn is_state_drift(self) -> bool {
        matches!(
            self,
            Self::MappingNotFound
                | Self::MappingIdentityMismatch
                | Self::CandidateTimestampMismatch
                | Self::CoreRevisionMismatch
        )
    }
}

impl fmt::Display for DeleteStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MappingNotFound => "delete candidate mapping was not found",
            Self::MappingIdentityMismatch => "delete candidate mapping identity does not match",
            Self::CandidateTimestampMismatch => {
                "delete candidate timestamp changed since scan"
            }
            Self::CoreRevisionMismatch => "delete candidate Core revision changed since scan",
            Self::DuplicateMappingPath => "delete candidate state has duplicate mapping path",
            Self::DuplicateMappingProviderId => {
                "delete candidate state has duplicate provider identity"
            }
            Self::InjectedFailure => "delete candidate state operation failed",
        })
    }
}

impl Error for DeleteStateError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteMappingState {
    pub provider_id: String,
    pub path: VaultPath,
    pub base_revision_id: Option<String>,
    pub delete_candidate_since: Option<SafeTimestamp>,
}

pub trait DeleteCandidateStateStore {
    fn current_mapping_count(&self) -> Result<u64, DeleteStateError>;

    fn mapping_state(
        &self,
        provider_id: &str,
        path: &VaultPath,
    ) -> Result<DeleteMappingState, DeleteStateError>;

    fn mark_candidate(
        &mut self,
        provider_id: &str,
        path: &VaultPath,
        detected_at: &SafeTimestamp,
    ) -> Result<(), DeleteStateError>;

    fn clear_candidate(
        &mut self,
        provider_id: &str,
        path: &VaultPath,
        expected_first_detected_at: &SafeTimestamp,
    ) -> Result<(), DeleteStateError>;

    fn retire_mapping(
        &mut self,
        provider_id: &str,
        path: &VaultPath,
        expected_first_detected_at: &SafeTimestamp,
        expected_base_revision_id: Option<&str>,
    ) -> Result<(), DeleteStateError>;
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

    fn mapping_for_identity(
        &self,
        provider_id: &str,
        path: &VaultPath,
    ) -> Result<&GDriveMapping, DeleteStateError> {
        let mapping = self
            .mappings_by_path
            .get(path)
            .ok_or(DeleteStateError::MappingNotFound)?;
        if mapping.drive_file_id != provider_id {
            return Err(DeleteStateError::MappingIdentityMismatch);
        }
        Ok(mapping)
    }

    fn mapping_for_identity_mut(
        &mut self,
        provider_id: &str,
        path: &VaultPath,
    ) -> Result<&mut GDriveMapping, DeleteStateError> {
        let mapping = self
            .mappings_by_path
            .get_mut(path)
            .ok_or(DeleteStateError::MappingNotFound)?;
        if mapping.drive_file_id != provider_id {
            return Err(DeleteStateError::MappingIdentityMismatch);
        }
        Ok(mapping)
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
    fn current_mapping_count(&self) -> Result<u64, DeleteStateError> {
        Ok(self.mappings_by_path.len() as u64)
    }

    fn mapping_state(
        &self,
        provider_id: &str,
        path: &VaultPath,
    ) -> Result<DeleteMappingState, DeleteStateError> {
        let mapping = self.mapping_for_identity(provider_id, path)?;
        Ok(DeleteMappingState {
            provider_id: mapping.drive_file_id.clone(),
            path: mapping.vault_path.clone(),
            base_revision_id: mapping.core_revision.clone(),
            delete_candidate_since: mapping.delete_candidate_since.clone(),
        })
    }

    fn mark_candidate(
        &mut self,
        provider_id: &str,
        path: &VaultPath,
        detected_at: &SafeTimestamp,
    ) -> Result<(), DeleteStateError> {
        if self.fail_mark {
            return Err(DeleteStateError::InjectedFailure);
        }
        let mapping = self.mapping_for_identity_mut(provider_id, path)?;
        match mapping.delete_candidate_since.as_ref() {
            None => {
                mapping.mark_delete_candidate(detected_at.clone());
                self.mark_count = self.mark_count.saturating_add(1);
                Ok(())
            }
            Some(current) if current == detected_at => Ok(()),
            Some(_) => Err(DeleteStateError::CandidateTimestampMismatch),
        }
    }

    fn clear_candidate(
        &mut self,
        provider_id: &str,
        path: &VaultPath,
        expected_first_detected_at: &SafeTimestamp,
    ) -> Result<(), DeleteStateError> {
        if self.fail_clear {
            return Err(DeleteStateError::InjectedFailure);
        }
        let mapping = self.mapping_for_identity_mut(provider_id, path)?;
        match mapping.delete_candidate_since.as_ref() {
            Some(current) if current == expected_first_detected_at => {
                mapping.clear_delete_candidate();
                self.clear_count = self.clear_count.saturating_add(1);
                Ok(())
            }
            None => Ok(()),
            Some(_) => Err(DeleteStateError::CandidateTimestampMismatch),
        }
    }

    fn retire_mapping(
        &mut self,
        provider_id: &str,
        path: &VaultPath,
        expected_first_detected_at: &SafeTimestamp,
        expected_base_revision_id: Option<&str>,
    ) -> Result<(), DeleteStateError> {
        if self.fail_retire {
            return Err(DeleteStateError::InjectedFailure);
        }
        let mapping = self.mapping_for_identity(provider_id, path)?;
        if mapping.delete_candidate_since.as_ref() != Some(expected_first_detected_at) {
            return Err(DeleteStateError::CandidateTimestampMismatch);
        }
        if mapping.core_revision.as_deref() != expected_base_revision_id {
            return Err(DeleteStateError::CoreRevisionMismatch);
        }
        self.mappings_by_path
            .remove(path)
            .ok_or(DeleteStateError::MappingNotFound)?;
        self.retire_count = self.retire_count.saturating_add(1);
        Ok(())
    }
}
