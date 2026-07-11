use super::*;
use crate::{
    application::{
        file_request_fingerprint, ApplicationActor, ApplicationIdempotency, ApplyFileCommand,
        ApplyFileOutcome, ServerApplicationServices,
    },
    state::{AuthState, ServerAppState},
};
use axum::{body::Body, http::Request, Extension};
use haze_sync_api::{
    auth::{AdapterPrincipal, AdapterRole},
    contracts::headers::{IDEMPOTENCY_KEY_HEADER, X_BASE_REVISION_ID_HEADER},
};
use haze_sync_common::{AdapterId, VaultPath};
use haze_sync_core::revision_service::compute_content_hash;
use haze_sync_storage::{test_support::connect_test_database_from_env, LocalObjectStore};
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
            "haze-sync-delete-route-parity-{}-{nonce}",
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

async fn seed_current_file(
    services: &ServerApplicationServices,
    actor: &ApplicationActor,
    path: &VaultPath,
    key: &str,
) -> haze_sync_common::RevisionId {
    let bytes = b"delete route seed";
    let content_hash = compute_content_hash(bytes);
    let outcome = services
        .apply_file(ApplyFileCommand {
            actor: actor.clone(),
            path: path.clone(),
            base_revision_id: None,
            content_hash,
            bytes: bytes.to_vec(),
            idempotency: ApplicationIdempotency::new(
                key,
                file_request_fingerprint(actor, path, None, content_hash, bytes),
            ),
        })
        .await
        .expect("seed file should be accepted");
    match outcome {
        ApplyFileOutcome::Accepted { revision_id, .. } => revision_id,
        other => panic!("unexpected seed outcome: {other:?}"),
    }
}

async fn request_json(
    state: ServerAppState,
    path: &str,
    key: &str,
    base: &str,
) -> (StatusCode, serde_json::Value) {
    let request = Request::builder()
        .method("DELETE")
        .uri(path)
        .header("authorization", "Bearer route_test_token")
        .header(IDEMPOTENCY_KEY_HEADER, key)
        .header(X_BASE_REVISION_ID_HEADER, base)
        .body(Body::empty())
        .expect("request should build");
    let response = router()
        .layer(Extension(state))
        .oneshot(request)
        .await
        .expect("router should respond");
    let status = response.status();
    let json = serde_json::from_slice(
        &response
            .into_body()
            .collect()
            .await
            .expect("body should collect")
            .to_bytes(),
    )
    .expect("response should be JSON");
    (status, json)
}

#[tokio::test]
async fn delegated_delete_route_preserves_tombstone_replay_and_stale_base_behavior() {
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
    let object_store = LocalObjectStore::new(root.0.clone());
    let services = ServerApplicationServices::new(pool.clone(), Some(object_store));
    let actor = ApplicationActor::new(AdapterId::parse("obsidian-plugin").unwrap());
    let path = VaultPath::parse("Notes/delete-route.md").unwrap();
    let revision_id = seed_current_file(&services, &actor, &path, "seed-delete-route").await;
    let state = ServerAppState::new(
        Some(pool.clone()),
        None,
        None,
        AuthState::StaticPrincipal {
            principal: principal(),
        },
    );

    let (status, body) = request_json(
        state.clone(),
        "/files/Notes/delete-route.md",
        "delete-route-1",
        revision_id.as_str(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "tombstoned");
    let first_tombstone = body["tombstone_id"].clone();
    let first_seq = body["seq"].clone();

    let (replay_status, replay_body) = request_json(
        state.clone(),
        "/files/Notes/delete-route.md",
        "delete-route-1",
        revision_id.as_str(),
    )
    .await;
    assert_eq!(replay_status, StatusCode::OK);
    assert_eq!(replay_body["tombstone_id"], first_tombstone);
    assert_eq!(replay_body["seq"], first_seq);

    let stale_path = VaultPath::parse("Notes/stale-delete.md").unwrap();
    let stale_revision =
        seed_current_file(&services, &actor, &stale_path, "seed-stale-route").await;
    let (stale_status, stale_body) = request_json(
        state,
        "/files/Notes/stale-delete.md",
        "delete-route-stale",
        "rev_stale",
    )
    .await;
    assert_eq!(stale_status, StatusCode::CONFLICT);
    assert_eq!(stale_body["status"], "rejected");
    assert_eq!(stale_body["reason"], "stale_base_revision");
    assert_ne!(stale_revision.as_str(), "rev_stale");
}

#[tokio::test]
async fn missing_database_maps_to_safe_service_unavailable_instead_of_panicking() {
    let state = ServerAppState::new(
        None,
        None,
        None,
        AuthState::StaticPrincipal {
            principal: principal(),
        },
    );
    let (status, body) = request_json(state, "/files/Notes/a.md", "safe-delete-key", "null").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    let rendered = body.to_string();
    assert!(!rendered.contains("safe-delete-key"));
    assert!(!rendered.contains("postgres://"));
    assert!(!rendered.contains("sqlx"));
}

#[test]
fn public_delete_error_mapping_remains_stable_and_safe() {
    let conflict = ApiError::from(DeleteRouteError::IdempotencyMismatch).into_response();
    assert_eq!(conflict.status(), StatusCode::CONFLICT);
    let forbidden = ApiError::from(DeleteRouteError::ForbiddenRole).into_response();
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
}
