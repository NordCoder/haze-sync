impl ControlPlaneServices {
async fn validate_job_generations(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
        complete: &CompleteOperationalJobRow,
    ) -> Result<(), ControlPlaneError> {
        let maintenance = lock_maintenance_control(transaction)
            .await
            .map_err(ControlPlaneError::from_repository)?;
        if let Some(expected) = complete.job.expected_maintenance_generation {
            if maintenance.maintenance_generation != expected {
                return Err(ControlPlaneError::StaleControlGeneration);
            }
        }
        if complete.job.maintenance_required {
            if maintenance.state != "maintenance" {
                return Err(ControlPlaneError::MaintenanceInProgress);
            }
            self.revalidate_quiescence_evidence(transaction, &maintenance)
                .await?;
        }
        for (adapter_id, expected_generation) in &complete.expected_adapter_generations {
            let desired = lock_adapter_desired_control(transaction, adapter_id)
                .await
                .map_err(ControlPlaneError::from_repository)?;
            if desired.adapter_control_generation != *expected_generation {
                return Err(ControlPlaneError::StaleControlGeneration);
            }
        }
        Ok(())
    }

    async fn finish_operational_job(
        &self,
        actor: &AuthenticatedIdentity,
        running: OperationalJobRow,
        lease_digest: &SecretDigest,
        outcome: ExecutorOutcome,
        slot_id: String,
        slot_version: i64,
    ) -> Result<OperationalJobResponse, ControlPlaneError> {
        let mut transaction = self.pool.begin().await.map_err(|_| ControlPlaneError::Internal)?;
        let current = lock_complete_operational_job(&mut transaction, &running.operation_id)
            .await
            .map_err(ControlPlaneError::from_repository)?;
        if current.job.job_version != running.job_version
            || current.job.executor_fence != running.executor_fence
            || current.job.lease_token_digest.as_ref() != Some(lease_digest)
        {
            return Err(ControlPlaneError::StaleExecutorFence);
        }
        let (state, safe_error_category, summary, checkpoint, uncertain) = match outcome {
            ExecutorOutcome::Succeeded {
                safe_summary,
                checkpoint,
            } => ("succeeded", None, safe_summary, checkpoint, false),
            ExecutorOutcome::Failed {
                safe_error_category,
                checkpoint,
            } => (
                "failed",
                Some(safe_error_category),
                current.job.safe_summary.clone(),
                checkpoint,
                false,
            ),
            ExecutorOutcome::Cancelled { checkpoint } => (
                "cancelled",
                None,
                current.job.safe_summary.clone(),
                checkpoint,
                false,
            ),
            ExecutorOutcome::Uncertain {
                safe_error_category,
                checkpoint,
            } => (
                "failed",
                Some(safe_error_category),
                current.job.safe_summary.clone(),
                checkpoint,
                true,
            ),
        };
        if let Some(checkpoint) = &checkpoint {
            insert_operational_evidence(
                &mut transaction,
                &OperationalEvidenceInput {
                    evidence_id: random_identifier("evidence_")?,
                    operation_id: current.job.operation_id.clone(),
                    job_version: current.job.job_version,
                    evidence_kind: "executor_checkpoint".into(),
                    evidence_schema: "haze-sync.executor-checkpoint.v1".into(),
                    evidence_version: 1,
                    evidence_digest: request_secret_digest(&[
                        "haze-sync.executor-checkpoint.v1",
                        &current.job.operation_id,
                        &current.job.job_version.to_string(),
                        &serde_json::to_string(checkpoint)
                            .map_err(|_| ControlPlaneError::Internal)?,
                    ])?,
                    status: if uncertain {
                        "blocked_uncertain".into()
                    } else if state == "succeeded" {
                        "passed".into()
                    } else {
                        "failed".into()
                    },
                    safe_metadata: checkpoint.clone(),
                    source_environment_id: None,
                    target_environment_id: None,
                    helper_attempt_id: None,
                },
            )
            .await
            .map_err(ControlPlaneError::from_repository)?;
        }
        if uncertain {
            cas_block_execution_slot_uncertain(
                &mut transaction,
                &slot_id,
                slot_version,
                &current.job.operation_id,
                current.job.executor_fence,
            )
            .await
            .map_err(ControlPlaneError::from_repository)?;
        }
        let terminal = cas_update_operational_job(
            &mut transaction,
            &current.job.operation_id,
            current.job.job_version,
            Some(current.job.executor_fence),
            Some(lease_digest),
            &terminal_job_update(
                &current.job,
                state,
                safe_error_category,
                summary,
                checkpoint,
            ),
        )
        .await
        .map_err(ControlPlaneError::from_repository)?;
        append_operational_audit_event(
            &mut transaction,
            &job_audit_event(
                match state {
                    "succeeded" => "operational_job_succeeded",
                    "cancelled" => "operational_job_cancelled",
                    _ if uncertain => "operational_job_failed_uncertain",
                    _ => "operational_job_failed",
                },
                state,
                actor,
                &terminal,
                json!({"uncertain": uncertain}),
            )?,
        )
        .await
        .map_err(ControlPlaneError::from_repository)?;
        if !uncertain {
            cas_release_execution_slot(
                &mut transaction,
                &slot_id,
                slot_version,
                &terminal.operation_id,
                terminal.executor_fence,
            )
            .await
            .map_err(ControlPlaneError::from_repository)?;
        }
        transaction
            .commit()
            .await
            .map_err(|_| ControlPlaneError::Internal)?;
        let mut read = self.pool.begin().await.map_err(|_| ControlPlaneError::Internal)?;
        let complete = haze_sync_storage::control_plane::read_complete_operational_job(
            &mut read,
            &terminal.operation_id,
        )
        .await
        .map_err(ControlPlaneError::from_repository)?;
        read.commit().await.map_err(|_| ControlPlaneError::Internal)?;
        job_response(complete)
    }

    #[allow(dead_code)]
    pub(crate) async fn heartbeat_operational_job(
        &self,
        operation_id: &str,
        expected_job_version: i64,
        executor_fence: i64,
        lease_token: &str,
    ) -> Result<OperationalJobResponse, ControlPlaneError> {
        let proof = self.secrets.lease_digest(operation_id, lease_token)?;
        let mut transaction = self.pool.begin().await.map_err(|_| ControlPlaneError::Internal)?;
        let current = lock_complete_operational_job(&mut transaction, operation_id)
            .await
            .map_err(ControlPlaneError::from_repository)?;
        if current.job.state != "running" {
            return Err(ControlPlaneError::StaleExecutorFence);
        }
        let now = Utc::now();
        cas_update_operational_job(
            &mut transaction,
            operation_id,
            expected_job_version,
            Some(executor_fence),
            Some(&proof),
            &OperationalJobUpdate {
                state: "running".into(),
                safe_summary: current.job.safe_summary.clone(),
                safe_error_category: current.job.safe_error_category.clone(),
                artifact_manifest_id: current.job.artifact_manifest_id.clone(),
                artifact_manifest_digest: current.job.artifact_manifest_digest.clone(),
                checkpoint: current.job.checkpoint.clone(),
                started_at: current.job.started_at,
                completed_at: None,
                cancel_requested_at: current.job.cancel_requested_at,
                confirmation_consumed_at: current.job.confirmation_consumed_at,
                executor_id: current.job.executor_id.clone(),
                executor_fence,
                lease_token_digest: Some(proof),
                lease_heartbeat_at: Some(now),
                lease_expires_at: Some(now + Duration::seconds(JOB_LEASE_SECONDS)),
                destructive_execution_slot_id: current.job.destructive_execution_slot_id.clone(),
            },
        )
        .await
        .map_err(ControlPlaneError::from_repository)?;
        let complete = lock_complete_operational_job(&mut transaction, operation_id)
            .await
            .map_err(ControlPlaneError::from_repository)?;
        transaction
            .commit()
            .await
            .map_err(|_| ControlPlaneError::Internal)?;
        job_response(complete)
    }
}
