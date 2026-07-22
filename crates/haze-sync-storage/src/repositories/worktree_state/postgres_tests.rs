use super::*;
use crate::test_support::prepare_test_database_from_env;
use chrono::TimeZone;

#[tokio::test]
#[ignore = "requires explicit HAZE_SYNC_TEST_DATABASE_URL for mandatory STOR-P10 evidence"]
async fn durable_instances_and_path_state_are_isolated_and_transactional() {
    let context = prepare_test_database_from_env().await.unwrap();
    context.clean_storage_tables().await.unwrap();

    let adapter_a = AdapterId::parse(&context.namespace().adapter_id("worktree-a")).unwrap();
    let adapter_b = AdapterId::parse(&context.namespace().adapter_id("worktree-b")).unwrap();
    let hash_a = ContentHash::parse(&"a".repeat(64)).unwrap();
    let hash_b = ContentHash::parse(&"b".repeat(64)).unwrap();
    let fingerprint_a = Sha256::parse(&"c".repeat(64)).unwrap();
    let fingerprint_b = Sha256::parse(&"d".repeat(64)).unwrap();
    let revision_a = RevisionId::parse(&format!(
        "rev_{}",
        context.namespace().child_id("worktree-revision-a")
    ))
    .unwrap();
    let revision_b = RevisionId::parse(&format!(
        "rev_{}",
        context.namespace().child_id("worktree-revision-b")
    ))
    .unwrap();
    let path_a = VaultPath::parse(&context.namespace().vault_path("a-state.md")).unwrap();
    let path_b = VaultPath::parse(&context.namespace().vault_path("b-state.md")).unwrap();
    let rollback_path =
        VaultPath::parse(&context.namespace().vault_path("rollback-state.md")).unwrap();

    seed_authoritative_facts(
        context.pool(),
        [&adapter_a, &adapter_b],
        [(&revision_a, hash_a), (&revision_b, hash_b)],
        &path_a,
    )
    .await;

    let mut transaction = context.pool().begin().await.unwrap();
    let binding_a = WorktreeInstanceBinding::new(adapter_a.clone(), fingerprint_a);
    let binding_b = WorktreeInstanceBinding::new(adapter_b.clone(), fingerprint_b);

    let first_bind = bind_or_verify_worktree_instance(&mut *transaction, &binding_a)
        .await
        .unwrap();
    let repeated_bind = bind_or_verify_worktree_instance(&mut *transaction, &binding_a)
        .await
        .unwrap();
    assert_eq!(first_bind, repeated_bind);
    assert_eq!(first_bind.adapter_id, adapter_a.as_str());
    assert_eq!(
        bind_or_verify_worktree_instance(
            &mut *transaction,
            &WorktreeInstanceBinding::new(
                adapter_a.clone(),
                Sha256::parse(&"e".repeat(64)).unwrap(),
            ),
        )
        .await,
        Err(RepositoryError::WorktreeInstanceBindingMismatch)
    );
    bind_or_verify_worktree_instance(&mut *transaction, &binding_b)
        .await
        .unwrap();

    let observed_at = Utc.timestamp_opt(1_700_100_000, 0).unwrap();
    let state_a = upsert_present_state(
        &mut *transaction,
        &WorktreePresentStateUpsert {
            adapter_id: &adapter_a,
            path: &path_a,
            last_applied_revision_id: &revision_a,
            content_hash: hash_a,
            observation: Some(WorktreeReconciliationObservation::new(
                42,
                Some(observed_at),
            )),
        },
    )
    .await
    .unwrap();
    let state_b = upsert_present_state(
        &mut *transaction,
        &WorktreePresentStateUpsert {
            adapter_id: &adapter_b,
            path: &path_a,
            last_applied_revision_id: &revision_b,
            content_hash: hash_b,
            observation: None,
        },
    )
    .await
    .unwrap();
    upsert_tombstoned_state(
        &mut *transaction,
        &WorktreeTombstonedStateUpsert {
            adapter_id: &adapter_a,
            path: &path_b,
            last_applied_revision_id: &revision_b,
        },
    )
    .await
    .unwrap();

    assert_eq!(state_a.adapter_id, adapter_a.as_str());
    assert_eq!(state_b.adapter_id, adapter_b.as_str());
    assert_ne!(state_a.content_sha256, state_b.content_sha256);
    assert_eq!(
        load_path_state(&mut *transaction, &adapter_a, &path_a)
            .await
            .unwrap(),
        Some(state_a.clone())
    );
    assert_eq!(
        load_path_state(&mut *transaction, &adapter_b, &path_a)
            .await
            .unwrap(),
        Some(state_b)
    );

    let first_page = load_snapshot(&mut *transaction, &adapter_a, None, 1)
        .await
        .unwrap();
    assert_eq!(first_page.states.len(), 1);
    assert_eq!(first_page.states[0].path, path_a.as_str());
    assert_eq!(first_page.next_after_path.as_ref(), Some(&path_a));
    let second_page = load_snapshot(
        &mut *transaction,
        &adapter_a,
        first_page.next_after_path.as_ref(),
        1,
    )
    .await
    .unwrap();
    assert_eq!(second_page.states.len(), 1);
    assert_eq!(second_page.states[0].path, path_b.as_str());
    assert!(second_page.next_after_path.is_none());

    assert!(update_reconciliation_observation(
        &mut *transaction,
        &WorktreeObservationUpdate {
            adapter_id: &adapter_a,
            path: &path_a,
            expected_revision_id: &revision_b,
            expected_content_hash: hash_a,
            observation: WorktreeReconciliationObservation::new(43, None),
        },
    )
    .await
    .unwrap()
    .is_none());
    let updated = update_reconciliation_observation(
        &mut *transaction,
        &WorktreeObservationUpdate {
            adapter_id: &adapter_a,
            path: &path_a,
            expected_revision_id: &revision_a,
            expected_content_hash: hash_a,
            observation: WorktreeReconciliationObservation::new(43, None),
        },
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(updated.observed_size_bytes, Some(43));

    transaction.commit().await.unwrap();

    let mut rollback = context.pool().begin().await.unwrap();
    upsert_present_state(
        &mut *rollback,
        &WorktreePresentStateUpsert {
            adapter_id: &adapter_a,
            path: &rollback_path,
            last_applied_revision_id: &revision_a,
            content_hash: hash_a,
            observation: None,
        },
    )
    .await
    .unwrap();
    rollback.rollback().await.unwrap();

    assert!(load_path_state(context.pool(), &adapter_a, &rollback_path)
        .await
        .unwrap()
        .is_none());

    context.clean_storage_tables().await.unwrap();
}

async fn seed_authoritative_facts(
    pool: &sqlx::PgPool,
    adapters: [&AdapterId; 2],
    revisions: [(&RevisionId, ContentHash); 2],
    path: &VaultPath,
) {
    let mut transaction = pool.begin().await.unwrap();
    for adapter in adapters {
        sqlx::query(
            "insert into sync_adapters (adapter_id, display_name, role, token_hash) \
             values ($1, $2, 'worktree', 'sha256:test-token-hash')",
        )
        .bind(adapter.as_str())
        .bind(format!("{} test adapter", adapter.as_str()))
        .execute(&mut *transaction)
        .await
        .unwrap();
    }

    for (_, hash) in revisions {
        sqlx::query(
            "insert into content_blobs (sha256, size_bytes, object_store_path) \
             values ($1, 42, $2)",
        )
        .bind(hash.to_string())
        .bind(format!("sha256/{}/fixture", &hash.as_hex()[..2]))
        .execute(&mut *transaction)
        .await
        .unwrap();
    }

    let object_id = "obj_stor_p10_fixture";
    sqlx::query(
        "insert into sync_objects (object_id, path, kind, updated_by) \
         values ($1, $2, 'file', $3)",
    )
    .bind(object_id)
    .bind(path.as_str())
    .bind(adapters[0].as_str())
    .execute(&mut *transaction)
    .await
    .unwrap();

    for (revision, hash) in revisions {
        sqlx::query(
            "insert into file_revisions ( \
                 revision_id, object_id, path, content_sha256, size_bytes, created_by \
             ) values ($1, $2, $3, $4, 42, $5)",
        )
        .bind(revision.as_str())
        .bind(object_id)
        .bind(path.as_str())
        .bind(hash.to_string())
        .bind(adapters[0].as_str())
        .execute(&mut *transaction)
        .await
        .unwrap();
    }

    transaction.commit().await.unwrap();
}
