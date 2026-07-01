//! V1 Core API route wiring for W2 normal file operations.
//!
//! This module wires already-merged W2 API helpers, storage repositories,
//! idempotency primitives, object store primitives, and readiness/auth state into
//! the normal PUT/GET/changes slice. Conflict preservation, delete behavior, and
//! adapter runtimes remain future-phase work.

use axum::{
    body::{Body, Bytes},
    extract::{Path, Query},
    http::{header, HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Extension, Json, Router,
};
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
        common::OperationKindDto,
        files::PutFileResponse,
        primitives::{
            AdapterIdDto, ConflictIdDto, ContentSha256Dto, RevisionIdDto, TimestampDto, TombstoneIdDto,
            VaultPathDto,
        },
        server::{ServerCapabilityDto, ServerInfoResponse},
    },
    routes::{
        changes::{changes_response_from_parts, parse_changes_query, ChangesRouteError},
        files::{
            accepted_upload_response, ignored_same_content_response, parse_get_file_request,
            parse_put_file_request, FileDownloadRouteHeaders, FileRouteError,
            GetFileRouteRequestParts, PutFileRouteRequest, PutFileRouteRequestParts,
            APPLICATION_OCTET_STREAM, X_REVISION_ID_HEADER, X_SIZE_BYTES_HEADER,
        },
    },
};
use haze_sync_common::{ContentHash, OperationId, RevisionId, VaultPath};
use haze_sync_core::{
    idempotency::{RequestFingerprint, StoredIdempotencyResponse},
    revision_service::compute_content_hash,
};
use haze_sync_storage::{
    locks::lock_vault_path,
    models::FileRevisionRow,
    object_store::ObjectStore,
    repositories::{
        idempotency::{
            compare_request_fingerprint, insert_idempotency_record, read_idempotency_record,
            IdempotencyRecordInput, IdempotencyRequestComparison, IdempotencyStoreOutcome,
        },
        objects::{
            create_or_find_sync_object_by_path, set_current_revision_by_object_id, NewSyncObject,
            SyncObjectKind,
        },
        operation_log::{
            AppendOperationLogEntry, ChangeFeedRow, OperationKindName, OperationLogRepository,
        },
        revisions::{get_current_revision_by_path, get_file_revision_by_id, insert_file_revision, NewFileRevision},
        RepositoryError,
    },
    LocalObjectStore, ObjectStoreError,
};
use serde_json::json;
use sha2::{Digest, Sha256 as Sha256Digest};
use sqlx::{PgPool, Row};
use std::{collections::HashMap, str::FromStr};

use crate::{
    http::errors::{not_implemented_response, ShellErrorResponse},
    state::{AuthState, ServerAppState},
};

const MAX_UPLOAD_BYTES: u64 = 52_428_800;

/// Builds the versioned `/v1` API namespace.
pub fn router() -> Router {
    Router::new()
        .route("/server-info", get(server_info))
        .route("/changes", get(changes_route))
        .route(
            "/files/*path",
            get(get_file_route)
                .put(put_file_route)
                .delete(core_route_not_implemented),
        )
        .route("/conflicts", get(core_route_not_implemented))
        .route(
            "/conflicts/{conflict_id}/resolve",
            axum::routing::post(core_route_not_implemented),
        )
}

/// Handles GET /v1/server-info with static, safe route-shell information.
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
    let principal = authenticate(&state, &headers, FilePermission::Write).await?;
    let request = parse_put_file_request(PutFileRouteRequestParts {
        route_path: route_path.as_str(),
        idempotency_key: optional_header(&headers, IDEMPOTENCY_KEY_HEADER, HeaderErrorKind::Idempotency)?,
        content_sha256: optional_header(&headers, X_CONTENT_SHA256_HEADER, HeaderErrorKind::ContentSha256)?,
        base_revision_id: optional_header(&headers, X_BASE_REVISION_ID_HEADER, HeaderErrorKind::BaseRevision)?,
        body: body.to_vec(),
        max_upload_bytes: Some(MAX_UPLOAD_BYTES),
    })?;

    let actual_hash = compute_content_hash(request.body());
    if actual_hash != request.content_sha256() {
        return Err(FileRouteError::InvalidContentSha256.into());
    }

    let pool = runtime_pool(&state)?;
    let object_store = runtime_object_store(&state)?;
    let fingerprint = request_fingerprint(&request, &principal);

    if let Some(replay) =
        read_existing_idempotency(pool, &principal, request.idempotency_key().as_str(), fingerprint)
            .await?
    {
        return replay_response(replay);
    }

    object_store
        .put_bytes(request.content_sha256(), request.body())
        .map_err(map_object_store_put_error)?;

    let mut transaction = pool.begin().await.map_err(|_error| ApiError::internal())?;
    lock_vault_path(&mut *transaction, request.path())
        .await
        .map_err(map_repository_error)?;

    let current_revision = get_current_revision_by_path(&mut *transaction, request.path())
        .await
        .map_err(map_repository_error)?
        .map(stored_revision_from_row)
        .transpose()?;

    let response_body = match current_revision {
        None => {
            if request.base_revision_id().is_some() {
                return Err(FileRouteError::Conflict.into());
            }
            accept_new_revision(
                &mut transaction,
                &principal,
                request.path(),
                None,
                request.content_sha256(),
                request.size_bytes(),
            )
            .await?
        }
        Some(current) if current.content_hash == request.content_sha256() => {
            ignored_same_content_response(request.path().clone())
        }
        Some(current) => {
            if request.base_revision_id() != Some(&current.revision_id) {
                return Err(FileRouteError::Conflict.into());
            }
            accept_new_revision(
                &mut transaction,
                &principal,
                request.path(),
                Some(&current.revision_id),
                request.content_sha256(),
                request.size_bytes(),
            )
            .await?
        }
    };

    if let Some(replay) = store_successful_idempotency(
        &mut transaction,
        &principal,
        request.idempotency_key().as_str(),
        fingerprint,
        &response_body,
    )
    .await?
    {
        transaction
            .rollback()
            .await
            .map_err(|_error| ApiError::internal())?;
        return replay_response(replay);
    }

    transaction
        .commit()
        .await
        .map_err(|_error| ApiError::internal())?;

    Ok((StatusCode::OK, Json(response_body)).into_response())
}

async fn get_file_route(
    Extension(state): Extension<ServerAppState>,
    Path(route_path): Path<String>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    authenticate(&state, &headers, FilePermission::Read).await?;
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
    let bytes = object_store
        .get_bytes(revision.content_hash)
        .map_err(map_object_store_get_error)?;
    if u64::try_from(bytes.len()).map_err(|_| ApiError::internal())? != revision.size_bytes {
        return Err(ApiError::internal());
    }

    raw_file_response(
        bytes,
        FileDownloadRouteHeaders::new(
            revision.revision_id,
            revision.content_hash,
            revision.size_bytes,
        ),
    )
}

async fn changes_route(
    Extension(state): Extension<ServerAppState>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    authenticate(&state, &headers, FilePermission::Read).await?;
    let request = parse_changes_query(
        query.get("since").map(String::as_str),
        query.get("limit").map(String::as_str),
    )?;
    let pool = runtime_pool(&state)?;
    let page = OperationLogRepository::new()
        .changes_since(pool, request.since_value(), request.limit_value())
        .await
        .map_err(map_repository_error)?;

    let changes = page
        .changes
        .into_iter()
        .map(change_entry_from_row)
        .collect::<Result<Vec<_>, _>>()?;
    let response = changes_response_from_parts(page.from_seq, page.to_seq, page.has_more, changes)?;

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// Placeholder for future Core routes whose business behavior is outside W2-F1.
pub async fn core_route_not_implemented() -> (StatusCode, Json<ShellErrorResponse>) {
    not_implemented_response()
}

#[derive(Clone, Copy)]
enum FilePermission {
    Read,
    Write,
}

async fn authenticate(
    state: &ServerAppState,
    headers: &HeaderMap,
    permission: FilePermission,
) -> Result<AdapterPrincipal, ApiError> {
    let header = required_auth_header(headers)?;
    let token =
        BearerToken::parse_authorization_header(header).map_err(|_error| ApiError::invalid_token())?;

    let principal = match state.auth() {
        AuthState::Disabled => return Err(ApiError::invalid_token()),
        AuthState::StaticPrincipal { principal } => principal.clone(),
        AuthState::Database { pool } => lookup_principal_by_token(pool, &token).await?,
    };

    match permission {
        FilePermission::Read if !principal.role().can_read_files() => {
            return Err(ApiError::forbidden_role());
        }
        FilePermission::Write if !principal.role().can_write_files() => {
            return Err(ApiError::forbidden_role());
        }
        _ => {}
    }

    Ok(principal)
}

async fn lookup_principal_by_token(
    pool: &PgPool,
    token: &BearerToken,
) -> Result<AdapterPrincipal, ApiError> {
    let hash = token.sha256_hash();
    let prefixed_hash = format!("sha256:{}", hash.digest_hex());
    let row = sqlx::query(
        "select adapter_id, role from sync_adapters \
         where enabled = true and (token_hash = $1 or token_hash = $2) \
         limit 1",
    )
    .bind(hash.digest_hex())
    .bind(prefixed_hash)
    .fetch_optional(pool)
    .await
    .map_err(|_error| ApiError::internal())?
    .ok_or_else(ApiError::invalid_token)?;

    let adapter_id: String = row.try_get("adapter_id").map_err(|_error| ApiError::internal())?;
    let role: String = row.try_get("role").map_err(|_error| ApiError::internal())?;
    let role = AdapterRole::from_str(&role).map_err(|_error| ApiError::invalid_token())?;

    AdapterPrincipal::new(adapter_id, role).map_err(|_error| ApiError::invalid_token())
}

fn required_auth_header(headers: &HeaderMap) -> Result<&str, ApiError> {
    let value = headers
        .get(AUTHORIZATION_HEADER)
        .ok_or_else(ApiError::missing_token)?;
    value.to_str().map_err(|_error| ApiError::invalid_token())
}

#[derive(Clone, Copy)]
enum HeaderErrorKind {
    Idempotency,
    ContentSha256,
    BaseRevision,
}

fn optional_header<'a>(
    headers: &'a HeaderMap,
    name: &'static str,
    kind: HeaderErrorKind,
) -> Result<Option<&'a str>, ApiError> {
    headers
        .get(name)
        .map(|value| {
            value.to_str().map_err(|_error| match kind {
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
    state
        .object_store()
        .ok_or_else(ApiError::storage_unavailable)
}

fn request_fingerprint(
    request: &PutFileRouteRequest,
    principal: &AdapterPrincipal,
) -> RequestFingerprint {
    let metadata = json!({
        "method": "PUT",
        "path": request.path().as_str(),
        "base_revision_id": request
            .base_revision_id()
            .map(RevisionId::as_str)
            .unwrap_or("null"),
        "content_sha256": request.content_sha256().to_string(),
        "adapter_id": principal.adapter_id(),
        "scope": "adapter",
    });
    RequestFingerprint::from_safe_json_metadata(&metadata)
}

async fn read_existing_idempotency(
    pool: &PgPool,
    principal: &AdapterPrincipal,
    idempotency_key: &str,
    fingerprint: RequestFingerprint,
) -> Result<Option<StoredIdempotencyResponse>, ApiError> {
    let mut connection = pool.acquire().await.map_err(|_error| ApiError::internal())?;
    let Some(record) = read_idempotency_record(
        &mut *connection,
        principal.common_adapter_id(),
        idempotency_key,
    )
    .await
    .map_err(|_error| ApiError::internal())?
    else {
        return Ok(None);
    };

    match compare_request_fingerprint(&record, fingerprint.as_sha256())
        .map_err(|_error| ApiError::internal())?
    {
        IdempotencyRequestComparison::SameRequest => {
            let response = serde_json::from_value(record.response_json)
                .map_err(|_error| ApiError::internal())?;
            Ok(Some(response))
        }
        IdempotencyRequestComparison::DifferentRequest => {
            Err(FileRouteError::IdempotencyMismatch.into())
        }
    }
}

async fn store_successful_idempotency(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    principal: &AdapterPrincipal,
    idempotency_key: &str,
    fingerprint: RequestFingerprint,
    response_body: &PutFileResponse,
) -> Result<Option<StoredIdempotencyResponse>, ApiError> {
    let stored_response = StoredIdempotencyResponse::json(
        StatusCode::OK.as_u16(),
        serde_json::to_value(response_body).map_err(|_error| ApiError::internal())?,
    )
    .map_err(|_error| ApiError::internal())?;
    let input = IdempotencyRecordInput::new(
        principal.common_adapter_id().clone(),
        idempotency_key,
        *fingerprint.as_sha256(),
        serde_json::to_value(stored_response).map_err(|_error| ApiError::internal())?,
    )
    .map_err(|_error| ApiError::internal())?;

    match insert_idempotency_record(&mut **transaction, &input)
        .await
        .map_err(|_error| ApiError::internal())?
    {
        IdempotencyStoreOutcome::Stored { .. } => Ok(None),
        IdempotencyStoreOutcome::AlreadyExists { record } => {
            match compare_request_fingerprint(&record, fingerprint.as_sha256())
                .map_err(|_error| ApiError::internal())?
            {
                IdempotencyRequestComparison::SameRequest => {
                    let response = serde_json::from_value(record.response_json)
                        .map_err(|_error| ApiError::internal())?;
                    Ok(Some(response))
                }
                IdempotencyRequestComparison::DifferentRequest => {
                    Err(FileRouteError::IdempotencyMismatch.into())
                }
            }
        }
    }
}

async fn accept_new_revision(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    principal: &AdapterPrincipal,
    path: &VaultPath,
    parent_revision_id: Option<&RevisionId>,
    content_hash: ContentHash,
    size_bytes: u64,
) -> Result<PutFileResponse, ApiError> {
    insert_content_blob(transaction, content_hash, size_bytes).await?;

    let content_hash_string = content_hash.to_string();
    let object_id = deterministic_identifier("obj_", &[
        path.as_str(),
        parent_revision_id.map(RevisionId::as_str).unwrap_or("null"),
        content_hash_string.as_str(),
    ]);
    let object = create_or_find_sync_object_by_path(
        &mut **transaction,
        NewSyncObject {
            object_id: object_id.as_str(),
            path,
            kind: SyncObjectKind::File,
            updated_by: principal.common_adapter_id(),
        },
    )
    .await
    .map_err(map_repository_error)?;

    let revision_id = RevisionId::parse(&deterministic_identifier(
        "rev_",
        &[
            object.object_id.as_str(),
            parent_revision_id.map(RevisionId::as_str).unwrap_or("null"),
            content_hash_string.as_str(),
            principal.adapter_id(),
        ],
    ))
    .map_err(|_error| ApiError::internal())?;

    let revision = insert_file_revision(
        &mut **transaction,
        NewFileRevision {
            revision_id: &revision_id,
            object_id: object.object_id.as_str(),
            path,
            parent_revision_id,
            content_hash,
            size_bytes,
            created_by: principal.common_adapter_id(),
        },
    )
    .await
    .map_err(map_repository_error)?;

    let _updated_object = set_current_revision_by_object_id(
        &mut **transaction,
        object.object_id.as_str(),
        Some(&revision_id),
        principal.common_adapter_id(),
    )
    .await
    .map_err(map_repository_error)?
    .ok_or_else(ApiError::internal)?;

    let op_id = OperationId::parse(&deterministic_identifier(
        "op_",
        &[
            revision.revision_id.as_str(),
            OperationKindName::UpsertFile.as_str(),
            path.as_str(),
            content_hash_string.as_str(),
        ],
    ))
    .map_err(|_error| ApiError::internal())?;

    let operation = OperationLogRepository::new()
        .append(
            &mut **transaction,
            &AppendOperationLogEntry {
                op_id,
                adapter_id: principal.common_adapter_id().clone(),
                kind: OperationKindName::UpsertFile,
                path: path.clone(),
                revision_id: Some(revision_id.clone()),
                tombstone_id: None,
                conflict_id: None,
            },
        )
        .await
        .map_err(map_repository_error)?;

    Ok(accepted_upload_response(
        path.clone(),
        revision_id,
        operation.seq,
    ))
}

async fn insert_content_blob(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    content_hash: ContentHash,
    size_bytes: u64,
) -> Result<(), ApiError> {
    let size_bytes = i64::try_from(size_bytes).map_err(|_| ApiError::internal())?;
    let object_store_path = LocalObjectStore::relative_blob_path(content_hash)
        .to_string_lossy()
        .replace('\\', "/");

    sqlx::query(
        "insert into content_blobs (sha256, size_bytes, object_store_path) \
         values ($1, $2, $3) \
         on conflict (sha256) do nothing",
    )
    .bind(content_hash.to_prefixed_string())
    .bind(size_bytes)
    .bind(object_store_path)
    .execute(&mut **transaction)
    .await
    .map_err(|_error| ApiError::internal())?;

    Ok(())
}

fn stored_revision_from_row(row: FileRevisionRow) -> Result<StoredRevisionMetadata, ApiError> {
    let revision_id =
        RevisionId::parse(row.revision_id.as_str()).map_err(|_error| ApiError::internal())?;
    let content_hash =
        ContentHash::parse(row.content_sha256.as_str()).map_err(|_error| ApiError::internal())?;
    let size_bytes = u64::try_from(row.size_bytes).map_err(|_| ApiError::internal())?;

    Ok(StoredRevisionMetadata {
        revision_id,
        content_hash,
        size_bytes,
    })
}

struct StoredRevisionMetadata {
    revision_id: RevisionId,
    content_hash: ContentHash,
    size_bytes: u64,
}

fn raw_file_response(bytes: Vec<u8>, headers: FileDownloadRouteHeaders) -> Result<Response, ApiError> {
    let mut response = Response::new(Body::from(bytes));
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(APPLICATION_OCTET_STREAM),
    );
    insert_header(
        response.headers_mut(),
        X_REVISION_ID_HEADER,
        headers.revision_id().as_str(),
    )?;
    insert_header(
        response.headers_mut(),
        X_CONTENT_SHA256_HEADER,
        &headers.content_sha256().to_string(),
    )?;
    insert_header(
        response.headers_mut(),
        X_SIZE_BYTES_HEADER,
        &headers.size_bytes().to_string(),
    )?;

    Ok(response)
}

fn replay_response(stored: StoredIdempotencyResponse) -> Result<Response, ApiError> {
    let status = StatusCode::from_u16(stored.status_code()).map_err(|_error| ApiError::internal())?;
    let mut response = (status, Json(stored.body().clone())).into_response();

    for (name, value) in stored.headers() {
        let name = HeaderName::from_bytes(name.as_bytes()).map_err(|_error| ApiError::internal())?;
        let value = HeaderValue::from_str(value).map_err(|_error| ApiError::internal())?;
        response.headers_mut().insert(name, value);
    }

    Ok(response)
}

fn insert_header(headers: &mut HeaderMap, name: &'static str, value: &str) -> Result<(), ApiError> {
    let name = HeaderName::from_bytes(name.as_bytes()).map_err(|_error| ApiError::internal())?;
    let value = HeaderValue::from_str(value).map_err(|_error| ApiError::internal())?;
    headers.insert(name, value);
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
        size_bytes: row
            .size_bytes
            .map(u64::try_from)
            .transpose()
            .map_err(|_error| ApiError::internal())?,
        tombstone_id: row.tombstone_id.map(TombstoneIdDto::from),
        conflict_id: row.conflict_id.map(ConflictIdDto::from),
        updated_by: AdapterIdDto::from(row.adapter_id),
        updated_at: TimestampDto::from(row.created_at.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
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

fn map_object_store_put_error(error: ObjectStoreError) -> ApiError {
    match error {
        ObjectStoreError::HashMismatch { .. } => FileRouteError::InvalidContentSha256.into(),
        _ => ApiError::internal(),
    }
}

fn map_object_store_get_error(error: ObjectStoreError) -> ApiError {
    match error {
        ObjectStoreError::MissingBlob { .. } => FileRouteError::NotFound.into(),
        _ => ApiError::internal(),
    }
}

fn map_repository_error(_error: RepositoryError) -> ApiError {
    ApiError::internal()
}

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
        Self {
            status,
            body: ErrorResponse::new(code, message),
        }
    }

    fn missing_token() -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            PublicErrorCode::MissingToken,
            "Missing bearer token",
        )
    }

    fn invalid_token() -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            PublicErrorCode::InvalidToken,
            "Invalid bearer token",
        )
    }

    fn forbidden_role() -> Self {
        Self::new(
            StatusCode::FORBIDDEN,
            PublicErrorCode::ForbiddenRole,
            "Authenticated adapter role is not allowed for this operation",
        )
    }

    fn storage_unavailable() -> Self {
        Self::new(
            StatusCode::SERVICE_UNAVAILABLE,
            PublicErrorCode::InternalError,
            "Core storage dependencies are not configured",
        )
    }

    fn internal() -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            PublicErrorCode::InternalError,
            "Core file operation failed",
        )
    }
}

impl From<FileRouteError> for ApiError {
    fn from(error: FileRouteError) -> Self {
        let status = StatusCode::from_u16(error.http_status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        Self {
            status,
            body: error.to_error_response(),
        }
    }
}

impl From<ChangesRouteError> for ApiError {
    fn from(error: ChangesRouteError) -> Self {
        let status = StatusCode::from_u16(error.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        Self {
            status,
            body: error.error_response(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use haze_sync_api::auth::AdapterRole;

    fn parsed_request(path: &str, base_revision_id: Option<&str>, content_sha256: &str) -> PutFileRouteRequest {
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

    #[test]
    fn id_fixtures_have_required_prefixes() {
        let revision_id = deterministic_identifier("rev_", &["Notes/a.md", "sha256"]);
        let op_id = deterministic_identifier("op_", &["rev_1", "upsert_file"]);

        assert!(RevisionId::parse(&revision_id).is_ok());
        assert!(OperationId::parse(&op_id).is_ok());
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
}
