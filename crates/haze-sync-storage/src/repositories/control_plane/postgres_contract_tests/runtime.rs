#[tokio::test]
async fn runtime_epoch_report_permit_and_uncertain_effect_primitives_are_fail_closed() {
    let context = connect_required_test_database_from_env().await.unwrap();
    let mut transaction = begin_isolated_schema(context.pool()).await;
    apply_all(&mut transaction).await;
    let now = Utc::now();
    cas_replace_adapter_inventory(
        &mut transaction,
        0,
        &[AdapterInventoryItem {
            adapter_id: "gdrive-a".into(),
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
        "gdrive-a",
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
    initialize_runtime_authority(&mut transaction, "gdrive-a")
        .await
        .unwrap();
    let lease_digest = SecretDigest::parse("5".repeat(64)).unwrap();
    let acquired = cas_acquire_runtime_lease(
        &mut transaction,
        &RuntimeLeaseAcquisition {
            adapter_id: "gdrive-a".into(),
            expected_runtime_lease_version: 0,
            runtime_instance_id: "runtime-a".into(),
            lease_token_digest: lease_digest.clone(),
            lease_heartbeat_at: now,
            lease_expires_at: now + chrono::Duration::minutes(2),
            takeover: false,
        },
    )
    .await
    .unwrap();
    assert_eq!(acquired.standalone_runtime_epoch, 1);

    let initial_fingerprint = SecretDigest::parse("7".repeat(64)).unwrap();
    let initial_report = RuntimeReportInput {
        adapter_id: "gdrive-a".into(),
        runtime_instance_id: "runtime-a".into(),
        standalone_runtime_epoch: 1,
        expected_runtime_lease_version: acquired.runtime_lease_version,
        report_sequence: 1,
        report_fingerprint: initial_fingerprint,
        lease_proof_digest: lease_digest.clone(),
        adapter_control_generation: 0,
        maintenance_generation: 0,
        effective_mode: Some("bidirectional".into()),
        runtime_lifecycle: "idle".into(),
        in_flight: false,
        connection_state: "connected".into(),
        checkpoint_summary: serde_json::json!({"core_seq":0}),
        safe_error_category: None,
        safe_error_code: None,
        reported_at: now,
        close_permit_ids: Vec::new(),
    };
    let initial = match accept_ordered_runtime_report(&mut transaction, &initial_report)
        .await
        .unwrap()
    {
        RuntimeReportOutcome::Accepted(row) => row,
        RuntimeReportOutcome::Replay(_) => panic!("first report cannot be replay"),
    };
    assert_eq!(initial.last_accepted_report_sequence, 1);

    let permitted = open_runtime_mutation_permit(
        &mut transaction,
        &MutationPermitInput {
            permit_id: "permit-a".into(),
            adapter_id: "gdrive-a".into(),
            runtime_instance_id: "runtime-a".into(),
            standalone_runtime_epoch: 1,
            expected_runtime_lease_version: initial.runtime_lease_version,
            adapter_control_generation: 0,
            maintenance_generation: 0,
            step_id: "provider-put-a".into(),
            lease_proof_digest: lease_digest.clone(),
            permit_proof_digest: SecretDigest::parse("6".repeat(64)).unwrap(),
            bounded_expires_at: now + chrono::Duration::seconds(30),
        },
    )
    .await
    .unwrap();
    assert_eq!(permitted.open_mutation_permits, 1);
    let in_flight = lock_adapter_effective_control(&mut transaction, "gdrive-a")
        .await
        .unwrap();
    assert!(in_flight.in_flight);
    assert_eq!(in_flight.runtime_lease_version, Some(permitted.runtime_lease_version));

    let terminal_fingerprint = SecretDigest::parse("8".repeat(64)).unwrap();
    let terminal_report = RuntimeReportInput {
        expected_runtime_lease_version: permitted.runtime_lease_version,
        report_sequence: 2,
        report_fingerprint: terminal_fingerprint.clone(),
        runtime_lifecycle: "idle".into(),
        in_flight: false,
        checkpoint_summary: serde_json::json!({"core_seq":1}),
        close_permit_ids: vec!["permit-a".into()],
        ..initial_report.clone()
    };
    let accepted = match accept_ordered_runtime_report(&mut transaction, &terminal_report)
        .await
        .unwrap()
    {
        RuntimeReportOutcome::Accepted(row) => row,
        RuntimeReportOutcome::Replay(_) => panic!("terminal report cannot initially replay"),
    };
    assert_eq!(accepted.last_accepted_report_sequence, 2);
    assert_eq!(accepted.open_mutation_permits, 0);
    assert!(matches!(
        accept_ordered_runtime_report(&mut transaction, &terminal_report)
            .await
            .unwrap(),
        RuntimeReportOutcome::Replay(_)
    ));
    let mut conflicting = terminal_report.clone();
    conflicting.report_fingerprint = SecretDigest::parse("9".repeat(64)).unwrap();
    assert_eq!(
        accept_ordered_runtime_report(&mut transaction, &conflicting).await,
        Err(ControlPlaneRepositoryError::RuntimeReportConflict)
    );
    let mut skipped = terminal_report.clone();
    skipped.report_sequence = 4;
    skipped.report_fingerprint = SecretDigest::parse("a".repeat(64)).unwrap();
    skipped.expected_runtime_lease_version = accepted.runtime_lease_version;
    skipped.close_permit_ids.clear();
    assert_eq!(
        accept_ordered_runtime_report(&mut transaction, &skipped).await,
        Err(ControlPlaneRepositoryError::StaleRuntimeReport)
    );

    let renewed = cas_renew_runtime_lease(
        &mut transaction,
        &RuntimeLeaseRenewal {
            adapter_id: "gdrive-a".into(),
            runtime_instance_id: "runtime-a".into(),
            expected_runtime_lease_version: accepted.runtime_lease_version,
            standalone_runtime_epoch: 1,
            lease_proof_digest: lease_digest.clone(),
            heartbeat_at: now + chrono::Duration::seconds(5),
            expires_at: now + chrono::Duration::minutes(3),
        },
    )
    .await
    .unwrap();
    let renewed_effective = lock_adapter_effective_control(&mut transaction, "gdrive-a")
        .await
        .unwrap();
    assert_eq!(
        renewed_effective.runtime_lease_version,
        Some(renewed.runtime_lease_version)
    );

    let uncertain = record_uncertain_external_effect(
        &mut transaction,
        &UncertainEffectInput {
            effect_id: "effect-a".into(),
            adapter_id: "gdrive-a".into(),
            runtime_instance_id: "runtime-a".into(),
            standalone_runtime_epoch: 1,
            expected_runtime_lease_version: renewed.runtime_lease_version,
            lease_proof_digest: lease_digest.clone(),
            permit_id: None,
            step_id: "provider-put-unknown".into(),
            effect_identity_digest: SecretDigest::parse("b".repeat(64)).unwrap(),
            safe_metadata: serde_json::json!({"kind":"provider_put"}),
        },
    )
    .await
    .unwrap();
    assert_eq!(uncertain.uncertain_external_effects, 1);
    assert_eq!(uncertain.takeover_state, "reconciliation_required");
    let reconciled = reconcile_uncertain_external_effect(
        &mut transaction,
        &UncertainEffectReconciliation {
            adapter_id: "gdrive-a".into(),
            effect_id: "effect-a".into(),
            expected_runtime_lease_version: uncertain.runtime_lease_version,
            resolution: "reconciled".into(),
            external_fence_id: None,
            clear_takeover_when_empty: true,
        },
    )
    .await
    .unwrap();
    assert_eq!(reconciled.uncertain_external_effects, 0);
    assert_eq!(reconciled.takeover_state, "clear");

    let resumed_report = RuntimeReportInput {
        expected_runtime_lease_version: reconciled.runtime_lease_version,
        report_sequence: 3,
        report_fingerprint: SecretDigest::parse("c".repeat(64)).unwrap(),
        close_permit_ids: Vec::new(),
        ..terminal_report.clone()
    };
    let resumed = match accept_ordered_runtime_report(&mut transaction, &resumed_report)
        .await
        .unwrap()
    {
        RuntimeReportOutcome::Accepted(row) => row,
        RuntimeReportOutcome::Replay(_) => panic!("new report cannot replay"),
    };
    let released = cas_release_runtime_lease(
        &mut transaction,
        "gdrive-a",
        "runtime-a",
        resumed.runtime_lease_version,
        1,
        &lease_digest,
    )
    .await
    .unwrap();
    assert!(released.runtime_instance_id.is_none());
    assert_eq!(released.standalone_runtime_epoch, 1);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn expired_runtime_takeover_converts_open_permits_to_reconciliation_evidence() {
    let context = connect_required_test_database_from_env().await.unwrap();
    let mut transaction = begin_isolated_schema(context.pool()).await;
    apply_all(&mut transaction).await;
    let now = Utc::now();
    cas_replace_adapter_inventory(
        &mut transaction,
        0,
        &[AdapterInventoryItem {
            adapter_id: "gdrive-takeover".into(),
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
        "gdrive-takeover",
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
    initialize_runtime_authority(&mut transaction, "gdrive-takeover")
        .await
        .unwrap();
    let old_digest = SecretDigest::parse("d".repeat(64)).unwrap();
    let acquired = cas_acquire_runtime_lease(
        &mut transaction,
        &RuntimeLeaseAcquisition {
            adapter_id: "gdrive-takeover".into(),
            expected_runtime_lease_version: 0,
            runtime_instance_id: "runtime-old".into(),
            lease_token_digest: old_digest.clone(),
            lease_heartbeat_at: now,
            lease_expires_at: now + chrono::Duration::minutes(2),
            takeover: false,
        },
    )
    .await
    .unwrap();
    let report = RuntimeReportInput {
        adapter_id: "gdrive-takeover".into(),
        runtime_instance_id: "runtime-old".into(),
        standalone_runtime_epoch: 1,
        expected_runtime_lease_version: acquired.runtime_lease_version,
        report_sequence: 1,
        report_fingerprint: SecretDigest::parse("e".repeat(64)).unwrap(),
        lease_proof_digest: old_digest.clone(),
        adapter_control_generation: 0,
        maintenance_generation: 0,
        effective_mode: Some("bidirectional".into()),
        runtime_lifecycle: "idle".into(),
        in_flight: false,
        connection_state: "connected".into(),
        checkpoint_summary: serde_json::json!({"core_seq":0}),
        safe_error_category: None,
        safe_error_code: None,
        reported_at: now,
        close_permit_ids: Vec::new(),
    };
    let reported = match accept_ordered_runtime_report(&mut transaction, &report)
        .await
        .unwrap()
    {
        RuntimeReportOutcome::Accepted(row) => row,
        RuntimeReportOutcome::Replay(_) => panic!("first report cannot replay"),
    };
    let permitted = open_runtime_mutation_permit(
        &mut transaction,
        &MutationPermitInput {
            permit_id: "permit-takeover".into(),
            adapter_id: "gdrive-takeover".into(),
            runtime_instance_id: "runtime-old".into(),
            standalone_runtime_epoch: 1,
            expected_runtime_lease_version: reported.runtime_lease_version,
            adapter_control_generation: 0,
            maintenance_generation: 0,
            step_id: "provider-delete".into(),
            lease_proof_digest: old_digest,
            permit_proof_digest: SecretDigest::parse("f".repeat(64)).unwrap(),
            bounded_expires_at: now + chrono::Duration::seconds(30),
        },
    )
    .await
    .unwrap();
    sqlx::query(
        "update gdrive_runtime_authorities set lease_heartbeat_at = now() - interval '2 minutes', \
         lease_expires_at = now() - interval '1 minute' where adapter_id = 'gdrive-takeover'",
    )
    .execute(&mut *transaction)
    .await
    .unwrap();

    let new_digest = SecretDigest::parse("1".repeat(64)).unwrap();
    assert_eq!(
        cas_acquire_runtime_lease(
            &mut transaction,
            &RuntimeLeaseAcquisition {
                adapter_id: "gdrive-takeover".into(),
                expected_runtime_lease_version: permitted.runtime_lease_version,
                runtime_instance_id: "runtime-new".into(),
                lease_token_digest: new_digest.clone(),
                lease_heartbeat_at: now,
                lease_expires_at: now + chrono::Duration::minutes(2),
                takeover: false,
            },
        )
        .await,
        Err(ControlPlaneRepositoryError::RuntimeLeaseConflict)
    );
    let takeover = cas_acquire_runtime_lease(
        &mut transaction,
        &RuntimeLeaseAcquisition {
            adapter_id: "gdrive-takeover".into(),
            expected_runtime_lease_version: permitted.runtime_lease_version,
            runtime_instance_id: "runtime-new".into(),
            lease_token_digest: new_digest.clone(),
            lease_heartbeat_at: now,
            lease_expires_at: now + chrono::Duration::minutes(2),
            takeover: true,
        },
    )
    .await
    .unwrap();
    assert_eq!(takeover.standalone_runtime_epoch, 2);
    assert_eq!(takeover.open_mutation_permits, 0);
    assert_eq!(takeover.uncertain_external_effects, 1);
    assert_eq!(takeover.takeover_state, "reconciliation_required");
    let permit_state: String = sqlx::query_scalar(
        "select state from gdrive_mutation_permits where permit_id='permit-takeover'",
    )
    .fetch_one(&mut *transaction)
    .await
    .unwrap();
    assert_eq!(permit_state, "uncertain");
    let effect_id: String = sqlx::query_scalar(
        "select effect_id from gdrive_uncertain_effects where adapter_id='gdrive-takeover' \
         and state='unresolved'",
    )
    .fetch_one(&mut *transaction)
    .await
    .unwrap();
    assert!(effect_id.starts_with("takeover-"));
    assert_eq!(
        accept_ordered_runtime_report(&mut transaction, &report).await,
        Err(ControlPlaneRepositoryError::StaleRuntimeEpoch)
    );
    assert_eq!(
        open_runtime_mutation_permit(
            &mut transaction,
            &MutationPermitInput {
                permit_id: "permit-new-blocked".into(),
                adapter_id: "gdrive-takeover".into(),
                runtime_instance_id: "runtime-new".into(),
                standalone_runtime_epoch: 2,
                expected_runtime_lease_version: takeover.runtime_lease_version,
                adapter_control_generation: 0,
                maintenance_generation: 0,
                step_id: "provider-put".into(),
                lease_proof_digest: new_digest.clone(),
                permit_proof_digest: SecretDigest::parse("2".repeat(64)).unwrap(),
                bounded_expires_at: now + chrono::Duration::seconds(30),
            },
        )
        .await,
        Err(ControlPlaneRepositoryError::OperationInProgress)
    );

    let reconciled = reconcile_uncertain_external_effect(
        &mut transaction,
        &UncertainEffectReconciliation {
            adapter_id: "gdrive-takeover".into(),
            effect_id,
            expected_runtime_lease_version: takeover.runtime_lease_version,
            resolution: "externally_fenced".into(),
            external_fence_id: Some("external-fence-a".into()),
            clear_takeover_when_empty: true,
        },
    )
    .await
    .unwrap();
    assert_eq!(reconciled.uncertain_external_effects, 0);
    assert_eq!(reconciled.takeover_state, "clear");
    let new_report = RuntimeReportInput {
        adapter_id: "gdrive-takeover".into(),
        runtime_instance_id: "runtime-new".into(),
        standalone_runtime_epoch: 2,
        expected_runtime_lease_version: reconciled.runtime_lease_version,
        report_sequence: 1,
        report_fingerprint: SecretDigest::parse("3".repeat(64)).unwrap(),
        lease_proof_digest: new_digest,
        adapter_control_generation: 0,
        maintenance_generation: 0,
        effective_mode: Some("bidirectional".into()),
        runtime_lifecycle: "idle".into(),
        in_flight: false,
        connection_state: "connected".into(),
        checkpoint_summary: serde_json::json!({"core_seq":0}),
        safe_error_category: None,
        safe_error_code: None,
        reported_at: now,
        close_permit_ids: Vec::new(),
    };
    assert!(matches!(
        accept_ordered_runtime_report(&mut transaction, &new_report)
            .await
            .unwrap(),
        RuntimeReportOutcome::Accepted(row) if row.standalone_runtime_epoch == 2
    ));
    transaction.rollback().await.unwrap();
}
