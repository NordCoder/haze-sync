/// Result returned by operational-control repositories.
pub type ControlPlaneResult<T> = Result<T, ControlPlaneRepositoryError>;

/// Secret-safe storage error boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ControlPlaneRepositoryError {
    InvalidIdentifier,
    InvalidDigest,
    InvalidSecretMaterial,
    InvalidGeneration,
    VersionOverflow,
    StaleVersion { namespace: &'static str },
    MissingRecord,
    IdempotencyConflict,
    OperationInProgress,
    RuntimeLeaseConflict,
    StaleRuntimeEpoch,
    StaleRuntimeReport,
    RuntimeReportConflict,
    LegacyVerifierCreationForbidden,
    ImmutableRecord,
    DatabaseOperationFailed,
}

impl ControlPlaneRepositoryError {
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::InvalidIdentifier => "invalid_storage_identifier",
            Self::InvalidDigest => "invalid_storage_digest",
            Self::InvalidSecretMaterial => "invalid_secret_material",
            Self::InvalidGeneration => "invalid_generation",
            Self::VersionOverflow => "storage_version_overflow",
            Self::StaleVersion { namespace } => match namespace {
                cas_namespaces::MAINTENANCE_GENERATION
                | cas_namespaces::ADAPTER_INVENTORY_GENERATION
                | cas_namespaces::ADAPTER_CONTROL_GENERATION => "stale_control_generation",
                cas_namespaces::EXECUTOR_FENCE => "stale_executor_fence",
                _ => "stale_record_version",
            },
            Self::MissingRecord => "storage_record_missing",
            Self::IdempotencyConflict => "idempotency_conflict",
            Self::OperationInProgress => "operation_in_progress",
            Self::RuntimeLeaseConflict => "runtime_lease_conflict",
            Self::StaleRuntimeEpoch => "stale_runtime_epoch",
            Self::StaleRuntimeReport => "stale_runtime_report",
            Self::RuntimeReportConflict => "runtime_report_conflict",
            Self::LegacyVerifierCreationForbidden => "legacy_verifier_creation_forbidden",
            Self::ImmutableRecord => "immutable_storage_record",
            Self::DatabaseOperationFailed => "storage_database_operation_failed",
        }
    }

    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::InvalidIdentifier => "storage identifier is invalid",
            Self::InvalidDigest => "storage digest is invalid",
            Self::InvalidSecretMaterial => "secret verification material is invalid",
            Self::InvalidGeneration => "generation or version is invalid",
            Self::VersionOverflow => "generation or version cannot be advanced safely",
            Self::StaleVersion { .. } => "the expected storage version is stale",
            Self::MissingRecord => "required storage record does not exist",
            Self::IdempotencyConflict => "idempotency identity conflicts with persisted facts",
            Self::OperationInProgress => "a conflicting operation already owns the execution slot",
            Self::RuntimeLeaseConflict => "another runtime currently owns the adapter lease",
            Self::StaleRuntimeEpoch => "the runtime epoch is stale",
            Self::StaleRuntimeReport => "the runtime report sequence is stale",
            Self::RuntimeReportConflict => "the runtime report conflicts with persisted facts",
            Self::LegacyVerifierCreationForbidden => {
                "legacy verifier records may only be created by the forward migration"
            }
            Self::ImmutableRecord => "the storage record is immutable",
            Self::DatabaseOperationFailed => "storage database operation failed",
        }
    }
}

impl fmt::Display for ControlPlaneRepositoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message())
    }
}

impl Error for ControlPlaneRepositoryError {}

/// A validated non-reversible SHA-256/HMAC-SHA-256 digest.
#[derive(Clone, Eq, PartialEq)]
pub struct SecretDigest(String);

impl SecretDigest {
    pub fn parse(value: impl Into<String>) -> ControlPlaneResult<Self> {
        let value = value.into();
        if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(ControlPlaneRepositoryError::InvalidDigest);
        }
        Ok(Self(value.to_ascii_lowercase()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretDigest([REDACTED])")
    }
}

impl fmt::Display for SecretDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("[REDACTED]")
    }
}

/// Private verifier material. Debug and Display never reveal its contents.
#[derive(Clone, Eq, PartialEq)]
pub struct VerifierMaterial(String);

impl VerifierMaterial {
    pub fn parse(value: impl Into<String>) -> ControlPlaneResult<Self> {
        let value = value.into();
        if value.is_empty() || value.len() > 4096 || value.chars().any(char::is_control) {
            return Err(ControlPlaneRepositoryError::InvalidSecretMaterial);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for VerifierMaterial {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("VerifierMaterial([REDACTED])")
    }
}

impl fmt::Display for VerifierMaterial {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("[REDACTED]")
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MaintenanceControlRow {
    pub schema_version: i32,
    pub maintenance_generation: i64,
    pub state: String,
    pub transition_operation_id: Option<String>,
    pub transition_requested_by: Option<String>,
    pub transition_requested_at: Option<DateTime<Utc>>,
    pub state_entered_at: DateTime<Utc>,
    pub admission_fence_closed: bool,
    pub admission_fence_closed_at: Option<DateTime<Utc>>,
    pub quiescence_evidence_version: i64,
    pub quiescence_evidence_id: Option<String>,
    pub active_maintenance_job_id: Option<String>,
    pub safe_error_category: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MaintenanceControlUpdate {
    pub state: String,
    pub transition_operation_id: Option<String>,
    pub transition_requested_by: Option<String>,
    pub transition_requested_at: Option<DateTime<Utc>>,
    pub state_entered_at: DateTime<Utc>,
    pub admission_fence_closed: bool,
    pub admission_fence_closed_at: Option<DateTime<Utc>>,
    pub quiescence_evidence_version: i64,
    pub quiescence_evidence_id: Option<String>,
    pub active_maintenance_job_id: Option<String>,
    pub safe_error_category: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AdapterInventoryStateRow {
    pub adapter_inventory_generation: i64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AdapterInventoryItem {
    pub adapter_id: String,
    pub adapter_kind: String,
    pub control_authority: String,
    pub configured_at: DateTime<Utc>,
    pub retired_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AdapterInventoryRow {
    pub adapter_id: String,
    pub adapter_kind: String,
    pub control_authority: String,
    pub inventory_generation: i64,
    pub configured_at: DateTime<Utc>,
    pub retired_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AdapterDesiredControlRow {
    pub adapter_id: String,
    pub desired_enabled: bool,
    pub desired_mode: String,
    pub adapter_control_generation: i64,
    pub maintenance_generation: i64,
    pub maintenance_hold: bool,
    pub updated_by: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AdapterDesiredControlUpdate {
    pub desired_enabled: bool,
    pub desired_mode: String,
    pub maintenance_generation: i64,
    pub maintenance_hold: bool,
    pub updated_by: String,
}

#[derive(Clone, PartialEq)]
pub struct AdapterEffectiveControlInput {
    pub adapter_id: String,
    pub effective_mode: Option<String>,
    pub runtime_lifecycle: String,
    pub last_applied_adapter_control_generation: Option<i64>,
    pub last_applied_maintenance_generation: Option<i64>,
    pub heartbeat_at: Option<DateTime<Utc>>,
    pub last_success_at: Option<DateTime<Utc>>,
    pub last_cycle_started_at: Option<DateTime<Utc>>,
    pub last_cycle_finished_at: Option<DateTime<Utc>>,
    pub in_flight: bool,
    pub connection_state: String,
    pub safe_error_category: Option<String>,
    pub safe_error_code: Option<String>,
    pub runtime_instance_id: Option<String>,
    pub standalone_runtime_epoch: Option<i64>,
    pub report_sequence: Option<i64>,
    pub runtime_lease_version: Option<i64>,
    pub runtime_lease_expires_at: Option<DateTime<Utc>>,
    pub checkpoint_summary: Value,
    pub reported_at: DateTime<Utc>,
}

#[derive(Clone, PartialEq)]
pub struct AdapterEffectiveControlRow {
    pub adapter_id: String,
    pub effective_mode: Option<String>,
    pub runtime_lifecycle: String,
    pub last_applied_adapter_control_generation: Option<i64>,
    pub last_applied_maintenance_generation: Option<i64>,
    pub heartbeat_at: Option<DateTime<Utc>>,
    pub last_success_at: Option<DateTime<Utc>>,
    pub last_cycle_started_at: Option<DateTime<Utc>>,
    pub last_cycle_finished_at: Option<DateTime<Utc>>,
    pub in_flight: bool,
    pub connection_state: String,
    pub safe_error_category: Option<String>,
    pub safe_error_code: Option<String>,
    pub runtime_instance_id: Option<String>,
    pub standalone_runtime_epoch: Option<i64>,
    pub report_sequence: Option<i64>,
    pub runtime_lease_version: Option<i64>,
    pub runtime_lease_expires_at: Option<DateTime<Utc>>,
    pub checkpoint_summary: Value,
    pub reported_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, PartialEq)]
pub struct QuiescenceEvidenceInput {
    pub quiescence_evidence_id: String,
    pub evidence_version: i64,
    pub maintenance_generation: i64,
    pub adapter_inventory_generation: i64,
    pub admission_fence_closed_at: DateTime<Utc>,
    pub server_instance_id: String,
    pub server_started_at: DateTime<Utc>,
    pub evidence_digest: SecretDigest,
    pub captured_at: DateTime<Utc>,
    pub adapter_snapshots: Vec<QuiescenceAdapterSnapshotInput>,
    pub runtime_snapshots: Vec<QuiescenceRuntimeSnapshotInput>,
}

#[derive(Clone, PartialEq)]
pub struct QuiescenceAdapterSnapshotInput {
    pub adapter_id: String,
    pub adapter_kind: String,
    pub control_authority: String,
    pub adapter_control_generation: i64,
    pub maintenance_generation: i64,
    pub desired_enabled: bool,
    pub desired_mode: String,
    pub last_applied_adapter_control_generation: Option<i64>,
    pub last_applied_maintenance_generation: Option<i64>,
    pub runtime_lifecycle: String,
    pub connection_state: String,
    pub drain_proof: String,
    pub external_fence_id: Option<String>,
    pub checkpoint_summary: Value,
    pub captured_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QuiescenceRuntimeSnapshotInput {
    pub adapter_id: String,
    pub runtime_instance_id: String,
    pub runtime_lease_version: i64,
    pub standalone_runtime_epoch: i64,
    pub last_accepted_report_sequence: i64,
    pub last_accepted_report_fingerprint: SecretDigest,
    pub runtime_lease_expires_at: Option<DateTime<Utc>>,
    pub external_fence_id: Option<String>,
    pub captured_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QuiescenceEvidenceInvalidationInput {
    pub invalidation_id: String,
    pub quiescence_evidence_id: String,
    pub maintenance_generation: i64,
    pub invalidation_reason: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PrincipalRow {
    pub principal_id: String,
    pub principal_kind: String,
    pub role: String,
    pub principal_enabled: bool,
    pub principal_version: i64,
    pub credential_set_generation: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub disabled_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PrincipalUpdate {
    pub role: String,
    pub principal_enabled: bool,
    pub disabled_at: Option<DateTime<Utc>>,
    pub advance_credential_set_generation: bool,
}

#[derive(Clone, PartialEq)]
pub struct CredentialRow {
    pub credential_id: String,
    pub credential_version: i64,
    pub principal_id: String,
    pub issuance_operation_id: Option<String>,
    pub credential_kind: String,
    pub status: String,
    pub verifier_scheme: String,
    pub verifier_material: VerifierMaterial,
    pub legacy_migrated: bool,
    pub created_at: DateTime<Utc>,
    pub not_before: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub grace_expires_at: Option<DateTime<Utc>>,
    pub rotated_from_credential_id: Option<String>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub revoked_by: Option<String>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

impl fmt::Debug for CredentialRow {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CredentialRow")
            .field("credential_id", &self.credential_id)
            .field("credential_version", &self.credential_version)
            .field("principal_id", &self.principal_id)
            .field("issuance_operation_id", &self.issuance_operation_id)
            .field("credential_kind", &self.credential_kind)
            .field("status", &self.status)
            .field("verifier_scheme", &self.verifier_scheme)
            .field("verifier_material", &"[REDACTED]")
            .field("legacy_migrated", &self.legacy_migrated)
            .field("created_at", &self.created_at)
            .field("not_before", &self.not_before)
            .field("expires_at", &self.expires_at)
            .field("grace_expires_at", &self.grace_expires_at)
            .field("rotated_from_credential_id", &self.rotated_from_credential_id)
            .field("revoked_at", &self.revoked_at)
            .field("revoked_by", &self.revoked_by)
            .field("last_used_at", &self.last_used_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

#[derive(Clone, PartialEq)]
pub struct NewCredential {
    pub credential_id: String,
    pub principal_id: String,
    pub issuance_operation_id: String,
    pub credential_kind: String,
    pub status: String,
    pub verifier_scheme: String,
    pub verifier_material: VerifierMaterial,
    pub created_at: DateTime<Utc>,
    pub not_before: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub grace_expires_at: Option<DateTime<Utc>>,
    pub rotated_from_credential_id: Option<String>,
}

impl fmt::Debug for NewCredential {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NewCredential")
            .field("credential_id", &self.credential_id)
            .field("principal_id", &self.principal_id)
            .field("issuance_operation_id", &self.issuance_operation_id)
            .field("credential_kind", &self.credential_kind)
            .field("status", &self.status)
            .field("verifier_scheme", &self.verifier_scheme)
            .field("verifier_material", &"[REDACTED]")
            .field("created_at", &self.created_at)
            .field("not_before", &self.not_before)
            .field("expires_at", &self.expires_at)
            .field("grace_expires_at", &self.grace_expires_at)
            .field("rotated_from_credential_id", &self.rotated_from_credential_id)
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CredentialMutationResult {
    pub principal: PrincipalRow,
    pub credential: CredentialRow,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CredentialLifecycleUpdate {
    pub status: String,
    pub grace_expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub revoked_by: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Clone, PartialEq)]
pub struct CredentialIssuanceInput {
    pub requester_principal_id: String,
    pub idempotency_key_digest: SecretDigest,
    pub request_fingerprint: SecretDigest,
    pub issuance_operation_id: String,
    pub action: String,
    pub target_principal_id: String,
    pub affected_credential_ids: Value,
    pub committed_outcome: String,
    pub safe_result: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub enum IdempotencyInsertOutcome<T> {
    Inserted(T),
    Replay(T),
}

#[derive(Clone, PartialEq)]
pub struct CredentialIssuanceRow {
    pub requester_principal_id: String,
    pub idempotency_key_digest: SecretDigest,
    pub request_fingerprint: SecretDigest,
    pub issuance_operation_id: String,
    pub action: String,
    pub target_principal_id: String,
    pub affected_credential_ids: Value,
    pub committed_outcome: String,
    pub safe_result: Value,
    pub committed_at: DateTime<Utc>,
}

#[derive(Clone, PartialEq)]
pub struct OperationalJobInput {
    pub operation_id: String,
    pub kind: String,
    pub maintenance_required: bool,
    pub destructive: bool,
    pub requester_principal_id: String,
    pub requester_credential_id: Option<String>,
    pub idempotency_scope: String,
    pub idempotency_key_digest: SecretDigest,
    pub request_fingerprint: SecretDigest,
    pub state: String,
    pub dry_run: bool,
    pub confirmation_required: bool,
    pub confirmation_digest: Option<SecretDigest>,
    pub confirmation_expires_at: Option<DateTime<Utc>>,
    pub expected_maintenance_generation: Option<i64>,
    pub safe_summary: Value,
    pub artifact_manifest_id: Option<String>,
    pub artifact_manifest_digest: Option<SecretDigest>,
    pub checkpoint: Option<Value>,
    pub execution_scope_digest: SecretDigest,
    pub expected_adapter_generations: Vec<(String, i64)>,
    pub retry_of_operation_id: Option<String>,
}

#[derive(Clone, PartialEq)]
pub struct OperationalJobRow {
    pub operation_id: String,
    pub job_version: i64,
    pub kind: String,
    pub maintenance_required: bool,
    pub destructive: bool,
    pub requester_principal_id: String,
    pub requester_credential_id: Option<String>,
    pub state: String,
    pub dry_run: bool,
    pub confirmation_required: bool,
    pub confirmation_digest: Option<SecretDigest>,
    pub confirmation_expires_at: Option<DateTime<Utc>>,
    pub confirmation_consumed_at: Option<DateTime<Utc>>,
    pub expected_maintenance_generation: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub cancel_requested_at: Option<DateTime<Utc>>,
    pub safe_summary: Value,
    pub safe_error_category: Option<String>,
    pub artifact_manifest_id: Option<String>,
    pub artifact_manifest_digest: Option<SecretDigest>,
    pub checkpoint: Option<Value>,
    pub execution_scope_digest: SecretDigest,
    pub executor_id: Option<String>,
    pub executor_fence: i64,
    pub lease_token_digest: Option<SecretDigest>,
    pub lease_heartbeat_at: Option<DateTime<Utc>>,
    pub lease_expires_at: Option<DateTime<Utc>>,
    pub destructive_execution_slot_id: Option<String>,
}

impl fmt::Debug for OperationalJobRow {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OperationalJobRow")
            .field("operation_id", &self.operation_id)
            .field("job_version", &self.job_version)
            .field("kind", &self.kind)
            .field("state", &self.state)
            .field("executor_fence", &self.executor_fence)
            .field("sensitive_and_structured_fields", &"[REDACTED]")
            .finish()
    }
}

#[derive(Clone, PartialEq)]
pub struct OperationalJobUpdate {
    pub state: String,
    pub safe_summary: Value,
    pub safe_error_category: Option<String>,
    pub artifact_manifest_id: Option<String>,
    pub artifact_manifest_digest: Option<SecretDigest>,
    pub checkpoint: Option<Value>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub cancel_requested_at: Option<DateTime<Utc>>,
    pub confirmation_consumed_at: Option<DateTime<Utc>>,
    pub executor_id: Option<String>,
    pub executor_fence: i64,
    pub lease_token_digest: Option<SecretDigest>,
    pub lease_heartbeat_at: Option<DateTime<Utc>>,
    pub lease_expires_at: Option<DateTime<Utc>>,
    pub destructive_execution_slot_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExecutionSlotRow {
    pub slot_id: String,
    pub slot_version: i64,
    pub slot_kind: String,
    pub execution_scope_digest: Option<SecretDigest>,
    pub operation_id: Option<String>,
    pub maintenance_generation: Option<i64>,
    pub executor_fence: Option<i64>,
    pub blocked_uncertain: bool,
    pub acquired_at: Option<DateTime<Utc>>,
    pub released_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, PartialEq)]
pub struct OperationalEvidenceInput {
    pub evidence_id: String,
    pub operation_id: String,
    pub job_version: i64,
    pub evidence_kind: String,
    pub evidence_schema: String,
    pub evidence_version: i64,
    pub evidence_digest: SecretDigest,
    pub status: String,
    pub safe_metadata: Value,
    pub source_environment_id: Option<String>,
    pub target_environment_id: Option<String>,
    pub helper_attempt_id: Option<String>,
}

#[derive(Clone, PartialEq)]
pub struct OperationalAuditEventInput {
    pub audit_id: String,
    pub occurred_at: DateTime<Utc>,
    pub event_type: String,
    pub outcome: String,
    pub actor_principal_id: Option<String>,
    pub actor_credential_id: Option<String>,
    pub actor_role: Option<String>,
    pub actor_origin: String,
    pub request_id: Option<String>,
    pub correlation_id: Option<String>,
    pub target_type: String,
    pub target_id: Option<String>,
    pub operation_id: Option<String>,
    pub maintenance_generation: Option<i64>,
    pub adapter_control_generation: Option<i64>,
    pub principal_version: Option<i64>,
    pub credential_set_generation: Option<i64>,
    pub credential_version: Option<i64>,
    pub job_version: Option<i64>,
    pub executor_fence: Option<i64>,
    pub runtime_lease_version: Option<i64>,
    pub standalone_runtime_epoch: Option<i64>,
    pub runtime_report_sequence: Option<i64>,
    pub issuance_operation_id: Option<String>,
    pub previous_state: Option<String>,
    pub next_state: Option<String>,
    pub safe_error_category: Option<String>,
    pub artifact_manifest_id: Option<String>,
    pub safe_metadata: Value,
    pub corrects_audit_id: Option<String>,
}

#[derive(Clone, PartialEq)]
pub struct RuntimeAuthorityRow {
    pub adapter_id: String,
    pub runtime_lease_version: i64,
    pub standalone_runtime_epoch: i64,
    pub runtime_instance_id: Option<String>,
    pub lease_token_digest: Option<SecretDigest>,
    pub lease_heartbeat_at: Option<DateTime<Utc>>,
    pub lease_expires_at: Option<DateTime<Utc>>,
    pub last_accepted_report_sequence: i64,
    pub last_accepted_report_fingerprint: Option<SecretDigest>,
    pub open_mutation_permits: i64,
    pub uncertain_external_effects: i64,
    pub takeover_state: String,
    pub updated_at: DateTime<Utc>,
}

impl fmt::Debug for RuntimeAuthorityRow {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RuntimeAuthorityRow")
            .field("adapter_id", &self.adapter_id)
            .field("runtime_lease_version", &self.runtime_lease_version)
            .field("standalone_runtime_epoch", &self.standalone_runtime_epoch)
            .field("runtime_instance_id", &self.runtime_instance_id)
            .field("lease_token_digest", &self.lease_token_digest)
            .field("lease_heartbeat_at", &self.lease_heartbeat_at)
            .field("lease_expires_at", &self.lease_expires_at)
            .field("last_accepted_report_sequence", &self.last_accepted_report_sequence)
            .field("last_accepted_report_fingerprint", &self.last_accepted_report_fingerprint)
            .field("open_mutation_permits", &self.open_mutation_permits)
            .field("uncertain_external_effects", &self.uncertain_external_effects)
            .field("takeover_state", &self.takeover_state)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeLeaseAcquisition {
    pub adapter_id: String,
    pub expected_runtime_lease_version: i64,
    pub runtime_instance_id: String,
    pub lease_token_digest: SecretDigest,
    pub lease_heartbeat_at: DateTime<Utc>,
    pub lease_expires_at: DateTime<Utc>,
    pub takeover: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeLeaseRenewal {
    pub adapter_id: String,
    pub runtime_instance_id: String,
    pub expected_runtime_lease_version: i64,
    pub standalone_runtime_epoch: i64,
    pub lease_proof_digest: SecretDigest,
    pub heartbeat_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Clone, PartialEq)]
pub struct RuntimeReportInput {
    pub adapter_id: String,
    pub runtime_instance_id: String,
    pub standalone_runtime_epoch: i64,
    pub expected_runtime_lease_version: i64,
    pub report_sequence: i64,
    pub report_fingerprint: SecretDigest,
    pub lease_proof_digest: SecretDigest,
    pub adapter_control_generation: i64,
    pub maintenance_generation: i64,
    pub effective_mode: Option<String>,
    pub runtime_lifecycle: String,
    pub in_flight: bool,
    pub connection_state: String,
    pub checkpoint_summary: Value,
    pub safe_error_category: Option<String>,
    pub safe_error_code: Option<String>,
    pub reported_at: DateTime<Utc>,
    pub close_permit_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RuntimeReportOutcome {
    Accepted(RuntimeAuthorityRow),
    Replay(RuntimeAuthorityRow),
}

#[derive(Clone, Debug, PartialEq)]
pub struct MutationPermitInput {
    pub permit_id: String,
    pub adapter_id: String,
    pub runtime_instance_id: String,
    pub standalone_runtime_epoch: i64,
    pub expected_runtime_lease_version: i64,
    pub adapter_control_generation: i64,
    pub maintenance_generation: i64,
    pub step_id: String,
    pub lease_proof_digest: SecretDigest,
    pub permit_proof_digest: SecretDigest,
    pub bounded_expires_at: DateTime<Utc>,
}

#[derive(Clone, PartialEq)]
pub struct UncertainEffectInput {
    pub effect_id: String,
    pub adapter_id: String,
    pub runtime_instance_id: String,
    pub standalone_runtime_epoch: i64,
    pub expected_runtime_lease_version: i64,
    pub lease_proof_digest: SecretDigest,
    pub permit_id: Option<String>,
    pub step_id: String,
    pub effect_identity_digest: SecretDigest,
    pub safe_metadata: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UncertainEffectReconciliation {
    pub adapter_id: String,
    pub effect_id: String,
    pub expected_runtime_lease_version: i64,
    pub resolution: String,
    pub external_fence_id: Option<String>,
    pub clear_takeover_when_empty: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StorageFacts {
    pub schema_name: String,
    pub migration_head: String,
    pub current_control_table_count: i64,
    pub table_counts: Vec<(String, i64)>,
    pub principal_count: i64,
    pub credential_count: i64,
    pub operational_job_count: i64,
    pub operational_audit_count: i64,
    pub maintenance_generation: i64,
    pub adapter_inventory_generation: i64,
    pub highest_operation_sequence: i64,
    pub object_blob_count: i64,
    pub object_blob_bytes: i64,
    pub worktree_file_count: i64,
    pub worktree_file_bytes: i64,
    pub restore_excluded_table_count: i64,
}

macro_rules! impl_redacted_debug {
    ($($type_name:ty),+ $(,)?) => {
        $(
            impl fmt::Debug for $type_name {
                fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                    formatter
                        .debug_struct(stringify!($type_name))
                        .field("structured_fields", &"[REDACTED]")
                        .finish()
                }
            }
        )+
    };
}

impl_redacted_debug!(
    AdapterEffectiveControlInput,
    AdapterEffectiveControlRow,
    QuiescenceEvidenceInput,
    QuiescenceAdapterSnapshotInput,
    CredentialIssuanceInput,
    CredentialIssuanceRow,
    OperationalJobInput,
    OperationalJobUpdate,
    OperationalEvidenceInput,
    OperationalAuditEventInput,
    RuntimeReportInput,
    UncertainEffectInput,
);
