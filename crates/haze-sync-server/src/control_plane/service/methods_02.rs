impl ControlPlaneServices {
async fn revalidate_quiescence_evidence(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
        maintenance: &MaintenanceControlRow,
    ) -> Result<(), ControlPlaneError> {
        let evidence_id = maintenance
            .quiescence_evidence_id
            .as_deref()
            .ok_or(ControlPlaneError::InvalidControlTransition)?;
        let evidence = lock_complete_quiescence_evidence(transaction, evidence_id)
            .await
            .map_err(ControlPlaneError::from_repository)?;
        if evidence.evidence.maintenance_generation != maintenance.maintenance_generation
            || evidence.evidence.evidence_version != maintenance.quiescence_evidence_version
            || !evidence.invalidations.is_empty()
        {
            return Err(ControlPlaneError::StaleControlGeneration);
        }
        Ok(())
    }

    async fn require_resume_safe(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
        maintenance: &MaintenanceControlRow,
    ) -> Result<(), ControlPlaneError> {
        if maintenance.active_maintenance_job_id.is_some() {
            return Err(ControlPlaneError::OperationInProgress);
        }
        let slot = lock_execution_slot(transaction, GLOBAL_DESTRUCTIVE_SLOT)
            .await
            .map_err(ControlPlaneError::from_repository)?;
        if slot.operation_id.is_some() || slot.blocked_uncertain {
            return Err(ControlPlaneError::OperationInProgress);
        }
        Ok(())
    }

    pub(crate) async fn principal_status(
        &self,
        actor: &AuthenticatedIdentity,
        principal_id: &str,
    ) -> Result<PrincipalStatusResponse, ControlPlaneError> {
        require_admin(actor)?;
        let mut transaction = self.pool.begin().await.map_err(|_| ControlPlaneError::Internal)?;
        let principal = lock_principal(&mut transaction, principal_id)
            .await
            .map_err(ControlPlaneError::from_repository)?;
        transaction
            .commit()
            .await
            .map_err(|_| ControlPlaneError::Internal)?;
        principal_response(principal)
    }

    pub(crate) async fn mutate_principal(
        &self,
        actor: &AuthenticatedIdentity,
        principal_id: &str,
        idempotency_key: &str,
        request: &PrincipalMutationRequest,
    ) -> Result<PrincipalStatusResponse, ControlPlaneError> {
        require_admin(actor)?;
        let stored_key = self
            .secrets
            .control_idempotency_key("principal-mutation", idempotency_key);
        let fingerprint = request_sha256(&[
            "haze-sync.principal-mutation.v1",
            principal_id,
            &request.expected_principal_version.to_string(),
            &request.expected_credential_set_generation.to_string(),
            &request.role.to_string(),
            if request.principal_enabled { "enabled" } else { "disabled" },
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
        let current = lock_principal(&mut transaction, principal_id)
            .await
            .map_err(ControlPlaneError::from_repository)?;
        let expected_principal_version = to_i64(request.expected_principal_version)?;
        let expected_set_generation = to_i64(request.expected_credential_set_generation)?;
        if current.principal_version != expected_principal_version
            || current.credential_set_generation != expected_set_generation
        {
            return Err(ControlPlaneError::StaleRecordVersion);
        }
        let updated = cas_update_principal(
            &mut transaction,
            principal_id,
            expected_principal_version,
            expected_set_generation,
            &PrincipalUpdate {
                role: request.role.to_string(),
                principal_enabled: request.principal_enabled,
                disabled_at: if request.principal_enabled {
                    None
                } else {
                    Some(Utc::now())
                },
                advance_credential_set_generation: current.role != request.role.to_string()
                    || current.principal_enabled != request.principal_enabled,
            },
        )
        .await
        .map_err(ControlPlaneError::from_repository)?;
        let response = principal_response(updated.clone())?;
        append_operational_audit_event(
            &mut transaction,
            &audit_event(
                "principal_mutated",
                "succeeded",
                actor,
                "principal",
                Some(principal_id.to_owned()),
                None,
                None,
                Some(updated.principal_version),
                Some(updated.credential_set_generation),
                None,
                Some(current.role),
                Some(updated.role.clone()),
                json!({"principal_enabled": updated.principal_enabled}),
            )?,
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

    pub(crate) async fn list_credentials(
        &self,
        actor: &AuthenticatedIdentity,
        principal_id: &str,
    ) -> Result<CredentialListResponse, ControlPlaneError> {
        require_admin(actor)?;
        let mut transaction = self.pool.begin().await.map_err(|_| ControlPlaneError::Internal)?;
        let principal = lock_principal(&mut transaction, principal_id)
            .await
            .map_err(ControlPlaneError::from_repository)?;
        let credentials = read_credentials_by_principal(&mut transaction, principal_id)
            .await
            .map_err(ControlPlaneError::from_repository)?;
        transaction
            .commit()
            .await
            .map_err(|_| ControlPlaneError::Internal)?;
        let role = adapter_role(&principal.role)?;
        Ok(CredentialListResponse {
            principal: principal_response(principal)?,
            credentials: credentials
                .into_iter()
                .map(|credential| credential_response(credential, role))
                .collect::<Result<Vec<_>, _>>()?,
        })
    }

    pub(crate) async fn credential_status(
        &self,
        actor: &AuthenticatedIdentity,
        credential_id: &str,
    ) -> Result<CredentialSummaryResponse, ControlPlaneError> {
        require_admin(actor)?;
        let mut transaction = self.pool.begin().await.map_err(|_| ControlPlaneError::Internal)?;
        let credential = read_credential_by_id(&mut transaction, credential_id)
            .await
            .map_err(ControlPlaneError::from_repository)?
            .ok_or(ControlPlaneError::NotFound)?;
        let principal = lock_principal(&mut transaction, &credential.principal_id)
            .await
            .map_err(ControlPlaneError::from_repository)?;
        transaction
            .commit()
            .await
            .map_err(|_| ControlPlaneError::Internal)?;
        credential_response(credential, adapter_role(&principal.role)?)
    }

    pub(crate) async fn create_credential(
        &self,
        actor: &AuthenticatedIdentity,
        target_principal_id: &str,
        idempotency_key: &str,
        request: &CredentialCreateRequest,
    ) -> Result<CredentialIssuanceResponse, ControlPlaneError> {
        require_admin(actor)?;
        self.admission
            .admit(AdmissionClass::CredentialCreateOrRotate)
            .map_err(ControlPlaneError::from_admission)?;
        let issued = issue_credential()?;
        let issuance_operation_id = random_identifier("issuance_")?;
        let idempotency_digest = self
            .secrets
            .credential_idempotency_digest(idempotency_key)?;
        let expires = request
            .requested_expires_at
            .as_ref()
            .map(parse_timestamp)
            .transpose()?;
        let fingerprint = request_secret_digest(&[
            "haze-sync.credential-issuance.v1",
            "create",
            target_principal_id,
            &request.credential_kind,
            request
                .requested_expires_at
                .as_ref()
                .map(TimestampDto::as_str)
                .unwrap_or("null"),
            &request.expected_principal_version.to_string(),
            &request.expected_credential_set_generation.to_string(),
            "affected_credentials=[]",
            "requested_grace_seconds=null",
        ])?;
        let safe_result = json!({
            "credential_id": issued.credential_id.clone(),
            "plaintext_available": false,
            "safe_code": "credential_secret_not_replayable"
        });
        let issuance_input = CredentialIssuanceInput {
            requester_principal_id: actor.principal.adapter_id().to_owned(),
            idempotency_key_digest: idempotency_digest,
            request_fingerprint: fingerprint,
            issuance_operation_id: issuance_operation_id.clone(),
            action: "create".into(),
            target_principal_id: target_principal_id.to_owned(),
            affected_credential_ids: json!([issued.credential_id.clone()]),
            committed_outcome: "created".into(),
            safe_result,
        };
        let mut transaction = self.pool.begin().await.map_err(|_| ControlPlaneError::Internal)?;
        match reserve_or_replay_credential_issuance(&mut transaction, &issuance_input)
            .await
            .map_err(ControlPlaneError::from_repository)?
        {
            CredentialIssuanceReservationOutcome::Replay(row) => {
                transaction
                    .rollback()
                    .await
                    .map_err(|_| ControlPlaneError::Internal)?;
                return self.credential_replay_response(row).await;
            }
            CredentialIssuanceReservationOutcome::InProgress => {
                return Err(ControlPlaneError::IdempotencyInProgress)
            }
            CredentialIssuanceReservationOutcome::Inserted(_) => {}
        }
        let principal = lock_principal(&mut transaction, target_principal_id)
            .await
            .map_err(ControlPlaneError::from_repository)?;
        if principal.principal_version != to_i64(request.expected_principal_version)?
            || principal.credential_set_generation
                != to_i64(request.expected_credential_set_generation)?
        {
            return Err(ControlPlaneError::StaleRecordVersion);
        }
        if read_credentials_by_principal(&mut transaction, target_principal_id)
            .await
            .map_err(ControlPlaneError::from_repository)?
            .iter()
            .any(|credential| credential.status == "active")
        {
            return Err(ControlPlaneError::Conflict);
        }
        let now = Utc::now();
        let mutation = insert_credential(
            &mut transaction,
            &NewCredential {
                credential_id: issued.credential_id.clone(),
                principal_id: target_principal_id.to_owned(),
                issuance_operation_id: issuance_operation_id.clone(),
                credential_kind: request.credential_kind.clone(),
                status: "active".into(),
                verifier_scheme: "argon2id_v1".into(),
                verifier_material: VerifierMaterial::parse(issued.encoded_verifier.clone())
                    .map_err(ControlPlaneError::from_repository)?,
                created_at: now,
                not_before: now,
                expires_at: expires,
                grace_expires_at: None,
                rotated_from_credential_id: None,
            },
            principal.principal_version,
            principal.credential_set_generation,
        )
        .await
        .map_err(ControlPlaneError::from_repository)?;
        append_operational_audit_event(
            &mut transaction,
            &audit_event_with_issuance(
                "credential_created",
                actor,
                &mutation,
                &issuance_operation_id,
                json!({"credential_kind": request.credential_kind}),
            )?,
        )
        .await
        .map_err(ControlPlaneError::from_repository)?;
        transaction
            .commit()
            .await
            .map_err(|_| ControlPlaneError::Internal)?;
        Ok(CredentialIssuanceResponse {
            issuance_operation_id,
            credential: credential_response(
                mutation.credential,
                adapter_role(&mutation.principal.role)?,
            )?,
            plaintext_available: true,
            plaintext_credential: Some(issued.plaintext),
            safe_code: None,
        })
    }

    }
