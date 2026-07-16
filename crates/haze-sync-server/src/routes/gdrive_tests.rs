use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use haze_sync_api::{
    auth::{AdapterPrincipal, AdapterRole},
    contracts::headers::IDEMPOTENCY_KEY_HEADER,
};
use haze_sync_common::AdapterId;
use haze_sync_storage::{
    repositories::gdrive_state::initialize_gdrive_adapter_state,
    test_support::{connect_required_test_database_from_env, TestNamespace},
};
use http_body_util::BodyExt as _;
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt as _;

use crate::{
    routes::build_router_with_state,
    state::{AuthState, ServerAppState},
};

const TEST_BEARER: &str = "Bearer server-gdrive-route-test";

#[tokio::test]
async fn gdrive_routes_enforce_auth_type_visibility_and_adapter_isolation() {
    let context = connect_required_test_database_from_env().await.unwrap();
    context.apply_migrations().await.unwrap();
    let namespace = TestNamespace::new("server-gdrive-auth");
    let adapter = namespace.adapter_id("private");
    let isolated = namespace.adapter_id("isolated");
    let wrong_type = namespace.adapter_id("wrong-type");
    register_adapter(context.pool(), &adapter, "gdrive_adapter", true).await;
    register_adapter(context.pool(), &isolated, "gdrive_adapter", true).await;
    register_adapter(context.pool(), &wrong_type, "worktree_adapter", false).await;

    let path = namespace.vault_path("private.md");
    let operation = namespace.operation_id("private-commit");
    let body = mapping_commit(
        0,
        0,
        Some((1, "private-raw-cursor")),
        1,
        &operation,
        'a',
        &path,
        "private-drive-file",
    );
    let (status, committed) = request_json(
        state(context.pool(), &adapter, AdapterRole::GdriveAdapter),
        "POST",
        &format!("/v1/adapters/{adapter}/gdrive/state/commit"),
        Some(body),
        true,
        Some("private-commit-key"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(committed["status"], "committed");

    let (status, private) = request_json(
        state(context.pool(), &adapter, AdapterRole::GdriveAdapter),
        "GET",
        &format!("/v1/adapters/{adapter}/gdrive/state"),
        None,
        true,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(private["adapter_id"], adapter);
    assert_eq!(private["state_version"], 1);
    assert_eq!(private["cursor"]["generation"], 1);
    assert_eq!(private["cursor"]["present"], true);
    assert_eq!(private["mappings"][0]["path"], path);
    assert_eq!(private["mappings"][0]["drive_file_id"], "private-drive-file");
    assert!(!private.to_string().contains("private-raw-cursor"));

    let (status, admin) = request_json(
        state(context.pool(), "admin-gdrive-test", AdapterRole::Admin),
        "GET",
        &format!("/v1/adapters/{adapter}/gdrive/state"),
        None,
        true,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(admin["adapter_id"], adapter);
    assert_eq!(admin["mapping_count"], 1);
    assert_eq!(admin["echo_counts"]["none"], 1);
    assert!(admin.get("mappings").is_none());
    assert!(!admin.to_string().contains("private-drive-file"));
    assert!(!admin.to_string().contains("private-raw-cursor"));

    let (status, unauthorized) = request_json(
        state(context.pool(), &adapter, AdapterRole::GdriveAdapter),
        "GET",
        &format!("/v1/adapters/{adapter}/gdrive/state"),
        None,
        false,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(unauthorized["error"]["code"], "unauthorized");

    let (status, forbidden) = request_json(
        state(context.pool(), &adapter, AdapterRole::WorktreeAdapter),
        "GET",
        &format!("/v1/adapters/{adapter}/gdrive/state"),
        None,
        true,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(forbidden["error"]["code"], "forbidden");

    let (status, mismatch) = request_json(
        state(context.pool(), &adapter, AdapterRole::GdriveAdapter),
        "GET",
        &format!("/v1/adapters/{isolated}/gdrive/state"),
        None,
        true,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(mismatch["error"]["code"], "forbidden");

    let (status, type_mismatch) = request_json(
        state(context.pool(), &wrong_type, AdapterRole::GdriveAdapter),
        "GET",
        &format!("/v1/adapters/{wrong_type}/gdrive/state"),
        None,
        true,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(type_mismatch["error"]["code"], "forbidden");

    let (status, isolated_state) = request_json(
        state(context.pool(), &isolated, AdapterRole::GdriveAdapter),
        "GET",
        &format!("/v1/adapters/{isolated}/gdrive/state"),
        None,
        true,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(isolated_state["state_version"], 0);
    assert_eq!(isolated_state["mappings"], json!([]));
}

#[tokio::test]
async fn gdrive_commit_route_is_replay_safe_and_rolls_back_all_partial_facts() {
    let context = connect_required_test_database_from_env().await.unwrap();
    context.apply_migrations().await.unwrap();
    let namespace = TestNamespace::new("server-gdrive-commit");
    let adapter = namespace.adapter_id("main");
    register_adapter(context.pool(), &adapter, "gdrive_adapter", true).await;
    let route = format!("/v1/adapters/{adapter}/gdrive/state/commit");
    let path_a = namespace.vault_path("a.md");
    let path_b = namespace.vault_path("b.md");
    let operation_a = namespace.operation_id("mapping-a");
    let first = mapping_commit(
        0,
        0,
        Some((1, "cursor-after-a")),
        1,
        &operation_a,
        '1',
        &path_a,
        "shared-drive-file",
    );

    let (status, committed) = request_json(
        state(context.pool(), &adapter, AdapterRole::GdriveAdapter),
        "POST",
        &route,
        Some(first.clone()),
        true,
        Some("commit-a-key"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(committed["status"], "committed");
    assert_eq!(committed["state_version"], 1);

    let (status, replayed) = request_json(
        state(context.pool(), &adapter, AdapterRole::GdriveAdapter),
        "POST",
        &route,
        Some(first.clone()),
        true,
        Some("commit-a-key"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(replayed["status"], "replayed");
    assert_eq!(replayed["state_version"], 1);

    let stale = checkpoint_commit(
        0,
        1,
        None,
        2,
        &namespace.operation_id("stale"),
        '2',
    );
    let (status, stale_response) = request_json(
        state(context.pool(), &adapter, AdapterRole::GdriveAdapter),
        "POST",
        &route,
        Some(stale),
        true,
        Some("stale-key"),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(stale_response["status"], "stale_state");

    for (next_generation, expected_code) in [(1, "cursor_regression"), (3, "cursor_gap")] {
        let invalid_cursor = checkpoint_commit(
            1,
            1,
            Some((next_generation, "invalid-cursor-sentinel")),
            2,
            &namespace.operation_id(expected_code),
            '3',
        );
        let (status, response) = request_json(
            state(context.pool(), &adapter, AdapterRole::GdriveAdapter),
            "POST",
            &route,
            Some(invalid_cursor),
            true,
            Some("invalid-cursor-key"),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(response["error"]["code"], expected_code);
        assert!(!response.to_string().contains("invalid-cursor-sentinel"));
    }

    let mut mapping_conflict = first.clone();
    mapping_conflict["expected_state_version"] = json!(1);
    mapping_conflict["cursor"] = json!({"expected_generation": 1});
    mapping_conflict["operation"]["operation_id"] =
        json!(namespace.operation_id("mapping-conflict"));
    mapping_conflict["operation"]["mapping_path"] = json!(path_b.clone());
    let (status, response) = request_json(
        state(context.pool(), &adapter, AdapterRole::GdriveAdapter),
        "POST",
        &route,
        Some(mapping_conflict),
        true,
        Some("mapping-conflict-key"),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(response["error"]["code"], "mapping_conflict");

    let mut idempotency_conflict = first.clone();
    idempotency_conflict["expected_state_version"] = json!(1);
    idempotency_conflict["cursor"] = json!({"expected_generation": 1});
    idempotency_conflict["operation"]["facts_fingerprint"] = json!("f".repeat(64));
    let (status, response) = request_json(
        state(context.pool(), &adapter, AdapterRole::GdriveAdapter),
        "POST",
        &route,
        Some(idempotency_conflict),
        true,
        Some("idempotency-conflict-key"),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(response["status"], "idempotency_conflict");

    let rollback_operation = namespace.operation_id("rollback");
    let mut rollback = mapping_commit(
        1,
        1,
        Some((2, "rollback-private-cursor")),
        2,
        &rollback_operation,
        '4',
        &path_b,
        "shared-drive-file",
    );
    rollback["mapping"]["echo"] = json!({
        "state": "pending",
        "operation_id": rollback_operation,
    });
    rollback["mapping"]["delete_candidate"] = json!({
        "first_seen_at": "2099-01-01T00:00:01Z",
        "last_seen_at": "2099-01-01T00:00:02Z",
        "generation": 2,
        "blocked": true,
    });
    rollback["operation"]["kind"] = json!("delete_candidate");
    let (status, rollback_response) = request_json(
        state(context.pool(), &adapter, AdapterRole::GdriveAdapter),
        "POST",
        &route,
        Some(rollback),
        true,
        Some("rollback-idempotency-sentinel"),
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(rollback_response["status"], "validation_failed");
    let rendered = rollback_response.to_string();
    for secret in [
        "rollback-private-cursor",
        "rollback-idempotency-sentinel",
        "shared-drive-file",
        "2099-01-01T00:00:01Z",
        "postgres://",
        "sqlx",
    ] {
        assert!(!rendered.contains(secret));
    }

    let state_version: i64 = sqlx::query_scalar(
        "select state_version from gdrive_adapter_state where adapter_id = $1",
    )
    .bind(&adapter)
    .fetch_one(context.pool())
    .await
    .unwrap();
    let mapping_count: i64 = sqlx::query_scalar(
        "select count(*) from gdrive_durable_items where adapter_id = $1",
    )
    .bind(&adapter)
    .fetch_one(context.pool())
    .await
    .unwrap();
    let rollback_path_count: i64 = sqlx::query_scalar(
        "select count(*) from gdrive_durable_items where adapter_id = $1 and path = $2",
    )
    .bind(&adapter)
    .bind(&path_b)
    .fetch_one(context.pool())
    .await
    .unwrap();
    let operation_count: i64 =
        sqlx::query_scalar("select count(*) from gdrive_operations where adapter_id = $1")
            .bind(&adapter)
            .fetch_one(context.pool())
            .await
            .unwrap();
    assert_eq!(state_version, 1);
    assert_eq!(mapping_count, 1);
    assert_eq!(rollback_path_count, 0);
    assert_eq!(operation_count, 1);
}

#[tokio::test]
async fn concurrent_gdrive_route_writer_loser_is_stale_and_other_adapter_is_unchanged() {
    let context = connect_required_test_database_from_env().await.unwrap();
    context.apply_migrations().await.unwrap();
    let namespace = TestNamespace::new("server-gdrive-race");
    let adapter = namespace.adapter_id("race");
    let isolated = namespace.adapter_id("race-isolated");
    register_adapter(context.pool(), &adapter, "gdrive_adapter", true).await;
    register_adapter(context.pool(), &isolated, "gdrive_adapter", true).await;
    let route = format!("/v1/adapters/{adapter}/gdrive/state/commit");
    let winner = checkpoint_commit(
        0,
        0,
        None,
        1,
        &namespace.operation_id("winner"),
        '5',
    );
    let loser = checkpoint_commit(
        0,
        0,
        None,
        2,
        &namespace.operation_id("loser"),
        '6',
    );
    let first = request_json(
        state(context.pool(), &adapter, AdapterRole::GdriveAdapter),
        "POST",
        &route,
        Some(winner),
        true,
        Some("race-winner-key"),
    );
    let second = request_json(
        state(context.pool(), &adapter, AdapterRole::GdriveAdapter),
        "POST",
        &route,
        Some(loser),
        true,
        Some("race-loser-key"),
    );
    let (first, second) = tokio::join!(first, second);
    let results = [first, second];
    assert_eq!(
        results
            .iter()
            .filter(|(status, body)| {
                *status == StatusCode::OK && body["status"] == "committed"
            })
            .count(),
        1
    );
    assert_eq!(
        results
            .iter()
            .filter(|(status, body)| {
                *status == StatusCode::CONFLICT && body["status"] == "stale_state"
            })
            .count(),
        1
    );

    let (status, isolated_state) = request_json(
        state(context.pool(), &isolated, AdapterRole::GdriveAdapter),
        "GET",
        &format!("/v1/adapters/{isolated}/gdrive/state"),
        None,
        true,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(isolated_state["state_version"], 0);
    assert_eq!(isolated_state["core_export_checkpoint"], 0);
    assert_eq!(isolated_state["mappings"], json!([]));
}

fn state(pool: &PgPool, adapter_id: &str, role: AdapterRole) -> ServerAppState {
    let principal = AdapterPrincipal::new(adapter_id, role).unwrap();
    ServerAppState::new(
        Some(pool.clone()),
        None,
        None,
        AuthState::StaticPrincipal { principal },
    )
}

async fn register_adapter(pool: &PgPool, adapter_id: &str, role: &str, initialize: bool) {
    let adapter = AdapterId::parse(adapter_id).unwrap();
    let mut transaction = pool.begin().await.unwrap();
    sqlx::query(
        "insert into sync_adapters (adapter_id, display_name, role, token_hash) \
         values ($1, $2, $3, 'server-gdrive-test-token-hash')",
    )
    .bind(adapter_id)
    .bind(format!("Server test {adapter_id}"))
    .bind(role)
    .execute(&mut *transaction)
    .await
    .unwrap();
    if initialize {
        initialize_gdrive_adapter_state(&mut transaction, &adapter)
            .await
            .unwrap();
    }
    transaction.commit().await.unwrap();
}

async fn request_json(
    state: ServerAppState,
    method: &str,
    uri: &str,
    body: Option<Value>,
    authorized: bool,
    idempotency_key: Option<&str>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    if authorized {
        builder = builder.header("authorization", TEST_BEARER);
    }
    if let Some(value) = idempotency_key {
        builder = builder.header(IDEMPOTENCY_KEY_HEADER, value);
    }
    let body = if let Some(body) = body {
        builder = builder.header("content-type", "application/json");
        Body::from(body.to_string())
    } else {
        Body::empty()
    };
    let response = build_router_with_state(state)
        .oneshot(builder.body(body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap();
    (status, json)
}

fn checkpoint_commit(
    expected_state_version: u64,
    expected_generation: u64,
    advance: Option<(u64, &str)>,
    checkpoint: u64,
    operation_id: &str,
    fingerprint: char,
) -> Value {
    json!({
        "expected_state_version": expected_state_version,
        "cursor": {
            "expected_generation": expected_generation,
            "advance": advance.map(|(next_generation, cursor)| json!({
                "next_generation": next_generation,
                "cursor": cursor,
            })),
        },
        "core_export_checkpoint": checkpoint,
        "operation": {
            "operation_id": operation_id,
            "kind": "cursor_checkpoint",
            "facts_fingerprint": fingerprint.to_string().repeat(64),
            "core_seq": checkpoint,
        },
    })
}

#[allow(clippy::too_many_arguments)]
fn mapping_commit(
    expected_state_version: u64,
    expected_generation: u64,
    advance: Option<(u64, &str)>,
    checkpoint: u64,
    operation_id: &str,
    fingerprint: char,
    path: &str,
    drive_file_id: &str,
) -> Value {
    json!({
        "expected_state_version": expected_state_version,
        "cursor": {
            "expected_generation": expected_generation,
            "advance": advance.map(|(next_generation, cursor)| json!({
                "next_generation": next_generation,
                "cursor": cursor,
            })),
        },
        "core_export_checkpoint": checkpoint,
        "mapping": {
            "path": path,
            "drive_file_id": drive_file_id,
            "drive_parent_id": "drive-parent",
            "drive_name": "note.md",
            "mime_type": "text/markdown",
            "md5_checksum": "aabbccddeeff00112233445566778899",
            "head_revision_id": "drive-head",
            "drive_version": "drive-version",
            "drive_modified_time": "2099-01-01T00:00:00Z",
            "core_seq": checkpoint,
            "echo": {"state": "none"},
            "last_seen_at": "2099-01-01T00:00:00Z",
        },
        "operation": {
            "operation_id": operation_id,
            "kind": "export",
            "facts_fingerprint": fingerprint.to_string().repeat(64),
            "mapping_path": path,
            "core_seq": checkpoint,
            "drive_version": "drive-version",
        },
    })
}
