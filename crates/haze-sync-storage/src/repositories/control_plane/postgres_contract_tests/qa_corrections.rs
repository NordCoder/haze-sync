#[tokio::test]
async fn recovery_complete_quiescence_evidence_round_trips_as_one_typed_bundle() {
    let context = connect_required_test_database_from_env().await.unwrap();
    let mut transaction = begin_isolated_schema(context.pool()).await;
    apply_all(&mut transaction).await;
    let now = Utc::now();

    sqlx::query(
        "update maintenance_control set maintenance_generation = 1, state = 'quiescing', \
         admission_fence_closed = true, admission_fence_closed_at = $1, updated_at = now() \
         where singleton_id = 1",
    )
    .bind(now)
    .execute(&mut *transaction)
    .await
    .unwrap();
    sqlx::query(
        "update adapter_inventory_state set adapter_inventory_generation = 1 where singleton_id = 1",
    )
    .execute(&mut *transaction)
    .await
    .unwrap();
    sqlx::query(
        "insert into adapter_inventory (adapter_id, adapter_kind, control_authority, inventory_generation) \
         values ('worktree-a','worktree','server',1)",
    )
    .execute(&mut *transaction)
    .await
    .unwrap();
    sqlx::query(
        "insert into adapter_desired_controls (adapter_id, desired_enabled, desired_mode, \
         adapter_control_generation, maintenance_generation, maintenance_hold, updated_by) \
         values ('worktree-a',false,'disabled',1,1,true,'admin-a')",
    )
    .execute(&mut *transaction)
    .await
    .unwrap();
    sqlx::query(
        "insert into adapter_effective_controls (adapter_id, effective_mode, runtime_lifecycle, \
         last_applied_adapter_control_generation, last_applied_maintenance_generation, in_flight, \
         connection_state, checkpoint_summary, reported_at) \
         values ('worktree-a','disabled','idle',1,1,false,'connected', \
         '{\"cursor_generation\":7}'::jsonb,$1)",
    )
    .bind(now)
    .execute(&mut *transaction)
    .await
    .unwrap();

    let bundle = insert_complete_quiescence_evidence(
        &mut transaction,
        &CompleteQuiescenceEvidenceInput {
            quiescence_evidence_id: "evidence-a".into(),
            evidence_version: 1,
            maintenance_generation: 1,
            adapter_inventory_generation: 1,
            admission_fence_closed_at: now,
            server_instance_id: "server-a".into(),
            server_started_at: now,
            active_authoritative_mutations: 0,
            open_authoritative_transactions: 0,
            active_cli_write_jobs: 0,
            object_store_writer_count: 0,
            database_writer_count: 0,
            worktree_external_writer_evidence: serde_json::json!({
                "root_identity": "vault-a",
                "exclusive_writer_proven": true
            }),
            evidence_digest: SecretDigest::parse("a".repeat(64)).unwrap(),
            captured_at: now,
            adapter_snapshots: vec![QuiescenceAdapterSnapshotInput {
                adapter_id: "worktree-a".into(),
                adapter_kind: "worktree".into(),
                control_authority: "server".into(),
                adapter_control_generation: 1,
                maintenance_generation: 1,
                desired_enabled: false,
                desired_mode: "disabled".into(),
                last_applied_adapter_control_generation: Some(1),
                last_applied_maintenance_generation: Some(1),
                runtime_lifecycle: "idle".into(),
                connection_state: "connected".into(),
                drain_proof: "acknowledged".into(),
                external_fence_id: None,
                checkpoint_summary: serde_json::json!({"cursor_generation":7}),
                captured_at: now,
            }],
            runtime_snapshots: vec![],
        },
    )
    .await
    .unwrap();
    assert_eq!(bundle.evidence.active_cli_write_jobs, 0);
    assert_eq!(bundle.evidence.object_store_writer_count, 0);
    assert_eq!(bundle.evidence.database_writer_count, 0);
    assert_eq!(bundle.adapter_snapshots.len(), 1);
    assert!(bundle.runtime_snapshots.is_empty());

    append_quiescence_evidence_invalidation(
        &mut transaction,
        &QuiescenceEvidenceInvalidationInput {
            invalidation_id: "invalidation-a".into(),
            quiescence_evidence_id: "evidence-a".into(),
            maintenance_generation: 1,
            invalidation_reason: "writer-fact-changed".into(),
        },
    )
    .await
    .unwrap();
    let reread = lock_complete_quiescence_evidence(&mut transaction, "evidence-a")
        .await
        .unwrap();
    assert_eq!(reread.invalidations.len(), 1);
    assert_eq!(reread.invalidations[0].invalidation_id, "invalidation-a");
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn operational_job_replay_returns_complete_idempotency_and_generation_binding() {
    let context = connect_required_test_database_from_env().await.unwrap();
    let mut transaction = begin_isolated_schema(context.pool()).await;
    apply_all(&mut transaction).await;
    let now = Utc::now();
    insert_principal(&mut transaction, "admin-a", "administrator", "admin", true, now)
        .await
        .unwrap();
    let input = OperationalJobInput {
        operation_id: "job-complete-a".into(),
        kind: "backup".into(),
        maintenance_required: false,
        destructive: false,
        requester_principal_id: "admin-a".into(),
        requester_credential_id: None,
        idempotency_scope: "operational_job".into(),
        idempotency_key_digest: SecretDigest::parse("1".repeat(64)).unwrap(),
        request_fingerprint: SecretDigest::parse("2".repeat(64)).unwrap(),
        state: "planned".into(),
        dry_run: false,
        confirmation_required: false,
        confirmation_digest: None,
        confirmation_expires_at: None,
        expected_maintenance_generation: Some(0),
        safe_summary: serde_json::json!({"status":"planned"}),
        artifact_manifest_id: None,
        artifact_manifest_digest: None,
        checkpoint: None,
        execution_scope_digest: SecretDigest::parse("3".repeat(64)).unwrap(),
        expected_adapter_generations: vec![("worktree-a".into(), 7)],
        retry_of_operation_id: None,
    };
    let inserted = insert_or_replay_complete_operational_job(&mut transaction, &input)
        .await
        .unwrap();
    let IdempotencyInsertOutcome::Inserted(inserted) = inserted else {
        panic!("first request must insert");
    };
    assert_eq!(inserted.job.operation_id, "job-complete-a");
    assert_eq!(inserted.idempotency_scope, "operational_job");
    assert_eq!(inserted.expected_adapter_generations, vec![("worktree-a".into(), 7)]);

    let mut replay = input.clone();
    replay.operation_id = "ignored-replay-id".into();
    let replayed = insert_or_replay_complete_operational_job(&mut transaction, &replay)
        .await
        .unwrap();
    let IdempotencyInsertOutcome::Replay(replayed) = replayed else {
        panic!("duplicate request must replay");
    };
    assert_eq!(replayed.job.operation_id, "job-complete-a");
    assert_eq!(replayed.request_fingerprint, input.request_fingerprint);
    assert_eq!(replayed.expected_adapter_generations, input.expected_adapter_generations);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn credential_issuance_duplicate_reports_in_progress_without_waiting_or_mutating() {
    let context = connect_required_test_database_from_env().await.unwrap();
    let pool = context.pool().clone();
    let schema = unique_test_id("issuance-reservation").replace('-', "_");
    sqlx::query(format!("create schema {schema}").as_str())
        .execute(&pool)
        .await
        .unwrap();
    let mut setup = pool.begin().await.unwrap();
    (&mut *setup)
        .execute(format!("set local search_path to {schema}").as_str())
        .await
        .unwrap();
    apply_all(&mut setup).await;
    let now = Utc::now();
    insert_principal(&mut setup, "admin-a", "administrator", "admin", true, now)
        .await
        .unwrap();
    insert_principal(&mut setup, "target-a", "adapter", "worktree_adapter", true, now)
        .await
        .unwrap();
    setup.commit().await.unwrap();

    let input = CredentialIssuanceInput {
        requester_principal_id: "admin-a".into(),
        idempotency_key_digest: SecretDigest::parse("4".repeat(64)).unwrap(),
        request_fingerprint: SecretDigest::parse("5".repeat(64)).unwrap(),
        issuance_operation_id: "issuance-a".into(),
        action: "create".into(),
        target_principal_id: "target-a".into(),
        affected_credential_ids: serde_json::json!(["credential-a"]),
        committed_outcome: "created".into(),
        safe_result: serde_json::json!({
            "credential_id": "credential-a",
            "plaintext_available": false,
            "safe_code": "credential_secret_not_replayable"
        }),
    };

    let mut winner = pool.begin().await.unwrap();
    (&mut *winner)
        .execute(format!("set local search_path to {schema}").as_str())
        .await
        .unwrap();
    assert!(matches!(
        reserve_or_replay_credential_issuance(&mut winner, &input)
            .await
            .unwrap(),
        CredentialIssuanceReservationOutcome::Inserted(_)
    ));

    let mut duplicate = pool.begin().await.unwrap();
    (&mut *duplicate)
        .execute(format!("set local search_path to {schema}").as_str())
        .await
        .unwrap();
    assert!(matches!(
        reserve_or_replay_credential_issuance(&mut duplicate, &input)
            .await
            .unwrap(),
        CredentialIssuanceReservationOutcome::InProgress
    ));
    duplicate.rollback().await.unwrap();
    winner.commit().await.unwrap();

    let mut replay = pool.begin().await.unwrap();
    (&mut *replay)
        .execute(format!("set local search_path to {schema}").as_str())
        .await
        .unwrap();
    assert!(matches!(
        reserve_or_replay_credential_issuance(&mut replay, &input)
            .await
            .unwrap(),
        CredentialIssuanceReservationOutcome::Replay(_)
    ));
    replay.rollback().await.unwrap();
    sqlx::query(format!("drop schema {schema} cascade").as_str())
        .execute(&pool)
        .await
        .unwrap();
}
