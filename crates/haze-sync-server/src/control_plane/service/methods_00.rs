impl ControlPlaneServices {

    pub(crate) async fn load(pool: PgPool, server_secret: &[u8]) -> Result<Self, ControlPlaneError> {
        Self::load_with_boundaries(
            pool,
            server_secret,
            Arc::new(EmptyInventoryAdapterControl),
            Arc::new(DeterministicFakeExecutor::default()),
        )
        .await
    }

    pub(crate) async fn load_with_boundaries(
        pool: PgPool,
        server_secret: &[u8],
        adapter_control: Arc<dyn AdapterControlEvidence>,
        executor: Arc<dyn OperationalExecutor>,
    ) -> Result<Self, ControlPlaneError> {
        let mut transaction = pool.begin().await.map_err(|_| ControlPlaneError::Internal)?;
        let maintenance = lock_maintenance_control(&mut transaction)
            .await
            .map_err(ControlPlaneError::from_repository)?;
        transaction
            .commit()
            .await
            .map_err(|_| ControlPlaneError::Internal)?;
        let state = DurableMaintenanceState::parse(&maintenance.state)
            .ok_or(ControlPlaneError::InvalidStoredState)?;
        if (state == DurableMaintenanceState::Normal) == maintenance.admission_fence_closed {
            return Err(ControlPlaneError::InvalidStoredState);
        }
        Ok(Self {
            pool,
            admission: AdmissionController::from_state(state),
            secrets: Arc::new(ControlPlaneSecrets::derive_from_server_secret(server_secret)),
            adapter_control,
            executor,
            server_instance_id: random_identifier("server_")?,
            server_started_at: Utc::now(),
        })
    }

    pub(crate) fn admission(&self) -> &AdmissionController {
        &self.admission
    }

    pub(crate) fn maintenance_state(&self) -> DurableMaintenanceState {
        self.admission.state()
    }

    pub(crate) async fn reconcile_startup(&self) -> Result<(), ControlPlaneError> {
        match self.maintenance_state() {
            DurableMaintenanceState::Quiescing => {
                let row = self.maintenance_row().await?;
                let _ = self.complete_quiescence(row.maintenance_generation).await;
            }
            DurableMaintenanceState::Resuming => {
                let row = self.maintenance_row().await?;
                let _ = self.complete_resume(row.maintenance_generation).await;
            }
            DurableMaintenanceState::Normal
            | DurableMaintenanceState::Quiesced
            | DurableMaintenanceState::Maintenance => {}
            DurableMaintenanceState::FailClosed => return Err(ControlPlaneError::InvalidStoredState),
        }
        Ok(())
    }

    pub(crate) async fn maintenance_status(
        &self,
    ) -> Result<MaintenanceStatusResponse, ControlPlaneError> {
        Ok(maintenance_response(self.maintenance_row().await?)?)
    }

    async fn maintenance_row(&self) -> Result<MaintenanceControlRow, ControlPlaneError> {
        let mut transaction = self.pool.begin().await.map_err(|_| ControlPlaneError::Internal)?;
        let row = lock_maintenance_control(&mut transaction)
            .await
            .map_err(ControlPlaneError::from_repository)?;
        transaction
            .commit()
            .await
            .map_err(|_| ControlPlaneError::Internal)?;
        Ok(row)
    }

    pub(crate) async fn request_maintenance_action(
        &self,
        actor: &AuthenticatedIdentity,
        idempotency_key: &str,
        action: MaintenanceActionDto,
        expected_generation: u64,
    ) -> Result<MaintenanceActionResponse, ControlPlaneError> {
        require_admin(actor)?;
        let expected_generation = to_i64(expected_generation)?;
        let stored_key = self
            .secrets
            .control_idempotency_key("maintenance", idempotency_key);
        let fingerprint = request_sha256(&[
            "haze-sync.maintenance-action.v1",
            action.as_str(),
            &expected_generation.to_string(),
        ])?;
        if let Some(mut replay) = read_control_replay::<MaintenanceActionResponse>(
            &self.pool,
            actor.principal.common_adapter_id(),
            &stored_key,
            &fingerprint,
        )
        .await?
        {
            replay.maintenance = self.maintenance_status().await?;
            return Ok(replay);
        }

        let previous_local = self.maintenance_state();
        let preclose = matches!(action, MaintenanceActionDto::Quiesce)
            && previous_local == DurableMaintenanceState::Normal;
        if preclose {
            self.admission.set_state(DurableMaintenanceState::Quiescing);
        }

        let result = self
            .request_maintenance_action_transaction(
                actor,
                &stored_key,
                fingerprint,
                action,
                expected_generation,
            )
            .await;
        let mut response = match result {
            Ok(response) => response,
            Err(error) => {
                if preclose {
                    if let Ok(row) = self.maintenance_row().await {
                        self.admission.set_state(
                            DurableMaintenanceState::parse(&row.state)
                                .unwrap_or(DurableMaintenanceState::FailClosed),
                        );
                    } else {
                        self.admission.set_state(DurableMaintenanceState::FailClosed);
                    }
                }
                return Err(error);
            }
        };

        let durable_state = DurableMaintenanceState::parse(response.maintenance.state.as_str())
            .ok_or(ControlPlaneError::InvalidStoredState)?;
        self.admission.set_state(durable_state);
        if response.accepted {
            match durable_state {
                DurableMaintenanceState::Quiescing => {
                    let _ = self
                        .complete_quiescence(to_i64(response.maintenance.maintenance_generation)?)
                        .await;
                }
                DurableMaintenanceState::Resuming => {
                    let _ = self
                        .complete_resume(to_i64(response.maintenance.maintenance_generation)?)
                        .await;
                }
                _ => {}
            }
            response.maintenance = self.maintenance_status().await?;
        }
        Ok(response)
    }

    async fn request_maintenance_action_transaction(
        &self,
        actor: &AuthenticatedIdentity,
        stored_key: &str,
        fingerprint: CommonSha256,
        action: MaintenanceActionDto,
        expected_generation: i64,
    ) -> Result<MaintenanceActionResponse, ControlPlaneError> {
        let mut transaction = self.pool.begin().await.map_err(|_| ControlPlaneError::Internal)?;
        if let Some(replay) = read_control_replay_on(
            &mut transaction,
            actor.principal.common_adapter_id(),
            stored_key,
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
        let current = lock_maintenance_control(&mut transaction)
            .await
            .map_err(ControlPlaneError::from_repository)?;
        if current.maintenance_generation != expected_generation {
            return Err(ControlPlaneError::StaleControlGeneration);
        }
        let operation_id = random_identifier("maintenance_")?;
        let now = Utc::now();
        let mut accepted = false;
        let next = match (current.state.as_str(), action) {
            ("normal", MaintenanceActionDto::Quiesce) => {
                accepted = true;
                let next_generation = expected_generation
                    .checked_add(1)
                    .ok_or(ControlPlaneError::InvalidInput)?;
                let update = MaintenanceControlUpdate {
                    state: "quiescing".into(),
                    transition_operation_id: Some(operation_id.clone()),
                    transition_requested_by: Some(actor.principal.adapter_id().to_owned()),
                    transition_requested_at: Some(now),
                    state_entered_at: now,
                    admission_fence_closed: true,
                    admission_fence_closed_at: Some(now),
                    quiescence_evidence_version: 0,
                    quiescence_evidence_id: None,
                    active_maintenance_job_id: None,
                    safe_error_category: None,
                };
                let changed = cas_update_maintenance_control(
                    &mut transaction,
                    expected_generation,
                    &update,
                )
                .await
                .map_err(ControlPlaneError::from_repository)?;
                self.apply_maintenance_hold(
                    &mut transaction,
                    next_generation,
                    true,
                    actor.principal.adapter_id(),
                )
                .await?;
                changed
            }
            ("normal", MaintenanceActionDto::Resume)
            | ("quiesced", MaintenanceActionDto::Quiesce)
            | ("maintenance", MaintenanceActionDto::Quiesce)
            | ("maintenance", MaintenanceActionDto::EnterMaintenance) => current.clone(),
            ("quiesced", MaintenanceActionDto::EnterMaintenance) => {
                self.revalidate_quiescence_evidence(&mut transaction, &current)
                    .await?;
                accepted = true;
                cas_update_maintenance_control(
                    &mut transaction,
                    expected_generation,
                    &MaintenanceControlUpdate {
                        state: "maintenance".into(),
                        transition_operation_id: Some(operation_id.clone()),
                        transition_requested_by: Some(actor.principal.adapter_id().to_owned()),
                        transition_requested_at: Some(now),
                        state_entered_at: now,
                        admission_fence_closed: true,
                        admission_fence_closed_at: current.admission_fence_closed_at,
                        quiescence_evidence_version: current.quiescence_evidence_version,
                        quiescence_evidence_id: current.quiescence_evidence_id.clone(),
                        active_maintenance_job_id: current.active_maintenance_job_id.clone(),
                        safe_error_category: None,
                    },
                )
                .await
                .map_err(ControlPlaneError::from_repository)?
            }
            ("quiesced", MaintenanceActionDto::Resume)
            | ("maintenance", MaintenanceActionDto::Resume) => {
                self.require_resume_safe(&mut transaction, &current).await?;
                accepted = true;
                let next_generation = expected_generation
                    .checked_add(1)
                    .ok_or(ControlPlaneError::InvalidInput)?;
                let changed = cas_update_maintenance_control(
                    &mut transaction,
                    expected_generation,
                    &MaintenanceControlUpdate {
                        state: "resuming".into(),
                        transition_operation_id: Some(operation_id.clone()),
                        transition_requested_by: Some(actor.principal.adapter_id().to_owned()),
                        transition_requested_at: Some(now),
                        state_entered_at: now,
                        admission_fence_closed: true,
                        admission_fence_closed_at: current.admission_fence_closed_at,
                        quiescence_evidence_version: current.quiescence_evidence_version,
                        quiescence_evidence_id: current.quiescence_evidence_id.clone(),
                        active_maintenance_job_id: None,
                        safe_error_category: None,
                    },
                )
                .await
                .map_err(ControlPlaneError::from_repository)?;
                self.apply_maintenance_hold(
                    &mut transaction,
                    next_generation,
                    false,
                    actor.principal.adapter_id(),
                )
                .await?;
                changed
            }
            ("quiescing" | "resuming", _) => {
                return Err(ControlPlaneError::ControlTransitionInProgress)
            }
            _ => return Err(ControlPlaneError::InvalidControlTransition),
        };
        let response = MaintenanceActionResponse {
            operation_id: next
                .transition_operation_id
                .clone()
                .unwrap_or(operation_id.clone()),
            action,
            accepted,
            maintenance: maintenance_response(next.clone())?,
        };
        append_operational_audit_event(
            &mut transaction,
            &audit_event(
                if accepted {
                    "maintenance_transition_accepted"
                } else {
                    "maintenance_transition_noop"
                },
                "accepted",
                actor,
                "maintenance_control",
                Some("singleton".into()),
                Some(response.operation_id.clone()),
                Some(next.maintenance_generation),
                None,
                None,
                None,
                Some(current.state),
                Some(next.state),
                json!({"action": action.as_str(), "accepted": accepted}),
            )?,
        )
        .await
        .map_err(ControlPlaneError::from_repository)?;
        if let Some(replay) = store_control_result_on(
            &mut transaction,
            actor.principal.common_adapter_id(),
            stored_key,
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

    }