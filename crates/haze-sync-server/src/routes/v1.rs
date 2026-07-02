//! V1 Core API route wiring for W2 file operations and W3 safety fan-in.

use axum::{
    body::{Body, Bytes},
    extract::{Path, Query},
    http::{header, HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Extension, Json, Router,
};
use chrono::{Duration, SecondsFormat, Utc};
use haze_sync_api::{
    auth::{AdapterPrincipal, AdapterRole, BearerToken},
    contracts::{
        errors::{ErrorResponse, PublicErrorCode},
        headers::{
            AUTHORIZATION_HEADER, IDEMPOTENCY_KEY_HEADER, X_BASE_REVISION_ID_HEADER,
            X_CONTENT_SHA256_HEADER,
        },
    },
    dto::{
        changes::ChangeEntryDto,
        common::{ConflictPolicyDto, ConflictResolutionDto, ConflictStatusDto, OperationKindDto},
        files::{DeleteFileResponse, PutFileResponse},
        primitives::{
            AdapterIdDto, ConflictIdDto, ContentSha256Dto, RevisionIdDto, TimestampDto,
            TombstoneIdDto, VaultPathDto,
        },
        server::{ServerCapabilityDto, ServerInfoResponse},
    },
    routes::{
        admin::{
            AdapterCursorSummary, AdapterListResponse, AdapterSummary, DependencyReadinessState,
            PauseStatusSummary, ServerStatus, StatusSummaryResponse,
        },
        changes::{changes_response_from_parts, parse_changes_query, ChangesRouteError},
        conflicts::{
            conflict_list_response_from_parts, parse_conflicts_query, parse_resolve_conflict_request,
            resolved_conflict_response, ConflictListRequestParts, ConflictRouteSummaryParts,
            ConflictsRouteError, ResolveConflictRequestParts,
        },
        delete::{
            not_found_delete_response, parse_delete_file_request, stale_base_delete_response,
            tombstoned_delete_response, unsafe_delete_response, DeleteFileRouteRequest,
            DeleteFileRouteRequestParts, DeleteRouteError,
        },
        files::{
            accepted_upload_response, ignored_same_content_response, parse_get_file_request,
            parse_put_file_request, FileDownloadRouteHeaders, FileRouteError,
            GetFileRouteRequestParts, PutFileRouteRequest, PutFileRouteRequestParts,
            APPLICATION_OCTET_STREAM, X_REVISION_ID_HEADER, X_SIZE_BYTES_HEADER,
        },
    },
};
use haze_sync_common::{
    AdapterId, AdapterMode, AdapterRole as CommonAdapterRole, ConflictId, ContentHash, OperationId,
    RevisionId, VaultPath,
};
use haze_sync_core::{
    delete_guard::{DeleteGuard, DeleteGuardDecision, DeleteGuardInput, DeleteGuardPolicy, DeleteRatioLimit, DeleteRunScope},
    idempotency::{RequestFingerprint, StoredIdempotencyResponse},
    revision_service::{
        AppendOperationRequest, ContentStore, InsertRevisionRequest, OperationKind, OperationLog,
        OperationLogEntry, RevisionRepository, RevisionService, RevisionServiceError, StoredContent,
        StoredRevision, UpsertFileRequest, UpsertOutcome,
    },
    safety_fan_in::{plan_conflict_saved_fan_in, ConflictSavedFanInPlan},
    tombstone_service::{
        TombstoneCreationInput, TombstoneId, TombstoneRetention, TombstoneService,
    },
};
use haze_sync_storage::{
    locks::lock_vault_path,
    models::{FileRevisionRow, OperationLogRow},
    object_store::ObjectStore,
    repositories::{
        idempotency::{
            compare_request_fingerprint, insert_idempotency_record, read_idempotency_record,
            IdempotencyRecordInput, IdempotencyRequestComparison, IdempotencyStoreOutcome,
        },
        objects::{
            create_or_find_sync_object_by_path, set_current_revision_by_object_id,
            set_sync_object_deleted_at_by_path, NewSyncObject, SyncObjectKind,
        },
        operation_log::{
            AppendOperationLogEntry, ChangeFeedRow, OperationKindName, OperationLogRepository,
        },
        revisions::{get_current_revision_by_path, get_file_revision_by_id, insert_file_revision, NewFileRevision},
        tombstones::{insert_tombstone, NewTombstone},
        RepositoryError,
    },
    LocalObjectStore, ObjectStoreError,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256 as Sha256Digest};
use sqlx::{PgPool, Row};
use std::{collections::HashMap, str::FromStr};

use crate::{
    http::errors::{not_implemented_response, ShellErrorResponse},
    state::{AuthState, ServerAppState},
};

const MAX_UPLOAD_BYTES: u64 = 52_428_800;
const DELETE_RETENTION_DAYS: i64 = 30;

pub fn router() -> Router {
    Router::new()
        .route("/server-info", get(server_info))
        .route("/changes", get(changes_route))
        .route(
            "/files/*path",
            get(get_file_route)
                .put(put_file_route)
                .delete(delete_file_route),
        )
        .route("/conflicts", get(list_conflicts_route))
        .route("/conflicts/{conflict_id}/resolve", post(resolve_conflict_route))
        .route("/admin/status", get(admin_status_route))
        .route("/admin/adapters", get(admin_adapters_route))
}

pub async fn server_info() -> Json<ServerInfoResponse> {
    Json(ServerInfoResponse {
        server_id: "haze-sync-route-shell".to_owned(),
        protocol_version: 1,
        max_upload_bytes: MAX_UPLOAD_BYTES,
        capabilities: vec![
            ServerCapabilityDto::Sha256,
            ServerCapabilityDto::OperationLog,
            ServerCapabilityDto::Tombstones,
            ServerCapabilityDto::Conflicts,
            ServerCapabilityDto::ConflictCenter,
            ServerCapabilityDto::BatchChanges,
        ],
    })
}

async fn put_file_route(
    Extension(state): Extension<ServerAppState>,
    Path(route_path): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, ApiError> {
    let principal = authenticate(&state, &headers, RoutePermission::Write).await?;
    let request = parse_put_file_request(PutFileRouteRequestParts {
        route_path: route_path.as_str(),
        idempotency_key: optional_header(&headers, IDEMPOTENCY_KEY_HEADER, HeaderErrorKind::Idempotency)?,
        content_sha256: optional_header(&headers, X_CONTENT_SHA256_HEADER, HeaderErrorKind::ContentSha256)?,
        base_revision_id: optional_header(&headers, X_BASE_REVISION_ID_HEADER, HeaderErrorKind::BaseRevision)?,
        body: body.to_vec(),
        max_upload_bytes: Some(MAX_UPLOAD_BYTES),
    })?;
    let pool = runtime_pool(&state)?;
    let object_store = runtime_object_store(&state)?;
    let fingerprint = put_request_fingerprint(&request, &principal);

    if let Some(replay) = read_existing_idempotency(pool, &principal, request.idempotency_key().as_str(), fingerprint).await? {
        return replay_response(replay);
    }

    let mut transaction = pool.begin().await.map_err(|_| ApiError::internal())?;
    lock_vault_path(&mut *transaction, request.path()).await.map_err(map_repository_error)?;
    let current_revision = get_current_revision_by_path(&mut *transaction, request.path())
        .await
        .map_err(map_repository_error)?
        .map(stored_revision_from_row)
        .transpose()?;
    let outcome = run_core_normal_upsert(&request, &principal, current_revision, object_store)?;
    let response_body = apply_upsert_outcome(&mut transaction, &principal, outcome, object_store).await?;
    let response_json = serde_json::to_value(&response_body).map_err(|_| ApiError::internal())?;

    if let Some(replay) = store_successful_idempotency(
        &mut transaction,
        &principal,
        request.idempotency_key().as_str(),
        fingerprint,
        StatusCode::OK,
        response_json,
    )
    .await?
    {
        transaction.rollback().await.map_err(|_| ApiError::internal())?;
        return replay_response(replay);
    }
    transaction.commit().await.map_err(|_| ApiError::internal())?;
    Ok((StatusCode::OK, Json(response_body)).into_response())
}

async fn get_file_route(
    Extension(state): Extension<ServerAppState>,
    Path(route_path): Path<String>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    authenticate(&state, &headers, RoutePermission::Read).await?;
    let request = parse_get_file_request(GetFileRouteRequestParts {
        route_path: route_path.as_str(),
        revision_id: query.get("revision_id").map(String::as_str),
    })?;
    let pool = runtime_pool(&state)?;
    let object_store = runtime_object_store(&state)?;
    let row = if let Some(revision_id) = request.revision_id() {
        let row = get_file_revision_by_id(pool, revision_id)
            .await
            .map_err(map_repository_error)?
            .ok_or(FileRouteError::NotFound)?;
        if row.path != request.path().as_str() {
            return Err(FileRouteError::NotFound.into());
        }
        row
    } else {
        get_current_revision_by_path(pool, request.path())
            .await
            .map_err(map_repository_error)?
            .ok_or(FileRouteError::NotFound)?
    };
    let revision = stored_revision_from_row(row)?;
    let bytes = object_store.get_bytes(revision.content_hash).map_err(map_object_store_get_error)?;
    if u64::try_from(bytes.len()).map_err(|_| ApiError::internal())? != revision.size_bytes {
        return Err(ApiError::internal());
    }
    raw_file_response(
        bytes,
        FileDownloadRouteHeaders::new(revision.revision_id, revision.content_hash, revision.size_bytes),
    )
}

async fn delete_file_route(
    Extension(state): Extension<ServerAppState>,
    Path(route_path): Path<String>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let principal = authenticate(&state, &headers, RoutePermission::Write).await?;
    let request = parse_delete_file_request(DeleteFileRouteRequestParts {
        route_path: route_path.as_str(),
        idempotency_key: optional_header(&headers, IDEMPOTENCY_KEY_HEADER, HeaderErrorKind::Idempotency)?,
        base_revision_id: optional_header(&headers, X_BASE_REVISION_ID_HEADER, HeaderErrorKind::BaseRevision)?,
        requested_delete_count: None,
    })?;
    request.auth_requirement().validate_role(principal.role())?;
    let pool = runtime_pool(&state)?;
    let fingerprint = delete_request_fingerprint(&request, &principal);

    if let Some(replay) = read_existing_idempotency(pool, &principal, request.idempotency_key().as_str(), fingerprint).await? {
        return replay_response(replay);
    }

    let mut transaction = pool.begin().await.map_err(|_| ApiError::internal())?;
    lock_vault_path(&mut *transaction, request.path()).await.map_err(map_repository_error)?;
    let current = get_current_revision_by_path(&mut *transaction, request.path())
        .await
        .map_err(map_repository_error)?
        .map(stored_revision_from_row)
        .transpose()?;
    let Some(current) = current else {
        return Ok((StatusCode::NOT_FOUND, Json(not_found_delete_response(request.path().clone()))).into_response());
    };
    if request.base_revision_id() != Some(&current.revision_id) {
        return Ok((StatusCode::CONFLICT, Json(stale_base_delete_response(request.path().clone()))).into_response());
    }
    if !single_delete_guard_allows(&principal)? {
        return Ok((StatusCode::CONFLICT, Json(unsafe_delete_response(request.path().clone()))).into_response());
    }

    let now = Utc::now();
    let retention_until = now + Duration::days(DELETE_RETENTION_DAYS);
    let tombstone_id = TombstoneId::parse(&deterministic_identifier(
        "tmb_",
        &[current.revision_id.as_str(), request.path().as_str(), principal.adapter_id()],
    ))
    .map_err(|_| ApiError::internal())?;
    let tombstone = TombstoneService::new()
        .create_tombstone(TombstoneCreationInput::new(
            tombstone_id,
            request.path().clone(),
            current.revision_id.clone(),
            Some(current.revision_id.clone()),
            principal.common_adapter_id().clone(),
            TombstoneRetention::new(retention_until, Some(DELETE_RETENTION_DAYS as u16)),
            Some(now),
        ))
        .map_err(|_| ApiError::internal())?;
    let tombstone_row = insert_tombstone(
        &mut **transaction,
        NewTombstone {
            tombstone_id: tombstone.tombstone_id.as_str(),
            path: &tombstone.path,
            deleted_revision_id: Some(tombstone.deleted_revision_id()),
            deleted_by: &tombstone.deleted_by,
            retention_until: tombstone.retention.retention_until,
        },
    )
    .await
    .map_err(map_repository_error)?;
    set_sync_object_deleted_at_by_path(
        &mut **transaction,
        request.path(),
        Some(tombstone_row.deleted_at),
        principal.common_adapter_id(),
    )
    .await
    .map_err(map_repository_error)?;
    let operation = append_operation_log(
        &mut transaction,
        principal.common_adapter_id().clone(),
        OperationKindName::DeleteFile,
        request.path().clone(),
        None,
        Some(tombstone_row.tombstone_id.clone()),
        None,
        &[tombstone_row.tombstone_id.as_str(), request.path().as_str()],
    )
    .await?;
    let response_body = tombstoned_delete_response(
        request.path().clone(),
        tombstone_row.tombstone_id,
        operation.seq,
        timestamp_string(tombstone_row.retention_until),
    );
    let response_json = serde_json::to_value(&response_body).map_err(|_| ApiError::internal())?;
    if let Some(replay) = store_successful_idempotency(
        &mut transaction,
        &principal,
        request.idempotency_key().as_str(),
        fingerprint,
        StatusCode::OK,
        response_json,
    )
    .await?
    {
        transaction.rollback().await.map_err(|_| ApiError::internal())?;
        return replay_response(replay);
    }
    transaction.commit().await.map_err(|_| ApiError::internal())?;
    Ok((StatusCode::OK, Json(response_body)).into_response())
}

async fn changes_route(
    Extension(state): Extension<ServerAppState>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    authenticate(&state, &headers, RoutePermission::Read).await?;
    let request = parse_changes_query(query.get("since").map(String::as_str), query.get("limit").map(String::as_str))?;
    let pool = runtime_pool(&state)?;
    let page = OperationLogRepository::new()
        .changes_since(pool, request.since_value(), request.limit_value())
        .await
        .map_err(map_repository_error)?;
    let changes = page.changes.into_iter().map(change_entry_from_row).collect::<Result<Vec<_>, _>>()?;
    let response = changes_response_from_parts(page.from_seq, page.to_seq, page.has_more, changes)?;
    Ok((StatusCode::OK, Json(response)).into_response())
}

async fn list_conflicts_route(
    Extension(state): Extension<ServerAppState>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    authenticate(&state, &headers, RoutePermission::Read).await?;
    let _request = parse_conflicts_query(ConflictListRequestParts { status: query.get("status").map(String::as_str) })?;
    let Some(pool) = state.db_pool() else {
        return Ok((StatusCode::OK, Json(conflict_list_response_from_parts(Vec::new()))).into_response());
    };
    let rows = sqlx::query(
        "select conflict_id, original_path, base_revision_id, current_revision_id, incoming_revision_id, \
         incoming_adapter_id, policy_applied, materialized_path, status, created_at, resolved_at \
         from conflicts where status = 'open' order by created_at asc, conflict_id asc limit 100",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal())?;
    let mut conflicts = Vec::with_capacity(rows.len());
    for row in rows {
        conflicts.push(conflict_summary_from_row(&row)?);
    }
    Ok((StatusCode::OK, Json(conflict_list_response_from_parts(conflicts))).into_response())
}

async fn resolve_conflict_route(
    Extension(state): Extension<ServerAppState>,
    Path(conflict_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Response, ApiError> {
    let principal = authenticate(&state, &headers, RoutePermission::ResolveConflict).await?;
    let object = body.as_object().ok_or_else(ConflictsRouteError::invalid_resolve_payload)?;
    let resolution = object.get("resolution").and_then(Value::as_str);
    let extra_fields = object.keys().filter(|key| key.as_str() != "resolution").map(String::as_str).collect::<Vec<_>>();
    let request = parse_resolve_conflict_request(ResolveConflictRequestParts {
        conflict_id: conflict_id.as_str(),
        resolution,
        extra_fields: &extra_fields,
    })?;
    if request.resolution == ConflictResolutionDto::AcceptConflict {
        return Ok(core_route_not_implemented().await.into_response());
    }
    let Some(pool) = state.db_pool() else {
        return Ok(core_route_not_implemented().await.into_response());
    };
    let mut transaction = pool.begin().await.map_err(|_| ApiError::internal())?;
    let row = sqlx::query(
        "update conflicts set status = 'resolved', resolved_at = now(), resolved_by = $2 \
         where conflict_id = $1 and status = 'open' returning original_path",
    )
    .bind(request.conflict_id.as_str())
    .bind(principal.adapter_id())
    .fetch_optional(&mut *transaction)
    .await
    .map_err(|_| ApiError::internal())?
    .ok_or_else(ConflictsRouteError::not_found)?;
    let path = VaultPath::parse(row.try_get::<String, _>("original_path")?.as_str()).map_err(|_| ApiError::internal())?;
    let operation = append_operation_log(
        &mut transaction,
        principal.common_adapter_id().clone(),
        OperationKindName::ConflictResolved,
        path,
        None,
        None,
        Some(request.conflict_id.clone()),
        &[request.conflict_id.as_str(), principal.adapter_id()],
    )
    .await?;
    transaction.commit().await.map_err(|_| ApiError::internal())?;
    Ok((StatusCode::OK, Json(resolved_conflict_response(request.conflict_id, request.resolution, operation.seq))).into_response())
}

async fn admin_status_route(
    Extension(state): Extension<ServerAppState>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    authenticate(&state, &headers, RoutePermission::Admin).await?;
    let last_operation_sequence = if let Some(pool) = state.db_pool() {
        sqlx::query_scalar::<_, Option<i64>>("select max(seq) from operation_log").fetch_one(pool).await.map_err(|_| ApiError::internal())?
    } else {
        None
    };
    let adapter_count = if let Some(pool) = state.db_pool() {
        Some(sqlx::query_scalar::<_, i64>("select count(*) from sync_adapters").fetch_one(pool).await.map_err(|_| ApiError::internal())? as u64)
    } else {
        None
    };
    let response = StatusSummaryResponse::from_safe_parts(
        if state.db_pool().is_some() && state.object_store().is_some() { ServerStatus::Ready } else { ServerStatus::NotReady },
        readiness_for(state.db_pool().is_some()),
        readiness_for(state.object_store().is_some()),
        last_operation_sequence,
        adapter_count,
        PauseStatusSummary::unsupported(),
    );
    Ok((StatusCode::OK, Json(response)).into_response())
}

async fn admin_adapters_route(
    Extension(state): Extension<ServerAppState>,
    Query(_query): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    authenticate(&state, &headers, RoutePermission::Admin).await?;
    let Some(pool) = state.db_pool() else {
        return Ok((StatusCode::OK, Json(AdapterListResponse::empty())).into_response());
    };
    let rows = sqlx::query("select adapter_id, role, enabled, last_seen_at from sync_adapters order by adapter_id asc limit 100")
        .fetch_all(pool)
        .await
        .map_err(|_| ApiError::internal())?;
    let mut adapters = Vec::with_capacity(rows.len());
    for row in rows {
        let adapter_id = AdapterId::parse(row.try_get::<String, _>("adapter_id")?.as_str()).map_err(|_| ApiError::internal())?;
        let role = CommonAdapterRole::from_str(row.try_get::<String, _>("role")?.as_str()).ok();
        let enabled = row.try_get::<bool, _>("enabled")?;
        let mut summary = AdapterSummary::new(adapter_id, role, enabled).with_mode(AdapterMode::Disabled);
        if let Some(last_seen_at) = row.try_get::<Option<chrono::DateTime<Utc>>, _>("last_seen_at")? {
            summary = summary.with_last_seen_at(TimestampDto::from(timestamp_string(last_seen_at)));
        }
        adapters.push(summary.with_cursor(AdapterCursorSummary::new(None, None, false)));
    }
    Ok((StatusCode::OK, Json(AdapterListResponse::new(adapters))).into_response())
}

pub async fn core_route_not_implemented() -> (StatusCode, Json<ShellErrorResponse>) {
    not_implemented_response()
}

#[derive(Clone, Copy)]
enum RoutePermission {
    Read,
    Write,
    ResolveConflict,
    Admin,
}

async fn authenticate(state: &ServerAppState, headers: &HeaderMap, permission: RoutePermission) -> Result<AdapterPrincipal, ApiError> {
    let header = required_auth_header(headers)?;
    let token = BearerToken::parse_authorization_header(header).map_err(|_| ApiError::invalid_token())?;
    let principal = match state.auth() {
        AuthState::Disabled => return Err(ApiError::invalid_token()),
        AuthState::StaticPrincipal { principal } => principal.clone(),
        AuthState::Database { pool } => lookup_principal_by_token(pool, &token).await?,
    };
    match permission {
        RoutePermission::Read if !principal.role().can_read_files() => return Err(ApiError::forbidden_role()),
        RoutePermission::Write if !principal.role().can_write_files() => return Err(ApiError::forbidden_role()),
        RoutePermission::ResolveConflict if !principal.role().can_resolve_conflicts() => return Err(ApiError::forbidden_role()),
        RoutePermission::Admin if !principal.role().can_admin() => return Err(ApiError::forbidden_role()),
        _ => {}
    }
    Ok(principal)
}

async fn lookup_principal_by_token(pool: &PgPool, token: &BearerToken) -> Result<AdapterPrincipal, ApiError> {
    let hash = token.sha256_hash();
    let prefixed_hash = format!("sha256:{}", hash.digest_hex());
    let row = sqlx::query("select adapter_id, role from sync_adapters where enabled = true and (token_hash = $1 or token_hash = $2) limit 1")
        .bind(hash.digest_hex())
        .bind(prefixed_hash)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::internal())?
        .ok_or_else(ApiError::invalid_token)?;
    let adapter_id: String = row.try_get("adapter_id").map_err(|_| ApiError::internal())?;
    let role: String = row.try_get("role").map_err(|_| ApiError::internal())?;
    let role = AdapterRole::from_str(&role).map_err(|_| ApiError::invalid_token())?;
    AdapterPrincipal::new(adapter_id, role).map_err(|_| ApiError::invalid_token())
}

fn required_auth_header(headers: &HeaderMap) -> Result<&str, ApiError> {
    let value = headers.get(AUTHORIZATION_HEADER).ok_or_else(ApiError::missing_token)?;
    value.to_str().map_err(|_| ApiError::invalid_token())
}

#[derive(Clone, Copy)]
enum HeaderErrorKind {
    Idempotency,
    ContentSha256,
    BaseRevision,
}

fn optional_header<'a>(headers: &'a HeaderMap, name: &'static str, kind: HeaderErrorKind) -> Result<Option<&'a str>, ApiError> {
    headers
        .get(name)
        .map(|value| {
            value.to_str().map_err(|_| match kind {
                HeaderErrorKind::Idempotency => FileRouteError::InvalidIdempotencyKey.into(),
                HeaderErrorKind::ContentSha256 => FileRouteError::InvalidContentSha256.into(),
                HeaderErrorKind::BaseRevision => FileRouteError::InvalidBaseRevision.into(),
            })
        })
        .transpose()
}

fn runtime_pool(state: &ServerAppState) -> Result<&PgPool, ApiError> {
    state.db_pool().ok_or_else(ApiError::storage_unavailable)
}

fn runtime_object_store(state: &ServerAppState) -> Result<&LocalObjectStore, ApiError> {
    state.object_store().ok_or_else(ApiError::storage_unavailable)
}

fn put_request_fingerprint(request: &PutFileRouteRequest, principal: &AdapterPrincipal) -> RequestFingerprint {
    RequestFingerprint::from_safe_json_metadata(&json!({
        "method": "PUT",
        "path": request.path().as_str(),
        "base_revision_id": request.base_revision_id().map(RevisionId::as_str).unwrap_or("null"),
        "content_sha256": request.content_sha256().to_string(),
        "adapter_id": principal.adapter_id(),
        "scope": "adapter",
    }))
}

fn delete_request_fingerprint(request: &DeleteFileRouteRequest, principal: &AdapterPrincipal) -> RequestFingerprint {
    RequestFingerprint::from_safe_json_metadata(&json!({
        "method": "DELETE",
        "path": request.path().as_str(),
        "base_revision_id": request.base_revision_id().map(RevisionId::as_str).unwrap_or("null"),
        "action": "tombstone",
        "requested_delete_count": request.delete_guard().requested_delete_count,
        "adapter_id": principal.adapter_id(),
        "scope": "adapter",
    }))
}

async fn read_existing_idempotency(pool: &PgPool, principal: &AdapterPrincipal, idempotency_key: &str, fingerprint: RequestFingerprint) -> Result<Option<StoredIdempotencyResponse>, ApiError> {
    let mut connection = pool.acquire().await.map_err(|_| ApiError::internal())?;
    let Some(record) = read_idempotency_record(&mut connection, principal.common_adapter_id(), idempotency_key).await.map_err(|_| ApiError::internal())? else {
        return Ok(None);
    };
    match compare_request_fingerprint(&record, fingerprint.as_sha256()).map_err(|_| ApiError::internal())? {
        IdempotencyRequestComparison::SameRequest => Ok(Some(serde_json::from_value(record.response_json).map_err(|_| ApiError::internal())?)),
        IdempotencyRequestComparison::DifferentRequest => Err(FileRouteError::IdempotencyMismatch.into()),
    }
}

async fn store_successful_idempotency(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    principal: &AdapterPrincipal,
    idempotency_key: &str,
    fingerprint: RequestFingerprint,
    status_code: StatusCode,
    response_body: Value,
) -> Result<Option<StoredIdempotencyResponse>, ApiError> {
    let stored_response = StoredIdempotencyResponse::json(status_code.as_u16(), response_body).map_err(|_| ApiError::internal())?;
    let input = IdempotencyRecordInput::new(
        principal.common_adapter_id().clone(),
        idempotency_key,
        *fingerprint.as_sha256(),
        serde_json::to_value(stored_response).map_err(|_| ApiError::internal())?,
    )
    .map_err(|_| ApiError::internal())?;
    match insert_idempotency_record(transaction, &input).await.map_err(|_| ApiError::internal())? {
        IdempotencyStoreOutcome::Stored { .. } => Ok(None),
        IdempotencyStoreOutcome::AlreadyExists { record } => match compare_request_fingerprint(&record, fingerprint.as_sha256()).map_err(|_| ApiError::internal())? {
            IdempotencyRequestComparison::SameRequest => Ok(Some(serde_json::from_value(record.response_json).map_err(|_| ApiError::internal())?)),
            IdempotencyRequestComparison::DifferentRequest => Err(FileRouteError::IdempotencyMismatch.into()),
        },
    }
}

fn run_core_normal_upsert(
    request: &PutFileRouteRequest,
    principal: &AdapterPrincipal,
    current_revision: Option<StoredRevision>,
    object_store: &LocalObjectStore,
) -> Result<UpsertOutcome, ApiError> {
    let mut service = RevisionService::new(
        PlanningRevisionRepository { current_revision },
        PlanningContentStore { object_store },
        PlanningOperationLog,
    );
    let mut upsert_request = UpsertFileRequest::new(
        request.path().clone(),
        principal.common_adapter_id().clone(),
        request.base_revision_id().cloned(),
        request.content_sha256(),
        request.body().to_vec(),
    );
    upsert_request = upsert_request.with_idempotency_key(request.idempotency_key().as_str());
    service.upsert_file(upsert_request).map_err(map_revision_service_error)
}

async fn apply_upsert_outcome(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    principal: &AdapterPrincipal,
    outcome: UpsertOutcome,
    object_store: &LocalObjectStore,
) -> Result<PutFileResponse, ApiError> {
    match outcome {
        UpsertOutcome::AcceptedNewFile { revision, .. } | UpsertOutcome::AcceptedNewRevision { revision, .. } => {
            let seq = persist_accepted_revision(transaction, principal, &revision).await?;
            Ok(accepted_upload_response(revision.path, revision.revision_id, seq))
        }
        UpsertOutcome::IgnoredDuplicateSameContent { current_revision } => Ok(ignored_same_content_response(current_revision.path)),
        UpsertOutcome::RejectedHashMismatch { .. } => Err(FileRouteError::InvalidContentSha256.into()),
        UpsertOutcome::RejectedStaleOrUnknownBase { conflict_saved: Some(conflict_saved), .. } => {
            persist_conflict_saved(transaction, principal, &conflict_saved, object_store).await
        }
        UpsertOutcome::RejectedStaleOrUnknownBase { .. } => Err(FileRouteError::Conflict.into()),
    }
}

async fn persist_conflict_saved(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    principal: &AdapterPrincipal,
    outcome: &haze_sync_core::revision_service::ConflictSavedOutcome,
    object_store: &LocalObjectStore,
) -> Result<PutFileResponse, ApiError> {
    let plan = plan_conflict_saved_fan_in(outcome, Utc::now()).map_err(|_| FileRouteError::Conflict)?;
    object_store.put_bytes(outcome.incoming_content.content_hash, &outcome.incoming_content.content).map_err(|_| ApiError::internal())?;
    insert_content_blob(transaction, outcome.incoming_content.content_hash, outcome.incoming_content.size_bytes).await?;
    let conflict_revision = persist_conflict_copy_revision(transaction, principal, &plan).await?;
    let conflict_id = ConflictId::parse(&deterministic_identifier(
        "conf_",
        &[plan.original_path.as_str(), plan.conflict_path.as_str(), conflict_revision.revision_id.as_str()],
    ))
    .map_err(|_| ApiError::internal())?;
    insert_conflict_row(transaction, &plan, &conflict_id, &conflict_revision.revision_id).await?;
    let operation = append_operation_log(
        transaction,
        principal.common_adapter_id().clone(),
        OperationKindName::ConflictCreated,
        plan.original_path.clone(),
        None,
        None,
        Some(conflict_id.clone()),
        &[conflict_id.as_str(), plan.original_path.as_str()],
    )
    .await?;
    Ok(PutFileResponse::ConflictSaved {
        path: VaultPathDto::from(plan.original_path),
        conflict_id: ConflictIdDto::from(conflict_id),
        materialized_path: VaultPathDto::from(plan.conflict_path),
        policy_applied: ConflictPolicyDto::PreserveBoth,
        seq: operation.seq,
    })
}

async fn persist_conflict_copy_revision(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    principal: &AdapterPrincipal,
    plan: &ConflictSavedFanInPlan,
) -> Result<StoredRevision, ApiError> {
    let object_id = deterministic_identifier("obj_", &[plan.conflict_path.as_str()]);
    let object = create_or_find_sync_object_by_path(
        &mut **transaction,
        NewSyncObject { object_id: object_id.as_str(), path: &plan.conflict_path, kind: SyncObjectKind::File, updated_by: principal.common_adapter_id() },
    )
    .await
    .map_err(map_repository_error)?;
    let content_hash_string = plan.incoming_backup.content_hash.to_string();
    let revision_id = RevisionId::parse(&deterministic_identifier(
        "rev_",
        &[plan.conflict_path.as_str(), plan.conflict_record.current_revision_id.as_str(), content_hash_string.as_str(), principal.adapter_id()],
    ))
    .map_err(|_| ApiError::internal())?;
    insert_file_revision(
        &mut **transaction,
        NewFileRevision {
            revision_id: &revision_id,
            object_id: object.object_id.as_str(),
            path: &plan.conflict_path,
            parent_revision_id: None,
            content_hash: plan.incoming_backup.content_hash,
            size_bytes: plan.incoming_backup.size_bytes,
            created_by: principal.common_adapter_id(),
        },
    )
    .await
    .map_err(map_repository_error)?;
    set_current_revision_by_object_id(&mut **transaction, object.object_id.as_str(), Some(&revision_id), principal.common_adapter_id())
        .await
        .map_err(map_repository_error)?
        .ok_or_else(ApiError::internal)?;
    Ok(StoredRevision {
        revision_id,
        path: plan.conflict_path.clone(),
        parent_revision_id: None,
        content_hash: plan.incoming_backup.content_hash,
        size_bytes: plan.incoming_backup.size_bytes,
        created_by: principal.common_adapter_id().clone(),
    })
}

async fn insert_conflict_row(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    plan: &ConflictSavedFanInPlan,
    conflict_id: &ConflictId,
    incoming_revision_id: &RevisionId,
) -> Result<(), ApiError> {
    sqlx::query(
        "insert into conflicts (conflict_id, original_path, base_revision_id, current_revision_id, incoming_revision_id, incoming_adapter_id, policy_applied, materialized_path, status) values ($1, $2, $3, $4, $5, $6, $7, $8, 'open')",
    )
    .bind(conflict_id.as_str())
    .bind(plan.original_path.as_str())
    .bind(plan.conflict_record.base_revision_id.as_ref().map(RevisionId::as_str))
    .bind(plan.conflict_record.current_revision_id.as_str())
    .bind(incoming_revision_id.as_str())
    .bind(plan.conflict_record.adapter_id.as_str())
    .bind(plan.conflict_record.policy_applied.as_str())
    .bind(plan.conflict_path.as_str())
    .execute(&mut **transaction)
    .await
    .map_err(|_| ApiError::internal())?;
    Ok(())
}

struct PlanningRevisionRepository {
    current_revision: Option<StoredRevision>,
}

impl RevisionRepository for PlanningRevisionRepository {
    fn current_revision(&mut self, _path: &VaultPath) -> Result<Option<StoredRevision>, RevisionServiceError> {
        Ok(self.current_revision.clone())
    }

    fn insert_revision(&mut self, request: InsertRevisionRequest) -> Result<StoredRevision, RevisionServiceError> {
        let content_hash_string = request.content_hash.to_string();
        let revision_id = RevisionId::parse(&deterministic_identifier(
            "rev_",
            &[
                request.path.as_str(),
                request.parent_revision_id.as_ref().map(RevisionId::as_str).unwrap_or("null"),
                content_hash_string.as_str(),
                request.adapter_id.as_str(),
            ],
        ))
        .map_err(|_| RevisionServiceError::repository("build_revision_id"))?;
        Ok(StoredRevision { revision_id, path: request.path, parent_revision_id: request.parent_revision_id, content_hash: request.content_hash, size_bytes: request.size_bytes, created_by: request.adapter_id })
    }
}

struct PlanningContentStore<'a> {
    object_store: &'a LocalObjectStore,
}

impl ContentStore for PlanningContentStore<'_> {
    fn put_content(&mut self, expected_hash: ContentHash, bytes: &[u8]) -> Result<StoredContent, RevisionServiceError> {
        self.object_store.put_bytes(expected_hash, bytes).map_err(|_| RevisionServiceError::content_store("put_content"))?;
        Ok(StoredContent { hash: expected_hash, size_bytes: bytes.len().try_into().map_err(|_| RevisionServiceError::content_store("content_size"))? })
    }
}

struct PlanningOperationLog;

impl OperationLog for PlanningOperationLog {
    fn append_operation(&mut self, request: AppendOperationRequest) -> Result<OperationLogEntry, RevisionServiceError> {
        let kind = match request.kind {
            OperationKind::UpsertFile => OperationKindName::UpsertFile.as_str(),
        };
        let op_id = OperationId::parse(&deterministic_identifier("op_", &[request.revision_id.as_str(), kind, request.path.as_str()]))
            .map_err(|_| RevisionServiceError::operation_log("build_operation_id"))?;
        Ok(OperationLogEntry { operation_id: op_id, seq: 0 })
    }
}

async fn persist_accepted_revision(transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>, principal: &AdapterPrincipal, revision: &StoredRevision) -> Result<i64, ApiError> {
    insert_content_blob(transaction, revision.content_hash, revision.size_bytes).await?;
    let object_id = deterministic_identifier("obj_", &[revision.path.as_str()]);
    let object = create_or_find_sync_object_by_path(
        &mut **transaction,
        NewSyncObject { object_id: object_id.as_str(), path: &revision.path, kind: SyncObjectKind::File, updated_by: principal.common_adapter_id() },
    )
    .await
    .map_err(map_repository_error)?;
    insert_file_revision(
        &mut **transaction,
        NewFileRevision {
            revision_id: &revision.revision_id,
            object_id: object.object_id.as_str(),
            path: &revision.path,
            parent_revision_id: revision.parent_revision_id.as_ref(),
            content_hash: revision.content_hash,
            size_bytes: revision.size_bytes,
            created_by: principal.common_adapter_id(),
        },
    )
    .await
    .map_err(map_repository_error)?;
    set_current_revision_by_object_id(&mut **transaction, object.object_id.as_str(), Some(&revision.revision_id), principal.common_adapter_id())
        .await
        .map_err(map_repository_error)?
        .ok_or_else(ApiError::internal)?;
    let operation = append_operation_log(
        transaction,
        principal.common_adapter_id().clone(),
        OperationKindName::UpsertFile,
        revision.path.clone(),
        Some(revision.revision_id.clone()),
        None,
        None,
        &[revision.revision_id.as_str(), revision.path.as_str()],
    )
    .await?;
    Ok(operation.seq)
}

async fn insert_content_blob(transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>, content_hash: ContentHash, size_bytes: u64) -> Result<(), ApiError> {
    let size_bytes = i64::try_from(size_bytes).map_err(|_| ApiError::internal())?;
    let object_store_path = LocalObjectStore::relative_blob_path(content_hash).to_string_lossy().replace('\\', "/");
    sqlx::query("insert into content_blobs (sha256, size_bytes, object_store_path) values ($1, $2, $3) on conflict (sha256) do nothing")
        .bind(content_hash.to_prefixed_string())
        .bind(size_bytes)
        .bind(object_store_path)
        .execute(&mut **transaction)
        .await
        .map_err(|_| ApiError::internal())?;
    Ok(())
}

async fn append_operation_log(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    adapter_id: AdapterId,
    kind: OperationKindName,
    path: VaultPath,
    revision_id: Option<RevisionId>,
    tombstone_id: Option<String>,
    conflict_id: Option<ConflictId>,
    id_parts: &[&str],
) -> Result<OperationLogRow, ApiError> {
    let mut parts = vec![kind.as_str(), path.as_str()];
    parts.extend_from_slice(id_parts);
    let op_id = OperationId::parse(&deterministic_identifier("op_", &parts)).map_err(|_| ApiError::internal())?;
    OperationLogRepository::new()
        .append(&mut **transaction, &AppendOperationLogEntry { op_id, adapter_id, kind, path, revision_id, tombstone_id, conflict_id })
        .await
        .map_err(map_repository_error)
}

fn stored_revision_from_row(row: FileRevisionRow) -> Result<StoredRevision, ApiError> {
    Ok(StoredRevision {
        revision_id: RevisionId::parse(row.revision_id.as_str()).map_err(|_| ApiError::internal())?,
        path: VaultPath::parse(row.path.as_str()).map_err(|_| ApiError::internal())?,
        parent_revision_id: row.parent_revision_id.as_deref().map(RevisionId::parse).transpose().map_err(|_| ApiError::internal())?,
        content_hash: ContentHash::parse(row.content_sha256.as_str()).map_err(|_| ApiError::internal())?,
        size_bytes: row.size_bytes.try_into().map_err(|_| ApiError::internal())?,
        created_by: AdapterId::parse(row.created_by.as_str()).map_err(|_| ApiError::internal())?,
    })
}

fn raw_file_response(bytes: Vec<u8>, headers: FileDownloadRouteHeaders) -> Result<Response, ApiError> {
    let mut response = Response::new(Body::from(bytes));
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static(APPLICATION_OCTET_STREAM));
    insert_header(response.headers_mut(), X_REVISION_ID_HEADER, headers.revision_id().as_str())?;
    insert_header(response.headers_mut(), X_CONTENT_SHA256_HEADER, &headers.content_sha256().to_string())?;
    insert_header(response.headers_mut(), X_SIZE_BYTES_HEADER, &headers.size_bytes().to_string())?;
    Ok(response)
}

fn replay_response(stored: StoredIdempotencyResponse) -> Result<Response, ApiError> {
    let status = StatusCode::from_u16(stored.status_code()).map_err(|_| ApiError::internal())?;
    let mut response = (status, Json(stored.body().clone())).into_response();
    for (name, value) in stored.headers() {
        response.headers_mut().insert(
            HeaderName::from_bytes(name.as_bytes()).map_err(|_| ApiError::internal())?,
            HeaderValue::from_str(value).map_err(|_| ApiError::internal())?,
        );
    }
    Ok(response)
}

fn insert_header(headers: &mut HeaderMap, name: &'static str, value: &str) -> Result<(), ApiError> {
    headers.insert(
        HeaderName::from_bytes(name.as_bytes()).map_err(|_| ApiError::internal())?,
        HeaderValue::from_str(value).map_err(|_| ApiError::internal())?,
    );
    Ok(())
}

fn change_entry_from_row(row: ChangeFeedRow) -> Result<ChangeEntryDto, ApiError> {
    let kind = OperationKindName::from_str(row.kind.as_str()).map_err(map_repository_error)?;
    Ok(ChangeEntryDto {
        seq: row.seq,
        kind: operation_kind_dto(kind),
        path: VaultPathDto::from(row.path),
        revision_id: row.revision_id.map(RevisionIdDto::from),
        content_sha256: row.content_sha256.map(ContentSha256Dto::from),
        size_bytes: row.size_bytes.map(u64::try_from).transpose().map_err(|_| ApiError::internal())?,
        tombstone_id: row.tombstone_id.map(TombstoneIdDto::from),
        conflict_id: row.conflict_id.map(ConflictIdDto::from),
        updated_by: AdapterIdDto::from(row.adapter_id),
        updated_at: TimestampDto::from(timestamp_string(row.created_at)),
    })
}

const fn operation_kind_dto(kind: OperationKindName) -> OperationKindDto {
    match kind {
        OperationKindName::UpsertFile => OperationKindDto::UpsertFile,
        OperationKindName::DeleteFile => OperationKindDto::DeleteFile,
        OperationKindName::RestoreFile => OperationKindDto::RestoreFile,
        OperationKindName::ConflictCreated => OperationKindDto::ConflictCreated,
        OperationKindName::ConflictResolved => OperationKindDto::ConflictResolved,
        OperationKindName::BackupCreated => OperationKindDto::BackupCreated,
    }
}

fn conflict_summary_from_row(row: &sqlx::postgres::PgRow) -> Result<ConflictRouteSummaryParts, ApiError> {
    let status = match row.try_get::<String, _>("status")?.as_str() {
        "open" => ConflictStatusDto::Open,
        "resolved" => ConflictStatusDto::Resolved,
        "ignored" => ConflictStatusDto::Ignored,
        _ => return Err(ApiError::internal()),
    };
    let policy_applied = match row.try_get::<String, _>("policy_applied")?.as_str() {
        "preserve_both" => ConflictPolicyDto::PreserveBoth,
        "current_wins_with_incoming_backup" => ConflictPolicyDto::CurrentWinsWithIncomingBackup,
        _ => return Err(ApiError::internal()),
    };
    let created_at = row.try_get::<chrono::DateTime<Utc>, _>("created_at")?;
    let resolved_at = row.try_get::<Option<chrono::DateTime<Utc>>, _>("resolved_at")?;
    Ok(ConflictRouteSummaryParts {
        conflict_id: ConflictId::parse(row.try_get::<String, _>("conflict_id")?.as_str()).map_err(|_| ApiError::internal())?,
        original_path: VaultPath::parse(row.try_get::<String, _>("original_path")?.as_str()).map_err(|_| ApiError::internal())?,
        conflict_path: VaultPath::parse(row.try_get::<String, _>("materialized_path")?.as_str()).map_err(|_| ApiError::internal())?,
        current_revision_id: RevisionId::parse(row.try_get::<String, _>("current_revision_id")?.as_str()).map_err(|_| ApiError::internal())?,
        conflict_revision_id: Some(RevisionId::parse(row.try_get::<String, _>("incoming_revision_id")?.as_str()).map_err(|_| ApiError::internal())?),
        incoming_revision_id: None,
        source_adapter_id: AdapterId::parse(row.try_get::<String, _>("incoming_adapter_id")?.as_str()).map_err(|_| ApiError::internal())?,
        policy_applied,
        status,
        created_at: Some(TimestampDto::from(timestamp_string(created_at))),
        updated_at: resolved_at.map(timestamp_string).map(TimestampDto::from),
    })
}

fn single_delete_guard_allows(principal: &AdapterPrincipal) -> Result<bool, ApiError> {
    let scope = DeleteRunScope::new(principal.common_adapter_id().clone(), "single-delete").map_err(|_| ApiError::internal())?;
    let guard = DeleteGuard::new(DeleteGuardPolicy::new(
        20,
        DeleteRatioLimit::percent(100).map_err(|_| ApiError::internal())?,
        false,
    ));
    Ok(matches!(guard.evaluate(&DeleteGuardInput::without_manual_unlock(scope, 1, 1)), DeleteGuardDecision::Allowed))
}

fn readiness_for(configured: bool) -> DependencyReadinessState {
    if configured { DependencyReadinessState::Ready } else { DependencyReadinessState::Unknown }
}

fn timestamp_string(value: chrono::DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn map_object_store_get_error(error: ObjectStoreError) -> ApiError {
    match error {
        ObjectStoreError::MissingBlob { .. } => FileRouteError::NotFound.into(),
        _ => ApiError::internal(),
    }
}

fn map_repository_error(_error: RepositoryError) -> ApiError { ApiError::internal() }
fn map_revision_service_error(_error: RevisionServiceError) -> ApiError { ApiError::internal() }

fn deterministic_identifier(prefix: &str, parts: &[&str]) -> String {
    let mut hasher = Sha256Digest::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update([0]);
    }
    let digest = hasher.finalize();
    format!("{prefix}{}", lower_hex(&digest[..16]))
}

fn lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    body: ErrorResponse,
}

impl ApiError {
    fn new(status: StatusCode, code: PublicErrorCode, message: &'static str) -> Self {
        Self { status, body: ErrorResponse::new(code, message) }
    }
    fn missing_token() -> Self { Self::new(StatusCode::UNAUTHORIZED, PublicErrorCode::MissingToken, "Missing bearer token") }
    fn invalid_token() -> Self { Self::new(StatusCode::UNAUTHORIZED, PublicErrorCode::InvalidToken, "Invalid bearer token") }
    fn forbidden_role() -> Self { Self::new(StatusCode::FORBIDDEN, PublicErrorCode::ForbiddenRole, "Authenticated adapter role is not allowed for this operation") }
    fn storage_unavailable() -> Self { Self::new(StatusCode::SERVICE_UNAVAILABLE, PublicErrorCode::InternalError, "Core storage dependencies are not configured") }
    fn internal() -> Self { Self::new(StatusCode::INTERNAL_SERVER_ERROR, PublicErrorCode::InternalError, "Core file operation failed") }
}

impl From<FileRouteError> for ApiError {
    fn from(error: FileRouteError) -> Self {
        Self { status: StatusCode::from_u16(error.http_status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR), body: error.to_error_response() }
    }
}
impl From<DeleteRouteError> for ApiError {
    fn from(error: DeleteRouteError) -> Self {
        Self { status: StatusCode::from_u16(error.http_status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR), body: error.to_error_response() }
    }
}
impl From<ChangesRouteError> for ApiError {
    fn from(error: ChangesRouteError) -> Self {
        Self { status: StatusCode::from_u16(error.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR), body: error.error_response() }
    }
}
impl From<ConflictsRouteError> for ApiError {
    fn from(error: ConflictsRouteError) -> Self {
        Self { status: StatusCode::from_u16(error.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR), body: error.error_response() }
    }
}
impl From<sqlx::Error> for ApiError { fn from(_: sqlx::Error) -> Self { Self::internal() } }
impl IntoResponse for ApiError { fn into_response(self) -> Response { (self.status, Json(self.body)).into_response() } }
