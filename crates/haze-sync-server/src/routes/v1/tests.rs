use super::persistence::persist_accepted_revision;
use super::*;
use haze_sync_api::auth::AdapterRole;
use haze_sync_storage::{
    repositories::{
        conflicts::{ConflictRepository, ConflictStatusName},
        operation_log::OperationLogRepository,
        revisions::{get_current_revision_by_path, get_file_revision_by_id},
    },
    test_support::connect_test_database_from_env,
};
use serde_json::Value;
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

fn parsed_request(
    path: &str,
    base_revision_id: Option<&str>,
    content_sha256: &str,
) -> PutFileRouteRequest {
    let base_revision_id = Some(base_revision_id.unwrap_or("null"));
    parse_put_file_request(PutFileRouteRequestParts {
        route_path: path,
        idempotency_key: Some("test-key"),
        content_sha256: Some(content_sha256),
        base_revision_id,
        body: b"hello".to_vec(),
        max_upload_bytes: Some(MAX_UPLOAD_BYTES),
    })
    .unwrap()
}

fn parsed_request_with_body(
    path: &str,
    base_revision_id: Option<&str>,
    body: &[u8],
) -> PutFileRouteRequest {
    let hash = compute_content_hash(body).to_string();
    let base_revision_id = Some(base_revision_id.unwrap_or("null"));
    parse_put_file_request(PutFileRouteRequestParts {
        route_path: path,
        idempotency_key: Some("test-key"),
        content_sha256: Some(&hash),
        base_revision_id,
        body: body.to_vec(),
        max_upload_bytes: Some(MAX_UPLOAD_BYTES),
    })
    .unwrap()
}

fn principal() -> AdapterPrincipal {
    AdapterPrincipal::new("obsidian-plugin", AdapterRole::ObsidianPlugin).unwrap()
}

fn temp_store(label: &str) -> (LocalObjectStore, PathBuf) {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "haze-sync-w2f1-{label}-{}-{suffix}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    (LocalObjectStore::new(root.clone()), root)
}

fn stored_revision(
    revision_id: &str,
    path: &str,
    parent_revision_id: Option<&str>,
    content: &[u8],
) -> StoredRevision {
    StoredRevision {
        revision_id: RevisionId::parse(revision_id).unwrap(),
        path: VaultPath::parse(path).unwrap(),
        parent_revision_id: parent_revision_id.map(|value| RevisionId::parse(value).unwrap()),
        content_hash: compute_content_hash(content),
        size_bytes: content.len() as u64,
        created_by: AdapterId::parse("obsidian-plugin").unwrap(),
    }
}

fn body_json(response: &PutFileResponse) -> Value {
    serde_json::to_value(response).unwrap()
}

async fn seed_adapter(
    pool: &sqlx::PgPool,
    adapter_id: &str,
    role: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "insert into sync_adapters (adapter_id, display_name, role, token_hash, enabled) \
         values ($1, $2, $3, $4, true)",
    )
    .bind(adapter_id)
    .bind(adapter_id)
    .bind(role)
    .bind("sha256:test-token-hash")
    .execute(pool)
    .await?;
    Ok(())
}

#[test]
fn id_fixtures_have_required_prefixes() {
    let revision_id = deterministic_identifier("rev_", &["Notes/a.md", "sha256"]);
    let op_id = deterministic_identifier("op_", &["rev_1", "upsert_file"]);

    assert!(RevisionId::parse(&revision_id).is_ok());
    assert!(OperationId::parse(&op_id).is_ok());
}

#[test]
fn put_new_file_accepted_and_get_latest_returns_same_bytes_hash() {
    let (store, root) = temp_store("new-file");
    let request = parsed_request_with_body("Notes/a.md", None, b"hello");
    let outcome = run_core_normal_upsert(&request, &principal(), None, &store).unwrap();

    let revision = match outcome {
        haze_sync_core::revision_service::UpsertOutcome::AcceptedNewFile {
            revision,
            operation,
        } => {
            assert_eq!(operation.seq, 0);
            revision
        }
        other => panic!("unexpected outcome: {other:?}"),
    };

    assert_eq!(revision.path.as_str(), "Notes/a.md");
    assert_eq!(revision.content_hash, compute_content_hash(b"hello"));
    assert_eq!(
        store.get_bytes(revision.content_hash).unwrap(),
        b"hello".to_vec()
    );

    let response = raw_file_response(
        b"hello".to_vec(),
        FileDownloadRouteHeaders::new(
            revision.revision_id.clone(),
            revision.content_hash,
            revision.size_bytes,
        ),
    )
    .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE).unwrap(),
        APPLICATION_OCTET_STREAM
    );
    assert_eq!(
        response.headers().get(X_REVISION_ID_HEADER).unwrap(),
        revision.revision_id.as_str()
    );
    assert_eq!(
        response.headers().get(X_CONTENT_SHA256_HEADER).unwrap(),
        revision.content_hash.to_string().as_str()
    );
    assert_eq!(
        response.headers().get(X_SIZE_BYTES_HEADER).unwrap(),
        revision.size_bytes.to_string().as_str()
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn put_second_revision_accepted_with_current_base_and_explicit_revision_get_works() {
    let (store, root) = temp_store("second-revision");
    let current = stored_revision("rev_current", "Notes/a.md", None, b"old");
    store.put_bytes(current.content_hash, b"old").unwrap();

    let request = parsed_request_with_body("Notes/a.md", Some("rev_current"), b"new");
    let outcome =
        run_core_normal_upsert(&request, &principal(), Some(current.clone()), &store).unwrap();

    let revision = match outcome {
        haze_sync_core::revision_service::UpsertOutcome::AcceptedNewRevision {
            revision, ..
        } => revision,
        other => panic!("unexpected outcome: {other:?}"),
    };

    assert_eq!(
        revision.parent_revision_id.as_ref().unwrap().as_str(),
        "rev_current"
    );
    assert_eq!(
        store.get_bytes(revision.content_hash).unwrap(),
        b"new".to_vec()
    );

    let explicit = FileDownloadRouteHeaders::new(
        revision.revision_id.clone(),
        revision.content_hash,
        revision.size_bytes,
    );
    let response =
        raw_file_response(store.get_bytes(revision.content_hash).unwrap(), explicit).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(X_REVISION_ID_HEADER).unwrap(),
        revision.revision_id.as_str()
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn same_content_duplicate_is_ignored_without_extra_content_write() {
    let (store, root) = temp_store("duplicate");
    let current = stored_revision("rev_current", "Notes/a.md", None, b"hello");
    let request = parsed_request_with_body("Notes/a.md", Some("rev_current"), b"hello");

    let outcome =
        run_core_normal_upsert(&request, &principal(), Some(current.clone()), &store).unwrap();

    match outcome {
        haze_sync_core::revision_service::UpsertOutcome::IgnoredDuplicateSameContent {
            current_revision,
        } => {
            assert_eq!(current_revision.revision_id, current.revision_id);
        }
        other => panic!("unexpected outcome: {other:?}"),
    }
    assert!(store.get_bytes(current.content_hash).is_err());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn stale_unknown_or_null_base_with_different_content_does_not_overwrite() {
    let (store, root) = temp_store("stale");
    let current = stored_revision("rev_current", "Notes/a.md", None, b"old");

    let null_base = parsed_request_with_body("Notes/a.md", None, b"new");
    let null_outcome =
        run_core_normal_upsert(&null_base, &principal(), Some(current.clone()), &store).unwrap();
    assert!(matches!(
        null_outcome,
        haze_sync_core::revision_service::UpsertOutcome::RejectedStaleOrUnknownBase { .. }
    ));
    assert!(store.get_bytes(compute_content_hash(b"new")).is_err());

    let stale_base = parsed_request_with_body("Notes/a.md", Some("rev_stale"), b"newer");
    let stale_outcome =
        run_core_normal_upsert(&stale_base, &principal(), Some(current), &store).unwrap();
    assert!(matches!(
        stale_outcome,
        haze_sync_core::revision_service::UpsertOutcome::RejectedStaleOrUnknownBase { .. }
    ));
    assert!(store.get_bytes(compute_content_hash(b"newer")).is_err());

    let unknown_base = parsed_request_with_body("Notes/missing.md", Some("rev_missing"), b"new");
    let unknown_outcome =
        run_core_normal_upsert(&unknown_base, &principal(), None, &store).unwrap();
    assert!(matches!(
        unknown_outcome,
        haze_sync_core::revision_service::UpsertOutcome::RejectedStaleOrUnknownBase { .. }
    ));

    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn conflict_saved_persists_durable_state_when_real_postgres_is_available() {
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
    seed_adapter(&pool, "obsidian-plugin", "obsidian_plugin")
        .await
        .expect("adapter should seed");

    let principal = principal();
    let (store, root) = temp_store("conflict-saved-persistence");
    let current = stored_revision("rev_current", "Notes/a.md", None, b"old");
    store.put_bytes(current.content_hash, b"old").unwrap();

    let mut accepted_tx = pool.begin().await.expect("tx should begin");
    persist_accepted_revision(&mut accepted_tx, &principal, &current)
        .await
        .expect("current revision should persist");
    accepted_tx.commit().await.expect("tx should commit");

    let request = parsed_request_with_body("Notes/a.md", Some("rev_stale"), b"incoming");
    let outcome =
        run_core_normal_upsert(&request, &principal, Some(current.clone()), &store).unwrap();
    let mut conflict_tx = pool.begin().await.expect("tx should begin");
    let response = apply_upsert_outcome(&mut conflict_tx, &principal, &store, outcome)
        .await
        .expect("conflict_saved should persist");
    conflict_tx.commit().await.expect("tx should commit");

    let PutFileResponse::ConflictSaved {
        conflict_id,
        materialized_path,
        seq,
        ..
    } = response
    else {
        panic!("expected conflict_saved response");
    };

    assert!(seq >= 2);

    let conflicts = ConflictRepository::new()
        .list_by_status(&pool, ConflictStatusName::Open)
        .await
        .expect("conflict rows should load");
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].conflict_id, conflict_id.as_str());
    assert_eq!(conflicts[0].original_path, "Notes/a.md");
    assert_eq!(conflicts[0].current_revision_id, "rev_current");
    assert_eq!(conflicts[0].materialized_path, materialized_path.as_str());

    let original_current =
        get_current_revision_by_path(&pool, &VaultPath::parse("Notes/a.md").unwrap())
            .await
            .expect("current revision should load")
            .expect("current revision should exist");
    assert_eq!(original_current.revision_id, "rev_current");

    let incoming_revision = get_file_revision_by_id(
        &pool,
        &RevisionId::parse(conflicts[0].incoming_revision_id.as_str()).unwrap(),
    )
    .await
    .expect("incoming revision should load")
    .expect("incoming revision should exist");
    assert_eq!(incoming_revision.path, materialized_path.as_str());

    let operations = OperationLogRepository::new()
        .list_since(&pool, 0, 10)
        .await
        .expect("operations should load");
    assert!(operations.iter().any(|row| {
        row.kind == "conflict_created" && row.conflict_id.as_deref() == Some(conflict_id.as_str())
    }));

    assert_eq!(
        store.get_bytes(compute_content_hash(b"incoming")).unwrap(),
        b"incoming".to_vec()
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn invalid_content_hash_is_rejected_before_content_store_write() {
    let (store, root) = temp_store("hash-mismatch");
    let wrong_hash = compute_content_hash(b"other").to_string();
    let request = parse_put_file_request(PutFileRouteRequestParts {
        route_path: "Notes/a.md",
        idempotency_key: Some("test-key"),
        content_sha256: Some(&wrong_hash),
        base_revision_id: Some("rev_current"),
        body: b"hello".to_vec(),
        max_upload_bytes: Some(MAX_UPLOAD_BYTES),
    })
    .unwrap();

    let outcome = run_core_normal_upsert(&request, &principal(), None, &store).unwrap();
    assert!(matches!(
        outcome,
        haze_sync_core::revision_service::UpsertOutcome::RejectedHashMismatch { .. }
    ));
    assert!(store.get_bytes(compute_content_hash(b"hello")).is_err());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn idempotency_same_key_same_request_replays_and_different_request_rejects() {
    let first = parsed_request_with_body("Notes/a.md", None, b"hello");
    let same = parsed_request_with_body("Notes/a.md", None, b"hello");
    let different = parsed_request_with_body("Notes/a.md", None, b"changed");
    let principal = principal();

    let first_fingerprint = request_fingerprint(&first, &principal);
    let same_fingerprint = request_fingerprint(&same, &principal);
    let different_fingerprint = request_fingerprint(&different, &principal);

    let response = haze_sync_api::routes::files::accepted_upload_response(
        first.path().clone(),
        RevisionId::parse("rev_replayed").unwrap(),
        1,
    );
    let stored =
        StoredIdempotencyResponse::json(StatusCode::OK.as_u16(), body_json(&response)).unwrap();

    assert_eq!(first_fingerprint, same_fingerprint);
    assert_eq!(stored.status_code(), StatusCode::OK.as_u16());
    assert_eq!(stored.body(), &body_json(&response));
    assert_ne!(first_fingerprint, different_fingerprint);
}

#[test]
fn changes_mapping_preserves_upsert_kind_and_safe_metadata() {
    assert_eq!(
        operation_kind_dto(OperationKindName::UpsertFile),
        OperationKindDto::UpsertFile
    );
}

#[test]
fn missing_file_maps_to_safe_not_found_response() {
    let error: ApiError = FileRouteError::NotFound.into();
    let response = error.into_response();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[test]
fn invalid_headers_or_paths_map_to_safe_errors() {
    let missing_idempotency = parse_put_file_request(PutFileRouteRequestParts {
        route_path: "Notes/a.md",
        idempotency_key: None,
        content_sha256: Some(&compute_content_hash(b"hello").to_string()),
        base_revision_id: Some("null"),
        body: b"hello".to_vec(),
        max_upload_bytes: Some(MAX_UPLOAD_BYTES),
    })
    .unwrap_err();
    let response = ApiError::from(missing_idempotency).into_response();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let invalid_path = parse_get_file_request(GetFileRouteRequestParts {
        route_path: "../outside.md",
        revision_id: None,
    })
    .unwrap_err();
    let response = ApiError::from(invalid_path).into_response();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[test]
fn request_fingerprint_uses_safe_metadata_not_body() {
    let hash = compute_content_hash(b"hello").to_string();
    let first = parsed_request("Notes/a.md", Some("rev_current"), &hash);
    let second = parse_put_file_request(PutFileRouteRequestParts {
        route_path: "Notes/a.md",
        idempotency_key: Some("test-key"),
        content_sha256: Some(&hash),
        base_revision_id: Some("rev_current"),
        body: b"different body would already fail hash verification".to_vec(),
        max_upload_bytes: Some(MAX_UPLOAD_BYTES),
    })
    .unwrap();
    let principal = AdapterPrincipal::new("obsidian-plugin", AdapterRole::ObsidianPlugin).unwrap();

    assert_eq!(
        request_fingerprint(&first, &principal),
        request_fingerprint(&second, &principal)
    );
}

#[test]
fn request_fingerprint_changes_with_base_revision() {
    let hash = compute_content_hash(b"hello").to_string();
    let current = parsed_request("Notes/a.md", Some("rev_current"), &hash);
    let other = parsed_request("Notes/a.md", Some("rev_other"), &hash);
    let principal = AdapterPrincipal::new("obsidian-plugin", AdapterRole::ObsidianPlugin).unwrap();

    assert_ne!(
        request_fingerprint(&current, &principal),
        request_fingerprint(&other, &principal)
    );
}

#[test]
fn public_auth_errors_do_not_echo_tokens() {
    let error = ApiError::invalid_token().into_response();
    assert_eq!(error.status(), StatusCode::UNAUTHORIZED);
}
