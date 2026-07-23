#[tokio::test]
async fn stale_cas_and_caller_rollback_leave_no_partial_control_change() {
    let context = connect_required_test_database_from_env().await.unwrap();
    let mut transaction = begin_isolated_schema(context.pool()).await;
    apply_all(&mut transaction).await;
    let now = Utc::now();
    let update = MaintenanceControlUpdate {
        state: "quiescing".into(),
        transition_operation_id: Some("transition-a".into()),
        transition_requested_by: Some("principal-a".into()),
        transition_requested_at: Some(now),
        state_entered_at: now,
        admission_fence_closed: true,
        admission_fence_closed_at: Some(now),
        quiescence_evidence_version: 0,
        quiescence_evidence_id: None,
        active_maintenance_job_id: None,
        safe_error_category: None,
    };
    let changed = cas_update_maintenance_control(&mut transaction, 0, &update)
        .await
        .unwrap();
    assert_eq!(changed.maintenance_generation, 1);
    assert_eq!(
        cas_update_maintenance_control(&mut transaction, 0, &update).await,
        Err(ControlPlaneRepositoryError::StaleVersion {
            namespace: cas_namespaces::MAINTENANCE_GENERATION
        })
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn idempotency_and_immutable_terminal_records_are_enforced() {
    let context = connect_required_test_database_from_env().await.unwrap();
    let mut transaction = begin_isolated_schema(context.pool()).await;
    apply_all(&mut transaction).await;
    let now = Utc::now();
    insert_principal(&mut transaction, "admin-a", "administrator", "admin", true, now)
        .await
        .unwrap();
    let input = OperationalJobInput {
        operation_id: "job-a".into(),
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
        expected_adapter_generations: vec![],
        retry_of_operation_id: None,
    };
    assert!(matches!(
        insert_or_replay_operational_job(&mut transaction, &input)
            .await
            .unwrap(),
        IdempotencyInsertOutcome::Inserted(_)
    ));
    let mut replay = input.clone();
    replay.operation_id = "job-ignored".into();
    assert!(matches!(
        insert_or_replay_operational_job(&mut transaction, &replay)
            .await
            .unwrap(),
        IdempotencyInsertOutcome::Replay(row) if row.operation_id == "job-a"
    ));
    replay.request_fingerprint = SecretDigest::parse("4".repeat(64)).unwrap();
    assert_eq!(
        insert_or_replay_operational_job(&mut transaction, &replay).await,
        Err(ControlPlaneRepositoryError::IdempotencyConflict)
    );
    (&mut *transaction).execute("savepoint no_delete").await.unwrap();
    assert!(sqlx::query("delete from maintenance_control where singleton_id=1")
        .execute(&mut *transaction)
        .await
        .is_err());
    (&mut *transaction)
        .execute("rollback to savepoint no_delete")
        .await
        .unwrap();
    sqlx::query(
        "update operational_jobs set state='succeeded', completed_at=now(), job_version=2 \
         where operation_id='job-a'",
    )
    .execute(&mut *transaction)
    .await
    .unwrap();
    (&mut *transaction).execute("savepoint terminal_immutable").await.unwrap();
    assert!(sqlx::query("update operational_jobs set safe_summary='{}' where operation_id='job-a'")
        .execute(&mut *transaction)
        .await
        .is_err());
    (&mut *transaction)
        .execute("rollback to savepoint terminal_immutable")
        .await
        .unwrap();
    transaction.rollback().await.unwrap();
}

