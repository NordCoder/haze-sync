use super::*;
use crate::state::{AuthState, ServerAppState};
use axum::{body::Body, http::Request, Extension};
use haze_sync_api::auth::{AdapterPrincipal, AdapterRole};
use haze_sync_api::contracts::headers::{
    IDEMPOTENCY_KEY_HEADER, X_BASE_REVISION_ID_HEADER, X_CONTENT_SHA256_HEADER,
};
use haze_sync_core::revision_service::compute_content_hash;
use haze_sync_storage::{
    test_support::connect_test_database_from_env, LocalObjectStore,
};
use http_body_util::BodyExt as _;
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use tower::ServiceExt as _;

struct TestStoreRoot(PathBuf);

impl TestStoreRoot {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be valid")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "haze-sync-v1-route-parity-{}-{nonce}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        Self(root)
    }
}

impl Drop for TestStoreRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn principal() -> AdapterPrincipal {
    AdapterPrincipal::new("obsidian-plugin", AdapterRole::ObsidianPlugin).unwrap()
}

async fn seed_adapter(pool: &sqlx::PgPool) {
    sqlx::query(
        "insert into sync_adapters (adapter_id, display_name, role, token_hash, enabled) \
         values ('obsidian-plugin', 'obsidian-plugin', 'obsidian_plugin', $1, true)",
    )
    .bind("sha256:test-token-hash")
    .execute(pool)
    .await
    .expect("adapter should seed");
}

#[tokio::test]
async fn delegated_routes_preserve_put_get_and_changes_wire_behavior() {
    let context = connect_test_database_from_env()
        .await
        .expect("test database lookup should stay safe")
        .expect("Component CI must provide a strict test database");
    context
        .apply_migrations()
        .await
        .expect("migrations should apply");
    context
        .clean_storage_tables()
        .await
        .expect("tables should clean");
    let pool = context.pool().clone();
    seed_adapter(&pool).await;

    let root = TestStoreRoot::new();
    let state = ServerAppState::new(
        Some(pool),
        Some(LocalObjectStore::new(root.0.clone())),
        None,
        AuthState::StaticPrincipal {
            principal: principal(),
        },
    );
    let bytes = b"route parity bytes";
    let hash = compute_content_hash(bytes).to_string();
    let put = Request::builder()
        .method("PUT")
        .uri("/files/Notes/route.md")
        .header("authorization", "Bearer route_test_token")
        .header(IDEMPOTENCY_KEY_HEADER, "route-put-1")
        .header(X_BASE_REVISION_ID_HEADER, "null")
        .header(X_CONTENT_SHA256_HEADER, hash.as_str())
        .body(Body::from(bytes.as_slice()))
        .expect("request should build");
    let put_response = router()
        .layer(Extension(state.clone()))
        .oneshot(put)
        .await
        .expect("router should respond");
    assert_eq!(put_response.status(), StatusCode::OK);
    let put_json: serde_json::Value = serde_json::from_slice(
        &put_response
            .into_body()
            .collect()
            .await
            .expect("body should collect")
            .to_bytes(),
    )
    .expect("PUT response should be JSON");
    assert_eq!(put_json["status"], "accepted");
    assert_eq!(put_json["path"], "Notes/route.md");

    let get = Request::builder()
        .method("GET")
        .uri("/files/Notes/route.md")
        .header("authorization", "Bearer route_test_token")
        .body(Body::empty())
        .expect("request should build");
    let get_response = router()
        .layer(Extension(state.clone()))
        .oneshot(get)
        .await
        .expect("router should respond");
    assert_eq!(get_response.status(), StatusCode::OK);
    assert_eq!(
        get_response.headers().get(X_CONTENT_SHA256_HEADER).unwrap(),
        hash.as_str()
    );
    assert_eq!(
        get_response
            .into_body()
            .collect()
            .await
            .expect("body should collect")
            .to_bytes(),
        bytes.as_slice()
    );

    let changes = Request::builder()
        .method("GET")
        .uri("/changes?since=0&limit=10")
        .header("authorization", "Bearer route_test_token")
        .body(Body::empty())
        .expect("request should build");
    let changes_response = router()
        .layer(Extension(state))
        .oneshot(changes)
        .await
        .expect("router should respond");
    assert_eq!(changes_response.status(), StatusCode::OK);
    let changes_json: serde_json::Value = serde_json::from_slice(
        &changes_response
            .into_body()
            .collect()
            .await
            .expect("body should collect")
            .to_bytes(),
    )
    .expect("changes response should be JSON");
    assert_eq!(changes_json["from_seq"], 0);
    assert_eq!(changes_json["changes"][0]["kind"], "upsert_file");
    assert_eq!(changes_json["changes"][0]["path"], "Notes/route.md");
}

#[test]
fn public_parsing_and_auth_errors_remain_safe() {
    let missing_idempotency = parse_put_file_request(PutFileRouteRequestParts {
        route_path: "Notes/a.md",
        idempotency_key: None,
        content_sha256: Some(&compute_content_hash(b"hello").to_string()),
        base_revision_id: Some("null"),
        body: b"hello".to_vec(),
        max_upload_bytes: Some(MAX_UPLOAD_BYTES),
    })
    .unwrap_err();
    assert_eq!(
        ApiError::from(missing_idempotency).into_response().status(),
        StatusCode::BAD_REQUEST
    );

    let invalid_path = parse_get_file_request(GetFileRouteRequestParts {
        route_path: "../outside.md",
        revision_id: None,
    })
    .unwrap_err();
    assert_eq!(
        ApiError::from(invalid_path).into_response().status(),
        StatusCode::BAD_REQUEST
    );

    let response = ApiError::invalid_token().into_response();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn changes_kind_mapping_remains_exhaustive() {
    assert_eq!(
        operation_kind_dto(OperationKindName::UpsertFile),
        OperationKindDto::UpsertFile
    );
    assert_eq!(
        operation_kind_dto(OperationKindName::DeleteFile),
        OperationKindDto::DeleteFile
    );
}
