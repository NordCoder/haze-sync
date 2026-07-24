impl ControlPlaneServices {
pub(crate) async fn revoke_credential(
        &self,
        actor: &AuthenticatedIdentity,
        credential_id: &str,
        idempotency_key: &str,
        request: &CredentialRevokeRequest,
    ) -> Result<CredentialSummaryResponse, ControlPlaneError> {
        require_admin(actor)?;
        self.admission
            .admit(AdmissionClass::CredentialRevoke)
            .map_err(ControlPlaneError::from_admission)?;
        let stored_key = self
            .secrets
            .control_idempotency_key("credential-revoke", idempotency_key);
        let fingerprint = request_sha256(&[
            "haze-sync.credential-revoke.v1",
            credential_id,
            &request.expected_principal_version.to_string(),
            &request.expected_credential_set_generation.to_string(),
            &request.expected_credential_version.to_string(),
        ])?;
        if let Some(replay) = read_control_replay(
            &self.pool,
            actor.principal.common_adapter_id(),
            &stored_key,
            &fingerprint,
        )
        .await?
        {
            return Ok(replay);
        }
        let mut transaction = self.pool.begin().await.map_err(|_| ControlPlaneError::Internal)?;
        if let Some(replay) = read_control_replay_on(
            &mut transaction,
            actor.principal.common_adapter_id(),
            &stored_key,
            &fingerprint,
        )
        .await?
        {
            transaction
                .rollback()
                .await
                .map_err(|_| ControlPlaneError::Internal)?;
            return serde_json::from_value(replay).map_err(|_| ControlPlaneError::Internal);
        }
        let observed = read_credential_by_id(&mut transaction, credential_id)
            .await
            .map_err(ControlPlaneError::from_repository)?
            .ok_or(ControlPlaneError::NotFound)?;
        let principal = lock_principal(&mut transaction, &observed.principal_id)
            .await
            .map_err(ControlPlaneError::from_repository)?;
        let current = lock_credential(&mut transaction, credential_id)
            .await
            .map_err(ControlPlaneError::from_repository)?;
        if principal.principal_version != to_i64(request.expected_principal_version)?
            || principal.credential_set_generation
                != to_i64(request.expected_credential_set_generation)?
            || current.credential_version != to_i64(request.expected_credential_version)?
        {
            return Err(ControlPlaneError::StaleRecordVersion);
        }
        let previous_status = current.status.clone();
        let (principal, credential) = if matches!(current.status.as_str(), "revoked" | "expired") {
            (principal, current)
        } else {
            let mutation = cas_update_credential_lifecycle(
                &mut transaction,
                credential_id,
                current.credential_version,
                principal.principal_version,
                principal.credential_set_generation,
                &CredentialLifecycleUpdate {
                    status: "revoked".into(),
                    grace_expires_at: None,
                    revoked_at: Some(Utc::now()),
                    revoked_by: Some(actor.principal.adapter_id().to_owned()),
                    expires_at: current.expires_at,
                },
            )
            .await
            .map_err(ControlPlaneError::from_repository)?;
            (mutation.principal, mutation.credential)
        };
        let response = credential_response(credential.clone(), adapter_role(&principal.role)?)?;
        append_operational_audit_event(
            &mut transaction,
            &OperationalAuditEventInput {
                audit_id: random_identifier("audit_")?,
                occurred_at: Utc::now(),
                event_type: "credential_revoked".into(),
                outcome: "succeeded".into(),
                actor_principal_id: Some(actor.principal.adapter_id().to_owned()),
                actor_credential_id: actor.credential_id.clone(),
                actor_role: Some(actor.principal.role().to_string()),
                actor_origin: "server_api".into(),
                request_id: None,
                correlation_id: None,
                target_type: "credential".into(),
                target_id: Some(credential_id.to_owned()),
                operation_id: None,
                maintenance_generation: None,
                adapter_control_generation: None,
                principal_version: Some(principal.principal_version),
                credential_set_generation: Some(principal.credential_set_generation),
                credential_version: Some(credential.credential_version),
                job_version: None,
                executor_fence: None,
                runtime_lease_version: None,
                standalone_runtime_epoch: None,
                runtime_report_sequence: None,
                issuance_operation_id: credential.issuance_operation_id.clone(),
                previous_state: Some(previous_status),
                next_state: Some(credential.status.clone()),
                safe_error_category: None,
                artifact_manifest_id: None,
                safe_metadata: json!({}),
                corrects_audit_id: None,
            },
        )
        .await
        .map_err(ControlPlaneError::from_repository)?;
        if let Some(replay) = store_control_result_on(
            &mut transaction,
            actor.principal.common_adapter_id(),
            &stored_key,
            fingerprint,
            &response,
        )
        .await?
        {
            transaction
                .rollback()
                .await
                .map_err(|_| ControlPlaneError::Internal)?;
            return serde_json::from_value(replay).map_err(|_| ControlPlaneError::Internal);
        }
        transaction
            .commit()
            .await
            .map_err(|_| ControlPlaneError::Internal)?;
        Ok(response)
    }

    pub(crate) async fn authenticate_bearer(
        &self,
        token: &BearerToken,
    ) -> Result<AuthenticatedIdentity, ControlPlaneError> {
        let raw = token.as_sensitive_str();
        let mut transaction = self.pool.begin().await.map_err(|_| ControlPlaneError::Internal)?;
        let credential = if raw.starts_with("hs1.") {
            let mut parts = raw.split('.');
            if parts.next() != Some("hs1") {
                return Err(ControlPlaneError::InvalidAuthentication);
            }
            let credential_id = parts.next().ok_or(ControlPlaneError::InvalidAuthentication)?;
            let secret = parts.next().ok_or(ControlPlaneError::InvalidAuthentication)?;
            if parts.next().is_some() || credential_id.is_empty() || secret.is_empty() {
                return Err(ControlPlaneError::InvalidAuthentication);
            }
            let credential = read_credential_by_id(&mut transaction, credential_id)
                .await
                .map_err(ControlPlaneError::from_repository)?
                .ok_or(ControlPlaneError::InvalidAuthentication)?;
            if credential.verifier_scheme != "argon2id_v1"
                || !verify_argon2id_secret(
                    secret,
                    credential_verifier_material_for_private_authentication(&credential),
                )
            {
                return Err(ControlPlaneError::InvalidAuthentication);
            }
            credential
        } else {
            let digest = legacy_sha256_digest(raw)?;
            read_legacy_credential_by_sha256(&mut transaction, &digest)
                .await
                .map_err(ControlPlaneError::from_repository)?
                .ok_or(ControlPlaneError::InvalidAuthentication)?
        };
        let principal = lock_principal(&mut transaction, &credential.principal_id)
            .await
            .map_err(ControlPlaneError::from_repository)?;
        let now = Utc::now();
        if !principal.principal_enabled
            || now < credential.not_before
            || credential.expires_at.is_some_and(|expires| now >= expires)
            || match credential.status.as_str() {
                "active" => false,
                "grace" => credential.grace_expires_at.is_none_or(|expires| now >= expires),
                _ => true,
            }
        {
            return Err(ControlPlaneError::InvalidAuthentication);
        }
        let role = adapter_role(&principal.role)?;
        let principal_value = AdapterPrincipal::new(principal.principal_id.clone(), role)
            .map_err(|_| ControlPlaneError::InvalidAuthentication)?;
        transaction
            .commit()
            .await
            .map_err(|_| ControlPlaneError::Internal)?;
        Ok(AuthenticatedIdentity {
            principal: principal_value,
            credential_id: Some(credential.credential_id),
        })
    }

    pub(crate) async fn create_operational_job(
        &self,
        actor: &AuthenticatedIdentity,
        idempotency_key: &str,
        request: &OperationalJobCreateRequest,
    ) -> Result<OperationalJobSubmissionResponse, ControlPlaneError> {
        require_admin(actor)?;
        self.admission
            .admit(AdmissionClass::OperationalPlan)
            .map_err(ControlPlaneError::from_admission)?;
        let contract = JobContract::for_request(request)?;
        let operation_id = random_identifier("job_")?;
        let idempotency_digest = self
            .secrets
            .operational_idempotency_digest(idempotency_key)?;
        let expected_adapter_binding = request
            .expected_adapter_control_generations
            .iter()
            .map(|entry| format!("{}:{}", entry.adapter_id, entry.adapter_control_generation))
            .collect::<Vec<_>>()
            .join(",");
        let request_fingerprint = request_secret_digest(&[
            "haze-sync.operational-job-request.v1",
            request.kind.as_str(),
            &request.target_scope,
            if request.dry_run { "dry_run" } else { "execute" },
            if contract.confirmation_required {
                "confirmation_required"
            } else {
                "confirmation_not_required"
            },
            &request
                .expected_maintenance_generation
                .map(|value| value.to_string())
                .unwrap_or_else(|| "null".into()),
            &expected_adapter_binding,
            request.retry_of_operation_id.as_deref().unwrap_or("null"),
        ])?;
        let execution_scope_digest = request_secret_digest(&[
            "haze-sync.execution-scope.v1",
            request.kind.as_str(),
            &request.target_scope,
        ])?;
        let artifact_manifest_id = if contract.confirmation_required {
            Some(random_identifier("manifest_")?)
        } else {
            None
        };
        let artifact_manifest_digest = artifact_manifest_id
            .as_ref()
            .map(|manifest_id| {
                request_secret_digest(&[
                    "haze-sync.operational-manifest.v1",
                    manifest_id,
                    request.kind.as_str(),
                    &request.target_scope,
                    &request_fingerprint.as_str(),
                ])
            })
            .transpose()?;
        let confirmation = if contract.confirmation_required {
            Some(random_secret("confirm_")?)
        } else {
            None
        };
        let confirmation_expires_at = confirmation
            .as_ref()
            .map(|_| Utc::now() + Duration::seconds(JOB_CONFIRMATION_SECONDS));
        let confirmation_digest = confirmation
            .as_ref()
            .map(|value| {
                self.secrets.confirmation_digest(
                    &confirmation_binding_from_parts(
                        &operation_id,
                        request.kind.as_str(),
                        request_fingerprint.as_str(),
                        request.expected_maintenance_generation,
                        &request.expected_adapter_control_generations,
                        2,
                        artifact_manifest_digest.as_ref(),
                        confirmation_expires_at,
                    ),
                    value,
                )
            })
            .transpose()?;
        let safe_summary = json!({
            "schema": OPERATIONAL_JOB_SCHEMA_V1,
            "kind": request.kind.as_str(),
            "target_scope": request.target_scope,
            "dry_run": request.dry_run,
            "writes": if request.dry_run { "none" } else { "bounded_fake_executor_only" }
        });
        let input = OperationalJobInput {
            operation_id: operation_id.clone(),
            kind: request.kind.as_str().into(),
            maintenance_required: contract.maintenance_required,
            destructive: contract.destructive,
            requester_principal_id: actor.principal.adapter_id().to_owned(),
            requester_credential_id: actor.credential_id.clone(),
            idempotency_scope: "operational_job".into(),
            idempotency_key_digest: idempotency_digest,
            request_fingerprint,
            state: "planned".into(),
            dry_run: request.dry_run,
            confirmation_required: contract.confirmation_required,
            confirmation_digest,
            confirmation_expires_at,
            expected_maintenance_generation: request
                .expected_maintenance_generation
                .map(to_i64)
                .transpose()?,
            safe_summary,
            artifact_manifest_id: artifact_manifest_id.clone(),
            artifact_manifest_digest,
            checkpoint: None,
            execution_scope_digest,
            expected_adapter_generations: request
                .expected_adapter_control_generations
                .iter()
                .map(|entry| Ok((entry.adapter_id.clone(), to_i64(entry.adapter_control_generation)?)))
                .collect::<Result<Vec<_>, ControlPlaneError>>()?,
            retry_of_operation_id: request.retry_of_operation_id.clone(),
        };
        let mut transaction = self.pool.begin().await.map_err(|_| ControlPlaneError::Internal)?;
        let inserted = match insert_or_replay_complete_operational_job(&mut transaction, &input)
            .await
            .map_err(ControlPlaneError::from_repository)?
        {
            IdempotencyInsertOutcome::Replay(existing) => {
                transaction
                    .rollback()
                    .await
                    .map_err(|_| ControlPlaneError::Internal)?;
                return Ok(OperationalJobSubmissionResponse {
                    job: job_response(existing)?,
                    confirmation: None,
                });
            }
            IdempotencyInsertOutcome::Inserted(inserted) => inserted,
        };
        append_operational_audit_event(
            &mut transaction,
            &job_audit_event("operational_job_created", "succeeded", actor, &inserted.job, json!({}))?,
        )
        .await
        .map_err(ControlPlaneError::from_repository)?;
        let prepared = if contract.confirmation_required {
            let updated = cas_update_operational_job(
                &mut transaction,
                &operation_id,
                inserted.job.job_version,
                None,
                None,
                &OperationalJobUpdate {
                    state: "awaiting_confirmation".into(),
                    safe_summary: inserted.job.safe_summary.clone(),
                    safe_error_category: None,
                    artifact_manifest_id: artifact_manifest_id.clone(),
                    artifact_manifest_digest: input.artifact_manifest_digest.clone(),
                    checkpoint: None,
                    started_at: None,
                    completed_at: None,
                    cancel_requested_at: None,
                    confirmation_consumed_at: None,
                    executor_id: None,
                    executor_fence: 0,
                    lease_token_digest: None,
                    lease_heartbeat_at: None,
                    lease_expires_at: None,
                    destructive_execution_slot_id: None,
                },
            )
            .await
            .map_err(ControlPlaneError::from_repository)?;
            append_operational_audit_event(
                &mut transaction,
                &job_audit_event(
                    "operational_job_confirmation_issued",
                    "succeeded",
                    actor,
                    &updated,
                    json!({"confirmation_expires_at": confirmation_expires_at}),
                )?,
            )
            .await
            .map_err(ControlPlaneError::from_repository)?;
            lock_complete_operational_job(&mut transaction, &operation_id)
                .await
                .map_err(ControlPlaneError::from_repository)?
        } else {
            inserted
        };
        transaction
            .commit()
            .await
            .map_err(|_| ControlPlaneError::Internal)?;
        if contract.confirmation_required {
            return Ok(OperationalJobSubmissionResponse {
                job: job_response(prepared)?,
                confirmation,
            });
        }
        let job = self
            .start_operational_job(actor, &operation_id, prepared.job.job_version, None)
            .await?;
        Ok(OperationalJobSubmissionResponse {
            job,
            confirmation: None,
        })
    }
}
