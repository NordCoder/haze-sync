//! Passive V1 operational-control DTOs.
//!
//! These values describe public JSON only. They do not authenticate callers,
//! access Storage, execute jobs, or apply runtime control.

use std::{collections::BTreeMap, error::Error, fmt};

use serde::{Deserialize, Serialize};

use crate::auth::AdapterRole;

use super::primitives::TimestampDto;

pub const OPERATIONAL_JOB_SCHEMA_V1: &str = "haze-sync.operational-job.v1";
pub const OPERATIONAL_AUDIT_SCHEMA_V1: &str = "haze-sync.audit-event.v1";
pub const MAX_CONTROL_IDENTIFIER_BYTES: usize = 256;
pub const MAX_SAFE_SUMMARY_BYTES: usize = 16_384;
pub const MAX_GRACE_SECONDS: u64 = 86_400;
pub const DEFAULT_GRACE_SECONDS: u64 = 900;

fn validate_identifier(field: &'static str, value: &str) -> Result<(), ControlPlaneValueError> {
    if value.is_empty() {
        return Err(ControlPlaneValueError::Empty { field });
    }
    if value.len() > MAX_CONTROL_IDENTIFIER_BYTES {
        return Err(ControlPlaneValueError::TooLong {
            field,
            max_bytes: MAX_CONTROL_IDENTIFIER_BYTES,
        });
    }
    if value
        .chars()
        .any(|character| character.is_control() || character.is_whitespace())
    {
        return Err(ControlPlaneValueError::InvalidCharacter { field });
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControlPlaneValueError {
    Empty {
        field: &'static str,
    },
    TooLong {
        field: &'static str,
        max_bytes: usize,
    },
    InvalidCharacter {
        field: &'static str,
    },
    InvalidRange {
        field: &'static str,
    },
    DuplicateIdentifier {
        field: &'static str,
    },
    InvalidCombination {
        field: &'static str,
    },
}

impl fmt::Display for ControlPlaneValueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty { field } => write!(formatter, "{field} must not be empty"),
            Self::TooLong { field, max_bytes } => {
                write!(formatter, "{field} must not exceed {max_bytes} bytes")
            }
            Self::InvalidCharacter { field } => {
                write!(formatter, "{field} contains an invalid character")
            }
            Self::InvalidRange { field } => write!(formatter, "{field} is outside the valid range"),
            Self::DuplicateIdentifier { field } => {
                write!(formatter, "{field} contains a duplicate identifier")
            }
            Self::InvalidCombination { field } => {
                write!(formatter, "{field} contains an invalid combination")
            }
        }
    }
}

impl Error for ControlPlaneValueError {}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MaintenanceStateDto {
    Normal,
    Quiescing,
    Quiesced,
    Maintenance,
    Resuming,
}

impl MaintenanceStateDto {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Quiescing => "quiescing",
            Self::Quiesced => "quiesced",
            Self::Maintenance => "maintenance",
            Self::Resuming => "resuming",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MaintenanceActionDto {
    Quiesce,
    EnterMaintenance,
    Resume,
}

impl MaintenanceActionDto {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Quiesce => "quiesce",
            Self::EnterMaintenance => "enter_maintenance",
            Self::Resume => "resume",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MaintenanceStatusResponse {
    pub schema_version: u32,
    pub maintenance_generation: u64,
    pub state: MaintenanceStateDto,
    pub transition_operation_id: Option<String>,
    pub state_entered_at: TimestampDto,
    pub admission_fence_closed: bool,
    pub quiescence_evidence_version: u64,
    pub quiescence_evidence_id: Option<String>,
    pub active_maintenance_job_id: Option<String>,
    pub safe_error_category: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MaintenanceActionRequest {
    pub expected_maintenance_generation: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MaintenanceActionResponse {
    pub operation_id: String,
    pub action: MaintenanceActionDto,
    pub accepted: bool,
    pub maintenance: MaintenanceStatusResponse,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PrincipalKindDto {
    Adapter,
    Administrator,
}

impl PrincipalKindDto {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Adapter => "adapter",
            Self::Administrator => "administrator",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PrincipalStatusResponse {
    pub principal_id: String,
    pub principal_kind: PrincipalKindDto,
    pub role: AdapterRole,
    pub principal_enabled: bool,
    pub principal_version: u64,
    pub credential_set_generation: u64,
    pub created_at: TimestampDto,
    pub updated_at: TimestampDto,
    pub disabled_at: Option<TimestampDto>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PrincipalMutationRequest {
    pub expected_principal_version: u64,
    pub expected_credential_set_generation: u64,
    pub role: AdapterRole,
    pub principal_enabled: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialStatusDto {
    Active,
    Grace,
    Revoked,
    Expired,
}

impl CredentialStatusDto {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Grace => "grace",
            Self::Revoked => "revoked",
            Self::Expired => "expired",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialVerifierSchemeDto {
    Argon2idV1,
    LegacySha256V0,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CredentialSummaryResponse {
    pub credential_id: String,
    pub principal_id: String,
    pub effective_role: AdapterRole,
    pub credential_kind: String,
    pub credential_version: u64,
    pub status: CredentialStatusDto,
    pub verifier_scheme: CredentialVerifierSchemeDto,
    pub created_at: TimestampDto,
    pub not_before: TimestampDto,
    pub expires_at: Option<TimestampDto>,
    pub grace_expires_at: Option<TimestampDto>,
    pub rotated_from_credential_id: Option<String>,
    pub revoked_at: Option<TimestampDto>,
    pub last_used_at: Option<TimestampDto>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CredentialListResponse {
    pub principal: PrincipalStatusResponse,
    pub credentials: Vec<CredentialSummaryResponse>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CredentialCreateRequest {
    pub expected_principal_version: u64,
    pub expected_credential_set_generation: u64,
    pub credential_kind: String,
    pub requested_expires_at: Option<TimestampDto>,
}

impl CredentialCreateRequest {
    pub fn validate(&self) -> Result<(), ControlPlaneValueError> {
        validate_identifier("credential_kind", &self.credential_kind)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AffectedCredentialRequest {
    pub credential_id: String,
    pub expected_credential_version: u64,
    pub current_status: CredentialStatusDto,
}

impl AffectedCredentialRequest {
    pub fn validate(&self) -> Result<(), ControlPlaneValueError> {
        validate_identifier("credential_id", &self.credential_id)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CredentialRotateRequest {
    pub expected_principal_version: u64,
    pub expected_credential_set_generation: u64,
    pub credential_kind: String,
    pub requested_expires_at: Option<TimestampDto>,
    pub requested_grace_seconds: Option<u64>,
    pub affected_credentials: Vec<AffectedCredentialRequest>,
}

impl CredentialRotateRequest {
    pub fn validate(&self) -> Result<(), ControlPlaneValueError> {
        validate_identifier("credential_kind", &self.credential_kind)?;
        if self
            .requested_grace_seconds
            .is_some_and(|seconds| seconds > MAX_GRACE_SECONDS)
        {
            return Err(ControlPlaneValueError::InvalidRange {
                field: "requested_grace_seconds",
            });
        }
        let mut ids = self
            .affected_credentials
            .iter()
            .map(|credential| credential.credential_id.as_str())
            .collect::<Vec<_>>();
        for credential in &self.affected_credentials {
            credential.validate()?;
        }
        ids.sort_unstable();
        if ids.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(ControlPlaneValueError::DuplicateIdentifier {
                field: "affected_credentials",
            });
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CredentialRevokeRequest {
    pub expected_principal_version: u64,
    pub expected_credential_set_generation: u64,
    pub expected_credential_version: u64,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CredentialIssuanceResponse {
    pub issuance_operation_id: String,
    pub credential: CredentialSummaryResponse,
    pub plaintext_available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plaintext_credential: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safe_code: Option<String>,
}

impl fmt::Debug for CredentialIssuanceResponse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CredentialIssuanceResponse")
            .field("issuance_operation_id", &self.issuance_operation_id)
            .field("credential", &self.credential)
            .field("plaintext_available", &self.plaintext_available)
            .field(
                "plaintext_credential",
                &self.plaintext_credential.as_ref().map(|_| "[REDACTED]"),
            )
            .field("safe_code", &self.safe_code)
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationalJobKindDto {
    AdapterDryRun,
    AdapterReconcile,
    Backup,
    Restore,
    Repair,
    RetentionCleanup,
    DeploymentRollout,
    DeploymentRollback,
}

impl OperationalJobKindDto {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AdapterDryRun => "adapter_dry_run",
            Self::AdapterReconcile => "adapter_reconcile",
            Self::Backup => "backup",
            Self::Restore => "restore",
            Self::Repair => "repair",
            Self::RetentionCleanup => "retention_cleanup",
            Self::DeploymentRollout => "deployment_rollout",
            Self::DeploymentRollback => "deployment_rollback",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationalJobStateDto {
    Planned,
    AwaitingConfirmation,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

impl OperationalJobStateDto {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::AwaitingConfirmation => "awaiting_confirmation",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedAdapterGenerationDto {
    pub adapter_id: String,
    pub adapter_control_generation: u64,
}

impl ExpectedAdapterGenerationDto {
    pub fn validate(&self) -> Result<(), ControlPlaneValueError> {
        validate_identifier("adapter_id", &self.adapter_id)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OperationalJobCreateRequest {
    pub kind: OperationalJobKindDto,
    pub dry_run: bool,
    pub target_scope: String,
    pub expected_maintenance_generation: Option<u64>,
    pub expected_adapter_control_generations: Vec<ExpectedAdapterGenerationDto>,
    pub retry_of_operation_id: Option<String>,
}

impl OperationalJobCreateRequest {
    pub fn validate(&self) -> Result<(), ControlPlaneValueError> {
        validate_identifier("target_scope", &self.target_scope)?;
        if let Some(operation_id) = &self.retry_of_operation_id {
            validate_identifier("retry_of_operation_id", operation_id)?;
        }
        let mut ids = self
            .expected_adapter_control_generations
            .iter()
            .map(|entry| entry.adapter_id.as_str())
            .collect::<Vec<_>>();
        for entry in &self.expected_adapter_control_generations {
            entry.validate()?;
        }
        ids.sort_unstable();
        if ids.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(ControlPlaneValueError::DuplicateIdentifier {
                field: "expected_adapter_control_generations",
            });
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OperationalJobResponse {
    pub schema: String,
    pub operation_id: String,
    pub job_version: u64,
    pub kind: OperationalJobKindDto,
    pub maintenance_required: bool,
    pub destructive: bool,
    pub state: OperationalJobStateDto,
    pub dry_run: bool,
    pub confirmation_required: bool,
    pub confirmation_expires_at: Option<TimestampDto>,
    pub expected_maintenance_generation: Option<u64>,
    pub expected_adapter_control_generations: Vec<ExpectedAdapterGenerationDto>,
    pub created_at: TimestampDto,
    pub updated_at: TimestampDto,
    pub started_at: Option<TimestampDto>,
    pub completed_at: Option<TimestampDto>,
    pub cancel_requested_at: Option<TimestampDto>,
    pub safe_summary: BTreeMap<String, String>,
    pub safe_error_category: Option<String>,
    pub artifact_manifest_id: Option<String>,
    pub checkpoint: Option<BTreeMap<String, String>>,
    pub executor_id: Option<String>,
    pub executor_fence: u64,
    pub lease_heartbeat_at: Option<TimestampDto>,
    pub lease_expires_at: Option<TimestampDto>,
    pub blocked_uncertain: bool,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OperationalJobConfirmationRequest {
    pub expected_job_version: u64,
    pub confirmation: String,
}

impl OperationalJobConfirmationRequest {
    pub fn validate(&self) -> Result<(), ControlPlaneValueError> {
        validate_identifier("confirmation", &self.confirmation)
    }
}

impl fmt::Debug for OperationalJobConfirmationRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OperationalJobConfirmationRequest")
            .field("expected_job_version", &self.expected_job_version)
            .field("confirmation", &"[REDACTED]")
            .finish()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OperationalJobCancelRequest {
    pub expected_job_version: u64,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OperationalJobSubmissionResponse {
    pub job: OperationalJobResponse,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmation: Option<String>,
}

impl fmt::Debug for OperationalJobSubmissionResponse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OperationalJobSubmissionResponse")
            .field("job", &self.job)
            .field("confirmation", &self.confirmation.as_ref().map(|_| "[REDACTED]"))
            .finish()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AuditEventSummaryResponse {
    pub schema: String,
    pub audit_id: String,
    pub occurred_at: TimestampDto,
    pub event_type: String,
    pub outcome: String,
    pub actor_principal_id: Option<String>,
    pub actor_role: Option<AdapterRole>,
    pub target_type: String,
    pub target_id: Option<String>,
    pub operation_id: Option<String>,
    pub maintenance_generation: Option<u64>,
    pub principal_version: Option<u64>,
    pub credential_set_generation: Option<u64>,
    pub credential_version: Option<u64>,
    pub job_version: Option<u64>,
    pub executor_fence: Option<u64>,
    pub previous_state: Option<String>,
    pub next_state: Option<String>,
    pub safe_error_category: Option<String>,
    pub safe_metadata: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AuditEventListResponse {
    pub events: Vec<AuditEventSummaryResponse>,
    pub next_cursor: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn credential_summary() -> CredentialSummaryResponse {
        CredentialSummaryResponse {
            credential_id: "cred_01JTEST".to_owned(),
            principal_id: "admin-cli".to_owned(),
            effective_role: AdapterRole::Admin,
            credential_kind: "bearer".to_owned(),
            credential_version: 1,
            status: CredentialStatusDto::Active,
            verifier_scheme: CredentialVerifierSchemeDto::Argon2idV1,
            created_at: TimestampDto::from("2026-07-24T00:00:00Z"),
            not_before: TimestampDto::from("2026-07-24T00:00:00Z"),
            expires_at: None,
            grace_expires_at: None,
            rotated_from_credential_id: None,
            revoked_at: None,
            last_used_at: None,
        }
    }

    #[test]
    fn credential_json_never_contains_verifier_material() {
        let value = serde_json::to_value(credential_summary()).unwrap();
        let rendered = value.to_string();
        assert!(!rendered.contains("verifier_material"));
        assert!(!rendered.contains("salt"));
        assert!(!rendered.contains("token_hash"));
    }

    #[test]
    fn issuance_debug_redacts_one_time_plaintext() {
        let raw = "hs1.cred_01JTEST.fixture-secret";
        let response = CredentialIssuanceResponse {
            issuance_operation_id: "issue_01JTEST".to_owned(),
            credential: credential_summary(),
            plaintext_available: true,
            plaintext_credential: Some(raw.to_owned()),
            safe_code: None,
        };
        let rendered = format!("{response:?}");
        assert!(rendered.contains("[REDACTED]"));
        assert!(!rendered.contains(raw));
        assert!(serde_json::to_string(&response).unwrap().contains(raw));
    }

    #[test]
    fn confirmation_debug_redacts_value() {
        let raw = "confirm_01JTEST";
        let request = OperationalJobConfirmationRequest {
            expected_job_version: 2,
            confirmation: raw.to_owned(),
        };
        assert!(!format!("{request:?}").contains(raw));
    }

    #[test]
    fn rotate_validation_rejects_duplicate_credential_ids_and_long_grace() {
        let affected = AffectedCredentialRequest {
            credential_id: "cred_01JTEST".to_owned(),
            expected_credential_version: 1,
            current_status: CredentialStatusDto::Active,
        };
        let request = CredentialRotateRequest {
            expected_principal_version: 1,
            expected_credential_set_generation: 1,
            credential_kind: "bearer".to_owned(),
            requested_expires_at: None,
            requested_grace_seconds: Some(MAX_GRACE_SECONDS + 1),
            affected_credentials: vec![affected.clone(), affected],
        };
        assert!(matches!(
            request.validate(),
            Err(ControlPlaneValueError::InvalidRange { .. })
        ));
    }

    #[test]
    fn operational_job_validation_rejects_duplicate_adapter_generations() {
        let generation = ExpectedAdapterGenerationDto {
            adapter_id: "gdrive-a".to_owned(),
            adapter_control_generation: 4,
        };
        let request = OperationalJobCreateRequest {
            kind: OperationalJobKindDto::AdapterDryRun,
            dry_run: true,
            target_scope: "gdrive-a".to_owned(),
            expected_maintenance_generation: None,
            expected_adapter_control_generations: vec![generation.clone(), generation],
            retry_of_operation_id: None,
        };
        assert!(matches!(
            request.validate(),
            Err(ControlPlaneValueError::DuplicateIdentifier { .. })
        ));
    }

    #[test]
    fn deny_unknown_fields_prevents_secret_smuggling() {
        let json = r#"{
            "expected_principal_version":1,
            "expected_credential_set_generation":1,
            "credential_kind":"bearer",
            "requested_expires_at":null,
            "verifier_material":"forbidden"
        }"#;
        assert!(serde_json::from_str::<CredentialCreateRequest>(json).is_err());
    }
}
