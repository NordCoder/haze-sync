//! Passive request validation and safe error mapping for operational-control routes.

use std::{error::Error, fmt};

use crate::{
    contracts::errors::{ErrorResponse, PublicErrorCode},
    dto::control_plane::{
        ControlPlaneValueError, CredentialCreateRequest, CredentialRotateRequest,
        OperationalJobConfirmationRequest, OperationalJobCreateRequest,
        MAX_CONTROL_IDENTIFIER_BYTES,
    },
};

pub const IDEMPOTENCY_KEY_HEADER: &str = "Idempotency-Key";
pub const MAX_IDEMPOTENCY_KEY_BYTES: usize = 512;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControlPlaneRouteError {
    InvalidRequest,
    InvalidIdempotencyKey,
    InvalidControlValue,
    MaintenanceInProgress,
    ControlTransitionInProgress,
    InvalidControlTransition,
    StaleControlGeneration,
    StaleRecordVersion,
    StaleExecutorFence,
    Conflict,
    IdempotencyConflict,
    IdempotencyInProgress,
    CredentialSecretNotReplayable,
    OperationInProgress,
    ConfirmationRequired,
    ConfirmationExpired,
    OperationNotCancellable,
    UnsupportedOperationKind,
    NotFound,
    Forbidden,
    Internal,
}

impl ControlPlaneRouteError {
    #[must_use]
    pub const fn status_code(self) -> u16 {
        match self {
            Self::InvalidRequest
            | Self::InvalidIdempotencyKey
            | Self::InvalidControlValue => 400,
            Self::Forbidden => 403,
            Self::NotFound => 404,
            Self::StaleControlGeneration
            | Self::StaleRecordVersion
            | Self::StaleExecutorFence
            | Self::Conflict
            | Self::IdempotencyConflict
            | Self::OperationInProgress
            | Self::ConfirmationRequired
            | Self::OperationNotCancellable
            | Self::UnsupportedOperationKind => 409,
            Self::CredentialSecretNotReplayable => 409,
            Self::ConfirmationExpired => 410,
            Self::MaintenanceInProgress
            | Self::ControlTransitionInProgress
            | Self::IdempotencyInProgress => 503,
            Self::InvalidControlTransition => 409,
            Self::Internal => 500,
        }
    }

    #[must_use]
    pub const fn code(self) -> PublicErrorCode {
        match self {
            Self::InvalidRequest | Self::InvalidIdempotencyKey => PublicErrorCode::InvalidRequest,
            Self::InvalidControlValue => PublicErrorCode::ValidationError,
            Self::MaintenanceInProgress => PublicErrorCode::MaintenanceInProgress,
            Self::ControlTransitionInProgress => PublicErrorCode::ControlTransitionInProgress,
            Self::InvalidControlTransition => PublicErrorCode::InvalidControlTransition,
            Self::StaleControlGeneration => PublicErrorCode::StaleControlGeneration,
            Self::StaleRecordVersion => PublicErrorCode::StaleRecordVersion,
            Self::StaleExecutorFence => PublicErrorCode::StaleExecutorFence,
            Self::Conflict => PublicErrorCode::Conflict,
            Self::IdempotencyConflict => PublicErrorCode::IdempotencyConflict,
            Self::IdempotencyInProgress => PublicErrorCode::IdempotencyInProgress,
            Self::CredentialSecretNotReplayable => PublicErrorCode::CredentialSecretNotReplayable,
            Self::OperationInProgress => PublicErrorCode::OperationInProgress,
            Self::ConfirmationRequired => PublicErrorCode::ConfirmationRequired,
            Self::ConfirmationExpired => PublicErrorCode::ConfirmationExpired,
            Self::OperationNotCancellable => PublicErrorCode::OperationNotCancellable,
            Self::UnsupportedOperationKind => PublicErrorCode::UnsupportedOperationKind,
            Self::NotFound => PublicErrorCode::NotFound,
            Self::Forbidden => PublicErrorCode::ForbiddenRole,
            Self::Internal => PublicErrorCode::InternalError,
        }
    }

    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::InvalidRequest => "Operational-control request is invalid",
            Self::InvalidIdempotencyKey => "Idempotency-Key is missing or invalid",
            Self::InvalidControlValue => "Operational-control request validation failed",
            Self::MaintenanceInProgress => "The requested operation is unavailable during maintenance",
            Self::ControlTransitionInProgress => "A maintenance transition is already in progress",
            Self::InvalidControlTransition => "The requested maintenance transition is invalid",
            Self::StaleControlGeneration => "The expected control generation is stale",
            Self::StaleRecordVersion => "The expected record version is stale",
            Self::StaleExecutorFence => "The executor fence is stale",
            Self::Conflict => "The requested operation conflicts with current state",
            Self::IdempotencyConflict => "The idempotency key was reused for different input",
            Self::IdempotencyInProgress => "An identical issuance request is still committing",
            Self::CredentialSecretNotReplayable => "Credential plaintext is not replayable",
            Self::OperationInProgress => "A conflicting operational job is already in progress",
            Self::ConfirmationRequired => "One-time confirmation is required",
            Self::ConfirmationExpired => "The one-time confirmation has expired",
            Self::OperationNotCancellable => "The operational job cannot be cancelled in its current state",
            Self::UnsupportedOperationKind => "The operational job kind is not supported",
            Self::NotFound => "The requested operational-control resource was not found",
            Self::Forbidden => "The authenticated role is not allowed for this operation",
            Self::Internal => "The operational-control operation failed",
        }
    }

    #[must_use]
    pub fn error_response(self) -> ErrorResponse {
        ErrorResponse::new(self.code(), self.message())
    }
}

impl fmt::Display for ControlPlaneRouteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message())
    }
}

impl Error for ControlPlaneRouteError {}

impl From<ControlPlaneValueError> for ControlPlaneRouteError {
    fn from(_value: ControlPlaneValueError) -> Self {
        Self::InvalidControlValue
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct ValidatedIdempotencyKey(String);

impl ValidatedIdempotencyKey {
    pub fn parse(value: Option<&str>) -> Result<Self, ControlPlaneRouteError> {
        let value = value.ok_or(ControlPlaneRouteError::InvalidIdempotencyKey)?;
        if value.is_empty()
            || value.len() > MAX_IDEMPOTENCY_KEY_BYTES
            || value.trim() != value
            || value.chars().any(char::is_control)
        {
            return Err(ControlPlaneRouteError::InvalidIdempotencyKey);
        }
        Ok(Self(value.to_owned()))
    }

    #[must_use]
    pub fn expose_for_private_digest(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for ValidatedIdempotencyKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ValidatedIdempotencyKey([REDACTED])")
    }
}

pub fn validate_control_identifier(
    field: &'static str,
    value: &str,
) -> Result<(), ControlPlaneRouteError> {
    if value.is_empty()
        || value.len() > MAX_CONTROL_IDENTIFIER_BYTES
        || value
            .chars()
            .any(|character| character.is_control() || character.is_whitespace())
    {
        return Err(match field {
            "idempotency_key" => ControlPlaneRouteError::InvalidIdempotencyKey,
            _ => ControlPlaneRouteError::InvalidControlValue,
        });
    }
    Ok(())
}

pub fn validate_credential_create_request(
    request: &CredentialCreateRequest,
) -> Result<(), ControlPlaneRouteError> {
    request.validate().map_err(Into::into)
}

pub fn validate_credential_rotate_request(
    request: &CredentialRotateRequest,
) -> Result<(), ControlPlaneRouteError> {
    request.validate().map_err(Into::into)
}

pub fn validate_operational_job_create_request(
    request: &OperationalJobCreateRequest,
) -> Result<(), ControlPlaneRouteError> {
    request.validate().map_err(Into::into)
}

pub fn validate_confirmation_request(
    request: &OperationalJobConfirmationRequest,
) -> Result<(), ControlPlaneRouteError> {
    request.validate().map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idempotency_key_is_redacted_and_bounded() {
        let raw = "fixture-idempotency-key";
        let key = ValidatedIdempotencyKey::parse(Some(raw)).unwrap();
        assert_eq!(key.expose_for_private_digest(), raw);
        assert!(!format!("{key:?}").contains(raw));
        assert_eq!(
            ValidatedIdempotencyKey::parse(None),
            Err(ControlPlaneRouteError::InvalidIdempotencyKey)
        );
    }

    #[test]
    fn every_operational_error_is_safe_and_stable() {
        for error in [
            ControlPlaneRouteError::MaintenanceInProgress,
            ControlPlaneRouteError::ControlTransitionInProgress,
            ControlPlaneRouteError::InvalidControlTransition,
            ControlPlaneRouteError::StaleControlGeneration,
            ControlPlaneRouteError::StaleRecordVersion,
            ControlPlaneRouteError::StaleExecutorFence,
            ControlPlaneRouteError::IdempotencyConflict,
            ControlPlaneRouteError::IdempotencyInProgress,
            ControlPlaneRouteError::CredentialSecretNotReplayable,
            ControlPlaneRouteError::OperationInProgress,
            ControlPlaneRouteError::ConfirmationRequired,
            ControlPlaneRouteError::ConfirmationExpired,
            ControlPlaneRouteError::OperationNotCancellable,
            ControlPlaneRouteError::UnsupportedOperationKind,
        ] {
            let json = serde_json::to_string(&error.error_response()).unwrap();
            assert!(!json.contains("postgres://"));
            assert!(!json.contains("verifier"));
            assert!(!json.contains("token_hash"));
            assert!(!json.contains("Idempotency-Key:"));
            assert!(!json.contains("sqlx"));
        }
    }
}
