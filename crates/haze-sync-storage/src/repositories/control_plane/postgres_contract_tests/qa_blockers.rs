fn effective_input(adapter_id: &str, now: DateTime<Utc>) -> AdapterEffectiveControlInput {
    AdapterEffectiveControlInput {
        adapter_id: adapter_id.into(),
        effective_mode: Some("bidirectional".into()),
        runtime_lifecycle: "idle".into(),
        last_applied_adapter_control_generation: Some(0),
        last_applied_maintenance_generation: Some(0),
        heartbeat_at: Some(now),
        last_success_at: None,
        last_cycle_started_at: None,
        last_cycle_finished_at: None,
        in_flight: false,
        connection_state: "connected".into(),
        safe_error_category: None,
        safe_error_code: None,
        runtime_instance_id: None,
        standalone_runtime_epoch: None,
        report_sequence: None,
        runtime_lease_version: None,
        runtime_lease_expires_at: None,
        checkpoint_summary: serde_json::json!({"core_seq": 0}),
        reported_at: now,
    }
}

fn runtime_report(
    adapter_id: &str,
    runtime_instance_id: &str,
    epoch: i64,
    lease_version: i64,
    sequence: i64,
    fingerprint: char,
    lease_digest: &SecretDigest,
    adapter_generation: i64,
    maintenance_generation: i64,
    now: DateTime<Utc>,
) -> RuntimeReportInput {
    RuntimeReportInput {
        adapter_id: adapter_id.into(),
        runtime_instance_id: runtime_instance_id.into(),
        standalone_runtime_epoch: epoch,
        expected_runtime_lease_version: lease_version,
        report_sequence: sequence,
        report_fingerprint: SecretDigest::parse(fingerprint.to_string().repeat(64)).unwrap(),
        lease_proof_digest: lease_digest.clone(),
        adapter_control_generation: adapter_generation,
        maintenance_generation,
        effective_mode: Some("bidirectional".into()),
        runtime_lifecycle: "idle".into(),
        in_flight: false,
        connection_state: "connected".into(),
        checkpoint_summary: serde_json::json!({"sequence": sequence}),
        safe_error_category: None,
        safe_error_code: None,
        reported_at: now,
        close_permit_ids: Vec::new(),
    }
}

async fn runtime_state(
    transaction: &mut Transaction<'_, Postgres>,
    adapter_id: &str,
) -> (
    RuntimeAuthorityRow,
    AdapterEffectiveControlRow,
    i64,
    Vec<(String, String)>,
) {
    let authority = lock_runtime_authority(transaction, adapter_id).await.unwrap();
    let effective = lock_adapter_effective_control(transaction, adapter_id)
        .await
        .unwrap();
    let reports = sqlx::query_scalar::<_, i64>(
        "select count(*)::bigint from gdrive_runtime_reports where adapter_id = $1",
    )
    .bind(adapter_id)
    .fetch_one(&mut **transaction)
    .await
    .unwrap();
    let permits = sqlx::query_as::<_, (String, String)>(
        "select permit_id, state from gdrive_mutation_permits \
         where adapter_id = $1 order by permit_id",
    )
    .bind(adapter_id)
    .fetch_all(&mut **transaction)
    .await
    .unwrap();
    (authority, effective, reports, permits)
}

fn principal_update(role: &str) -> PrincipalUpdate {
    PrincipalUpdate {
        role: role.into(),
        principal_enabled: true,
        disabled_at: None,
        advance_credential_set_generation: false,
    }
}

fn credential_input(principal_id: &str, now: DateTime<Utc>) -> NewCredential {
    NewCredential {
        credential_id: "credential-cas-a".into(),
        principal_id: principal_id.into(),
        issuance_operation_id: "issuance-cas-a".into(),
        credential_kind: "bearer".into(),
        status: "active".into(),
        verifier_scheme: "argon2id_v1".into(),
        verifier_material: VerifierMaterial::parse(
            "$argon2id$v=19$m=65536,t=3,p=1$c2FsdA$dmVyaWZpZXI",
        )
        .unwrap(),
        created_at: now,
        not_before: now,
        expires_at: None,
        grace_expires_at: None,
        rotated_from_credential_id: None,
    }
}

fn operational_job_input(principal_id: &str) -> OperationalJobInput {
    OperationalJobInput {
        operation_id: "job-cas-a".into(),
        kind: "backup".into(),
        maintenance_required: false,
        destructive: false,
        requester_principal_id: principal_id.into(),
        requester_credential_id: Some("credential-cas-a".into()),
        idempotency_scope: "operational_job".into(),
        idempotency_key_digest: SecretDigest::parse("4".repeat(64)).unwrap(),
        request_fingerprint: SecretDigest::parse("5".repeat(64)).unwrap(),
        state: "planned".into(),
        dry_run: false,
        confirmation_required: false,
        confirmation_digest: None,
        confirmation_expires_at: None,
        expected_maintenance_generation: Some(0),
        safe_summary: serde_json::json!({"status": "planned"}),
        artifact_manifest_id: None,
        artifact_manifest_digest: None,
        checkpoint: None,
        execution_scope_digest: SecretDigest::parse("6".repeat(64)).unwrap(),
        expected_adapter_generations: Vec::new(),
        retry_of_operation_id: None,
    }
}

fn operational_job_update(
    state: &str,
    executor_fence: i64,
    lease_digest: Option<SecretDigest>,
    now: DateTime<Utc>,
    checkpoint: i64,
) -> OperationalJobUpdate {
    OperationalJobUpdate {
        state: state.into(),
        safe_summary: serde_json::json!({"status": state}),
        safe_error_category: None,
        artifact_manifest_id: None,
        artifact_manifest_digest: None,
        checkpoint: Some(serde_json::json!({"checkpoint": checkpoint})),
        started_at: Some(now),
        completed_at: terminal_job_state(state).then_some(now),
        cancel_requested_at: None,
        confirmation_consumed_at: None,
        executor_id: Some("executor-a".into()),
        executor_fence,
        lease_token_digest: lease_digest,
        lease_heartbeat_at: Some(now),
        lease_expires_at: Some(now + chrono::Duration::minutes(5)),
        destructive_execution_slot_id: None,
    }
}

#[test]
fn generic_effective_writer_is_not_publicly_exported() {
    let boundary = include_str!("../repository.rs");
    assert!(!boundary.contains(
        "pub use effective_records::{lock_adapter_effective_control, upsert_adapter_effective_control}"
    ));
    assert!(!boundary.contains("pub use effective_records::upsert_adapter_effective_control"));
    assert!(boundary.contains("pub async fn upsert_hosted_adapter_effective_control"));
    assert!(boundary.contains("pub async fn accept_ordered_runtime_report"));
}

#[tokio::test]
async fn hosted_effective_writer_cannot_bypass_standalone_runtime_authority() {
    let context = connect_required_test_database_from_env().await.unwrap();
    let mut transaction = begin_isolated_schema(context.pool()).await;
    apply_all(&mut transaction).await;
    let now = Utc::now();

    cas_replace_adapter_inventory(
        &mut transaction,
        0,
        &[
            AdapterInventoryItem {
                adapter_id: "worktree-hosted".into(),
                adapter_kind: "worktree".into(),
                control_authority: "server".into(),
                configured_at: now,
                retired_at: None,
            },
            AdapterInventoryItem {
                adapter_id: "gdrive-standalone".into(),
                adapter_kind: "gdrive".into(),
                control_authority: "standalone".into(),
                configured_at: now,
                retired_at: None,
            },
        ],
    )
    .await
    .unwrap();
    for adapter_id in ["worktree-hosted", "gdrive-standalone"] {
        initialize_adapter_desired_control(
            &mut transaction,
            adapter_id,
            &AdapterDesiredControlUpdate {
                desired_enabled: true,
                desired_mode: "bidirectional".into(),
                maintenance_generation: 0,
                maintenance_hold: false,
                updated_by: "admin-a".into(),
            },
        )
        .await
        .unwrap();
    }
    initialize_runtime_authority(&mut transaction, "gdrive-standalone")
        .await
        .unwrap();

    let hosted = upsert_hosted_adapter_effective_control(
        &mut transaction,
        &effective_input("worktree-hosted", now),
    )
    .await
    .unwrap();
    assert!(hosted.runtime_instance_id.is_none());
    assert!(hosted.runtime_lease_version.is_none());

    assert_eq!(
        upsert_hosted_adapter_effective_control(
            &mut transaction,
            &effective_input("gdrive-standalone", now),
        )
        .await,
        Err(ControlPlaneRepositoryError::RuntimeLeaseConflict)
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "select count(*)::bigint from adapter_effective_controls \
             where adapter_id = 'gdrive-standalone'",
        )
        .fetch_one(&mut *transaction)
        .await
        .unwrap(),
        0
    );

    let before = lock_adapter_effective_control(&mut transaction, "worktree-hosted")
        .await
        .unwrap();
    let mut runtime_shaped = effective_input("worktree-hosted", now);
    runtime_shaped.runtime_instance_id = Some("runtime-bypass".into());
    runtime_shaped.standalone_runtime_epoch = Some(1);
    runtime_shaped.report_sequence = Some(1);
    runtime_shaped.runtime_lease_version = Some(1);
    assert_eq!(
        upsert_hosted_adapter_effective_control(&mut transaction, &runtime_shaped).await,
        Err(ControlPlaneRepositoryError::RuntimeReportConflict)
    );
    assert_eq!(
        lock_adapter_effective_control(&mut transaction, "worktree-hosted")
            .await
            .unwrap(),
        before
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn ordered_runtime_report_is_fail_closed_idempotent_and_rollback_safe() {
    let context = connect_required_test_database_from_env().await.unwrap();
    let mut transaction = begin_isolated_schema(context.pool()).await;
    apply_all(&mut transaction).await;
    let now = Utc::now();

    cas_replace_adapter_inventory(
        &mut transaction,
        0,
        &[AdapterInventoryItem {
            adapter_id: "gdrive-report".into(),
            adapter_kind: "gdrive".into(),
            control_authority: "standalone".into(),
            configured_at: now,
            retired_at: None,
        }],
    )
    .await
    .unwrap();
    initialize_adapter_desired_control(
        &mut transaction,
        "gdrive-report",
        &AdapterDesiredControlUpdate {
            desired_enabled: true,
            desired_mode: "bidirectional".into(),
            maintenance_generation: 0,
            maintenance_hold: false,
            updated_by: "admin-a".into(),
        },
    )
    .await
    .unwrap();
    initialize_runtime_authority(&mut transaction, "gdrive-report")
        .await
        .unwrap();

    let lease_a = SecretDigest::parse("1".repeat(64)).unwrap();
    let acquired = cas_acquire_runtime_lease(
        &mut transaction,
        &RuntimeLeaseAcquisition {
            adapter_id: "gdrive-report".into(),
            expected_runtime_lease_version: 0,
            runtime_instance_id: "runtime-a".into(),
            lease_token_digest: lease_a.clone(),
            lease_heartbeat_at: now,
            lease_expires_at: now + chrono::Duration::minutes(10),
            takeover: false,
        },
    )
    .await
    .unwrap();
    let report_one = runtime_report(
        "gdrive-report",
        "runtime-a",
        acquired.standalone_runtime_epoch,
        acquired.runtime_lease_version,
        1,
        '2',
        &lease_a,
        0,
        0,
        now,
    );
    let accepted_one = match accept_ordered_runtime_report(&mut transaction, &report_one)
        .await
        .unwrap()
    {
        RuntimeReportOutcome::Accepted(row) => row,
        RuntimeReportOutcome::Replay(_) => panic!("first report cannot replay"),
    };
    let initial_state = runtime_state(&mut transaction, "gdrive-report").await;

    assert!(matches!(
        accept_ordered_runtime_report(&mut transaction, &report_one)
            .await
            .unwrap(),
        RuntimeReportOutcome::Replay(_)
    ));
    let mut conflict = report_one.clone();
    conflict.report_fingerprint = SecretDigest::parse("3".repeat(64)).unwrap();
    assert_eq!(
        accept_ordered_runtime_report(&mut transaction, &conflict).await,
        Err(ControlPlaneRepositoryError::RuntimeReportConflict)
    );

    let mut stale_instance = runtime_report(
        "gdrive-report",
        "runtime-stale",
        acquired.standalone_runtime_epoch,
        accepted_one.runtime_lease_version,
        2,
        '4',
        &lease_a,
        0,
        0,
        now,
    );
    assert_eq!(
        accept_ordered_runtime_report(&mut transaction, &stale_instance).await,
        Err(ControlPlaneRepositoryError::StaleRuntimeEpoch)
    );
    stale_instance.runtime_instance_id = "runtime-a".into();
    stale_instance.standalone_runtime_epoch = acquired.standalone_runtime_epoch - 1;
    assert_eq!(
        accept_ordered_runtime_report(&mut transaction, &stale_instance).await,
        Err(ControlPlaneRepositoryError::StaleRuntimeEpoch)
    );

    let mut stale_lease = runtime_report(
        "gdrive-report",
        "runtime-a",
        acquired.standalone_runtime_epoch,
        accepted_one.runtime_lease_version - 1,
        2,
        '5',
        &lease_a,
        0,
        0,
        now,
    );
    assert_eq!(
        accept_ordered_runtime_report(&mut transaction, &stale_lease).await,
        Err(ControlPlaneRepositoryError::StaleVersion {
            namespace: cas_namespaces::RUNTIME_LEASE_VERSION,
        })
    );
    stale_lease.expected_runtime_lease_version = accepted_one.runtime_lease_version;
    stale_lease.report_sequence = 3;
    assert_eq!(
        accept_ordered_runtime_report(&mut transaction, &stale_lease).await,
        Err(ControlPlaneRepositoryError::StaleRuntimeReport)
    );
    assert_eq!(
        runtime_state(&mut transaction, "gdrive-report").await,
        initial_state
    );

    let desired = cas_update_adapter_desired_control(
        &mut transaction,
        "gdrive-report",
        0,
        &AdapterDesiredControlUpdate {
            desired_enabled: true,
            desired_mode: "bidirectional".into(),
            maintenance_generation: 0,
            maintenance_hold: false,
            updated_by: "admin-a".into(),
        },
    )
    .await
    .unwrap();
    let current_maintenance = lock_maintenance_control(&mut transaction).await.unwrap();
    let maintenance = cas_update_maintenance_control(
        &mut transaction,
        current_maintenance.maintenance_generation,
        &MaintenanceControlUpdate {
            state: current_maintenance.state,
            transition_operation_id: current_maintenance.transition_operation_id,
            transition_requested_by: current_maintenance.transition_requested_by,
            transition_requested_at: current_maintenance.transition_requested_at,
            state_entered_at: current_maintenance.state_entered_at,
            admission_fence_closed: current_maintenance.admission_fence_closed,
            admission_fence_closed_at: current_maintenance.admission_fence_closed_at,
            quiescence_evidence_version: current_maintenance.quiescence_evidence_version,
            quiescence_evidence_id: current_maintenance.quiescence_evidence_id,
            active_maintenance_job_id: current_maintenance.active_maintenance_job_id,
            safe_error_category: current_maintenance.safe_error_category,
        },
    )
    .await
    .unwrap();

    let stale_generation_state = runtime_state(&mut transaction, "gdrive-report").await;
    let mut report_two = runtime_report(
        "gdrive-report",
        "runtime-a",
        acquired.standalone_runtime_epoch,
        accepted_one.runtime_lease_version,
        2,
        '6',
        &lease_a,
        0,
        maintenance.maintenance_generation,
        now,
    );
    assert_eq!(
        accept_ordered_runtime_report(&mut transaction, &report_two).await,
        Err(ControlPlaneRepositoryError::StaleVersion {
            namespace: cas_namespaces::ADAPTER_CONTROL_GENERATION,
        })
    );
    report_two.adapter_control_generation = desired.adapter_control_generation;
    report_two.maintenance_generation = 0;
    assert_eq!(
        accept_ordered_runtime_report(&mut transaction, &report_two).await,
        Err(ControlPlaneRepositoryError::StaleVersion {
            namespace: cas_namespaces::MAINTENANCE_GENERATION,
        })
    );
    assert_eq!(
        runtime_state(&mut transaction, "gdrive-report").await,
        stale_generation_state
    );

    report_two.maintenance_generation = maintenance.maintenance_generation;
    let accepted_two = match accept_ordered_runtime_report(&mut transaction, &report_two)
        .await
        .unwrap()
    {
        RuntimeReportOutcome::Accepted(row) => row,
        RuntimeReportOutcome::Replay(_) => panic!("second report cannot replay"),
    };

    let permitted = open_runtime_mutation_permit(
        &mut transaction,
        &MutationPermitInput {
            permit_id: "permit-report".into(),
            adapter_id: "gdrive-report".into(),
            runtime_instance_id: "runtime-a".into(),
            standalone_runtime_epoch: acquired.standalone_runtime_epoch,
            expected_runtime_lease_version: accepted_two.runtime_lease_version,
            adapter_control_generation: desired.adapter_control_generation,
            maintenance_generation: maintenance.maintenance_generation,
            step_id: "provider-put".into(),
            lease_proof_digest: lease_a.clone(),
            permit_proof_digest: SecretDigest::parse("7".repeat(64)).unwrap(),
            bounded_expires_at: now + chrono::Duration::minutes(1),
        },
    )
    .await
    .unwrap();
    let before_rollback = runtime_state(&mut transaction, "gdrive-report").await;
    (&mut *transaction)
        .execute("savepoint ordered_report_rollback")
        .await
        .unwrap();

    let mut report_three = runtime_report(
        "gdrive-report",
        "runtime-a",
        acquired.standalone_runtime_epoch,
        permitted.runtime_lease_version,
        3,
        '8',
        &lease_a,
        desired.adapter_control_generation,
        maintenance.maintenance_generation,
        now,
    );
    report_three.close_permit_ids = vec!["permit-report".into()];
    accept_ordered_runtime_report(&mut transaction, &report_three)
        .await
        .unwrap();
    (&mut *transaction)
        .execute("rollback to savepoint ordered_report_rollback")
        .await
        .unwrap();
    assert_eq!(
        runtime_state(&mut transaction, "gdrive-report").await,
        before_rollback
    );

    let accepted_three = match accept_ordered_runtime_report(&mut transaction, &report_three)
        .await
        .unwrap()
    {
        RuntimeReportOutcome::Accepted(row) => row,
        RuntimeReportOutcome::Replay(_) => panic!("terminal report cannot replay"),
    };
    let released = cas_release_runtime_lease(
        &mut transaction,
        "gdrive-report",
        "runtime-a",
        accepted_three.runtime_lease_version,
        acquired.standalone_runtime_epoch,
        &lease_a,
    )
    .await
    .unwrap();

    let lease_b = SecretDigest::parse("9".repeat(64)).unwrap();
    let acquired_b = cas_acquire_runtime_lease(
        &mut transaction,
        &RuntimeLeaseAcquisition {
            adapter_id: "gdrive-report".into(),
            expected_runtime_lease_version: released.runtime_lease_version,
            runtime_instance_id: "runtime-b".into(),
            lease_token_digest: lease_b.clone(),
            lease_heartbeat_at: now,
            lease_expires_at: now + chrono::Duration::minutes(10),
            takeover: false,
        },
    )
    .await
    .unwrap();
    let report_b = runtime_report(
        "gdrive-report",
        "runtime-b",
        acquired_b.standalone_runtime_epoch,
        acquired_b.runtime_lease_version,
        1,
        'a',
        &lease_b,
        desired.adapter_control_generation,
        maintenance.maintenance_generation,
        now,
    );
    accept_ordered_runtime_report(&mut transaction, &report_b)
        .await
        .unwrap();
    let latest = runtime_state(&mut transaction, "gdrive-report").await;

    let stale_old_runtime = runtime_report(
        "gdrive-report",
        "runtime-a",
        acquired.standalone_runtime_epoch,
        accepted_three.runtime_lease_version,
        4,
        'b',
        &lease_a,
        desired.adapter_control_generation,
        maintenance.maintenance_generation,
        now,
    );
    assert_eq!(
        accept_ordered_runtime_report(&mut transaction, &stale_old_runtime).await,
        Err(ControlPlaneRepositoryError::StaleRuntimeEpoch)
    );
    assert_eq!(
        runtime_state(&mut transaction, "gdrive-report").await,
        latest
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn principal_credential_job_slot_and_evidence_cas_namespaces_are_exact() {
    let context = connect_required_test_database_from_env().await.unwrap();
    let mut transaction = begin_isolated_schema(context.pool()).await;
    apply_all(&mut transaction).await;
    let now = Utc::now();

    let principal = insert_principal(
        &mut transaction,
        "admin-cas",
        "administrator",
        "admin",
        true,
        now,
    )
    .await
    .unwrap();
    assert_eq!(
        cas_update_principal(
            &mut transaction,
            "admin-cas",
            principal.principal_version - 1,
            principal.credential_set_generation,
            &principal_update("admin"),
        )
        .await,
        Err(ControlPlaneRepositoryError::StaleVersion {
            namespace: cas_namespaces::PRINCIPAL_VERSION,
        })
    );
    assert_eq!(
        cas_update_principal(
            &mut transaction,
            "admin-cas",
            principal.principal_version,
            principal.credential_set_generation - 1,
            &principal_update("admin"),
        )
        .await,
        Err(ControlPlaneRepositoryError::StaleVersion {
            namespace: cas_namespaces::CREDENTIAL_SET_GENERATION,
        })
    );
    assert_eq!(
        cas_update_principal(
            &mut transaction,
            "admin-cas",
            principal.principal_version - 1,
            principal.credential_set_generation - 1,
            &principal_update("admin"),
        )
        .await,
        Err(ControlPlaneRepositoryError::StaleVersion {
            namespace: cas_namespaces::PRINCIPAL_VERSION,
        })
    );
    let principal = cas_update_principal(
        &mut transaction,
        "admin-cas",
        principal.principal_version,
        principal.credential_set_generation,
        &principal_update("admin"),
    )
    .await
    .unwrap();

    let credential = credential_input("admin-cas", now);
    assert_eq!(
        insert_credential(
            &mut transaction,
            &credential,
            principal.principal_version - 1,
            principal.credential_set_generation,
        )
        .await,
        Err(ControlPlaneRepositoryError::StaleVersion {
            namespace: cas_namespaces::PRINCIPAL_VERSION,
        })
    );
    assert_eq!(
        insert_credential(
            &mut transaction,
            &credential,
            principal.principal_version,
            principal.credential_set_generation - 1,
        )
        .await,
        Err(ControlPlaneRepositoryError::StaleVersion {
            namespace: cas_namespaces::CREDENTIAL_SET_GENERATION,
        })
    );
    assert!(
        read_credential_by_id(&mut transaction, "credential-cas-a")
            .await
            .unwrap()
            .is_none()
    );

    let created = insert_credential(
        &mut transaction,
        &credential,
        principal.principal_version,
        principal.credential_set_generation,
    )
    .await
    .unwrap();
    let lifecycle = CredentialLifecycleUpdate {
        status: "grace".into(),
        grace_expires_at: Some(now + chrono::Duration::minutes(5)),
        revoked_at: None,
        revoked_by: None,
        expires_at: None,
    };
    let before_lifecycle = created.clone();
    assert_eq!(
        cas_update_credential_lifecycle(
            &mut transaction,
            "credential-cas-a",
            created.credential.credential_version,
            created.principal.principal_version - 1,
            created.principal.credential_set_generation,
            &lifecycle,
        )
        .await,
        Err(ControlPlaneRepositoryError::StaleVersion {
            namespace: cas_namespaces::PRINCIPAL_VERSION,
        })
    );
    assert_eq!(
        cas_update_credential_lifecycle(
            &mut transaction,
            "credential-cas-a",
            created.credential.credential_version,
            created.principal.principal_version,
            created.principal.credential_set_generation - 1,
            &lifecycle,
        )
        .await,
        Err(ControlPlaneRepositoryError::StaleVersion {
            namespace: cas_namespaces::CREDENTIAL_SET_GENERATION,
        })
    );
    assert_eq!(
        cas_update_credential_lifecycle(
            &mut transaction,
            "credential-cas-a",
            created.credential.credential_version - 1,
            created.principal.principal_version,
            created.principal.credential_set_generation,
            &lifecycle,
        )
        .await,
        Err(ControlPlaneRepositoryError::StaleVersion {
            namespace: cas_namespaces::CREDENTIAL_VERSION,
        })
    );
    assert_eq!(
        cas_update_credential_lifecycle(
            &mut transaction,
            "credential-cas-a",
            created.credential.credential_version - 1,
            created.principal.principal_version - 1,
            created.principal.credential_set_generation - 1,
            &lifecycle,
        )
        .await,
        Err(ControlPlaneRepositoryError::StaleVersion {
            namespace: cas_namespaces::PRINCIPAL_VERSION,
        })
    );
    assert_eq!(
        lock_principal(&mut transaction, "admin-cas").await.unwrap(),
        before_lifecycle.principal
    );
    assert_eq!(
        lock_credential(&mut transaction, "credential-cas-a")
            .await
            .unwrap(),
        before_lifecycle.credential
    );
    let lifecycle_changed = cas_update_credential_lifecycle(
        &mut transaction,
        "credential-cas-a",
        created.credential.credential_version,
        created.principal.principal_version,
        created.principal.credential_set_generation,
        &lifecycle,
    )
    .await
    .unwrap();

    insert_or_replay_complete_operational_job(
        &mut transaction,
        &operational_job_input("admin-cas"),
    )
    .await
    .unwrap();
    let lease = SecretDigest::parse("c".repeat(64)).unwrap();
    let running_update = operational_job_update("running", 1, Some(lease.clone()), now, 1);
    let running = cas_update_operational_job(
        &mut transaction,
        "job-cas-a",
        1,
        Some(0),
        None,
        &running_update,
    )
    .await
    .unwrap();
    let before_job_errors = running.clone();
    let checkpoint_update = operational_job_update("running", 1, Some(lease.clone()), now, 2);

    assert_eq!(
        cas_update_operational_job(
            &mut transaction,
            "job-cas-a",
            running.job_version - 1,
            Some(running.executor_fence),
            Some(&lease),
            &checkpoint_update,
        )
        .await,
        Err(ControlPlaneRepositoryError::StaleVersion {
            namespace: cas_namespaces::JOB_VERSION,
        })
    );
    assert_eq!(
        cas_update_operational_job(
            &mut transaction,
            "job-cas-a",
            running.job_version - 1,
            Some(running.executor_fence - 1),
            Some(&lease),
            &checkpoint_update,
        )
        .await,
        Err(ControlPlaneRepositoryError::StaleVersion {
            namespace: cas_namespaces::JOB_VERSION,
        })
    );
    assert_eq!(
        cas_update_operational_job(
            &mut transaction,
            "job-cas-a",
            running.job_version,
            Some(running.executor_fence - 1),
            Some(&lease),
            &checkpoint_update,
        )
        .await,
        Err(ControlPlaneRepositoryError::StaleVersion {
            namespace: cas_namespaces::EXECUTOR_FENCE,
        })
    );
    let wrong_lease = SecretDigest::parse("d".repeat(64)).unwrap();
    assert_eq!(
        cas_update_operational_job(
            &mut transaction,
            "job-cas-a",
            running.job_version,
            Some(running.executor_fence),
            Some(&wrong_lease),
            &checkpoint_update,
        )
        .await,
        Err(ControlPlaneRepositoryError::StaleVersion {
            namespace: cas_namespaces::EXECUTOR_LEASE,
        })
    );
    assert_eq!(
        job_records::lock_operational_job(&mut transaction, "job-cas-a")
            .await
            .unwrap(),
        before_job_errors
    );

    let checkpointed = cas_update_operational_job(
        &mut transaction,
        "job-cas-a",
        running.job_version,
        Some(running.executor_fence),
        Some(&lease),
        &checkpoint_update,
    )
    .await
    .unwrap();

    (&mut *transaction)
        .execute("savepoint job_update_rollback")
        .await
        .unwrap();
    let rollback_update = operational_job_update("running", 1, Some(lease.clone()), now, 3);
    cas_update_operational_job(
        &mut transaction,
        "job-cas-a",
        checkpointed.job_version,
        Some(checkpointed.executor_fence),
        Some(&lease),
        &rollback_update,
    )
    .await
    .unwrap();
    (&mut *transaction)
        .execute("rollback to savepoint job_update_rollback")
        .await
        .unwrap();
    assert_eq!(
        job_records::lock_operational_job(&mut transaction, "job-cas-a")
            .await
            .unwrap(),
        checkpointed
    );

    let stale_evidence = OperationalEvidenceInput {
        evidence_id: "evidence-stale".into(),
        operation_id: "job-cas-a".into(),
        job_version: checkpointed.job_version - 1,
        evidence_kind: "checkpoint".into(),
        evidence_schema: "haze-sync.test-evidence.v1".into(),
        evidence_version: 1,
        evidence_digest: SecretDigest::parse("e".repeat(64)).unwrap(),
        status: "passed".into(),
        safe_metadata: serde_json::json!({"safe": true}),
        source_environment_id: None,
        target_environment_id: None,
        helper_attempt_id: None,
    };
    assert_eq!(
        insert_operational_evidence(&mut transaction, &stale_evidence).await,
        Err(ControlPlaneRepositoryError::StaleVersion {
            namespace: cas_namespaces::JOB_VERSION,
        })
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "select count(*)::bigint from operational_job_evidence \
             where evidence_id = 'evidence-stale'",
        )
        .fetch_one(&mut *transaction)
        .await
        .unwrap(),
        0
    );
    let mut exact_evidence = stale_evidence.clone();
    exact_evidence.evidence_id = "evidence-current".into();
    exact_evidence.job_version = checkpointed.job_version;
    insert_operational_evidence(&mut transaction, &exact_evidence)
        .await
        .unwrap();

    let slot = cas_acquire_execution_slot(
        &mut transaction,
        "global-destructive",
        0,
        "job-cas-a",
        Some(0),
        checkpointed.executor_fence,
    )
    .await
    .unwrap();
    assert_eq!(
        cas_release_execution_slot(
            &mut transaction,
            "global-destructive",
            slot.slot_version - 1,
            "job-cas-a",
            checkpointed.executor_fence - 1,
        )
        .await,
        Err(ControlPlaneRepositoryError::StaleVersion {
            namespace: cas_namespaces::SLOT_VERSION,
        })
    );
    assert_eq!(
        cas_release_execution_slot(
            &mut transaction,
            "global-destructive",
            slot.slot_version,
            "job-cas-a",
            checkpointed.executor_fence - 1,
        )
        .await,
        Err(ControlPlaneRepositoryError::StaleVersion {
            namespace: cas_namespaces::EXECUTOR_FENCE,
        })
    );
    assert_eq!(
        cas_release_execution_slot(
            &mut transaction,
            "global-destructive",
            slot.slot_version,
            "other-job",
            checkpointed.executor_fence,
        )
        .await,
        Err(ControlPlaneRepositoryError::OperationInProgress)
    );
    assert_eq!(
        lock_execution_slot(&mut transaction, "global-destructive")
            .await
            .unwrap(),
        slot
    );

    let blocked = cas_block_execution_slot_uncertain(
        &mut transaction,
        "global-destructive",
        slot.slot_version,
        "job-cas-a",
        checkpointed.executor_fence,
    )
    .await
    .unwrap();
    assert_eq!(
        cas_reconcile_execution_slot_uncertainty(
            &mut transaction,
            "global-destructive",
            blocked.slot_version,
            "job-cas-a",
            checkpointed.executor_fence - 1,
        )
        .await,
        Err(ControlPlaneRepositoryError::StaleVersion {
            namespace: cas_namespaces::EXECUTOR_FENCE,
        })
    );
    let reconciled = cas_reconcile_execution_slot_uncertainty(
        &mut transaction,
        "global-destructive",
        blocked.slot_version,
        "job-cas-a",
        checkpointed.executor_fence,
    )
    .await
    .unwrap();
    cas_release_execution_slot(
        &mut transaction,
        "global-destructive",
        reconciled.slot_version,
        "job-cas-a",
        checkpointed.executor_fence,
    )
    .await
    .unwrap();

    let terminal_update = operational_job_update("succeeded", 1, Some(lease.clone()), now, 4);
    let terminal = cas_update_operational_job(
        &mut transaction,
        "job-cas-a",
        checkpointed.job_version,
        Some(checkpointed.executor_fence),
        Some(&lease),
        &terminal_update,
    )
    .await
    .unwrap();
    assert_eq!(
        cas_update_operational_job(
            &mut transaction,
            "job-cas-a",
            terminal.job_version,
            Some(terminal.executor_fence),
            Some(&lease),
            &terminal_update,
        )
        .await,
        Err(ControlPlaneRepositoryError::ImmutableRecord)
    );
    assert_eq!(
        cas_update_operational_job(
            &mut transaction,
            "missing-job",
            1,
            Some(0),
            None,
            &running_update,
        )
        .await,
        Err(ControlPlaneRepositoryError::MissingRecord)
    );

    assert_eq!(lifecycle_changed.credential.status, "grace");
    transaction.rollback().await.unwrap();
}
