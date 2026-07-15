use super::model::{CoreDeleteGuardDecision, CoreDeleteRejectedReason};
use crate::state::VaultPath;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreDeleteGuardRequest {
    pub adapter_id: String,
    pub run_id: String,
    pub proposed_delete_count: u64,
    pub total_files_before_run: u64,
}

#[derive(Clone, PartialEq, Eq)]
pub struct CoreDeleteRequest {
    pub operation_id: String,
    pub path: VaultPath,
    pub base_revision_id: Option<String>,
}

impl fmt::Debug for CoreDeleteRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CoreDeleteRequest")
            .field("operation_id", &"<redacted-idempotency-key>")
            .field("path", &self.path)
            .field("base_revision_id", &self.base_revision_id)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreDeleteResponse {
    Tombstoned,
    NotFound,
    Rejected { reason: CoreDeleteRejectedReason },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreDeleteErrorCategory {
    Auth,
    RateLimit,
    Unavailable,
    Conflict,
    InvalidRequest,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreDeleteError {
    operation: &'static str,
    category: CoreDeleteErrorCategory,
    safe_detail: &'static str,
}

impl CoreDeleteError {
    pub const fn new(
        operation: &'static str,
        category: CoreDeleteErrorCategory,
        safe_detail: &'static str,
    ) -> Self {
        Self {
            operation,
            category,
            safe_detail,
        }
    }

    pub const fn operation(&self) -> &'static str {
        self.operation
    }

    pub const fn category(&self) -> CoreDeleteErrorCategory {
        self.category
    }
}

impl fmt::Display for CoreDeleteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Core delete operation {} failed as {:?}: {}",
            self.operation, self.category, self.safe_detail
        )
    }
}

impl Error for CoreDeleteError {}

pub trait CoreDeleteGateway {
    fn evaluate_delete_guard(
        &mut self,
        request: &CoreDeleteGuardRequest,
    ) -> Result<CoreDeleteGuardDecision, CoreDeleteError>;

    fn delete_file(
        &mut self,
        request: CoreDeleteRequest,
    ) -> Result<CoreDeleteResponse, CoreDeleteError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DeleteFingerprint {
    path: VaultPath,
    base_revision_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AppliedDelete {
    fingerprint: DeleteFingerprint,
    response: CoreDeleteResponse,
}

#[derive(Clone)]
pub struct FakeCoreDeleteGateway {
    guard_decision: CoreDeleteGuardDecision,
    guard_error: Option<CoreDeleteError>,
    responses_by_path: BTreeMap<VaultPath, CoreDeleteResponse>,
    errors_by_path: BTreeMap<VaultPath, CoreDeleteError>,
    applied_by_operation_id: BTreeMap<String, AppliedDelete>,
    guard_evaluation_count: usize,
    delete_call_count: usize,
}

impl FakeCoreDeleteGateway {
    #[must_use]
    pub fn new() -> Self {
        Self {
            guard_decision: CoreDeleteGuardDecision::Allowed,
            guard_error: None,
            responses_by_path: BTreeMap::new(),
            errors_by_path: BTreeMap::new(),
            applied_by_operation_id: BTreeMap::new(),
            guard_evaluation_count: 0,
            delete_call_count: 0,
        }
    }

    #[must_use]
    pub fn with_guard_decision(mut self, decision: CoreDeleteGuardDecision) -> Self {
        self.guard_decision = decision;
        self
    }

    #[must_use]
    pub fn with_guard_error(mut self, error: CoreDeleteError) -> Self {
        self.guard_error = Some(error);
        self
    }

    #[must_use]
    pub fn with_response(mut self, path: VaultPath, response: CoreDeleteResponse) -> Self {
        self.responses_by_path.insert(path, response);
        self
    }

    #[must_use]
    pub fn with_error(mut self, path: VaultPath, error: CoreDeleteError) -> Self {
        self.errors_by_path.insert(path, error);
        self
    }

    #[must_use]
    pub const fn guard_evaluation_count(&self) -> usize {
        self.guard_evaluation_count
    }

    #[must_use]
    pub const fn delete_call_count(&self) -> usize {
        self.delete_call_count
    }
}

impl Default for FakeCoreDeleteGateway {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for FakeCoreDeleteGateway {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FakeCoreDeleteGateway")
            .field("guard_decision", &self.guard_decision)
            .field("configured_response_count", &self.responses_by_path.len())
            .field("configured_error_count", &self.errors_by_path.len())
            .field("applied_delete_count", &self.applied_by_operation_id.len())
            .field("guard_evaluation_count", &self.guard_evaluation_count)
            .field("delete_call_count", &self.delete_call_count)
            .finish()
    }
}

impl CoreDeleteGateway for FakeCoreDeleteGateway {
    fn evaluate_delete_guard(
        &mut self,
        _request: &CoreDeleteGuardRequest,
    ) -> Result<CoreDeleteGuardDecision, CoreDeleteError> {
        self.guard_evaluation_count = self.guard_evaluation_count.saturating_add(1);
        if let Some(error) = &self.guard_error {
            return Err(error.clone());
        }
        Ok(self.guard_decision.clone())
    }

    fn delete_file(
        &mut self,
        request: CoreDeleteRequest,
    ) -> Result<CoreDeleteResponse, CoreDeleteError> {
        self.delete_call_count = self.delete_call_count.saturating_add(1);
        if let Some(error) = self.errors_by_path.get(&request.path) {
            return Err(error.clone());
        }

        let fingerprint = DeleteFingerprint {
            path: request.path.clone(),
            base_revision_id: request.base_revision_id.clone(),
        };
        if let Some(applied) = self.applied_by_operation_id.get(&request.operation_id) {
            if applied.fingerprint == fingerprint {
                return Ok(applied.response.clone());
            }
            return Err(CoreDeleteError::new(
                "delete_file",
                CoreDeleteErrorCategory::Conflict,
                "delete idempotency fingerprint conflict",
            ));
        }

        let response = self
            .responses_by_path
            .get(&request.path)
            .cloned()
            .unwrap_or(CoreDeleteResponse::Tombstoned);
        self.applied_by_operation_id.insert(
            request.operation_id,
            AppliedDelete {
                fingerprint,
                response: response.clone(),
            },
        );
        Ok(response)
    }
}
