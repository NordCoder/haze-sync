impl ControlPlaneServices {
pub(crate) async fn rotate_credential(
        &self,
        actor: &AuthenticatedIdentity,
        target_principal_id: &str,
        idempotency_key: &str,
        request: &CredentialRotateRequest,
    ) -> Result<CredentialIssuanceResponse, ControlPlaneError> {
        require_admin(actor)?;
        self.admission
            .admit(AdmissionClass::CredentialCreateOrRotate)
            .map_err(ControlPlaneError::from_admission)?;
        let mut affected = request.affected_credentials.clone();
        affected.sort_by(|left, right| left.credential_id.cmp(&right.credential_id));
        let affected_fingerprint = affected
            .iter()
            .map(|entry| {
                format!(
                    "{}:{}:{}",
                    entry.credential_id,
                    entry.expected_credential_version,
                    entry.current_status.as_str()
                )
            })
            .collect::<Vec<_>>()
            .join(",");
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
        let grace_seconds = request.requested_grace_seconds.unwrap_or(900);
        let fingerprint = request_secret_digest(&[
            "haze-sync.credential-issuance.v1",
            "rotate",
            target_principal_id,
            &request.credential_kind,
            request
                .requested_expires_at
                .as_ref()
                .map(TimestampDto::as_str)
                .unwrap_or("null"),
            &request.expected_principal_version.to_string(),
            &request.expected_credential_set_generation.to_string(),
            &affected_fingerprint,
            &grace_seconds.to_string(),
        ])?;
        let affected_ids = std::iter::once(issued.credential_id.clone())
            .chain(affected.iter().map(|entry| entry.credential_id.clone()))
            .collect::<Vec<_>>();
        let issuance_input = CredentialIssuanceInput {
            requester_principal_id: actor.principal.adapter_id().to_owned(),
            idempotency_key_digest: idempotency_digest,
            request_fingerprint: fingerprint,
            issuance_operation_id: issuance_operation_id.clone(),
            action: "rotate".into(),
            target_principal_id: target_principal_id.to_owned(),
            affected_credential_ids: json!({
                "new_credential_id": issued.credential_id.clone(),
                "credential_ids": affected_ids
            }),
            committed_outcome: "rotated".into(),
            safe_result: json!({
                "new_credential_id": issued.credential_id.clone(),
                "plaintext_available": false,
                "safe_code": "credential_secret_not_replayable"
            }),
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
        let current = read_credentials_by_principal(&mut transaction, target_principal_id)
            .await
            .map_err(ControlPlaneError::from_repository)?;
        verify_rotation_affected_set(&current, &affected)?;
        let active = current
            .iter()
            .find(|credential| credential.status == "active")
            .ok_or(ControlPlaneError::Conflict)?;
        let now = Utc::now();
        let grace_expires_at = if grace_seconds == 0 {
            None
        } else {
            Some(now + Duration::seconds(to_i64(grace_seconds)?))
        };
        let mut mutations = Vec::new();
        for expected in &affected {
            let current = current
                .iter()
                .find(|credential| credential.credential_id == expected.credential_id)
                .ok_or(ControlPlaneError::StaleRecordVersion)?;
            let update = if current.status == "active" && grace_expires_at.is_some() {
                CredentialLifecycleUpdate {
                    status: "grace".into(),
                    grace_expires_at,
                    revoked_at: None,
                    revoked_by: None,
                    expires_at: current.expires_at,
                }
            } else {
                CredentialLifecycleUpdate {
                    status: "revoked".into(),
                    grace_expires_at: None,
                    revoked_at: Some(now),
                    revoked_by: Some(actor.principal.adapter_id().to_owned()),
                    expires_at: current.expires_at,
                }
            };
            mutations.push(CredentialRotationMutation {
                credential_id: expected.credential_id.clone(),
                expected_credential_version: to_i64(expected.expected_credential_version)?,
                expected_status: expected.current_status.as_str().to_owned(),
                update,
            });
        }
        let result = rotate_credential_set(
            &mut transaction,
            &CredentialSetRotationInput {
                expected_principal_version: principal.principal_version,
                expected_credential_set_generation: principal.credential_set_generation,
                new_credential: NewCredential {
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
                    rotated_from_credential_id: Some(active.credential_id.clone()),
                },
                affected_credentials: mutations,
            },
        )
        .await
        .map_err(ControlPlaneError::from_repository)?;
        append_operational_audit_event(
            &mut transaction,
            &OperationalAuditEventInput {
                audit_id: random_identifier("audit_")?,
                occurred_at: now,
                event_type: "credential_rotated".into(),
                outcome: "succeeded".into(),
                actor_principal_id: Some(actor.principal.adapter_id().to_owned()),
                actor_credential_id: actor.credential_id.clone(),
                actor_role: Some(actor.principal.role().to_string()),
                actor_origin: "server_api".into(),
                request_id: None,
                correlation_id: None,
                target_type: "credential_set".into(),
                target_id: Some(target_principal_id.to_owned()),
                operation_id: None,
                maintenance_generation: None,
                adapter_control_generation: None,
                principal_version: Some(result.principal.principal_version),
                credential_set_generation: Some(result.principal.credential_set_generation),
                credential_version: Some(result.new_credential.credential_version),
                job_version: None,
                executor_fence: None,
                runtime_lease_version: None,
                standalone_runtime_epoch: None,
                runtime_report_sequence: None,
                issuance_operation_id: Some(issuance_operation_id.clone()),
                previous_state: Some("active".into()),
                next_state: Some(if grace_seconds == 0 {
                    "revoked".into()
                } else {
                    "grace".into()
                }),
                safe_error_category: None,
                artifact_manifest_id: None,
                safe_metadata: json!({
                    "new_credential_id": result.new_credential.credential_id.clone(),
                    "affected_count": result.affected_credentials.len(),
                    "forced_grace_replacement": result.affected_credentials.iter().any(|row| row.status == "revoked")
                }),
                corrects_audit_id: None,
            },
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
                result.new_credential,
                adapter_role(&result.principal.role)?,
            )?,
            plaintext_available: true,
            plaintext_credential: Some(issued.plaintext),
            safe_code: None,
        })
    }

    async fn credential_replay_response(
        &self,
        row: haze_sync_storage::control_plane::CredentialIssuanceRow,
    ) -> Result<CredentialIssuanceResponse, ControlPlaneError> {
        let credential_id = row
            .safe_result
            .get("credential_id")
            .or_else(|| row.safe_result.get("new_credential_id"))
            .and_then(Value::as_str)
            .or_else(|| {
                row.affected_credential_ids
                    .as_array()
                    .and_then(|values| values.first())
                    .and_then(Value::as_str)
            })
            .ok_or(ControlPlaneError::Internal)?;
        let mut transaction = self.pool.begin().await.map_err(|_| ControlPlaneError::Internal)?;
        let credential = read_credential_by_id(&mut transaction, credential_id)
            .await
            .map_err(ControlPlaneError::from_repository)?
            .ok_or(ControlPlaneError::Internal)?;
        let principal = lock_principal(&mut transaction, &credential.principal_id)
            .await
            .map_err(ControlPlaneError::from_repository)?;
        transaction
            .commit()
            .await
            .map_err(|_| ControlPlaneError::Internal)?;
        Ok(CredentialIssuanceResponse {
            issuance_operation_id: row.issuance_operation_id,
            credential: credential_response(credential, adapter_role(&principal.role)?)?,
            plaintext_available: false,
            plaintext_credential: None,
            safe_code: Some("credential_secret_not_replayable".into()),
        })
    }

    }
