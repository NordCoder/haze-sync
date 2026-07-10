use super::*;
use crate::test_support::connect_test_database_from_env;
use serde_json::json;

#[tokio::test]
async fn cursor_updates_roundtrip_and_reject_regression_in_caller_transaction() {
    let Some(context) = connect_test_database_from_env().await.unwrap() else {
        return;
    };
    context.apply_migrations().await.unwrap();
    context.clean_storage_tables().await.unwrap();

    let adapter_id = AdapterId::parse("gdrive-adapter").unwrap();
    let second_adapter_id = AdapterId::parse("worktree-adapter").unwrap();
    let repository = AdapterCursorRepository::new();
    let mut tx = context.pool().begin().await.unwrap();

    for adapter in [&adapter_id, &second_adapter_id] {
        sqlx::query(
            "insert into sync_adapters (adapter_id, display_name, role, token_hash) \
             values ($1, $2, $3, $4)",
        )
        .bind(adapter.as_str())
        .bind(format!("{} test adapter", adapter.as_str()))
        .bind("test")
        .bind("sha256:test-token-hash")
        .execute(&mut *tx)
        .await
        .unwrap();
    }

    let first_update = AdapterCursorUpdate {
        adapter_id: adapter_id.clone(),
        last_core_seq: 7,
        external_cursor_json: Some(json!({ "page_token": "opaque-test-token" })),
        mark_success: true,
    };
    let advanced = match repository
        .update_monotonic(&mut *tx, &first_update)
        .await
        .unwrap()
    {
        AdapterCursorUpdateOutcome::Updated(row) => row,
        AdapterCursorUpdateOutcome::RejectedRegression { .. } => {
            panic!("first cursor update must initialize and advance the row")
        }
    };
    assert_eq!(advanced.last_core_seq, 7);
    assert_eq!(
        advanced.external_cursor_json,
        json!({ "page_token": "opaque-test-token" })
    );
    assert!(advanced.last_success_at.is_some());

    let loaded = repository
        .get_by_adapter_id(&mut *tx, &adapter_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(loaded, advanced);

    let regression = repository
        .update_monotonic(
            &mut *tx,
            &AdapterCursorUpdate {
                adapter_id: adapter_id.clone(),
                last_core_seq: 6,
                external_cursor_json: Some(json!({ "page_token": "must-not-win" })),
                mark_success: false,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        regression,
        AdapterCursorUpdateOutcome::RejectedRegression {
            current: advanced.clone(),
            requested_seq: 6,
        }
    );

    let after_regression = repository
        .get_by_adapter_id(&mut *tx, &adapter_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(after_regression, advanced);
    let summary = AdapterCursorSummary::from(&after_regression);
    let summary_json = serde_json::to_string(&summary).unwrap();
    assert!(summary.has_external_cursor);
    assert!(!summary_json.contains("opaque-test-token"));
    assert!(!summary_json.contains("page_token"));

    let initialized = repository
        .initialize_if_missing(&mut *tx, &second_adapter_id)
        .await
        .unwrap();
    assert_eq!(initialized.last_core_seq, 0);
    assert_eq!(initialized.external_cursor_json, json!({}));
    assert!(initialized.last_success_at.is_none());
    assert_eq!(
        repository
            .initialize_if_missing(&mut *tx, &second_adapter_id)
            .await
            .unwrap(),
        initialized
    );

    assert_eq!(
        repository
            .update_monotonic(
                &mut *tx,
                &AdapterCursorUpdate {
                    adapter_id: second_adapter_id,
                    last_core_seq: -1,
                    external_cursor_json: None,
                    mark_success: false,
                },
            )
            .await
            .unwrap_err(),
        RepositoryError::InvalidSequence
    );

    tx.commit().await.unwrap();
    context.clean_storage_tables().await.unwrap();
}
