#[tokio::test]
async fn credential_slot_audit_and_savepoint_composition_are_transactional() {
    let context = connect_required_test_database_from_env().await.unwrap();
    let mut transaction = begin_isolated_schema(context.pool()).await;
    apply_all(&mut transaction).await;
    let now = Utc::now();

    let principal = insert_principal(
        &mut transaction,
        "admin-a",
        "administrator",
        "admin",
        true,
        now,
    )
    .await
    .unwrap();
    let role_changed = cas_update_principal(
        &mut transaction,
        "admin-a",
        principal.principal_version,
        principal.credential_set_generation,
        &PrincipalUpdate {
            role: "operator".into(),
            principal_enabled: true,
            disabled_at: None,
            advance_credential_set_generation: false,
        },
    )
    .await
    .unwrap();
    assert_eq!(role_changed.principal_version, 2);
    assert_eq!(role_changed.credential_set_generation, 2);
    assert_eq!(
        cas_update_principal(
            &mut transaction,
            "admin-a",
            principal.principal_version,
            principal.credential_set_generation,
            &PrincipalUpdate {
                role: "viewer".into(),
                principal_enabled: true,
                disabled_at: None,
                advance_credential_set_generation: false,
            },
        )
        .await,
        Err(ControlPlaneRepositoryError::StaleVersion {
            namespace: cas_namespaces::PRINCIPAL_VERSION
        })
    );

    let legacy = NewCredential {
        credential_id: "credential-legacy-forbidden".into(),
        principal_id: "admin-a".into(),
        issuance_operation_id: "issuance-legacy-forbidden".into(),
        credential_kind: "bearer".into(),
        status: "active".into(),
        verifier_scheme: "legacy_sha256_v0".into(),
        verifier_material: VerifierMaterial::parse("d".repeat(64)).unwrap(),
        created_at: now,
        not_before: now,
        expires_at: None,
        grace_expires_at: None,
        rotated_from_credential_id: None,
    };
    assert_eq!(
        insert_credential(
            &mut transaction,
            &legacy,
            role_changed.principal_version,
            role_changed.credential_set_generation,
        )
        .await,
        Err(ControlPlaneRepositoryError::LegacyVerifierCreationForbidden)
    );

    let created = insert_credential(
        &mut transaction,
        &NewCredential {
            credential_id: "credential-a".into(),
            principal_id: "admin-a".into(),
            issuance_operation_id: "issuance-a".into(),
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
        },
        role_changed.principal_version,
        role_changed.credential_set_generation,
    )
    .await
    .unwrap();
    assert_eq!(created.principal.principal_version, 3);
    assert_eq!(created.principal.credential_set_generation, 3);
    assert!(!format!("{:?}", created.credential).contains("dmVyaWZpZXI"));

    let issuance = CredentialIssuanceInput {
        requester_principal_id: "admin-a".into(),
        idempotency_key_digest: SecretDigest::parse("1".repeat(64)).unwrap(),
        request_fingerprint: SecretDigest::parse("2".repeat(64)).unwrap(),
        issuance_operation_id: "issuance-a".into(),
        action: "create".into(),
        target_principal_id: "admin-a".into(),
        affected_credential_ids: serde_json::json!(["credential-a"]),
        committed_outcome: "created".into(),
        safe_result: serde_json::json!({
            "credential_id":"credential-a",
            "plaintext_available":false
        }),
    };
    assert!(matches!(
        insert_or_replay_credential_issuance(&mut transaction, &issuance)
            .await
            .unwrap(),
        IdempotencyInsertOutcome::Inserted(_)
    ));
    assert!(matches!(
        insert_or_replay_credential_issuance(&mut transaction, &issuance)
            .await
            .unwrap(),
        IdempotencyInsertOutcome::Replay(_)
    ));
    let mut issuance_conflict = issuance.clone();
    issuance_conflict.request_fingerprint = SecretDigest::parse("3".repeat(64)).unwrap();
    assert_eq!(
        insert_or_replay_credential_issuance(&mut transaction, &issuance_conflict).await,
        Err(ControlPlaneRepositoryError::IdempotencyConflict)
    );

    let job = OperationalJobInput {
        operation_id: "job-slot-a".into(),
        kind: "restore".into(),
        maintenance_required: true,
        destructive: true,
        requester_principal_id: "admin-a".into(),
        requester_credential_id: Some("credential-a".into()),
        idempotency_scope: "operational_job".into(),
        idempotency_key_digest: SecretDigest::parse("4".repeat(64)).unwrap(),
        request_fingerprint: SecretDigest::parse("5".repeat(64)).unwrap(),
        state: "planned".into(),
        dry_run: false,
        confirmation_required: true,
        confirmation_digest: Some(SecretDigest::parse("6".repeat(64)).unwrap()),
        confirmation_expires_at: Some(now + chrono::Duration::minutes(5)),
        expected_maintenance_generation: Some(0),
        safe_summary: serde_json::json!({"status":"planned"}),
        artifact_manifest_id: Some("manifest-a".into()),
        artifact_manifest_digest: Some(SecretDigest::parse("7".repeat(64)).unwrap()),
        checkpoint: None,
        execution_scope_digest: SecretDigest::parse("8".repeat(64)).unwrap(),
        expected_adapter_generations: Vec::new(),
        retry_of_operation_id: None,
    };
    assert!(matches!(
        insert_or_replay_operational_job(&mut transaction, &job)
            .await
            .unwrap(),
        IdempotencyInsertOutcome::Inserted(_)
    ));
    let acquired = cas_acquire_execution_slot(
        &mut transaction,
        "global-destructive",
        0,
        "job-slot-a",
        Some(0),
        1,
    )
    .await
    .unwrap();
    assert_eq!(acquired.slot_version, 1);
    assert_eq!(
        cas_acquire_execution_slot(
            &mut transaction,
            "global-destructive",
            acquired.slot_version,
            "job-slot-a",
            Some(0),
            1,
        )
        .await,
        Err(ControlPlaneRepositoryError::OperationInProgress)
    );
    let blocked = cas_block_execution_slot_uncertain(
        &mut transaction,
        "global-destructive",
        acquired.slot_version,
        "job-slot-a",
        1,
    )
    .await
    .unwrap();
    assert!(blocked.blocked_uncertain);
    assert_eq!(
        cas_release_execution_slot(
            &mut transaction,
            "global-destructive",
            blocked.slot_version,
            "job-slot-a",
            1,
        )
        .await,
        Err(ControlPlaneRepositoryError::OperationInProgress)
    );
    let reconciled = cas_reconcile_execution_slot_uncertainty(
        &mut transaction,
        "global-destructive",
        blocked.slot_version,
        "job-slot-a",
        1,
    )
    .await
    .unwrap();
    let released = cas_release_execution_slot(
        &mut transaction,
        "global-destructive",
        reconciled.slot_version,
        "job-slot-a",
        1,
    )
    .await
    .unwrap();
    assert!(released.operation_id.is_none());
    assert!(released.acquired_at.is_none());

    append_operational_audit_event(
        &mut transaction,
        &OperationalAuditEventInput {
            audit_id: "audit-a".into(),
            occurred_at: now,
            event_type: "credential_created".into(),
            outcome: "committed".into(),
            actor_principal_id: Some("admin-a".into()),
            actor_credential_id: Some("credential-a".into()),
            actor_role: Some("operator".into()),
            actor_origin: "test".into(),
            request_id: Some("request-a".into()),
            correlation_id: None,
            target_type: "credential".into(),
            target_id: Some("credential-a".into()),
            operation_id: Some("job-slot-a".into()),
            maintenance_generation: Some(0),
            adapter_control_generation: None,
            principal_version: Some(3),
            credential_set_generation: Some(3),
            credential_version: Some(1),
            job_version: Some(1),
            executor_fence: Some(1),
            runtime_lease_version: None,
            standalone_runtime_epoch: None,
            runtime_report_sequence: None,
            issuance_operation_id: Some("issuance-a".into()),
            previous_state: None,
            next_state: Some("active".into()),
            safe_error_category: None,
            artifact_manifest_id: Some("manifest-a".into()),
            safe_metadata: serde_json::json!({"plaintext_available":false}),
            corrects_audit_id: None,
        },
    )
    .await
    .unwrap();
    (&mut *transaction)
        .execute("savepoint audit_immutable")
        .await
        .unwrap();
    assert!(sqlx::query("update operational_audit_events set outcome='changed' where audit_id='audit-a'")
        .execute(&mut *transaction)
        .await
        .is_err());
    (&mut *transaction)
        .execute("rollback to savepoint audit_immutable")
        .await
        .unwrap();

    (&mut *transaction)
        .execute("savepoint caller_composition")
        .await
        .unwrap();
    insert_principal(
        &mut transaction,
        "temporary-principal",
        "administrator",
        "admin",
        true,
        now,
    )
    .await
    .unwrap();
    append_operational_audit_event(
        &mut transaction,
        &OperationalAuditEventInput {
            audit_id: "temporary-audit".into(),
            occurred_at: now,
            event_type: "principal_created".into(),
            outcome: "committed".into(),
            actor_principal_id: Some("temporary-principal".into()),
            actor_credential_id: None,
            actor_role: Some("admin".into()),
            actor_origin: "test".into(),
            request_id: None,
            correlation_id: None,
            target_type: "principal".into(),
            target_id: Some("temporary-principal".into()),
            operation_id: None,
            maintenance_generation: None,
            adapter_control_generation: None,
            principal_version: Some(1),
            credential_set_generation: Some(1),
            credential_version: None,
            job_version: None,
            executor_fence: None,
            runtime_lease_version: None,
            standalone_runtime_epoch: None,
            runtime_report_sequence: None,
            issuance_operation_id: None,
            previous_state: None,
            next_state: Some("enabled".into()),
            safe_error_category: None,
            artifact_manifest_id: None,
            safe_metadata: serde_json::json!({}),
            corrects_audit_id: None,
        },
    )
    .await
    .unwrap();
    (&mut *transaction)
        .execute("rollback to savepoint caller_composition")
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "select count(*)::bigint from principals where principal_id='temporary-principal'",
        )
        .fetch_one(&mut *transaction)
        .await
        .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "select count(*)::bigint from operational_audit_events where audit_id='temporary-audit'",
        )
        .fetch_one(&mut *transaction)
        .await
        .unwrap(),
        0
    );
    transaction.rollback().await.unwrap();
}
