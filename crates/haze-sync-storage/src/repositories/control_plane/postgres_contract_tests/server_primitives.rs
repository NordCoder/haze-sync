#[tokio::test]
async fn server_primitives_preserve_generation_and_rotate_one_set_epoch() {
    let context = connect_required_test_database_from_env().await.unwrap();
    let mut transaction = begin_isolated_schema(context.pool()).await;
    apply_all(&mut transaction).await;
    let now = Utc::now();

    let quiescing = cas_update_maintenance_control(
        &mut transaction,
        0,
        &MaintenanceControlUpdate {
            state: "quiescing".into(),
            transition_operation_id: Some("maintenance-a".into()),
            transition_requested_by: Some("admin-a".into()),
            transition_requested_at: Some(now),
            state_entered_at: now,
            admission_fence_closed: true,
            admission_fence_closed_at: Some(now),
            quiescence_evidence_version: 0,
            quiescence_evidence_id: None,
            active_maintenance_job_id: None,
            safe_error_category: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(quiescing.maintenance_generation, 1);
    let quiesced = cas_complete_maintenance_transition(
        &mut transaction,
        1,
        "quiescing",
        &MaintenanceControlUpdate {
            state: "quiesced".into(),
            transition_operation_id: Some("maintenance-a".into()),
            transition_requested_by: Some("admin-a".into()),
            transition_requested_at: Some(now),
            state_entered_at: now,
            admission_fence_closed: true,
            admission_fence_closed_at: Some(now),
            quiescence_evidence_version: 1,
            quiescence_evidence_id: Some("evidence-a".into()),
            active_maintenance_job_id: None,
            safe_error_category: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(quiesced.maintenance_generation, 1);
    assert_eq!(quiesced.state, "quiesced");
    assert_eq!(
        cas_complete_maintenance_transition(
            &mut transaction,
            1,
            "quiescing",
            &MaintenanceControlUpdate {
                state: "quiesced".into(),
                transition_operation_id: None,
                transition_requested_by: None,
                transition_requested_at: None,
                state_entered_at: now,
                admission_fence_closed: true,
                admission_fence_closed_at: Some(now),
                quiescence_evidence_version: 1,
                quiescence_evidence_id: Some("evidence-a".into()),
                active_maintenance_job_id: None,
                safe_error_category: None,
            },
        )
        .await,
        Err(ControlPlaneRepositoryError::StaleVersion {
            namespace: cas_namespaces::MAINTENANCE_GENERATION
        })
    );

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
    let first = insert_credential(
        &mut transaction,
        &NewCredential {
            credential_id: "credential-a".into(),
            principal_id: principal.principal_id.clone(),
            issuance_operation_id: "issuance-a".into(),
            credential_kind: "bearer".into(),
            status: "active".into(),
            verifier_scheme: "argon2id_v1".into(),
            verifier_material: VerifierMaterial::parse("$argon2id$v=19$m=65536,t=3,p=1$c2FsdA$dmVyaWZpZXI").unwrap(),
            created_at: now,
            not_before: now,
            expires_at: None,
            grace_expires_at: None,
            rotated_from_credential_id: None,
        },
        principal.principal_version,
        principal.credential_set_generation,
    )
    .await
    .unwrap();
    let first_grace = cas_update_credential_lifecycle(
        &mut transaction,
        "credential-a",
        first.credential.credential_version,
        first.principal.principal_version,
        first.principal.credential_set_generation,
        &CredentialLifecycleUpdate {
            status: "grace".into(),
            grace_expires_at: Some(now + chrono::Duration::minutes(15)),
            revoked_at: None,
            revoked_by: None,
            expires_at: None,
        },
    )
    .await
    .unwrap();
    let second = insert_credential(
        &mut transaction,
        &NewCredential {
            credential_id: "credential-b".into(),
            principal_id: principal.principal_id.clone(),
            issuance_operation_id: "issuance-b".into(),
            credential_kind: "bearer".into(),
            status: "active".into(),
            verifier_scheme: "argon2id_v1".into(),
            verifier_material: VerifierMaterial::parse("$argon2id$v=19$m=65536,t=3,p=1$c2FsdDI$dmVyaWZpZXIy").unwrap(),
            created_at: now,
            not_before: now,
            expires_at: None,
            grace_expires_at: None,
            rotated_from_credential_id: Some("credential-a".into()),
        },
        first_grace.principal.principal_version,
        first_grace.principal.credential_set_generation,
    )
    .await
    .unwrap();
    let before_principal_version = second.principal.principal_version;
    let before_set_generation = second.principal.credential_set_generation;
    let rotated = rotate_credential_set(
        &mut transaction,
        &CredentialSetRotationInput {
            expected_principal_version: before_principal_version,
            expected_credential_set_generation: before_set_generation,
            new_credential: NewCredential {
                credential_id: "credential-c".into(),
                principal_id: principal.principal_id.clone(),
                issuance_operation_id: "issuance-c".into(),
                credential_kind: "bearer".into(),
                status: "active".into(),
                verifier_scheme: "argon2id_v1".into(),
                verifier_material: VerifierMaterial::parse("$argon2id$v=19$m=65536,t=3,p=1$c2FsdDM$dmVyaWZpZXIz").unwrap(),
                created_at: now,
                not_before: now,
                expires_at: None,
                grace_expires_at: None,
                rotated_from_credential_id: Some("credential-b".into()),
            },
            affected_credentials: vec![
                CredentialRotationMutation {
                    credential_id: "credential-b".into(),
                    expected_credential_version: second.credential.credential_version,
                    expected_status: "active".into(),
                    update: CredentialLifecycleUpdate {
                        status: "grace".into(),
                        grace_expires_at: Some(now + chrono::Duration::minutes(15)),
                        revoked_at: None,
                        revoked_by: None,
                        expires_at: None,
                    },
                },
                CredentialRotationMutation {
                    credential_id: "credential-a".into(),
                    expected_credential_version: first_grace.credential.credential_version,
                    expected_status: "grace".into(),
                    update: CredentialLifecycleUpdate {
                        status: "revoked".into(),
                        grace_expires_at: None,
                        revoked_at: Some(now),
                        revoked_by: Some("admin-a".into()),
                        expires_at: None,
                    },
                },
            ],
        },
    )
    .await
    .unwrap();
    assert_eq!(rotated.principal.principal_version, before_principal_version + 1);
    assert_eq!(
        rotated.principal.credential_set_generation,
        before_set_generation + 1
    );
    assert_eq!(rotated.new_credential.status, "active");
    assert_eq!(
        rotated
            .affected_credentials
            .iter()
            .find(|row| row.credential_id == "credential-a")
            .unwrap()
            .status,
        "revoked"
    );
    assert_eq!(
        rotated
            .affected_credentials
            .iter()
            .find(|row| row.credential_id == "credential-b")
            .unwrap()
            .status,
        "grace"
    );

    let listed = read_credentials_by_principal(&mut transaction, "admin-a")
        .await
        .unwrap();
    assert_eq!(
        listed
            .iter()
            .map(|row| row.credential_id.as_str())
            .collect::<Vec<_>>(),
        vec!["credential-a", "credential-b", "credential-c"]
    );
    let encoded = credential_verifier_material_for_private_authentication(
        listed
            .iter()
            .find(|row| row.credential_id == "credential-c")
            .unwrap(),
    );
    assert!(encoded.starts_with("$argon2id$"));
    assert!(!format!("{:?}", listed.last().unwrap()).contains(encoded));

    assert_eq!(
        rotate_credential_set(
            &mut transaction,
            &CredentialSetRotationInput {
                expected_principal_version: before_principal_version,
                expected_credential_set_generation: before_set_generation,
                new_credential: NewCredential {
                    credential_id: "credential-stale".into(),
                    principal_id: principal.principal_id,
                    issuance_operation_id: "issuance-stale".into(),
                    credential_kind: "bearer".into(),
                    status: "active".into(),
                    verifier_scheme: "argon2id_v1".into(),
                    verifier_material: VerifierMaterial::parse("redacted-stale-verifier").unwrap(),
                    created_at: now,
                    not_before: now,
                    expires_at: None,
                    grace_expires_at: None,
                    rotated_from_credential_id: Some("credential-c".into()),
                },
                affected_credentials: vec![CredentialRotationMutation {
                    credential_id: "credential-c".into(),
                    expected_credential_version: 1,
                    expected_status: "active".into(),
                    update: CredentialLifecycleUpdate {
                        status: "revoked".into(),
                        grace_expires_at: None,
                        revoked_at: Some(now),
                        revoked_by: Some("admin-a".into()),
                        expires_at: None,
                    },
                }],
            },
        )
        .await,
        Err(ControlPlaneRepositoryError::StaleVersion {
            namespace: cas_namespaces::PRINCIPAL_VERSION
        })
    );
    assert!(read_credential_by_id(&mut transaction, "credential-stale")
        .await
        .unwrap()
        .is_none());
    transaction.rollback().await.unwrap();
}
