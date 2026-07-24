

#[derive(Clone, Debug)]
pub(crate) struct AuthenticatedIdentity {
    pub(crate) principal: AdapterPrincipal,
    pub(crate) credential_id: Option<String>,
}

impl AuthenticatedIdentity {
    pub(crate) fn static_principal(principal: AdapterPrincipal) -> Self {
        Self {
            principal,
            credential_id: None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct JobContract {
    maintenance_required: bool,
    destructive: bool,
    confirmation_required: bool,
}

impl JobContract {
    fn for_request(request: &OperationalJobCreateRequest) -> Result<Self, ControlPlaneError> {
        let contract = match request.kind {
            OperationalJobKindDto::AdapterDryRun => {
                if !request.dry_run {
                    return Err(ControlPlaneError::InvalidInput);
                }
                Self {
                    maintenance_required: false,
                    destructive: false,
                    confirmation_required: false,
                }
            }
            OperationalJobKindDto::AdapterReconcile | OperationalJobKindDto::Backup => Self {
                maintenance_required: false,
                destructive: false,
                confirmation_required: false,
            },
            OperationalJobKindDto::Restore
            | OperationalJobKindDto::Repair
            | OperationalJobKindDto::RetentionCleanup
            | OperationalJobKindDto::DeploymentRollout
            | OperationalJobKindDto::DeploymentRollback => Self {
                maintenance_required: true,
                destructive: true,
                confirmation_required: true,
            },
        };
        if request.dry_run && contract.destructive {
            return Ok(Self {
                maintenance_required: false,
                destructive: false,
                confirmation_required: false,
            });
        }
        if contract.maintenance_required && request.expected_maintenance_generation.is_none() {
            return Err(ControlPlaneError::InvalidInput);
        }
        Ok(contract)
    }
}

async fn read_control_replay<T: DeserializeOwned>(
    pool: &PgPool,
    actor_id: &AdapterId,
    stored_key: &str,
    fingerprint: &CommonSha256,
) -> Result<Option<T>, ControlPlaneError> {
    let mut connection = pool.acquire().await.map_err(|_| ControlPlaneError::Internal)?;
    let Some(record) = read_idempotency_record(&mut connection, actor_id, stored_key)
        .await
        .map_err(|_| ControlPlaneError::Internal)?
    else {
        return Ok(None);
    };
    match compare_request_fingerprint(&record, fingerprint)
        .map_err(|_| ControlPlaneError::Internal)?
    {
        IdempotencyRequestComparison::SameRequest => serde_json::from_value(record.response_json)
            .map(Some)
            .map_err(|_| ControlPlaneError::Internal),
        IdempotencyRequestComparison::DifferentRequest => {
            Err(ControlPlaneError::IdempotencyConflict)
        }
    }
}

async fn read_control_replay_on(
    transaction: &mut Transaction<'_, Postgres>,
    actor_id: &AdapterId,
    stored_key: &str,
    fingerprint: &CommonSha256,
) -> Result<Option<Value>, ControlPlaneError> {
    let Some(record) = read_idempotency_record(&mut **transaction, actor_id, stored_key)
        .await
        .map_err(|_| ControlPlaneError::Internal)?
    else {
        return Ok(None);
    };
    match compare_request_fingerprint(&record, fingerprint)
        .map_err(|_| ControlPlaneError::Internal)?
    {
        IdempotencyRequestComparison::SameRequest => Ok(Some(record.response_json)),
        IdempotencyRequestComparison::DifferentRequest => {
            Err(ControlPlaneError::IdempotencyConflict)
        }
    }
}

async fn store_control_result_on<T: Serialize>(
    transaction: &mut Transaction<'_, Postgres>,
    actor_id: &AdapterId,
    stored_key: &str,
    fingerprint: CommonSha256,
    result: &T,
) -> Result<Option<Value>, ControlPlaneError> {
    let response_json = serde_json::to_value(result).map_err(|_| ControlPlaneError::Internal)?;
    let input = IdempotencyRecordInput::new(
        actor_id.clone(),
        stored_key,
        fingerprint,
        response_json,
    )
    .map_err(|_| ControlPlaneError::InvalidInput)?;
    match insert_idempotency_record(&mut **transaction, &input)
        .await
        .map_err(|_| ControlPlaneError::Internal)?
    {
        IdempotencyStoreOutcome::Stored { .. } => Ok(None),
        IdempotencyStoreOutcome::AlreadyExists { record } => {
            match compare_request_fingerprint(&record, input.request_hash())
                .map_err(|_| ControlPlaneError::Internal)?
            {
                IdempotencyRequestComparison::SameRequest => Ok(Some(record.response_json)),
                IdempotencyRequestComparison::DifferentRequest => {
                    Err(ControlPlaneError::IdempotencyConflict)
                }
            }
        }
    }
}

fn maintenance_response(
    row: MaintenanceControlRow,
) -> Result<MaintenanceStatusResponse, ControlPlaneError> {
    let state = match row.state.as_str() {
        "normal" => MaintenanceStateDto::Normal,
        "quiescing" => MaintenanceStateDto::Quiescing,
        "quiesced" => MaintenanceStateDto::Quiesced,
        "maintenance" => MaintenanceStateDto::Maintenance,
        "resuming" => MaintenanceStateDto::Resuming,
        _ => return Err(ControlPlaneError::InvalidStoredState),
    };
    Ok(MaintenanceStatusResponse {
        schema_version: u32::try_from(row.schema_version)
            .map_err(|_| ControlPlaneError::InvalidStoredState)?,
        maintenance_generation: to_u64(row.maintenance_generation)?,
        state,
        transition_operation_id: row.transition_operation_id,
        state_entered_at: timestamp(row.state_entered_at),
        admission_fence_closed: row.admission_fence_closed,
        quiescence_evidence_version: to_u64(row.quiescence_evidence_version)?,
        quiescence_evidence_id: row.quiescence_evidence_id,
        active_maintenance_job_id: row.active_maintenance_job_id,
        safe_error_category: row.safe_error_category,
    })
}

fn principal_response(row: PrincipalRow) -> Result<PrincipalStatusResponse, ControlPlaneError> {
    Ok(PrincipalStatusResponse {
        principal_id: row.principal_id,
        principal_kind: match row.principal_kind.as_str() {
            "adapter" => PrincipalKindDto::Adapter,
            "administrator" => PrincipalKindDto::Administrator,
            _ => return Err(ControlPlaneError::InvalidStoredState),
        },
        role: adapter_role(&row.role)?,
        principal_enabled: row.principal_enabled,
        principal_version: to_u64(row.principal_version)?,
        credential_set_generation: to_u64(row.credential_set_generation)?,
        created_at: timestamp(row.created_at),
        updated_at: timestamp(row.updated_at),
        disabled_at: row.disabled_at.map(timestamp),
    })
}

fn credential_response(
    row: haze_sync_storage::control_plane::CredentialRow,
    effective_role: AdapterRole,
) -> Result<CredentialSummaryResponse, ControlPlaneError> {
    Ok(CredentialSummaryResponse {
        credential_id: row.credential_id,
        principal_id: row.principal_id,
        effective_role,
        credential_kind: row.credential_kind,
        credential_version: to_u64(row.credential_version)?,
        status: match row.status.as_str() {
            "active" => CredentialStatusDto::Active,
            "grace" => CredentialStatusDto::Grace,
            "revoked" => CredentialStatusDto::Revoked,
            "expired" => CredentialStatusDto::Expired,
            _ => return Err(ControlPlaneError::InvalidStoredState),
        },
        verifier_scheme: match row.verifier_scheme.as_str() {
            "argon2id_v1" => CredentialVerifierSchemeDto::Argon2idV1,
            "legacy_sha256_v0" => CredentialVerifierSchemeDto::LegacySha256V0,
            _ => return Err(ControlPlaneError::InvalidStoredState),
        },
        created_at: timestamp(row.created_at),
        not_before: timestamp(row.not_before),
        expires_at: row.expires_at.map(timestamp),
        grace_expires_at: row.grace_expires_at.map(timestamp),
        rotated_from_credential_id: row.rotated_from_credential_id,
        revoked_at: row.revoked_at.map(timestamp),
        last_used_at: row.last_used_at.map(timestamp),
    })
}

fn job_response(row: CompleteOperationalJobRow) -> Result<OperationalJobResponse, ControlPlaneError> {
    let job = row.job;
    Ok(OperationalJobResponse {
        schema: OPERATIONAL_JOB_SCHEMA_V1.into(),
        operation_id: job.operation_id,
        job_version: to_u64(job.job_version)?,
        kind: job_kind(&job.kind)?,
        maintenance_required: job.maintenance_required,
        destructive: job.destructive,
        state: job_state(&job.state)?,
        dry_run: job.dry_run,
        confirmation_required: job.confirmation_required,
        confirmation_expires_at: job.confirmation_expires_at.map(timestamp),
        expected_maintenance_generation: job
            .expected_maintenance_generation
            .map(to_u64)
            .transpose()?,
        expected_adapter_control_generations: row
            .expected_adapter_generations
            .into_iter()
            .map(|(adapter_id, generation)| {
                Ok(ExpectedAdapterGenerationDto {
                    adapter_id,
                    adapter_control_generation: to_u64(generation)?,
                })
            })
            .collect::<Result<Vec<_>, ControlPlaneError>>()?,
        created_at: timestamp(job.created_at),
        updated_at: timestamp(job.updated_at),
        started_at: job.started_at.map(timestamp),
        completed_at: job.completed_at.map(timestamp),
        cancel_requested_at: job.cancel_requested_at.map(timestamp),
        safe_summary: safe_map(&job.safe_summary),
        safe_error_category: job.safe_error_category.clone(),
        artifact_manifest_id: job.artifact_manifest_id,
        checkpoint: job.checkpoint.as_ref().map(safe_map),
        executor_id: job.executor_id,
        executor_fence: to_u64(job.executor_fence)?,
        lease_heartbeat_at: job.lease_heartbeat_at.map(timestamp),
        lease_expires_at: job.lease_expires_at.map(timestamp),
        blocked_uncertain: job
            .safe_error_category
            .as_deref()
            .is_some_and(|category| category.contains("uncertain")),
    })
}

fn job_kind(value: &str) -> Result<OperationalJobKindDto, ControlPlaneError> {
    match value {
        "adapter_dry_run" => Ok(OperationalJobKindDto::AdapterDryRun),
        "adapter_reconcile" => Ok(OperationalJobKindDto::AdapterReconcile),
        "backup" => Ok(OperationalJobKindDto::Backup),
        "restore" => Ok(OperationalJobKindDto::Restore),
        "repair" => Ok(OperationalJobKindDto::Repair),
        "retention_cleanup" => Ok(OperationalJobKindDto::RetentionCleanup),
        "deployment_rollout" => Ok(OperationalJobKindDto::DeploymentRollout),
        "deployment_rollback" => Ok(OperationalJobKindDto::DeploymentRollback),
        _ => Err(ControlPlaneError::InvalidStoredState),
    }
}

fn job_state(value: &str) -> Result<OperationalJobStateDto, ControlPlaneError> {
    match value {
        "planned" => Ok(OperationalJobStateDto::Planned),
        "awaiting_confirmation" => Ok(OperationalJobStateDto::AwaitingConfirmation),
        "running" => Ok(OperationalJobStateDto::Running),
        "succeeded" => Ok(OperationalJobStateDto::Succeeded),
        "failed" => Ok(OperationalJobStateDto::Failed),
        "cancelled" => Ok(OperationalJobStateDto::Cancelled),
        _ => Err(ControlPlaneError::InvalidStoredState),
    }
}


fn quiescence_collection_binding(
    collection: &super::fakes::QuiescenceCollection,
) -> Result<String, ControlPlaneError> {
    let adapters = collection
        .adapter_snapshots
        .iter()
        .map(|snapshot| {
            json!({
                "adapter_id": snapshot.adapter_id.as_str(),
                "adapter_kind": snapshot.adapter_kind.as_str(),
                "control_authority": snapshot.control_authority.as_str(),
                "adapter_control_generation": snapshot.adapter_control_generation,
                "maintenance_generation": snapshot.maintenance_generation,
                "desired_enabled": snapshot.desired_enabled,
                "desired_mode": snapshot.desired_mode.as_str(),
                "last_applied_adapter_control_generation": snapshot.last_applied_adapter_control_generation,
                "last_applied_maintenance_generation": snapshot.last_applied_maintenance_generation,
                "runtime_lifecycle": snapshot.runtime_lifecycle.as_str(),
                "connection_state": snapshot.connection_state.as_str(),
                "drain_proof": snapshot.drain_proof.as_str(),
                "external_fence_id": snapshot.external_fence_id.as_deref(),
                "checkpoint_summary": &snapshot.checkpoint_summary,
                "captured_at": snapshot.captured_at.to_rfc3339(),
            })
        })
        .collect::<Vec<_>>();
    let runtimes = collection
        .runtime_snapshots
        .iter()
        .map(|snapshot| {
            json!({
                "adapter_id": snapshot.adapter_id.as_str(),
                "runtime_instance_id": snapshot.runtime_instance_id.as_str(),
                "runtime_lease_version": snapshot.runtime_lease_version,
                "standalone_runtime_epoch": snapshot.standalone_runtime_epoch,
                "last_accepted_report_sequence": snapshot.last_accepted_report_sequence,
                "last_accepted_report_fingerprint": snapshot.last_accepted_report_fingerprint.as_str(),
                "runtime_lease_expires_at": snapshot.runtime_lease_expires_at.map(|value| value.to_rfc3339()),
                "external_fence_id": snapshot.external_fence_id.as_deref(),
                "captured_at": snapshot.captured_at.to_rfc3339(),
            })
        })
        .collect::<Vec<_>>();
    serde_json::to_string(&json!({
        "adapter_snapshots": adapters,
        "runtime_snapshots": runtimes,
        "worktree_external_writer_evidence": &collection.worktree_external_writer_evidence,
    }))
    .map_err(|_| ControlPlaneError::Internal)
}

fn safe_map(value: &Value) -> BTreeMap<String, String> {
    match value {
        Value::Object(object) => object
            .iter()
            .map(|(key, value)| {
                let rendered = value
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| value.to_string());
                (key.clone(), rendered)
            })
            .collect(),
        other => BTreeMap::from([("value".into(), other.to_string())]),
    }
}

fn verify_rotation_affected_set(
    current: &[haze_sync_storage::control_plane::CredentialRow],
    requested: &[AffectedCredentialRequest],
) -> Result<(), ControlPlaneError> {
    let mut current_set = current
        .iter()
        .filter(|credential| matches!(credential.status.as_str(), "active" | "grace"))
        .map(|credential| {
            (
                credential.credential_id.as_str(),
                credential.credential_version,
                credential.status.as_str(),
            )
        })
        .collect::<Vec<_>>();
    let mut requested_set = requested
        .iter()
        .map(|credential| {
            Ok((
                credential.credential_id.as_str(),
                to_i64(credential.expected_credential_version)?,
                credential.current_status.as_str(),
            ))
        })
        .collect::<Result<Vec<_>, ControlPlaneError>>()?;
    current_set.sort_unstable();
    requested_set.sort_unstable();
    if current_set != requested_set {
        return Err(ControlPlaneError::StaleRecordVersion);
    }
    Ok(())
}

fn confirmation_binding(complete: &CompleteOperationalJobRow) -> Result<String, ControlPlaneError> {
    let adapter_generations = complete
        .expected_adapter_generations
        .iter()
        .map(|(adapter_id, generation)| {
            Ok(ExpectedAdapterGenerationDto {
                adapter_id: adapter_id.clone(),
                adapter_control_generation: to_u64(*generation)?,
            })
        })
        .collect::<Result<Vec<_>, ControlPlaneError>>()?;
    Ok(confirmation_binding_from_parts(
        &complete.job.operation_id,
        &complete.job.kind,
        complete.request_fingerprint.as_str(),
        complete
            .job
            .expected_maintenance_generation
            .map(to_u64)
            .transpose()?,
        &adapter_generations,
        complete.job.job_version,
        complete.job.artifact_manifest_digest.as_ref(),
        complete.job.confirmation_expires_at,
    ))
}

fn confirmation_binding_from_parts(
    operation_id: &str,
    kind: &str,
    request_fingerprint: &str,
    maintenance_generation: Option<u64>,
    adapter_generations: &[ExpectedAdapterGenerationDto],
    job_version: i64,
    artifact_manifest_digest: Option<&SecretDigest>,
    expires_at: Option<DateTime<Utc>>,
) -> String {
    let mut generations = adapter_generations
        .iter()
        .map(|entry| format!("{}:{}", entry.adapter_id, entry.adapter_control_generation))
        .collect::<Vec<_>>();
    generations.sort();
    format!(
        "haze-sync.job-confirmation.v1\u{1f}{operation_id}\u{1f}{kind}\u{1f}{request_fingerprint}\u{1f}{}\u{1f}{}\u{1f}{job_version}\u{1f}{}\u{1f}{}",
        maintenance_generation
            .map(|generation| generation.to_string())
            .unwrap_or_else(|| "null".into()),
        generations.join(","),
        artifact_manifest_digest
            .map(SecretDigest::as_str)
            .unwrap_or("null"),
        expires_at
            .map(|value| value.to_rfc3339())
            .unwrap_or_else(|| "null".into())
    )
}

fn terminal_job_update(
    current: &OperationalJobRow,
    state: &str,
    safe_error_category: Option<String>,
    safe_summary: Value,
    checkpoint: Option<Value>,
) -> OperationalJobUpdate {
    OperationalJobUpdate {
        state: state.into(),
        safe_summary,
        safe_error_category,
        artifact_manifest_id: current.artifact_manifest_id.clone(),
        artifact_manifest_digest: current.artifact_manifest_digest.clone(),
        checkpoint,
        started_at: current.started_at,
        completed_at: Some(Utc::now()),
        cancel_requested_at: current.cancel_requested_at,
        confirmation_consumed_at: current.confirmation_consumed_at,
        executor_id: current.executor_id.clone(),
        executor_fence: current.executor_fence,
        lease_token_digest: current.lease_token_digest.clone(),
        lease_heartbeat_at: current.lease_heartbeat_at,
        lease_expires_at: current.lease_expires_at,
        destructive_execution_slot_id: current.destructive_execution_slot_id.clone(),
    }
}

fn audit_event(
    event_type: &str,
    outcome: &str,
    actor: &AuthenticatedIdentity,
    target_type: &str,
    target_id: Option<String>,
    operation_id: Option<String>,
    maintenance_generation: Option<i64>,
    principal_version: Option<i64>,
    credential_set_generation: Option<i64>,
    credential_version: Option<i64>,
    previous_state: Option<String>,
    next_state: Option<String>,
    safe_metadata: Value,
) -> Result<OperationalAuditEventInput, ControlPlaneError> {
    Ok(OperationalAuditEventInput {
        audit_id: random_identifier("audit_")?,
        occurred_at: Utc::now(),
        event_type: event_type.into(),
        outcome: outcome.into(),
        actor_principal_id: Some(actor.principal.adapter_id().to_owned()),
        actor_credential_id: actor.credential_id.clone(),
        actor_role: Some(actor.principal.role().to_string()),
        actor_origin: "server_api".into(),
        request_id: None,
        correlation_id: None,
        target_type: target_type.into(),
        target_id,
        operation_id,
        maintenance_generation,
        adapter_control_generation: None,
        principal_version,
        credential_set_generation,
        credential_version,
        job_version: None,
        executor_fence: None,
        runtime_lease_version: None,
        standalone_runtime_epoch: None,
        runtime_report_sequence: None,
        issuance_operation_id: None,
        previous_state,
        next_state,
        safe_error_category: None,
        artifact_manifest_id: None,
        safe_metadata,
        corrects_audit_id: None,
    })
}

fn system_audit_event(
    event_type: &str,
    outcome: &str,
    target_type: &str,
    target_id: Option<String>,
    operation_id: Option<String>,
    maintenance_generation: Option<i64>,
    previous_state: Option<String>,
    next_state: Option<String>,
    safe_metadata: Value,
) -> Result<OperationalAuditEventInput, ControlPlaneError> {
    Ok(OperationalAuditEventInput {
        audit_id: random_identifier("audit_")?,
        occurred_at: Utc::now(),
        event_type: event_type.into(),
        outcome: outcome.into(),
        actor_principal_id: None,
        actor_credential_id: None,
        actor_role: None,
        actor_origin: "server_runtime".into(),
        request_id: None,
        correlation_id: None,
        target_type: target_type.into(),
        target_id,
        operation_id,
        maintenance_generation,
        adapter_control_generation: None,
        principal_version: None,
        credential_set_generation: None,
        credential_version: None,
        job_version: None,
        executor_fence: None,
        runtime_lease_version: None,
        standalone_runtime_epoch: None,
        runtime_report_sequence: None,
        issuance_operation_id: None,
        previous_state,
        next_state,
        safe_error_category: None,
        artifact_manifest_id: None,
        safe_metadata,
        corrects_audit_id: None,
    })
}

fn audit_event_with_issuance(
    event_type: &str,
    actor: &AuthenticatedIdentity,
    mutation: &haze_sync_storage::control_plane::CredentialMutationResult,
    issuance_operation_id: &str,
    safe_metadata: Value,
) -> Result<OperationalAuditEventInput, ControlPlaneError> {
    let mut event = audit_event(
        event_type,
        "succeeded",
        actor,
        "credential",
        Some(mutation.credential.credential_id.clone()),
        None,
        None,
        Some(mutation.principal.principal_version),
        Some(mutation.principal.credential_set_generation),
        Some(mutation.credential.credential_version),
        None,
        Some(mutation.credential.status.clone()),
        safe_metadata,
    )?;
    event.issuance_operation_id = Some(issuance_operation_id.into());
    Ok(event)
}

fn job_audit_event(
    event_type: &str,
    outcome: &str,
    actor: &AuthenticatedIdentity,
    job: &OperationalJobRow,
    safe_metadata: Value,
) -> Result<OperationalAuditEventInput, ControlPlaneError> {
    let mut event = audit_event(
        event_type,
        outcome,
        actor,
        "operational_job",
        Some(job.operation_id.clone()),
        Some(job.operation_id.clone()),
        job.expected_maintenance_generation,
        None,
        None,
        None,
        None,
        Some(job.state.clone()),
        safe_metadata,
    )?;
    event.job_version = Some(job.job_version);
    event.executor_fence = Some(job.executor_fence);
    event.artifact_manifest_id = job.artifact_manifest_id.clone();
    event.safe_error_category = job.safe_error_category.clone();
    Ok(event)
}

fn adapter_role(value: &str) -> Result<AdapterRole, ControlPlaneError> {
    AdapterRole::from_str(value).map_err(|_| ControlPlaneError::InvalidStoredState)
}

fn require_admin(actor: &AuthenticatedIdentity) -> Result<(), ControlPlaneError> {
    if actor.principal.role().can_admin() {
        Ok(())
    } else {
        Err(ControlPlaneError::Forbidden)
    }
}

fn timestamp(value: DateTime<Utc>) -> TimestampDto {
    TimestampDto::from(value.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
}

fn parse_timestamp(value: &TimestampDto) -> Result<DateTime<Utc>, ControlPlaneError> {
    DateTime::parse_from_rfc3339(value.as_str())
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| ControlPlaneError::InvalidInput)
}

fn to_i64(value: u64) -> Result<i64, ControlPlaneError> {
    i64::try_from(value).map_err(|_| ControlPlaneError::InvalidInput)
}

fn to_u64(value: i64) -> Result<u64, ControlPlaneError> {
    u64::try_from(value).map_err(|_| ControlPlaneError::InvalidStoredState)
}
