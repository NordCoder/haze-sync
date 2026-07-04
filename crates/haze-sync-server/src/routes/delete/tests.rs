use super::*;
use crate::state::{AuthState, ServerAppState};
use axum::{body::Body, http::Request, Extension};
use haze_sync_api::auth::AdapterRole;
use haze_sync_api::{
    contracts::headers::{IDEMPOTENCY_KEY_HEADER, X_BASE_REVISION_ID_HEADER},
    dto::{
        files::{DeleteFileResponse, DeleteRejectedReasonDto},
        primitives::{TombstoneIdDto, VaultPathDto},
    },
    routes::delete::{rejected_delete_response, tombstoned_delete_response},
};
use haze_sync_common::AdapterId;
use haze_sync_core::delete_guard::{DeleteGuardPolicy, DeleteRatioLimit};
use haze_sync_storage::{
    locks::{lock_vault_path, path_lock_key},
    repositories::{
        objects::{
            create_or_find_sync_object_by_path, set_current_revision_by_object_id, NewSyncObject,
            SyncObjectKind,
        },
        revisions::{insert_file_revision, NewFileRevision},
    },
    test_support::connect_test_database_from_env,
};
use http_body_util::BodyExt as _;
use serde_json::Value;
use tower::ServiceExt as _;

fn principal() -> AdapterPrincipal {
    AdapterPrincipal::new("obsidian-plugin", AdapterRole::ObsidianPlugin).unwrap()
}

fn parsed_delete(path: &str, base_revision_id: &str) -> DeleteFileRouteRequest {
    parse_delete_file_request(DeleteFileRouteRequestParts {
        route_path: path,
        idempotency_key: Some("delete-key-1"),
        base_revision_id: Some(base_revision_id),
        requested_delete_count: None,
    })
    .unwrap()
}

async fn seed_adapter(pool: &sqlx::PgPool, adapter_id: &str, role: &str) {
    sqlx::query(
        "insert into sync_adapters (adapter_id, display_name, role, token_hash, enabled) \
         values ($1, $2, $3, $4, true)",
    )
    .bind(adapter_id)
    .bind(adapter_id)
    .bind(role)
    .bind("sha256:test-token-hash")
    .execute(pool)
    .await
    .expect("adapter should seed");
}

async fn seed_current_file(
    pool: &sqlx::PgPool,
    principal: &AdapterPrincipal,
    path: &VaultPath,
    revision_id: &RevisionId,
) {
    let mut tx = pool.begin().await.expect("tx should begin");
    let object_id = format!("obj_seed_{}", revision_id.as_str());
    let object = create_or_find_sync_object_by_path(
        &mut *tx,
        NewSyncObject {
            object_id: object_id.as_str(),
            path,
            kind: SyncObjectKind::File,
            updated_by: principal.common_adapter_id(),
        },
    )
    .await
    .expect("object should seed");

    insert_file_revision(
        &mut *tx,
        NewFileRevision {
            revision_id,
            object_id: object.object_id.as_str(),
            path,
            parent_revision_id: None,
            content_hash: haze_sync_common::ContentHash::parse(&format!(
                "sha256:{}",
                "a".repeat(64)
            ))
            .unwrap(),
            size_bytes: 3,
            created_by: principal.common_adapter_id(),
        },
    )
    .await
    .expect("revision should seed");

    set_current_revision_by_object_id(
        &mut *tx,
        object.object_id.as_str(),
        Some(revision_id),
        principal.common_adapter_id(),
    )
    .await
    .expect("object update should succeed")
    .expect("object should exist");

    tx.commit().await.expect("tx should commit");
}

fn runtime_state(pool: sqlx::PgPool, principal: AdapterPrincipal) -> ServerAppState {
    ServerAppState::new(
        Some(pool),
        None,
        None,
        AuthState::StaticPrincipal { principal },
    )
}

async fn request_json(
    state: ServerAppState,
    path: &str,
    idempotency_key: &str,
    base_revision_id: &str,
) -> (StatusCode, Value) {
    let request = Request::builder()
        .method("DELETE")
        .uri(path)
        .header("authorization", "Bearer route_test_token")
        .header(IDEMPOTENCY_KEY_HEADER, idempotency_key)
        .header(X_BASE_REVISION_ID_HEADER, base_revision_id)
        .body(Body::empty())
        .expect("request should build");
    let response = router()
        .layer(Extension(state))
        .oneshot(request)
        .await
        .expect("router should respond");
    let status = response.status();
    let body = response
        .into_body()
        .collect()
        .await
        .expect("body should collect")
        .to_bytes();
    let json = serde_json::from_slice(&body).expect("response should be json");
    (status, json)
}

#[test]
fn delete_request_fingerprint_uses_safe_metadata_not_key_or_token() {
    let principal = principal();
    let first = parsed_delete("Notes/a.md", "rev_current");
    let same = parsed_delete("Notes/a.md", "rev_current");
    let different_base = parsed_delete("Notes/a.md", "rev_other");

    assert_eq!(
        delete_request_fingerprint(&first, &principal),
        delete_request_fingerprint(&same, &principal)
    );
    assert_ne!(
        delete_request_fingerprint(&first, &principal),
        delete_request_fingerprint(&different_base, &principal)
    );

    let rendered = format!("{:?}", first);
    assert!(!rendered.contains("delete-key-1"));
    assert!(!rendered.contains("Bearer"));
}

#[test]
fn delete_base_revision_must_match_current() {
    let current = RevisionId::parse("rev_current").unwrap();
    let matching = parsed_delete("Notes/a.md", "rev_current");
    let stale = parsed_delete("Notes/a.md", "rev_stale");
    let explicit_null = parsed_delete("Notes/a.md", "null");

    assert!(delete_base_is_current(&matching, &current));
    assert!(!delete_base_is_current(&stale, &current));
    assert!(!delete_base_is_current(&explicit_null, &current));
}

#[test]
fn delete_guard_blocks_unsafe_count_or_ratio() {
    let scope = DeleteRunScope::new(AdapterId::parse("gdrive-adapter").unwrap(), "scan-1").unwrap();
    let count_guard = DeleteGuard::new(DeleteGuardPolicy::new(
        2,
        DeleteRatioLimit::percent(100).unwrap(),
        false,
    ));
    let ratio_guard = DeleteGuard::new(DeleteGuardPolicy::new(
        20,
        DeleteRatioLimit::percent(5).unwrap(),
        false,
    ));

    assert!(matches!(
        count_guard.evaluate(&DeleteGuardInput::without_manual_unlock(
            scope.clone(),
            3,
            100
        )),
        DeleteGuardDecision::BlockedTooManyDeletes { .. }
    ));
    assert!(matches!(
        ratio_guard.evaluate(&DeleteGuardInput::without_manual_unlock(scope, 6, 100)),
        DeleteGuardDecision::BlockedDeleteRatio { .. }
    ));
}

#[test]
fn route_single_delete_guard_floor_allows_normal_single_delete() {
    let request = parsed_delete("Notes/a.md", "rev_current");
    let allowed = delete_guard_allows(
        &principal(),
        &request,
        delete_guard_total_files(1, u64::from(request.delete_guard().requested_delete_count)),
    )
    .unwrap();

    assert!(allowed);
}

#[test]
fn build_tombstone_uses_service_primitives_without_hard_delete() {
    let current = RevisionId::parse("rev_current").unwrap();
    let path = VaultPath::parse("Notes/old.md").unwrap();
    let tombstone = build_tombstone(&principal(), &path, &current).unwrap();

    assert!(tombstone.tombstone_id.as_str().starts_with("tmb_"));
    assert_eq!(tombstone.path.as_str(), "Notes/old.md");
    assert_eq!(tombstone.deleted_revision_id().as_str(), "rev_current");
    assert_eq!(tombstone.current_revision_id().as_str(), "rev_current");
    assert_eq!(tombstone.deleted_by.as_str(), "obsidian-plugin");
    assert!(tombstone.retention.cleanup_after_retention_only);
}

#[test]
fn tombstone_and_operation_response_metadata_is_safe_json() {
    let response = tombstoned_delete_response(
        VaultPath::parse("Notes/old.md").unwrap(),
        "tmb_01JDELETE",
        7,
        "2026-08-01T00:00:00Z",
    );
    let stored = StoredIdempotencyResponse::json(
        StatusCode::OK.as_u16(),
        serde_json::to_value(&response).unwrap(),
    )
    .unwrap();
    let json = serde_json::to_string(stored.body()).unwrap();

    assert_eq!(delete_response_status(&response), StatusCode::OK);
    assert!(json.contains("tombstoned"));
    assert!(json.contains("tmb_01JDELETE"));
    assert!(!json.contains("Bearer"));
    assert!(!json.contains("postgres://"));
    assert!(!json.contains("/srv/"));
}

#[test]
fn delete_operation_kind_maps_to_delete_file() {
    assert_eq!(OperationKindName::DeleteFile.as_str(), "delete_file");
    let op_id = OperationId::parse(&deterministic_identifier(
        "op_",
        &[
            "rev_current",
            OperationKindName::DeleteFile.as_str(),
            "Notes/a.md",
        ],
    ));
    assert!(op_id.is_ok());
}

#[test]
fn rejected_delete_responses_map_to_safe_statuses() {
    let stale = stale_base_delete_response(VaultPath::parse("Notes/a.md").unwrap());
    let unsafe_delete = unsafe_delete_response(VaultPath::parse("Notes/a.md").unwrap());
    let custom = rejected_delete_response(
        VaultPath::parse("Notes/a.md").unwrap(),
        DeleteRejectedReasonDto::IdempotencyConflict,
    );

    assert_eq!(delete_response_status(&stale), StatusCode::CONFLICT);
    assert_eq!(delete_response_status(&unsafe_delete), StatusCode::CONFLICT);
    assert_eq!(delete_response_status(&custom), StatusCode::CONFLICT);
}

#[test]
fn public_delete_errors_do_not_leak_tokens_paths_or_stack_details() {
    let error = ApiError::from(DeleteRouteError::IdempotencyMismatch).into_response();
    assert_eq!(error.status(), StatusCode::CONFLICT);

    let forbidden = ApiError::from(DeleteRouteError::ForbiddenRole).into_response();
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);

    let path_error = ApiError::from(DeleteRouteError::InvalidPath);
    let rendered = serde_json::to_string(&path_error.body).unwrap();
    assert!(!rendered.contains("../secret.md"));
    assert!(!rendered.contains("Bearer"));
    assert!(!rendered.contains("postgres://"));
    assert!(!rendered.contains("stack"));
}

#[test]
fn stored_replay_body_preserves_delete_json_metadata() {
    let body = DeleteFileResponse::Tombstoned {
        path: VaultPathDto::from("Notes/old.md"),
        tombstone_id: TombstoneIdDto::from("tmb_01JDELETE"),
        seq: 8,
        retention_until: haze_sync_api::dto::primitives::TimestampDto::from("2026-08-01T00:00:00Z"),
    };
    let stored = StoredIdempotencyResponse::json(
        StatusCode::OK.as_u16(),
        serde_json::to_value(&body).unwrap(),
    )
    .unwrap();
    let replayed: Value = stored.body().clone();

    assert_eq!(stored.status_code(), StatusCode::OK.as_u16());
    assert_eq!(replayed["status"], "tombstoned");
    assert_eq!(replayed["path"], "Notes/old.md");
    assert_eq!(replayed["tombstone_id"], "tmb_01JDELETE");
    assert_eq!(replayed["seq"], 8);
}

#[tokio::test]
async fn stale_base_delete_rejects_without_persisting_tombstone_when_real_postgres_is_available() {
    let Some(context) = connect_test_database_from_env()
        .await
        .expect("test database lookup should stay safe")
    else {
        return;
    };
    context
        .apply_migrations()
        .await
        .expect("migrations should apply");
    context
        .clean_storage_tables()
        .await
        .expect("tables should clean");

    let pool = context.pool().clone();
    let principal = principal();
    seed_adapter(&pool, principal.adapter_id(), "obsidian_plugin").await;
    let path = VaultPath::parse("Notes/a.md").unwrap();
    let revision_id = RevisionId::parse("rev_current").unwrap();
    seed_current_file(&pool, &principal, &path, &revision_id).await;

    let state = runtime_state(pool.clone(), principal);
    let (status, json) =
        request_json(state, "/files/Notes/a.md", "delete-key-stale", "rev_stale").await;

    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(json["status"], "rejected");
    assert_eq!(json["reason"], "stale_base_revision");

    let tombstone_count: i64 = sqlx::query_scalar("select count(*) from tombstones")
        .fetch_one(&pool)
        .await
        .expect("count should load");
    let operation_count: i64 = sqlx::query_scalar("select count(*) from operation_log")
        .fetch_one(&pool)
        .await
        .expect("count should load");
    assert_eq!(tombstone_count, 0);
    assert_eq!(operation_count, 0);
}

#[tokio::test]
async fn delete_idempotency_replays_and_mismatch_conflicts_when_real_postgres_is_available() {
    let Some(context) = connect_test_database_from_env()
        .await
        .expect("test database lookup should stay safe")
    else {
        return;
    };
    context
        .apply_migrations()
        .await
        .expect("migrations should apply");
    context
        .clean_storage_tables()
        .await
        .expect("tables should clean");

    let pool = context.pool().clone();
    let principal = principal();
    seed_adapter(&pool, principal.adapter_id(), "obsidian_plugin").await;
    let path = VaultPath::parse("Notes/a.md").unwrap();
    let revision_id = RevisionId::parse("rev_current").unwrap();
    seed_current_file(&pool, &principal, &path, &revision_id).await;

    let first_state = runtime_state(pool.clone(), principal.clone());
    let (first_status, first_json) = request_json(
        first_state,
        "/files/Notes/a.md",
        "delete-key-1",
        "rev_current",
    )
    .await;
    assert_eq!(first_status, StatusCode::OK);
    assert_eq!(first_json["status"], "tombstoned");

    let replay_state = runtime_state(pool.clone(), principal.clone());
    let (replay_status, replay_json) = request_json(
        replay_state,
        "/files/Notes/a.md",
        "delete-key-1",
        "rev_current",
    )
    .await;
    assert_eq!(replay_status, StatusCode::OK);
    assert_eq!(replay_json, first_json);

    let mismatch_state = runtime_state(pool.clone(), principal);
    let (mismatch_status, mismatch_json) = request_json(
        mismatch_state,
        "/files/Notes/a.md",
        "delete-key-1",
        "rev_other",
    )
    .await;
    assert_eq!(mismatch_status, StatusCode::CONFLICT);
    assert_eq!(mismatch_json["error"]["code"], "idempotency_conflict");

    let tombstone_count: i64 = sqlx::query_scalar("select count(*) from tombstones")
        .fetch_one(&pool)
        .await
        .expect("count should load");
    let operation_count: i64 = sqlx::query_scalar("select count(*) from operation_log")
        .fetch_one(&pool)
        .await
        .expect("count should load");
    let idempotency_count: i64 = sqlx::query_scalar("select count(*) from idempotency_records")
        .fetch_one(&pool)
        .await
        .expect("count should load");
    assert_eq!(tombstone_count, 1);
    assert_eq!(operation_count, 1);
    assert_eq!(idempotency_count, 1);
}

#[tokio::test]
async fn advisory_lock_blocks_same_path_in_second_transaction_when_real_postgres_is_available() {
    let Some(context) = connect_test_database_from_env()
        .await
        .expect("test database lookup should stay safe")
    else {
        return;
    };
    context
        .apply_migrations()
        .await
        .expect("migrations should apply");

    let pool = context.pool().clone();
    let path = VaultPath::parse("Notes/a.md").unwrap();

    let mut tx1 = pool.begin().await.expect("tx1 should begin");
    let key = lock_vault_path(&mut *tx1, &path)
        .await
        .expect("first lock should acquire");

    let mut tx2 = pool.begin().await.expect("tx2 should begin");
    let acquired: bool = sqlx::query_scalar("select pg_try_advisory_xact_lock($1)")
        .bind(key.as_i64())
        .fetch_one(&mut *tx2)
        .await
        .expect("try lock should execute");

    assert_eq!(key, path_lock_key(&path));
    assert!(!acquired);

    tx2.rollback().await.expect("tx2 should rollback");
    tx1.rollback().await.expect("tx1 should rollback");
}
