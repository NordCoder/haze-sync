impl ControlPlaneServices {
pub(crate) async fn operational_job_status(
        &self,
        actor: &AuthenticatedIdentity,
        operation_id: &str,
    ) -> Result<OperationalJobResponse, ControlPlaneError> {
        require_admin(actor)?;
        let mut transaction = self.pool.begin().await.map_err(|_| ControlPlaneError::Internal)?;
        let row = haze_sync_storage::control_plane::read_complete_operational_job(
            &mut transaction,
            operation_id,
        )
        .await
        .map_err(ControlPlaneError::from_repository)?;
        transaction
            .commit()
            .await
            .map_err(|_| ControlPlaneError::Internal)?;
        job_response(row)
    }

    pub(crate) async fn confirm_operational_job(
        &self,
        actor: &AuthenticatedIdentity,
        operation_id: &str,
        request: &OperationalJobConfirmationRequest,
    ) -> Result<OperationalJobResponse, ControlPlaneError> {
        require_admin(actor)?;
        let mut transaction = self.pool.begin().await.map_err(|_| ControlPlaneError::Internal)?;
        let complete = lock_complete_operational_job(&mut transaction, operation_id)
            .await
            .map_err(ControlPlaneError::from_repository)?;
        if complete.job.job_version != to_i64(request.expected_job_version)? {
            return Err(ControlPlaneError::StaleRecordVersion);
        }
        if complete.job.state != "awaiting_confirmation" {
            return Err(ControlPlaneError::ConfirmationRequired);
        }
        let expires_at = complete
            .job
            .confirmation_expires_at
            .ok_or(ControlPlaneError::ConfirmationRequired)?;
        if Utc::now() >= expires_at {
            let cancelled = terminal_job_update(
                &complete.job,
                "cancelled",
                Some("confirmation_expired".into()),
                complete.job.safe_summary.clone(),
                complete.job.checkpoint.clone(),
            );
            let row = cas_update_operational_job(
                &mut transaction,
                operation_id,
                complete.job.job_version,
                None,
                None,
                &cancelled,
            )
            .await
            .map_err(ControlPlaneError::from_repository)?;
            append_operational_audit_event(
                &mut transaction,
                &job_audit_event(
                    "operational_job_confirmation_expired",
                    "cancelled",
                    actor,
                    &row,
                    json!({}),
                )?,
            )
            .await
            .map_err(ControlPlaneError::from_repository)?;
            transaction
                .commit()
                .await
                .map_err(|_| ControlPlaneError::Internal)?;
            return Err(ControlPlaneError::ConfirmationExpired);
        }
        let expected = complete
            .job
            .confirmation_digest
            .as_ref()
            .ok_or(ControlPlaneError::ConfirmationRequired)?;
        let actual = self.secrets.confirmation_digest(
            &confirmation_binding(&complete)?,
            &request.confirmation,
        )?;
        if !constant_time_digest_eq(&actual, expected) {
            return Err(ControlPlaneError::ConfirmationRequired);
        }
        transaction
            .rollback()
            .await
            .map_err(|_| ControlPlaneError::Internal)?;
        self.start_operational_job(actor, operation_id, to_i64(request.expected_job_version)?, Some(Utc::now()))
            .await
    }

    pub(crate) async fn cancel_operational_job(
        &self,
        actor: &AuthenticatedIdentity,
        operation_id: &str,
        request: &OperationalJobCancelRequest,
    ) -> Result<OperationalJobResponse, ControlPlaneError> {
        require_admin(actor)?;
        let mut transaction = self.pool.begin().await.map_err(|_| ControlPlaneError::Internal)?;
        let complete = lock_complete_operational_job(&mut transaction, operation_id)
            .await
            .map_err(ControlPlaneError::from_repository)?;
        if complete.job.job_version != to_i64(request.expected_job_version)? {
            return Err(ControlPlaneError::StaleRecordVersion);
        }
        let now = Utc::now();
        let update = match complete.job.state.as_str() {
            "planned" | "awaiting_confirmation" => terminal_job_update(
                &complete.job,
                "cancelled",
                None,
                complete.job.safe_summary.clone(),
                complete.job.checkpoint.clone(),
            ),
            "running" => OperationalJobUpdate {
                state: "running".into(),
                safe_summary: complete.job.safe_summary.clone(),
                safe_error_category: complete.job.safe_error_category.clone(),
                artifact_manifest_id: complete.job.artifact_manifest_id.clone(),
                artifact_manifest_digest: complete.job.artifact_manifest_digest.clone(),
                checkpoint: complete.job.checkpoint.clone(),
                started_at: complete.job.started_at,
                completed_at: None,
                cancel_requested_at: Some(now),
                confirmation_consumed_at: complete.job.confirmation_consumed_at,
                executor_id: complete.job.executor_id.clone(),
                executor_fence: complete.job.executor_fence,
                lease_token_digest: complete.job.lease_token_digest.clone(),
                lease_heartbeat_at: complete.job.lease_heartbeat_at,
                lease_expires_at: complete.job.lease_expires_at,
                destructive_execution_slot_id: complete.job.destructive_execution_slot_id.clone(),
            },
            _ => return Err(ControlPlaneError::OperationNotCancellable),
        };
        let updated = cas_update_operational_job(
            &mut transaction,
            operation_id,
            complete.job.job_version,
            if complete.job.state == "running" {
                Some(complete.job.executor_fence)
            } else {
                None
            },
            complete.job.lease_token_digest.as_ref(),
            &update,
        )
        .await
        .map_err(ControlPlaneError::from_repository)?;
        append_operational_audit_event(
            &mut transaction,
            &job_audit_event(
                "operational_job_cancel_requested",
                "accepted",
                actor,
                &updated,
                json!({"terminal": updated.state == "cancelled"}),
            )?,
        )
        .await
        .map_err(ControlPlaneError::from_repository)?;
        transaction
            .commit()
            .await
            .map_err(|_| ControlPlaneError::Internal)?;
        let mut read = self.pool.begin().await.map_err(|_| ControlPlaneError::Internal)?;
        let complete = lock_complete_operational_job(&mut read, operation_id)
            .await
            .map_err(ControlPlaneError::from_repository)?;
        read.commit().await.map_err(|_| ControlPlaneError::Internal)?;
        job_response(complete)
    }

    async fn start_operational_job(
        &self,
        actor: &AuthenticatedIdentity,
        operation_id: &str,
        expected_job_version: i64,
        confirmation_consumed_at: Option<DateTime<Utc>>,
    ) -> Result<OperationalJobResponse, ControlPlaneError> {
        let lease_token = random_secret("lease_")?;
        let lease_digest = self.secrets.lease_digest(operation_id, &lease_token)?;
        let executor_id = random_identifier("executor_")?;
        let now = Utc::now();
        let mut transaction = self.pool.begin().await.map_err(|_| ControlPlaneError::Internal)?;
        let complete = lock_complete_operational_job(&mut transaction, operation_id)
            .await
            .map_err(ControlPlaneError::from_repository)?;
        if complete.job.job_version != expected_job_version {
            return Err(ControlPlaneError::StaleRecordVersion);
        }
        if !matches!(complete.job.state.as_str(), "planned" | "awaiting_confirmation") {
            return Err(ControlPlaneError::OperationInProgress);
        }
        self.validate_job_generations(&mut transaction, &complete).await?;
        let executor_fence = complete
            .job
            .executor_fence
            .checked_add(1)
            .ok_or(ControlPlaneError::InvalidInput)?;
        let slot = if complete.job.destructive || complete.job.maintenance_required {
            lock_execution_slot(&mut transaction, GLOBAL_DESTRUCTIVE_SLOT)
                .await
                .map_err(ControlPlaneError::from_repository)?
        } else {
            let slot_id = format!(
                "scope-{}",
                &complete.job.execution_scope_digest.as_str()[..32]
            );
            ensure_scoped_execution_slot(
                &mut transaction,
                &slot_id,
                &complete.job.execution_scope_digest,
            )
            .await
            .map_err(ControlPlaneError::from_repository)?
        };
        let acquired = cas_acquire_execution_slot(
            &mut transaction,
            &slot.slot_id,
            slot.slot_version,
            operation_id,
            complete.job.expected_maintenance_generation,
            executor_fence,
        )
        .await
        .map_err(ControlPlaneError::from_repository)?;
        let running = cas_update_operational_job(
            &mut transaction,
            operation_id,
            complete.job.job_version,
            None,
            None,
            &OperationalJobUpdate {
                state: "running".into(),
                safe_summary: complete.job.safe_summary.clone(),
                safe_error_category: None,
                artifact_manifest_id: complete.job.artifact_manifest_id.clone(),
                artifact_manifest_digest: complete.job.artifact_manifest_digest.clone(),
                checkpoint: complete.job.checkpoint.clone(),
                started_at: Some(now),
                completed_at: None,
                cancel_requested_at: None,
                confirmation_consumed_at,
                executor_id: Some(executor_id.clone()),
                executor_fence,
                lease_token_digest: Some(lease_digest.clone()),
                lease_heartbeat_at: Some(now),
                lease_expires_at: Some(now + Duration::seconds(JOB_LEASE_SECONDS)),
                destructive_execution_slot_id: Some(acquired.slot_id.clone()),
            },
        )
        .await
        .map_err(ControlPlaneError::from_repository)?;
        append_operational_audit_event(
            &mut transaction,
            &job_audit_event(
                "operational_job_started",
                "succeeded",
                actor,
                &running,
                json!({"executor_id": executor_id}),
            )?,
        )
        .await
        .map_err(ControlPlaneError::from_repository)?;
        transaction
            .commit()
            .await
            .map_err(|_| ControlPlaneError::Internal)?;

        let outcome = self
            .executor
            .execute(ExecutorCommand {
                operation_id: operation_id.to_owned(),
                kind: running.kind.clone(),
                dry_run: running.dry_run,
                executor_fence,
                safe_summary: running.safe_summary.clone(),
            })
            .await
            .unwrap_or_else(|error| ExecutorOutcome::Failed {
                safe_error_category: error.safe_code().into(),
                checkpoint: None,
            });
        self.finish_operational_job(
            actor,
            running,
            &lease_digest,
            outcome,
            acquired.slot_id,
            acquired.slot_version,
        )
        .await
    }

}
