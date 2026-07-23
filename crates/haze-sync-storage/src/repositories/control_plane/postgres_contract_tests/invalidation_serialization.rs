#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn committed_invalidation_serializes_before_complete_evidence_admission() {
    let context = connect_required_test_database_from_env().await.unwrap();
    let pool = context.pool().clone();
    let schema = unique_test_id("evidence-invalidation").replace('-', "_");
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
    let now = exact_test_time();
    sqlx::query(
        "insert into quiescence_evidence (quiescence_evidence_id, evidence_version, \
         maintenance_generation, adapter_inventory_generation, admission_fence_closed_at, \
         server_instance_id, server_started_at, active_authoritative_mutations, \
         open_authoritative_transactions, obsidian_authoritative_mutation_gate, \
         active_cli_write_jobs, object_store_writer_count, database_writer_count, \
         worktree_external_writer_evidence, evidence_digest, captured_at) \
         values ('evidence-race',1,0,0,$1,'server-a',$1,0,0,'closed',0,0,0, \
         '{\"exclusive_writer_proven\":true}'::jsonb,$2,$1)",
    )
    .bind(now)
    .bind("a".repeat(64))
    .execute(&mut *setup)
    .await
    .unwrap();
    setup.commit().await.unwrap();

    let mut invalidation = pool.begin().await.unwrap();
    (&mut *invalidation)
        .execute(format!("set local search_path to {schema}").as_str())
        .await
        .unwrap();
    append_quiescence_evidence_invalidation(
        &mut invalidation,
        &QuiescenceEvidenceInvalidationInput {
            invalidation_id: "invalidation-race".into(),
            quiescence_evidence_id: "evidence-race".into(),
            maintenance_generation: 0,
            invalidation_reason: "writer-fact-changed".into(),
        },
    )
    .await
    .unwrap();

    let reader_pool = pool.clone();
    let reader_schema = schema.clone();
    let reader = tokio::spawn(async move {
        let mut transaction = reader_pool.begin().await.unwrap();
        (&mut *transaction)
            .execute(format!("set local search_path to {reader_schema}").as_str())
            .await
            .unwrap();
        let bundle = lock_complete_quiescence_evidence(&mut transaction, "evidence-race")
            .await
            .unwrap();
        transaction.rollback().await.unwrap();
        bundle
    });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert!(
        !reader.is_finished(),
        "complete evidence admission did not wait for the invalidation parent lock"
    );

    invalidation.commit().await.unwrap();
    let bundle = reader.await.unwrap();
    assert_eq!(bundle.invalidations.len(), 1);
    assert_eq!(
        bundle.invalidations[0].invalidation_id,
        "invalidation-race"
    );

    sqlx::query(format!("drop schema {schema} cascade").as_str())
        .execute(&pool)
        .await
        .unwrap();
}
