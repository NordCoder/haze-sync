use std::{
    collections::BTreeMap,
    str::FromStr,
    sync::Arc,
    time::Duration as StdDuration,
};

use chrono::{DateTime, Duration, Utc};
use haze_sync_api::{
    auth::{AdapterPrincipal, AdapterRole, BearerToken},
    dto::{
        control_plane::{
            AffectedCredentialRequest, CredentialCreateRequest,
            CredentialIssuanceResponse, CredentialListResponse, CredentialRevokeRequest,
            CredentialRotateRequest, CredentialStatusDto, CredentialSummaryResponse,
            CredentialVerifierSchemeDto, ExpectedAdapterGenerationDto, MaintenanceActionDto,
            MaintenanceActionResponse, MaintenanceStateDto, MaintenanceStatusResponse,
            OperationalJobCancelRequest, OperationalJobConfirmationRequest,
            OperationalJobCreateRequest, OperationalJobKindDto, OperationalJobResponse,
            OperationalJobStateDto, OperationalJobSubmissionResponse, PrincipalKindDto,
            PrincipalMutationRequest, PrincipalStatusResponse, OPERATIONAL_JOB_SCHEMA_V1,
        },
        primitives::TimestampDto,
    },
};
use haze_sync_common::{AdapterId, Sha256 as CommonSha256};
use haze_sync_storage::{
    control_plane::{
        append_operational_audit_event, cas_acquire_execution_slot,
        cas_block_execution_slot_uncertain, cas_complete_maintenance_transition,
        cas_release_execution_slot, cas_update_adapter_desired_control,
        cas_update_credential_lifecycle, cas_update_maintenance_control,
        cas_update_operational_job, cas_update_principal,
        credential_verifier_material_for_private_authentication,
        ensure_scoped_execution_slot, insert_complete_quiescence_evidence, insert_credential,
        insert_operational_evidence, insert_or_replay_complete_operational_job,
        lock_adapter_desired_control, lock_complete_operational_job,
        lock_complete_quiescence_evidence, lock_credential, lock_execution_slot,
        lock_maintenance_control, lock_principal, read_adapter_inventory_snapshot,
        read_credential_by_id, read_credentials_by_principal, read_legacy_credential_by_sha256,
        reserve_or_replay_credential_issuance, rotate_credential_set,
        CompleteQuiescenceEvidenceInput, CompleteOperationalJobRow, CredentialIssuanceInput,
        CredentialIssuanceReservationOutcome, CredentialLifecycleUpdate,
        CredentialRotationMutation, CredentialSetRotationInput, IdempotencyInsertOutcome,
        MaintenanceControlRow, MaintenanceControlUpdate, NewCredential,
        OperationalAuditEventInput, OperationalEvidenceInput, OperationalJobInput,
        OperationalJobRow, OperationalJobUpdate, PrincipalRow, PrincipalUpdate, SecretDigest,
        VerifierMaterial,
    },
    repositories::idempotency::{
        compare_request_fingerprint, insert_idempotency_record, read_idempotency_record,
        IdempotencyRecordInput, IdempotencyRequestComparison, IdempotencyStoreOutcome,
    },
};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};
use sqlx::{PgPool, Postgres, Transaction};

use super::{
    admission::{AdmissionClass, AdmissionController, DurableMaintenanceState},
    crypto::{
        constant_time_digest_eq, issue_credential, legacy_sha256_digest, random_identifier,
        random_secret, request_secret_digest, request_sha256, verify_argon2id_secret,
        ControlPlaneSecrets,
    },
    fakes::{
        AdapterControlEvidence, AdapterControlTarget, DeterministicFakeExecutor,
        EmptyInventoryAdapterControl, ExecutorCommand, ExecutorOutcome, OperationalExecutor,
    },
    ControlPlaneError,
};

const QUIESCENCE_TIMEOUT_SECONDS: u64 = 30;
const JOB_CONFIRMATION_SECONDS: i64 = 600;
const JOB_LEASE_SECONDS: i64 = 30;
const GLOBAL_DESTRUCTIVE_SLOT: &str = "global-destructive";

#[derive(Clone)]
pub(crate) struct ControlPlaneServices {
    pool: PgPool,
    admission: AdmissionController,
    secrets: Arc<ControlPlaneSecrets>,
    adapter_control: Arc<dyn AdapterControlEvidence>,
    executor: Arc<dyn OperationalExecutor>,
    server_instance_id: String,
    server_started_at: DateTime<Utc>,
}
