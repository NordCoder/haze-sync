//! Server wiring for the W3 conflict API fan-in slice.

use axum::{
    extract::{Path, Query},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Extension, Json, Router,
};
use haze_sync_api::{
    auth::{AdapterPrincipal, BearerToken},
    contracts::{
        errors::{ErrorResponse, PublicErrorCode},
        headers::AUTHORIZATION_HEADER,
    },
    dto::{
        common::{ConflictPolicyDto, ConflictResolutionDto, ConflictStatusDto},
        primitives::TimestampDto,
    },
    routes::conflicts::{
        conflict_list_response_from_parts, parse_conflicts_query, parse_resolve_conflict_request,
        resolved_conflict_response, ConflictListRequestParts, ConflictRouteSummaryParts,
        ConflictsRouteError, ResolveConflictRequestParts,
    },
};
use haze_sync_common::{AdapterId, ConflictId, OperationId, RevisionId, VaultPath};
use haze_sync_storage::{
    models::ConflictRow,
    repositories::{
        conflicts::{ConflictRepository, ConflictStatusName},
        operation_log::{AppendOperationLogEntry, OperationKindName, OperationLogRepository},
        RepositoryError,
    },
};
use serde_json::Value;
use sha2::{Digest, Sha256 as Sha256Digest};
use sqlx::PgPool;
use std::{collections::HashMap, str::FromStr};

use crate::{
    http::errors::not_implemented_response,
    state::{AuthState, ServerAppState},
};

pub fn router() -> Router {
    Router::new()
        .route("/conflicts", get(list_conflicts_route))
        .route(
            "/conflicts/:conflict_id/resolve",
            post(resolve_conflict_route),
        )
}

pub(super) async fn list_conflicts_route(
    Extension(state): Extension<ServerAppState>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    authenticate(&state, &headers, ConflictPermission::Read).await?;
    let request = parse_conflicts_query(ConflictListRequestParts {
        status: query.get("status").map(String::as_str),
    })?;

    let Some(pool) = state.db_pool() else {
        return Ok((
            StatusCode::OK,
            Json(conflict_list_response_from_parts(Vec::new())),
        )
            .into_response());
    };

    let status = match request.status {
        Some(ConflictStatusDto::Open) | None => ConflictStatusName::Open,
        Some(_) => return Err(ConflictsRouteError::unsupported_status().into()),
    };

    let conflicts = ConflictRepository::new()
        .list_by_status(pool, status)
        .await
        .map_err(map_repository_error)?
        .into_iter()
        .map(conflict_route_summary_from_row)
        .collect::<Result<Vec<_>, _>>()?;

    Ok((
        StatusCode::OK,
        Json(conflict_list_response_from_parts(conflicts)),
    )
        .into_response())
}

pub(super) async fn resolve_conflict_route(
    Extension(state): Extension<ServerAppState>,
    Path(conflict_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Response, ApiError> {
    let principal = authenticate(&state, &headers, ConflictPermission::Resolve).await?;
    let request = parse_resolve_body(conflict_id.as_str(), &body)?;

    if !resolution_is_metadata_only(&request.resolution) {
        return Ok(not_implemented_response().into_response());
    }

    let Some(pool) = state.db_pool() else {
        return Ok(not_implemented_response().into_response());
    };

    let seq = mark_conflict_resolved_with_operation(pool, &principal, &request.conflict_id).await?;
    let response = resolved_conflict_response(request.conflict_id, request.resolution, seq);
    Ok((StatusCode::OK, Json(response)).into_response())
}

fn parse_resolve_body(
    conflict_id: &str,
    body: &Value,
) -> Result<haze_sync_api::routes::conflicts::ResolveConflictRouteRequest, ConflictsRouteError> {
    let Some(object) = body.as_object() else {
        return Err(ConflictsRouteError::invalid_resolve_payload());
    };

    let extra_fields = object
        .keys()
        .filter(|key| key.as_str() != "resolution")
        .map(String::as_str)
        .collect::<Vec<_>>();
    let resolution = object.get("resolution").and_then(Value::as_str);

    parse_resolve_conflict_request(ResolveConflictRequestParts {
        conflict_id,
        resolution,
        extra_fields: &extra_fields,
    })
}

async fn mark_conflict_resolved_with_operation(
    pool: &PgPool,
    principal: &AdapterPrincipal,
    conflict_id: &ConflictId,
) -> Result<i64, ApiError> {
    let mut transaction = pool.begin().await.map_err(|_error| ApiError::internal())?;
    let repository = ConflictRepository::new();
    let conflict = repository
        .get_by_id(&mut *transaction, conflict_id)
        .await
        .map_err(map_repository_error)?
        .ok_or_else(ConflictsRouteError::not_found)?;

    if ConflictStatusName::from_str(conflict.status.as_str()).map_err(map_repository_error)?
        != ConflictStatusName::Open
    {
        return Err(ConflictsRouteError::already_resolved().into());
    }

    let path =
        VaultPath::parse(conflict.original_path.as_str()).map_err(|_error| ApiError::internal())?;
    repository
        .mark_open_resolved(
            &mut *transaction,
            conflict_id,
            principal.common_adapter_id(),
        )
        .await
        .map_err(map_repository_error)?
        .ok_or_else(ConflictsRouteError::already_resolved)?;

    let operation = OperationLogRepository::new()
        .append(
            &mut *transaction,
            &AppendOperationLogEntry {
                op_id: conflict_resolved_operation_id(conflict_id, principal.common_adapter_id())?,
                adapter_id: principal.common_adapter_id().clone(),
                kind: OperationKindName::ConflictResolved,
                path,
                revision_id: None,
                tombstone_id: None,
                conflict_id: Some(conflict_id.clone()),
            },
        )
        .await
        .map_err(map_repository_error)?;

    transaction
        .commit()
        .await
        .map_err(|_error| ApiError::internal())?;
    Ok(operation.seq)
}

fn conflict_route_summary_from_row(
    row: ConflictRow,
) -> Result<ConflictRouteSummaryParts, ApiError> {
    let created_at = TimestampDto::from(row.created_at.format("%Y-%m-%dT%H:%M:%SZ").to_string());
    let updated_at = row
        .resolved_at
        .map(|value| TimestampDto::from(value.format("%Y-%m-%dT%H:%M:%SZ").to_string()));

    Ok(ConflictRouteSummaryParts {
        conflict_id: ConflictId::parse(row.conflict_id.as_str())
            .map_err(|_error| ApiError::internal())?,
        original_path: VaultPath::parse(row.original_path.as_str())
            .map_err(|_error| ApiError::internal())?,
        conflict_path: VaultPath::parse(row.materialized_path.as_str())
            .map_err(|_error| ApiError::internal())?,
        current_revision_id: RevisionId::parse(row.current_revision_id.as_str())
            .map_err(|_error| ApiError::internal())?,
        conflict_revision_id: Some(
            RevisionId::parse(row.incoming_revision_id.as_str())
                .map_err(|_error| ApiError::internal())?,
        ),
        incoming_revision_id: Some(
            RevisionId::parse(row.incoming_revision_id.as_str())
                .map_err(|_error| ApiError::internal())?,
        ),
        source_adapter_id: AdapterId::parse(row.incoming_adapter_id.as_str())
            .map_err(|_error| ApiError::internal())?,
        policy_applied: match row.policy_applied.as_str() {
            "preserve_both" => ConflictPolicyDto::PreserveBoth,
            "current_wins_with_incoming_backup" => ConflictPolicyDto::CurrentWinsWithIncomingBackup,
            _ => return Err(ApiError::internal()),
        },
        status: match ConflictStatusName::from_str(row.status.as_str())
            .map_err(map_repository_error)?
        {
            ConflictStatusName::Open => ConflictStatusDto::Open,
            ConflictStatusName::Resolved => ConflictStatusDto::Resolved,
            ConflictStatusName::Ignored => ConflictStatusDto::Ignored,
        },
        created_at: Some(created_at),
        updated_at,
    })
}

fn resolution_is_metadata_only(resolution: &ConflictResolutionDto) -> bool {
    matches!(
        resolution,
        ConflictResolutionDto::AcceptCurrent
            | ConflictResolutionDto::KeepBoth
            | ConflictResolutionDto::MarkResolved
    )
}

#[derive(Clone, Copy)]
enum ConflictPermission {
    Read,
    Resolve,
}

async fn authenticate(
    state: &ServerAppState,
    headers: &HeaderMap,
    permission: ConflictPermission,
) -> Result<AdapterPrincipal, ApiError> {
    let header = required_auth_header(headers)?;
    BearerToken::parse_authorization_header(header).map_err(|_error| ApiError::invalid_token())?;
    let principal = match state.auth() {
        AuthState::Disabled => return Err(ApiError::invalid_token()),
        AuthState::StaticPrincipal { principal } => principal.clone(),
        AuthState::Database { .. } => return Err(ApiError::invalid_token()),
    };

    match permission {
        ConflictPermission::Read if !principal.role().can_read_files() => {
            return Err(ApiError::forbidden_role());
        }
        ConflictPermission::Resolve if !principal.role().can_resolve_conflicts() => {
            return Err(ApiError::forbidden_role());
        }
        _ => {}
    }
    Ok(principal)
}

fn required_auth_header(headers: &HeaderMap) -> Result<&str, ApiError> {
    let value = headers
        .get(AUTHORIZATION_HEADER)
        .ok_or_else(ApiError::missing_token)?;
    value.to_str().map_err(|_error| ApiError::invalid_token())
}

fn conflict_resolved_operation_id(
    conflict_id: &ConflictId,
    adapter_id: &AdapterId,
) -> Result<OperationId, ApiError> {
    OperationId::parse(&deterministic_identifier(
        "op_",
        &[
            conflict_id.as_str(),
            OperationKindName::ConflictResolved.as_str(),
            adapter_id.as_str(),
        ],
    ))
    .map_err(|_error| ApiError::internal())
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

fn map_repository_error(_error: RepositoryError) -> ApiError {
    ApiError::internal()
}

#[derive(Debug)]
pub(super) struct ApiError {
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

    fn internal() -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            PublicErrorCode::InternalError,
            "Core conflict operation failed",
        )
    }
}

impl From<ConflictsRouteError> for ApiError {
    fn from(error: ConflictsRouteError) -> Self {
        Self {
            status: StatusCode::from_u16(error.status_code())
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
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
    use axum::{body::Body, http::Request};
    use haze_sync_api::auth::AdapterRole;
    use http_body_util::BodyExt as _;
    use serde_json::json;
    use tower::ServiceExt as _;

    fn static_state() -> ServerAppState {
        let principal =
            AdapterPrincipal::new("obsidian-plugin", AdapterRole::ObsidianPlugin).unwrap();
        ServerAppState::with_static_principal(principal)
    }

    async fn request_json(method: &str, uri: &str, body: Body) -> (StatusCode, Value) {
        let request = Request::builder()
            .method(method)
            .uri(uri)
            .header(AUTHORIZATION_HEADER, "Bearer conflict_route_test_token")
            .header("content-type", "application/json")
            .body(body)
            .expect("test request should build");
        let response = router()
            .layer(Extension(static_state()))
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

    #[tokio::test]
    async fn get_open_conflicts_without_storage_returns_safe_empty_dto() {
        let (status, json) = request_json("GET", "/conflicts?status=open", Body::empty()).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["conflicts"], json!([]));
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
}
