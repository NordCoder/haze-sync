#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn concurrent_maintenance_cas_allows_exactly_one_commit() {
    let context = connect_required_test_database_from_env().await.unwrap();
    let pool = context.pool().clone();
    let schema = unique_test_id("control-cas").replace('-', "_");
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
    setup.commit().await.unwrap();

    let marker_identity = format!("haze-sync-control-cas:{schema}");
    let first_pool = pool.clone();
    let first_schema = schema.clone();
    let first_marker = marker_identity.clone();
    let first = tokio::spawn(async move {
        let mut transaction = first_pool.begin().await.unwrap();
        (&mut *transaction)
            .execute(format!("set local search_path to {first_schema}").as_str())
            .await
            .unwrap();
        lock_maintenance_control(&mut transaction).await.unwrap();
        sqlx::query("select pg_advisory_xact_lock(hashtextextended($1, 0))")
            .bind(&first_marker)
            .execute(&mut *transaction)
            .await
            .unwrap();
        sqlx::query("select pg_sleep(0.2)")
            .execute(&mut *transaction)
            .await
            .unwrap();
        let now = Utc::now();
        let result = cas_update_maintenance_control(
            &mut transaction,
            0,
            &MaintenanceControlUpdate {
                state: "quiescing".into(),
                transition_operation_id: Some("first".into()),
                transition_requested_by: Some("admin".into()),
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
        .await;
        transaction.commit().await.unwrap();
        result
    });

    let mut marker_seen = false;
    for _ in 0..400 {
        let acquired: bool = sqlx::query_scalar(
            "select pg_try_advisory_xact_lock(hashtextextended($1, 0))",
        )
        .bind(&marker_identity)
        .fetch_one(&pool)
        .await
        .unwrap();
        if acquired {
            std::thread::sleep(std::time::Duration::from_millis(5));
        } else {
            marker_seen = true;
            break;
        }
    }
    assert!(marker_seen, "first CAS transaction did not acquire its marker");

    let second_pool = pool.clone();
    let second_schema = schema.clone();
    let second = tokio::spawn(async move {
        let mut transaction = second_pool.begin().await.unwrap();
        (&mut *transaction)
            .execute(format!("set local search_path to {second_schema}").as_str())
            .await
            .unwrap();
        let now = Utc::now();
        let result = cas_update_maintenance_control(
            &mut transaction,
            0,
            &MaintenanceControlUpdate {
                state: "quiescing".into(),
                transition_operation_id: Some("second".into()),
                transition_requested_by: Some("admin".into()),
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
        .await;
        transaction.rollback().await.unwrap();
        result
    });

    let first_result = first.await.unwrap();
    let second_result = second.await.unwrap();
    assert!(first_result.is_ok());
    assert_eq!(
        second_result,
        Err(ControlPlaneRepositoryError::StaleVersion {
            namespace: cas_namespaces::MAINTENANCE_GENERATION
        })
    );
    let generation: i64 = sqlx::query_scalar(
        format!("select maintenance_generation from {schema}.maintenance_control").as_str(),
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(generation, 1);
    sqlx::query(format!("drop schema {schema} cascade").as_str())
        .execute(&pool)
        .await
        .unwrap();
}
