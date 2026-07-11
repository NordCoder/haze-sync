use super::*;
use crate::test_support::prepare_test_database_from_env;
use serde_json::json;

#[tokio::test]
#[ignore = "requires explicit HAZE_SYNC_TEST_DATABASE_URL for mandatory STOR-P10 evidence"]
async fn exact_cursor_progression_is_locked_contiguous_and_rollback_safe() {
    let context = prepare_test_database_from_env().await.unwrap();
    context.clean_storage_tables().await.unwrap();

    let adapter_id = AdapterId::parse(&context.namespace().adapter_id("worktree-cursor")).unwrap();
    let missing_id = AdapterId::parse(&context.namespace().adapter_id("missing-cursor")).unwrap();
    for adapter in [&adapter_id, &missing_id] {
        sqlx::query(
            "insert into sync_adapters (adapter_id, display_name, role, token_hash) \
             values ($1, $2, 'worktree', 'sha256:test-token-hash')",
        )
        .bind(adapter.as_str())
        .bind(format!("{} test adapter", adapter.as_str()))
        .execute(context.pool())
        .await
        .unwrap();
    }

    let repository = AdapterCursorRepository::new();
    let mut transaction = context.pool().begin().await.unwrap();
    let initialized = repository
        .initialize_and_lock(&mut transaction, &adapter_id)
        .await
        .unwrap();
    assert_eq!(initialized.last_core_seq, 0);
    assert!(!initialized.has_external_cursor);
    assert_eq!(
        repository
            .lock_current(&mut transaction, &adapter_id)
            .await
            .unwrap(),
        Some(initialized)
    );
    assert_eq!(
        repository
            .advance_exact_contiguous(&mut transaction, &missing_id, 0, 1)
            .await,
        Err(RepositoryError::CursorMissing)
    );
    assert_eq!(
        repository
            .advance_exact_contiguous(&mut transaction, &adapter_id, 0, 2)
            .await,
        Err(RepositoryError::CursorGap)
    );
    assert_eq!(
        repository
            .advance_exact_contiguous(&mut transaction, &adapter_id, 0, 0)
            .await,
        Err(RepositoryError::CursorRegression)
    );
    let advanced = repository
        .advance_exact_contiguous(&mut transaction, &adapter_id, 0, 1)
        .await
        .unwrap();
    assert_eq!(advanced.last_core_seq, 1);
    assert!(advanced.last_success_at.is_some());
    transaction.commit().await.unwrap();

    let mut stale = context.pool().begin().await.unwrap();
    assert_eq!(
        repository
            .advance_exact_contiguous(&mut stale, &adapter_id, 0, 1)
            .await,
        Err(RepositoryError::CursorStaleExpected)
    );
    stale.rollback().await.unwrap();

    let mut rollback = context.pool().begin().await.unwrap();
    assert_eq!(
        repository
            .advance_exact_contiguous(&mut rollback, &adapter_id, 1, 2)
            .await
            .unwrap()
            .last_core_seq,
        2
    );
    rollback.rollback().await.unwrap();
    assert_eq!(
        repository
            .get_by_adapter_id(context.pool(), &adapter_id)
            .await
            .unwrap()
            .unwrap()
            .last_core_seq,
        1
    );

    let mut winner = context.pool().begin().await.unwrap();
    repository
        .lock_current(&mut winner, &adapter_id)
        .await
        .unwrap()
        .unwrap();
    let pool = context.pool().clone();
    let racing_adapter = adapter_id.clone();
    let loser = tokio::spawn(async move {
        let repository = AdapterCursorRepository::new();
        let mut transaction = pool.begin().await.unwrap();
        let result = repository
            .advance_exact_contiguous(&mut transaction, &racing_adapter, 1, 2)
            .await;
        transaction.rollback().await.unwrap();
        result
    });

    let winner_result = repository
        .advance_exact_contiguous(&mut winner, &adapter_id, 1, 2)
        .await
        .unwrap();
    assert_eq!(winner_result.last_core_seq, 2);
    winner.commit().await.unwrap();
    assert_eq!(
        loser.await.unwrap(),
        Err(RepositoryError::CursorStaleExpected)
    );

    let internal = repository
        .get_by_adapter_id(context.pool(), &adapter_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(internal.last_core_seq, 2);
    assert_eq!(internal.external_cursor_json, json!({}));
    let summary = AdapterCursorSummary::from(&internal);
    let rendered = serde_json::to_string(&summary).unwrap();
    assert!(!rendered.contains("external_cursor_json"));
    assert!(!rendered.contains("page_token"));

    context.clean_storage_tables().await.unwrap();
}
