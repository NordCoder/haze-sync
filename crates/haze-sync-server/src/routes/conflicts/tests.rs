use super::*;
use axum::{body::Body, http::Request};
use haze_sync_api::auth::{AdapterPrincipal, AdapterRole};
use haze_sync_storage::{
    repositories::operation_log::OperationLogRepository,
    test_support::connect_test_database_from_env,
};
use http_body_util::BodyExt as _;
use sqlx::PgPool;
use tower::ServiceExt as _;

fn principal() -> AdapterPrincipal {
    AdapterPrincipal::new("obsidian-plugin", AdapterRole::ObsidianPlugin)
        .expect("fixture principal should parse")
}

fn static_state() -> ServerAppState {
    ServerAppState::with_static_principal(principal())
}

fn runtime_state(pool: PgPool, principal: AdapterPrincipal) -> ServerAppState {
    ServerAppState::new(
        Some(pool),
        None,
        None,
        crate::state::AuthState::StaticPrincipal { principal },
    )
}

async fn request_json(method: &str, uri: &str, body: Body) -> (StatusCode, Value) {
    request_json_with_state(static_state(), method, uri, body).await
}

async fn request_json_with_state(
    state: ServerAppState,
    method: &str,
    uri: &str,
    body: Body,
) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", "Bearer conflict_route_test_token")
        .header("content-type", "application/json")
        .body(body)
        .expect("test request should build");
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
        .expect("response body should collect")
        .to_bytes();
    let json = serde_json::from_slice(&body).expect("response body should be JSON");
    (status, json)
}

async fn seed_adapter(pool: &PgPool, adapter_id: &str, role: &str) {
    sqlx::query(
        "insert into sync_adapters (adapter_id, display_name, role, token_hash, enabled) \
         values ($1, $2, $3, $4, true)",
    )
    .bind(adapter_id)
    .bind(adapter_id)
    .bind(role)
    .bind(format!("sha256:{adapter_id}"))
    .execute(pool)
    .await
    .expect("adapter should seed");
}

async fn seed_conflict_fixture(
    pool: &PgPool,
    conflict_id: &str,
    status: &str,
) -> (String, String, String) {
    let original_path = "Notes/a.md";
    let conflict_path = "_haze_conflicts/open/Notes/a.conflict.obsidian-plugin.20260704T000000Z.md";
    let current_revision_id = "rev_current";
    let incoming_revision_id = "rev_incoming";
    let current_object_id = "obj_current";
    let conflict_object_id = "obj_conflict";
    let current_hash = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let incoming_hash = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    sqlx::query(
        "insert into content_blobs (sha256, size_bytes, object_store_path) \
         values ($1, 3, $2), ($3, 8, $4)",
    )
    .bind(current_hash)
    .bind(format!("blobs/{current_hash}"))
    .bind(incoming_hash)
    .bind(format!("blobs/{incoming_hash}"))
    .execute(pool)
    .await
    .expect("content blobs should seed");

    sqlx::query(
        "insert into sync_objects (object_id, path, kind, current_revision_id, updated_by) \
         values ($1, $2, 'file', null, 'obsidian-plugin'), \
                ($3, $4, 'file', null, 'obsidian-plugin')",
    )
    .bind(current_object_id)
    .bind(original_path)
    .bind(conflict_object_id)
    .bind(conflict_path)
    .execute(pool)
    .await
    .expect("objects should seed");

    sqlx::query(
        "insert into file_revisions \
         (revision_id, object_id, path, parent_revision_id, content_sha256, size_bytes, created_by) \
         values ($1, $2, $3, null, $4, 3, 'obsidian-plugin'), \
                ($5, $6, $7, null, $8, 8, 'obsidian-plugin')",
    )
    .bind(current_revision_id)
    .bind(current_object_id)
    .bind(original_path)
    .bind(current_hash)
    .bind(incoming_revision_id)
    .bind(conflict_object_id)
    .bind(conflict_path)
    .bind(incoming_hash)
    .execute(pool)
    .await
    .expect("revisions should seed");

    sqlx::query(
        "update sync_objects set current_revision_id = $1 where object_id = $2; \
         update sync_objects set current_revision_id = $3 where object_id = $4",
    )
    .bind(current_revision_id)
    .bind(current_object_id)
    .bind(incoming_revision_id)
    .bind(conflict_object_id)
    .execute(pool)
    .await
    .expect("object heads should seed");

    sqlx::query(
        "insert into conflicts \
         (conflict_id, original_path, base_revision_id, current_revision_id, incoming_revision_id, \
          incoming_adapter_id, policy_applied, materialized_path, status) \
         values ($1, $2, null, $3, $4, 'obsidian-plugin', \
                 'current_wins_with_incoming_backup', $5, $6)",
    )
    .bind(conflict_id)
    .bind(original_path)
    .bind(current_revision_id)
    .bind(incoming_revision_id)
    .bind(conflict_path)
    .bind(status)
    .execute(pool)
    .await
    .expect("conflict should seed");

    (
        original_path.to_string(),
        conflict_path.to_string(),
        current_revision_id.to_string(),
    )
}

#[tokio::test]
async fn get_open_conflicts_without_storage_returns_safe_unavailable_error() {
    let (status, json) = request_json("GET", "/conflicts?status=open", Body::empty()).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(json["error"]["code"], "internal_error");
    let rendered = json.to_string();
    assert!(!rendered.contains("postgres://"));
    assert!(!rendered.contains("conflict_route_test_token"));
    assert!(!rendered.contains("sqlx::Error"));
}

#[tokio::test]
async fn unsupported_conflict_status_returns_safe_bad_request() {
    let (status, json) = request_json("GET", "/conflicts?status=resolved", Body::empty()).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(json["error"]["code"], "validation_error");
}

#[tokio::test]
async fn resolve_actions_without_storage_are_safe_and_deterministic() {
    for action in [
        "accept_current",
        "accept_conflict",
        "keep_both",
        "mark_resolved",
    ] {
        let body = Body::from(format!(r#"{{"resolution":"{action}"}}"#));
        let (status, json) = request_json("POST", "/conflicts/conf_01J/resolve", body).await;
        assert_eq!(status, StatusCode::NOT_IMPLEMENTED, "action {action}");
        assert_eq!(json["error"]["code"], "not_implemented");
    }
}

#[test]
fn conflict_resolution_action_support_is_narrow_and_explicit() {
    assert!(resolution_is_metadata_only(
        &ConflictResolutionDto::AcceptCurrent
    ));
    assert!(resolution_is_metadata_only(
        &ConflictResolutionDto::KeepBoth
    ));
    assert!(resolution_is_metadata_only(
        &ConflictResolutionDto::MarkResolved
    ));
    assert!(!resolution_is_metadata_only(
        &ConflictResolutionDto::AcceptConflict
    ));
}

#[test]
fn deterministic_conflict_resolved_operation_id_is_safe() {
    let conflict_id = ConflictId::parse("conf_01J").unwrap();
    let adapter_id = AdapterId::parse("obsidian-plugin").unwrap();
    let op_id = conflict_resolved_operation_id(&conflict_id, &adapter_id).unwrap();
    assert!(op_id.as_str().starts_with("op_"));
}

#[tokio::test]
async fn metadata_only_resolution_actions_persist_resolution_without_mutating_current_file_when_real_postgres_is_available(
) {
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

    for action in ["accept_current", "keep_both", "mark_resolved"] {
        context
            .clean_storage_tables()
            .await
            .expect("tables should clean");

        let pool = context.pool().clone();
        let principal = principal();
        seed_adapter(&pool, principal.adapter_id(), "obsidian_plugin").await;
        let (original_path, conflict_path, current_revision_id) =
            seed_conflict_fixture(&pool, "conf_01JMETA", "open").await;

        let state = runtime_state(pool.clone(), principal);
        let (status, json) = request_json_with_state(
            state,
            "POST",
            "/conflicts/conf_01JMETA/resolve",
            Body::from(format!(r#"{{"resolution":"{action}"}}"#)),
        )
        .await;

        assert_eq!(status, StatusCode::OK, "action {action}");
        assert_eq!(json["status"], "resolved");
        assert_eq!(json["resolution"], action);

        let conflict = ConflictRepository::new()
            .get_by_id(&pool, &ConflictId::parse("conf_01JMETA").unwrap())
            .await
            .expect("conflict should load")
            .expect("conflict should exist");
        assert_eq!(conflict.status, "resolved");
        assert_eq!(conflict.original_path, original_path);
        assert_eq!(conflict.materialized_path, conflict_path);
        assert_eq!(conflict.current_revision_id, current_revision_id);
        assert_eq!(conflict.resolved_by.as_deref(), Some("obsidian-plugin"));
        assert!(conflict.resolved_at.is_some());

        let current_head: Option<String> =
            sqlx::query_scalar("select current_revision_id from sync_objects where path = $1")
                .bind(original_path.as_str())
                .fetch_one(&pool)
                .await
                .expect("current object head should load");
        assert_eq!(current_head.as_deref(), Some(current_revision_id.as_str()));

        let preserved_conflict_head: Option<String> =
            sqlx::query_scalar("select current_revision_id from sync_objects where path = $1")
                .bind(conflict_path.as_str())
                .fetch_one(&pool)
                .await
                .expect("conflict object head should load");
        assert_eq!(preserved_conflict_head.as_deref(), Some("rev_incoming"));

        let operations = OperationLogRepository::new()
            .list_since(&pool, 0, 10)
            .await
            .expect("operations should load");
        assert_eq!(operations.len(), 1, "action {action}");
        assert_eq!(operations[0].kind, "conflict_resolved");
        assert_eq!(operations[0].path, original_path);
        assert_eq!(operations[0].revision_id, None);
        assert_eq!(operations[0].conflict_id.as_deref(), Some("conf_01JMETA"));
    }
}

#[tokio::test]
async fn accept_conflict_stays_not_implemented_and_does_not_mutate_state_when_real_postgres_is_available(
) {
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
    let (original_path, _conflict_path, current_revision_id) =
        seed_conflict_fixture(&pool, "conf_01JDEFER", "open").await;

    let state = runtime_state(pool.clone(), principal);
    let (status, json) = request_json_with_state(
        state,
        "POST",
        "/conflicts/conf_01JDEFER/resolve",
        Body::from(r#"{"resolution":"accept_conflict"}"#),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
    assert_eq!(json["error"]["code"], "not_implemented");

    let conflict = ConflictRepository::new()
        .get_by_id(&pool, &ConflictId::parse("conf_01JDEFER").unwrap())
        .await
        .expect("conflict should load")
        .expect("conflict should exist");
    assert_eq!(conflict.status, "open");
    assert!(conflict.resolved_at.is_none());
    assert!(conflict.resolved_by.is_none());

    let current_head: Option<String> =
        sqlx::query_scalar("select current_revision_id from sync_objects where path = $1")
            .bind(original_path.as_str())
            .fetch_one(&pool)
            .await
            .expect("current object head should load");
    assert_eq!(current_head.as_deref(), Some(current_revision_id.as_str()));

    let operation_count: i64 = sqlx::query_scalar("select count(*) from operation_log")
        .fetch_one(&pool)
        .await
        .expect("count should load");
    assert_eq!(operation_count, 0);
}

#[tokio::test]
async fn resolved_conflicts_cannot_be_resolved_twice_when_real_postgres_is_available() {
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
    seed_conflict_fixture(&pool, "conf_01JTWICE", "open").await;

    let first_state = runtime_state(pool.clone(), principal.clone());
    let (first_status, first_json) = request_json_with_state(
        first_state,
        "POST",
        "/conflicts/conf_01JTWICE/resolve",
        Body::from(r#"{"resolution":"mark_resolved"}"#),
    )
    .await;
    assert_eq!(first_status, StatusCode::OK);
    assert_eq!(first_json["status"], "resolved");

    let second_state = runtime_state(pool.clone(), principal);
    let (second_status, second_json) = request_json_with_state(
        second_state,
        "POST",
        "/conflicts/conf_01JTWICE/resolve",
        Body::from(r#"{"resolution":"mark_resolved"}"#),
    )
    .await;
    assert_eq!(second_status, StatusCode::CONFLICT);
    assert_eq!(second_json["error"]["code"], "conflict");

    let operation_count: i64 = sqlx::query_scalar("select count(*) from operation_log")
        .fetch_one(&pool)
        .await
        .expect("count should load");
    assert_eq!(operation_count, 1);
}
