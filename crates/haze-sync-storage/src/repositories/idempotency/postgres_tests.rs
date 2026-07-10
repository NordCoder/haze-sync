use super::*;
use crate::test_support::connect_test_database_from_env;
use serde_json::json;

fn response_snapshot(status: &str) -> Value {
    json!({
        "status_code": 201,
        "headers": {
            "content-type": "application/json",
            "x-revision-id": "rev_124"
        },
        "body": {
            "status": status,
            "path": "Projects/Haze/plan.md"
        }
    })
}

#[tokio::test]
async fn idempotency_check_or_store_roundtrips_first_writer_facts() {
    let Some(context) = connect_test_database_from_env().await.unwrap() else {
        return;
    };
    context.apply_migrations().await.unwrap();
    context.clean_storage_tables().await.unwrap();

    let adapter_id = AdapterId::parse("iphone-anna").unwrap();
    let request_hash = Sha256::parse(&"a".repeat(64)).unwrap();
    let different_hash = Sha256::parse(&"b".repeat(64)).unwrap();
    let key = "iphone:iphone-anna:op-001";
    let mut tx = context.pool().begin().await.unwrap();

    sqlx::query(
        "insert into sync_adapters (adapter_id, display_name, role, token_hash) \
         values ($1, $2, $3, $4)",
    )
    .bind(adapter_id.as_str())
    .bind("iPhone Anna")
    .bind("client")
    .bind("sha256:test-token-hash")
    .execute(&mut *tx)
    .await
    .unwrap();

    let first_input = IdempotencyRecordInput::new(
        adapter_id.clone(),
        key,
        request_hash,
        response_snapshot("accepted"),
    )
    .unwrap();
    let first_record = match check_or_store_idempotency_record(&mut *tx, &first_input)
        .await
        .unwrap()
    {
        IdempotencyRepositoryOutcome::NewRequest { record } => record,
        other => panic!("first request must be stored as new, got {other:?}"),
    };
    assert_eq!(first_record.adapter_id, adapter_id.as_str());
    assert_eq!(first_record.idempotency_key, key);
    assert_eq!(first_record.request_hash, request_hash.to_string());
    assert_eq!(first_record.response_json, response_snapshot("accepted"));

    let loaded = read_idempotency_record(&mut *tx, &adapter_id, key)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(loaded, first_record);

    let replay_input = IdempotencyRecordInput::new(
        adapter_id.clone(),
        key,
        request_hash,
        response_snapshot("must-not-replace-first-response"),
    )
    .unwrap();
    assert_eq!(
        check_or_store_idempotency_record(&mut *tx, &replay_input)
            .await
            .unwrap(),
        IdempotencyRepositoryOutcome::ReplaySameRequest {
            record: first_record.clone()
        }
    );

    let conflict_input = IdempotencyRecordInput::new(
        adapter_id.clone(),
        key,
        different_hash,
        response_snapshot("must-not-be-stored"),
    )
    .unwrap();
    assert_eq!(
        check_or_store_idempotency_record(&mut *tx, &conflict_input)
            .await
            .unwrap(),
        IdempotencyRepositoryOutcome::ConflictDifferentRequest {
            record: first_record.clone()
        }
    );

    assert_eq!(
        insert_idempotency_record(&mut *tx, &replay_input)
            .await
            .unwrap(),
        IdempotencyStoreOutcome::AlreadyExists {
            record: first_record.clone()
        }
    );
    assert_eq!(
        read_idempotency_record(&mut *tx, &adapter_id, key)
            .await
            .unwrap()
            .unwrap(),
        first_record
    );

    tx.commit().await.unwrap();
    context.clean_storage_tables().await.unwrap();
}
