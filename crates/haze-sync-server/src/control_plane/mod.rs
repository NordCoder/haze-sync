//! Server-owned Stage 11 operational-control orchestration.
//!
//! API DTOs remain passive, Storage remains caller-transactional, and this
//! module owns transition policy, shared admission, authentication, job
//! execution fencing, and secret-safe audit production.

mod admission;
mod crypto;
mod fakes;
mod service;

pub(crate) use admission::{
    AdmissionClass, AdmissionController, AuthoritativeMutationPermit, DurableMaintenanceState,
};
pub(crate) use service::{AuthenticatedIdentity, ControlPlaneServices};

use haze_sync_storage::control_plane::ControlPlaneRepositoryError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ControlPlaneError {
    InvalidInput,
    InvalidStoredState,
    InvalidAuthentication,
    Forbidden,
    NotFound,
    Conflict,
    MaintenanceInProgress,
    ControlTransitionInProgress,
    InvalidControlTransition,
    StaleControlGeneration,
    StaleRecordVersion,
    StaleExecutorFence,
    IdempotencyConflict,
    IdempotencyInProgress,
    CredentialSecretNotReplayable,
    OperationInProgress,
    ConfirmationRequired,
    ConfirmationExpired,
    OperationNotCancellable,
    UnsupportedOperationKind,
    AdapterControlUnavailable,
    QuiescenceTimeout,
    Internal,
}

impl ControlPlaneError {
    pub(crate) fn from_repository(error: ControlPlaneRepositoryError) -> Self {
        match error {
            ControlPlaneRepositoryError::MissingRecord => Self::NotFound,
            ControlPlaneRepositoryError::IdempotencyConflict => Self::IdempotencyConflict,
            ControlPlaneRepositoryError::OperationInProgress
            | ControlPlaneRepositoryError::RuntimeLeaseConflict => Self::OperationInProgress,
            ControlPlaneRepositoryError::StaleVersion { namespace } => match namespace {
                "maintenance_generation"
                | "adapter_inventory_generation"
                | "adapter_control_generation" => Self::StaleControlGeneration,
                "executor_fence" | "executor_lease" => Self::StaleExecutorFence,
                _ => Self::StaleRecordVersion,
            },
            ControlPlaneRepositoryError::ImmutableRecord => Self::OperationNotCancellable,
            ControlPlaneRepositoryError::InvalidIdentifier
            | ControlPlaneRepositoryError::InvalidDigest
            | ControlPlaneRepositoryError::InvalidSecretMaterial
            | ControlPlaneRepositoryError::InvalidGeneration
            | ControlPlaneRepositoryError::LegacyVerifierCreationForbidden => Self::InvalidInput,
            ControlPlaneRepositoryError::VersionOverflow
            | ControlPlaneRepositoryError::DatabaseOperationFailed
            | ControlPlaneRepositoryError::StaleRuntimeEpoch
            | ControlPlaneRepositoryError::StaleRuntimeReport
            | ControlPlaneRepositoryError::RuntimeReportConflict => Self::Internal,
            _ => Self::Internal,
        }
    }

    fn from_admission(error: admission::AdmissionDenied) -> Self {
        match error {
            admission::AdmissionDenied::Maintenance(_) => Self::MaintenanceInProgress,
            admission::AdmissionDenied::DrainTimeout => Self::QuiescenceTimeout,
        }
    }

    pub(crate) const fn safe_code(self) -> &'static str {
        match self {
            Self::InvalidInput => "invalid_request",
            Self::InvalidStoredState | Self::Internal => "internal_error",
            Self::InvalidAuthentication => "invalid_token",
            Self::Forbidden => "forbidden_role",
            Self::NotFound => "not_found",
            Self::Conflict => "conflict",
            Self::MaintenanceInProgress => "maintenance_in_progress",
            Self::ControlTransitionInProgress => "control_transition_in_progress",
            Self::InvalidControlTransition => "invalid_control_transition",
            Self::StaleControlGeneration => "stale_control_generation",
            Self::StaleRecordVersion => "stale_record_version",
            Self::StaleExecutorFence => "stale_executor_fence",
            Self::IdempotencyConflict => "idempotency_conflict",
            Self::IdempotencyInProgress => "idempotency_in_progress",
            Self::CredentialSecretNotReplayable => "credential_secret_not_replayable",
            Self::OperationInProgress => "operation_in_progress",
            Self::ConfirmationRequired => "confirmation_required",
            Self::ConfirmationExpired => "confirmation_expired",
            Self::OperationNotCancellable => "operation_not_cancellable",
            Self::UnsupportedOperationKind => "unsupported_operation_kind",
            Self::AdapterControlUnavailable => "adapter_control_unavailable",
            Self::QuiescenceTimeout => "quiescence_timeout",
        }
    }
}

impl From<crypto::CryptoError> for ControlPlaneError {
    fn from(_error: crypto::CryptoError) -> Self {
        Self::Internal
    }
}

impl std::fmt::Display for ControlPlaneError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.safe_code())
    }
}

impl std::error::Error for ControlPlaneError {}
