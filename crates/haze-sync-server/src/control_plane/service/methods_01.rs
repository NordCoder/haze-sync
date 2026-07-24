impl ControlPlaneServices {
async fn apply_maintenance_hold(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
        maintenance_generation: i64,
        hold: bool,
        updated_by: &str,
    ) -> Result<Vec<AdapterControlTarget>, ControlPlaneError> {
        let (_, inventory) = read_adapter_inventory_snapshot(transaction)
            .await
            .map_err(ControlPlaneError::from_repository)?;
        let mut targets = Vec::with_capacity(inventory.len());
        for item in inventory {
            let desired = lock_adapter_desired_control(transaction, &item.adapter_id)
                .await
                .map_err(ControlPlaneError::from_repository)?;
            let changed = if desired.maintenance_hold == hold
                && desired.maintenance_generation == maintenance_generation
            {
                desired
            } else {
                cas_update_adapter_desired_control(
                    transaction,
                    &item.adapter_id,
                    desired.adapter_control_generation,
                    &haze_sync_storage::control_plane::AdapterDesiredControlUpdate {
                        desired_enabled: desired.desired_enabled,
                        desired_mode: desired.desired_mode.clone(),
                        maintenance_generation,
                        maintenance_hold: hold,
                        updated_by: updated_by.to_owned(),
                    },
                )
                .await
                .map_err(ControlPlaneError::from_repository)?
            };
            targets.push(AdapterControlTarget {
                adapter_id: item.adapter_id,
                adapter_kind: item.adapter_kind,
                control_authority: item.control_authority,
                desired_enabled: changed.desired_enabled,
                desired_mode: changed.desired_mode,
                adapter_control_generation: changed.adapter_control_generation,
                maintenance_generation: changed.maintenance_generation,
            });
        }
        Ok(targets)
    }

    async fn current_control_targets(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
    ) -> Result<(i64, Vec<AdapterControlTarget>), ControlPlaneError> {
        let (state, inventory) = read_adapter_inventory_snapshot(transaction)
            .await
            .map_err(ControlPlaneError::from_repository)?;
        let mut targets = Vec::with_capacity(inventory.len());
        for item in inventory {
            let desired = lock_adapter_desired_control(transaction, &item.adapter_id)
                .await
                .map_err(ControlPlaneError::from_repository)?;
            targets.push(AdapterControlTarget {
                adapter_id: item.adapter_id,
                adapter_kind: item.adapter_kind,
                control_authority: item.control_authority,
                desired_enabled: desired.desired_enabled,
                desired_mode: desired.desired_mode,
                adapter_control_generation: desired.adapter_control_generation,
                maintenance_generation: desired.maintenance_generation,
            });
        }
        Ok((state.adapter_inventory_generation, targets))
    }

    async fn complete_quiescence(&self, generation: i64) -> Result<(), ControlPlaneError> {
        self.admission
            .wait_for_authoritative_drain(StdDuration::from_secs(QUIESCENCE_TIMEOUT_SECONDS))
            .await
            .map_err(ControlPlaneError::from_admission)?;
        let mut read_tx = self.pool.begin().await.map_err(|_| ControlPlaneError::Internal)?;
        let current = lock_maintenance_control(&mut read_tx)
            .await
            .map_err(ControlPlaneError::from_repository)?;
        if current.maintenance_generation != generation || current.state != "quiescing" {
            return Err(ControlPlaneError::StaleControlGeneration);
        }
        let (inventory_generation, targets) = self.current_control_targets(&mut read_tx).await?;
        read_tx
            .commit()
            .await
            .map_err(|_| ControlPlaneError::Internal)?;
        let collection = match tokio::time::timeout(
            StdDuration::from_secs(QUIESCENCE_TIMEOUT_SECONDS),
            self.adapter_control.collect_quiescence(&targets),
        )
        .await
        {
            Ok(result) => result?,
            Err(_) => return self.record_quiescence_failure(generation, "timeout").await,
        };
        let mut transaction = self.pool.begin().await.map_err(|_| ControlPlaneError::Internal)?;
        let current = lock_maintenance_control(&mut transaction)
            .await
            .map_err(ControlPlaneError::from_repository)?;
        if current.maintenance_generation != generation || current.state != "quiescing" {
            return Err(ControlPlaneError::StaleControlGeneration);
        }
        let fence_closed_at = current
            .admission_fence_closed_at
            .ok_or(ControlPlaneError::InvalidStoredState)?;
        let evidence_id = random_identifier("quiescence_")?;
        let evidence_version = current
            .quiescence_evidence_version
            .checked_add(1)
            .ok_or(ControlPlaneError::InvalidInput)?;
        let captured_at = Utc::now();
        let evidence_binding = quiescence_collection_binding(&collection)?;
        let digest = request_secret_digest(&[
            "haze-sync.quiescence-evidence.v1",
            &generation.to_string(),
            &inventory_generation.to_string(),
            &evidence_version.to_string(),
            &evidence_binding,
        ])?;
        insert_complete_quiescence_evidence(
            &mut transaction,
            &CompleteQuiescenceEvidenceInput {
                quiescence_evidence_id: evidence_id.clone(),
                evidence_version,
                maintenance_generation: generation,
                adapter_inventory_generation: inventory_generation,
                admission_fence_closed_at: fence_closed_at,
                server_instance_id: self.server_instance_id.clone(),
                server_started_at: self.server_started_at,
                active_authoritative_mutations: 0,
                open_authoritative_transactions: 0,
                active_cli_write_jobs: 0,
                object_store_writer_count: 0,
                database_writer_count: 0,
                worktree_external_writer_evidence: collection.worktree_external_writer_evidence,
                evidence_digest: digest,
                captured_at,
                adapter_snapshots: collection.adapter_snapshots,
                runtime_snapshots: collection.runtime_snapshots,
            },
        )
        .await
        .map_err(ControlPlaneError::from_repository)?;
        let completed = cas_complete_maintenance_transition(
            &mut transaction,
            generation,
            "quiescing",
            &MaintenanceControlUpdate {
                state: "quiesced".into(),
                transition_operation_id: current.transition_operation_id.clone(),
                transition_requested_by: current.transition_requested_by.clone(),
                transition_requested_at: current.transition_requested_at,
                state_entered_at: captured_at,
                admission_fence_closed: true,
                admission_fence_closed_at: current.admission_fence_closed_at,
                quiescence_evidence_version: evidence_version,
                quiescence_evidence_id: Some(evidence_id.clone()),
                active_maintenance_job_id: None,
                safe_error_category: None,
            },
        )
        .await
        .map_err(ControlPlaneError::from_repository)?;
        append_operational_audit_event(
            &mut transaction,
            &system_audit_event(
                "maintenance_transition_completed",
                "succeeded",
                "maintenance_control",
                Some("singleton".into()),
                completed.transition_operation_id.clone(),
                Some(generation),
                Some("quiescing".into()),
                Some("quiesced".into()),
                json!({"quiescence_evidence_id": evidence_id}),
            )?,
        )
        .await
        .map_err(ControlPlaneError::from_repository)?;
        transaction
            .commit()
            .await
            .map_err(|_| ControlPlaneError::Internal)?;
        self.admission.set_state(DurableMaintenanceState::Quiesced);
        Ok(())
    }

    async fn record_quiescence_failure(
        &self,
        generation: i64,
        category: &str,
    ) -> Result<(), ControlPlaneError> {
        let mut transaction = self.pool.begin().await.map_err(|_| ControlPlaneError::Internal)?;
        let current = lock_maintenance_control(&mut transaction)
            .await
            .map_err(ControlPlaneError::from_repository)?;
        if current.maintenance_generation != generation || current.state != "quiescing" {
            return Err(ControlPlaneError::StaleControlGeneration);
        }
        cas_complete_maintenance_transition(
            &mut transaction,
            generation,
            "quiescing",
            &MaintenanceControlUpdate {
                state: "quiescing".into(),
                transition_operation_id: current.transition_operation_id.clone(),
                transition_requested_by: current.transition_requested_by.clone(),
                transition_requested_at: current.transition_requested_at,
                state_entered_at: current.state_entered_at,
                admission_fence_closed: true,
                admission_fence_closed_at: current.admission_fence_closed_at,
                quiescence_evidence_version: current.quiescence_evidence_version,
                quiescence_evidence_id: current.quiescence_evidence_id.clone(),
                active_maintenance_job_id: current.active_maintenance_job_id.clone(),
                safe_error_category: Some(category.to_owned()),
            },
        )
        .await
        .map_err(ControlPlaneError::from_repository)?;
        transaction
            .commit()
            .await
            .map_err(|_| ControlPlaneError::Internal)?;
        Err(ControlPlaneError::QuiescenceTimeout)
    }

    async fn complete_resume(&self, generation: i64) -> Result<(), ControlPlaneError> {
        let mut read_tx = self.pool.begin().await.map_err(|_| ControlPlaneError::Internal)?;
        let current = lock_maintenance_control(&mut read_tx)
            .await
            .map_err(ControlPlaneError::from_repository)?;
        if current.maintenance_generation != generation || current.state != "resuming" {
            return Err(ControlPlaneError::StaleControlGeneration);
        }
        let (_, targets) = self.current_control_targets(&mut read_tx).await?;
        read_tx
            .commit()
            .await
            .map_err(|_| ControlPlaneError::Internal)?;
        self.adapter_control.await_resume(&targets).await?;
        let mut transaction = self.pool.begin().await.map_err(|_| ControlPlaneError::Internal)?;
        let current = lock_maintenance_control(&mut transaction)
            .await
            .map_err(ControlPlaneError::from_repository)?;
        if current.maintenance_generation != generation || current.state != "resuming" {
            return Err(ControlPlaneError::StaleControlGeneration);
        }
        let completed_at = Utc::now();
        let completed = cas_complete_maintenance_transition(
            &mut transaction,
            generation,
            "resuming",
            &MaintenanceControlUpdate {
                state: "normal".into(),
                transition_operation_id: current.transition_operation_id.clone(),
                transition_requested_by: current.transition_requested_by.clone(),
                transition_requested_at: current.transition_requested_at,
                state_entered_at: completed_at,
                admission_fence_closed: false,
                admission_fence_closed_at: None,
                quiescence_evidence_version: current.quiescence_evidence_version,
                quiescence_evidence_id: current.quiescence_evidence_id.clone(),
                active_maintenance_job_id: None,
                safe_error_category: None,
            },
        )
        .await
        .map_err(ControlPlaneError::from_repository)?;
        append_operational_audit_event(
            &mut transaction,
            &system_audit_event(
                "maintenance_transition_completed",
                "succeeded",
                "maintenance_control",
                Some("singleton".into()),
                completed.transition_operation_id.clone(),
                Some(generation),
                Some("resuming".into()),
                Some("normal".into()),
                json!({"admission_fence_closed": false}),
            )?,
        )
        .await
        .map_err(ControlPlaneError::from_repository)?;
        transaction
            .commit()
            .await
            .map_err(|_| ControlPlaneError::Internal)?;
        self.admission.set_state(DurableMaintenanceState::Normal);
        Ok(())
    }

    }
