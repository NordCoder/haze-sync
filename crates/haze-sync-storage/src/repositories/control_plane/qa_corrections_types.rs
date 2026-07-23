/// Recovery-complete immutable quiescence evidence input.
#[derive(Clone, PartialEq)]
pub struct CompleteQuiescenceEvidenceInput {
    pub quiescence_evidence_id: String,
    pub evidence_version: i64,
    pub maintenance_generation: i64,
    pub adapter_inventory_generation: i64,
    pub admission_fence_closed_at: DateTime<Utc>,
    pub server_instance_id: String,
    pub server_started_at: DateTime<Utc>,
    pub active_authoritative_mutations: i64,
    pub open_authoritative_transactions: i64,
    pub active_cli_write_jobs: i64,
    pub object_store_writer_count: i64,
    pub database_writer_count: i64,
    pub worktree_external_writer_evidence: Value,
    pub evidence_digest: SecretDigest,
    pub captured_at: DateTime<Utc>,
    pub adapter_snapshots: Vec<QuiescenceAdapterSnapshotInput>,
    pub runtime_snapshots: Vec<QuiescenceRuntimeSnapshotInput>,
}

/// Recovery-complete immutable quiescence evidence row.
#[derive(Clone, PartialEq)]
pub struct CompleteQuiescenceEvidenceRow {
    pub quiescence_evidence_id: String,
    pub schema_version: i32,
    pub evidence_version: i64,
    pub maintenance_generation: i64,
    pub adapter_inventory_generation: i64,
    pub admission_fence_closed_at: DateTime<Utc>,
    pub server_instance_id: String,
    pub server_started_at: DateTime<Utc>,
    pub active_authoritative_mutations: i64,
    pub open_authoritative_transactions: i64,
    pub obsidian_authoritative_mutation_gate: String,
    pub active_cli_write_jobs: i64,
    pub object_store_writer_count: i64,
    pub database_writer_count: i64,
    pub worktree_external_writer_evidence: Value,
    pub evidence_digest: SecretDigest,
    pub captured_at: DateTime<Utc>,
}

#[derive(Clone, PartialEq)]
pub struct QuiescenceAdapterSnapshotRow {
    pub quiescence_evidence_id: String,
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
    pub in_flight: bool,
    pub drain_proof: String,
    pub external_fence_id: Option<String>,
    pub checkpoint_summary: Value,
    pub captured_at: DateTime<Utc>,
}

#[derive(Clone, PartialEq)]
pub struct QuiescenceRuntimeSnapshotRow {
    pub quiescence_evidence_id: String,
    pub adapter_id: String,
    pub runtime_instance_id: String,
    pub runtime_lease_version: i64,
    pub standalone_runtime_epoch: i64,
    pub last_accepted_report_sequence: i64,
    pub last_accepted_report_fingerprint: SecretDigest,
    pub open_mutation_permits: i64,
    pub uncertain_external_effects: i64,
    pub takeover_state: String,
    pub runtime_lease_expires_at: Option<DateTime<Utc>>,
    pub external_fence_id: Option<String>,
    pub captured_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QuiescenceEvidenceInvalidationRow {
    pub invalidation_id: String,
    pub quiescence_evidence_id: String,
    pub maintenance_generation: i64,
    pub invalidation_reason: String,
    pub invalidated_at: DateTime<Utc>,
}

/// One transactionally re-readable immutable quiescence evidence bundle.
#[derive(Clone, PartialEq)]
pub struct CompleteQuiescenceEvidenceBundle {
    pub evidence: CompleteQuiescenceEvidenceRow,
    pub adapter_snapshots: Vec<QuiescenceAdapterSnapshotRow>,
    pub runtime_snapshots: Vec<QuiescenceRuntimeSnapshotRow>,
    pub invalidations: Vec<QuiescenceEvidenceInvalidationRow>,
}

/// Canonical operational-job row plus its idempotency and generation bindings.
#[derive(Clone, PartialEq)]
pub struct CompleteOperationalJobRow {
    pub job: OperationalJobRow,
    pub idempotency_scope: String,
    pub idempotency_key_digest: SecretDigest,
    pub request_fingerprint: SecretDigest,
    pub expected_adapter_generations: Vec<(String, i64)>,
}

/// Non-blocking credential issuance reservation outcome.
#[derive(Clone, PartialEq)]
pub enum CredentialIssuanceReservationOutcome {
    Inserted(CredentialIssuanceRow),
    Replay(CredentialIssuanceRow),
    InProgress,
}

impl_redacted_debug!(
    CompleteQuiescenceEvidenceInput,
    CompleteQuiescenceEvidenceRow,
    QuiescenceAdapterSnapshotRow,
    QuiescenceRuntimeSnapshotRow,
    CompleteQuiescenceEvidenceBundle,
    CompleteOperationalJobRow,
    CredentialIssuanceReservationOutcome,
);
