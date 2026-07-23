//! Authenticated Google Drive durable-state HTTP and transaction boundary.

use axum::{
    body::Bytes,
    extract::{Extension, Path, Query},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use haze_sync_api::{
    contracts::headers::IDEMPOTENCY_KEY_HEADER,
    dto::{
        gdrive::{
            GDriveDeleteCandidateFactsDto, GDriveEchoFactsDto, GDriveEchoStateDto,
            GDriveLastOperationsSummaryDto, GDriveMappingFactsDto, GDriveOperationKindDto,
            GDrivePrivateCursorStateDto, GDriveProviderIdentifierDto, GDriveRawCursorDto,
            GDriveStateCommitRequest, GDriveStateCommitResponse, GDriveStateErrorResponse,
            GDriveStateSnapshotResponse,
        },
        primitives::{AdapterIdDto, OperationIdDto, RevisionIdDto, TimestampDto, VaultPathDto},
    },
    routes::gdrive::{
        parse_authenticated_gdrive_state_commit_request,
        parse_authenticated_get_gdrive_state_request, sanitize_snapshot_for_admin,
        validate_private_snapshot, GDriveStateCommitRouteParts, GDriveStateReadAccess,
        GDriveStateRouteError, GetGDriveStateRouteParts,
    },
};
use haze_sync_common::{AdapterId, OperationId, RevisionId, VaultPath};
use haze_sync_storage::repositories::{
    gdrive_state::{
        compare_and_commit_gdrive_state, load_gdrive_state_snapshot, GDriveCommitOutcome,
        GDriveCursor, GDriveCursorAdvance, GDriveDurableItemRow, GDriveEchoState, GDriveItemUpsert,
        GDriveOperationFingerprint, GDriveOperationInput, GDriveOperationKind, GDriveStateCommit,
        GDriveStateSnapshotPage,
    },
    RepositoryError,
};
use serde::Deserialize;
use sqlx::{Postgres, Row, Transaction};

use crate::{
    routes::auth::{authenticate_principal, AuthFailure},
    state::ServerAppState,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GDriveStateQuery {
    after_path: Option<String>,
    limit: Option<usize>,
}

pub fn router() -> axum::Router {
    axum::Router::new()
        .route(
            "/adapters/:adapter_id/gdrive/state",
            axum::routing::get(get_state_route),
        )
        .route(
            "/adapters/:adapter_id/gdrive/state/commit",
            axum::routing::post(commit_state_route),
        )
}

async fn get_state_route(
    Extension(state): Extension<ServerAppState>,
    Path(adapter_id): Path<String>,
    Query(query): Query<GDriveStateQuery>,
    headers: HeaderMap,
) -> Result<Response, GDriveHttpError> {
    let principal = authenticate_principal(&state, &headers)
        .await
        .map_err(GDriveHttpError::from_auth)?;
    let request = parse_authenticated_get_gdrive_state_request(
        GetGDriveStateRouteParts {
            adapter_id: &adapter_id,
            after_path: query.after_path.as_deref(),
            limit: query.limit,
        },
        Some(&principal),
    )
    .map_err(GDriveHttpError::from_route)?;
    let limit = u32::try_from(request.limit())
        .map_err(|_| GDriveHttpError::from_route(GDriveStateRouteError::ValidationError))?;
    let pool = state
        .db_pool()
        .ok_or_else(|| GDriveHttpError::from_route(GDriveStateRouteError::Unavailable))?;
    let mut transaction = pool
        .begin()
        .await
        .map_err(|_| GDriveHttpError::internal())?;

    if let Err(error) = verify_gdrive_adapter(&mut transaction, request.adapter_id()).await {
        let _ = transaction.rollback().await;
        return Err(error);
    }

    let page = match load_gdrive_state_snapshot(
        &mut transaction,
        request.adapter_id(),
        request.after_path(),
        limit,
    )
    .await
    {
        Ok(Some(page)) => page,
        Ok(None) => {
            let _ = transaction.rollback().await;
            return Err(GDriveHttpError::from_route(
                GDriveStateRouteError::AdapterNotFound,
            ));
        }
        Err(error) => {
            let _ = transaction.rollback().await;
            return Err(repository_error(error));
        }
    };

    let snapshot = match snapshot_response(page) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            let _ = transaction.rollback().await;
            return Err(error);
        }
    };
    if let Err(error) = validate_private_snapshot(&snapshot) {
        let _ = transaction.rollback().await;
        return Err(GDriveHttpError::from_route(error));
    }
    transaction
        .commit()
        .await
        .map_err(|_| GDriveHttpError::internal())?;

    match request.access() {
        GDriveStateReadAccess::AdapterPrivate => {
            Ok((StatusCode::OK, Json(snapshot)).into_response())
        }
        GDriveStateReadAccess::AdminSanitized => {
            let summary =
                sanitize_snapshot_for_admin(&snapshot).map_err(GDriveHttpError::from_route)?;
            Ok((StatusCode::OK, Json(summary)).into_response())
        }
    }
}

async fn commit_state_route(
    Extension(state): Extension<ServerAppState>,
    Path(adapter_id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, GDriveHttpError> {
    let principal = authenticate_principal(&state, &headers)
        .await
        .map_err(GDriveHttpError::from_auth)?;
    let request_body = serde_json::from_slice::<GDriveStateCommitRequest>(&body)
        .map_err(|_| GDriveHttpError::from_route(GDriveStateRouteError::ValidationError))?;
    let idempotency_key = headers
        .get(IDEMPOTENCY_KEY_HEADER)
        .and_then(|value| value.to_str().ok());
    let request = parse_authenticated_gdrive_state_commit_request(
        GDriveStateCommitRouteParts {
            adapter_id: &adapter_id,
            idempotency_key,
            body: request_body,
        },
        Some(&principal),
    )
    .map_err(GDriveHttpError::from_route)?;
    let prepared = PreparedCommit::try_from_request(request.adapter_id().clone(), request.body())?;
    let pool = state
        .db_pool()
        .ok_or_else(|| GDriveHttpError::from_route(GDriveStateRouteError::Unavailable))?;
    let mut transaction = pool
        .begin()
        .await
        .map_err(|_| GDriveHttpError::internal())?;

    if let Err(error) = verify_gdrive_adapter(&mut transaction, request.adapter_id()).await {
        let _ = transaction.rollback().await;
        return Err(error);
    }

    let storage_input = prepared.as_storage_input();
    let outcome = compare_and_commit_gdrive_state(&mut transaction, &storage_input).await;
    match outcome {
        Ok(GDriveCommitOutcome::Committed { state, .. }) => {
            transaction
                .commit()
                .await
                .map_err(|_| GDriveHttpError::internal())?;
            Ok((
                StatusCode::OK,
                Json(GDriveStateCommitResponse::Committed {
                    state_version: to_u64(state.state_version)?,
                    cursor_generation: to_u64(state.drive_cursor_generation)?,
                    core_export_checkpoint: to_u64(state.core_export_seq)?,
                }),
            )
                .into_response())
        }
        Ok(GDriveCommitOutcome::Replayed { operation }) => {
            transaction
                .commit()
                .await
                .map_err(|_| GDriveHttpError::internal())?;
            Ok((
                StatusCode::OK,
                Json(GDriveStateCommitResponse::Replayed {
                    state_version: to_u64(operation.committed_state_version)?,
                }),
            )
                .into_response())
        }
        Err(error) => {
            let _ = transaction.rollback().await;
            match commit_error_response(error) {
                Ok((status, response)) => Ok((status, Json(response)).into_response()),
                Err(error) => Err(error),
            }
        }
    }
}

async fn verify_gdrive_adapter(
    transaction: &mut Transaction<'_, Postgres>,
    adapter_id: &AdapterId,
) -> Result<(), GDriveHttpError> {
    let row =
        sqlx::query("select role from sync_adapters where adapter_id = $1 and enabled = true")
            .bind(adapter_id.as_str())
            .fetch_optional(&mut **transaction)
            .await
            .map_err(|_| GDriveHttpError::internal())?
            .ok_or_else(|| GDriveHttpError::from_route(GDriveStateRouteError::AdapterNotFound))?;
    let role: String = row
        .try_get("role")
        .map_err(|_| GDriveHttpError::internal())?;
    if role != "gdrive_adapter" {
        return Err(GDriveHttpError::from_route(
            GDriveStateRouteError::Forbidden,
        ));
    }
    Ok(())
}

fn snapshot_response(
    page: GDriveStateSnapshotPage,
) -> Result<GDriveStateSnapshotResponse, GDriveHttpError> {
    let mappings = page
        .items
        .into_iter()
        .map(mapping_response)
        .collect::<Result<Vec<_>, _>>()?;
    let cursor_generation = to_u64(page.state.drive_cursor_generation)?;
    let cursor = match page.state.drive_cursor.as_deref() {
        Some(cursor) => GDrivePrivateCursorStateDto::Present {
            generation: cursor_generation,
            cursor: GDriveRawCursorDto::parse(cursor).map_err(|_| GDriveHttpError::internal())?,
        },
        None => GDrivePrivateCursorStateDto::Absent {
            generation: cursor_generation,
        },
    };
    Ok(GDriveStateSnapshotResponse {
        adapter_id: AdapterIdDto::new(page.state.adapter_id),
        state_format_version: u32::try_from(page.state.state_format_version)
            .map_err(|_| GDriveHttpError::internal())?,
        state_version: to_u64(page.state.state_version)?,
        cursor,
        core_export_checkpoint: to_u64(page.state.core_export_seq)?,
        last_operations: GDriveLastOperationsSummaryDto {
            import: page.state.last_import_operation_id.map(OperationIdDto::new),
            export: page.state.last_export_operation_id.map(OperationIdDto::new),
            provider_mutation: page
                .state
                .last_provider_mutation_operation_id
                .map(OperationIdDto::new),
        },
        mappings,
        next_after_path: page.next_after_path.map(VaultPathDto::from),
    })
}

fn mapping_response(row: GDriveDurableItemRow) -> Result<GDriveMappingFactsDto, GDriveHttpError> {
    let echo = GDriveEchoFactsDto {
        state: match row.echo_state.as_str() {
            "none" => GDriveEchoStateDto::None,
            "pending" => GDriveEchoStateDto::Pending,
            "confirmed" => GDriveEchoStateDto::Confirmed,
            _ => return Err(GDriveHttpError::internal()),
        },
        operation_id: row.echo_operation_id.map(OperationIdDto::new),
        provider_version: row
            .echo_provider_version
            .map(parse_provider_identifier)
            .transpose()?,
    };
    let delete_candidate = match (
        row.delete_candidate_first_seen_at,
        row.delete_candidate_last_seen_at,
        row.delete_candidate_generation,
    ) {
        (None, None, None) => None,
        (Some(first), Some(last), Some(generation)) => Some(GDriveDeleteCandidateFactsDto {
            first_seen_at: timestamp(first),
            last_seen_at: timestamp(last),
            generation: to_u64(generation)?,
            blocked: row.delete_candidate_blocked,
            confirmation_audit_id: row
                .delete_confirmation_audit_id
                .map(parse_provider_identifier)
                .transpose()?,
        }),
        _ => return Err(GDriveHttpError::internal()),
    };
    Ok(GDriveMappingFactsDto {
        path: VaultPathDto::new(row.path),
        drive_file_id: row
            .drive_file_id
            .map(parse_provider_identifier)
            .transpose()?,
        drive_parent_id: row
            .drive_parent_id
            .map(parse_provider_identifier)
            .transpose()?,
        drive_name: row.drive_name,
        mime_type: row.mime_type,
        md5_checksum: row.md5_checksum,
        head_revision_id: row
            .head_revision_id
            .map(parse_provider_identifier)
            .transpose()?,
        drive_version: row
            .drive_version
            .map(parse_provider_identifier)
            .transpose()?,
        drive_modified_time: row.drive_modified_time.map(timestamp),
        core_object_id: row.core_object_id,
        core_revision_id: row.core_revision_id.map(RevisionIdDto::new),
        core_seq: row.core_seq.map(to_u64).transpose()?,
        echo,
        delete_candidate,
        last_imported_at: row.last_imported_at.map(timestamp),
        last_exported_at: row.last_exported_at.map(timestamp),
        last_seen_at: row.last_seen_at.map(timestamp),
    })
}

fn parse_provider_identifier(
    value: String,
) -> Result<GDriveProviderIdentifierDto, GDriveHttpError> {
    GDriveProviderIdentifierDto::parse(value).map_err(|_| GDriveHttpError::internal())
}

fn timestamp(value: DateTime<Utc>) -> TimestampDto {
    TimestampDto::new(value.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
}

fn to_u64(value: i64) -> Result<u64, GDriveHttpError> {
    u64::try_from(value).map_err(|_| GDriveHttpError::internal())
}

struct PreparedCommit {
    adapter_id: AdapterId,
    expected_state_version: i64,
    cursor: Option<GDriveCursor>,
    cursor_expected_generation: i64,
    cursor_next_generation: Option<i64>,
    core_export_seq: Option<i64>,
    item: Option<PreparedItem>,
    operation_id: OperationId,
    operation_kind: GDriveOperationKind,
    facts_fingerprint: GDriveOperationFingerprint,
    operation_mapping_path: Option<VaultPath>,
    operation_core_seq: Option<i64>,
    operation_drive_version: Option<String>,
}

struct PreparedItem {
    path: VaultPath,
    drive_file_id: Option<String>,
    drive_parent_id: Option<String>,
    drive_name: Option<String>,
    mime_type: Option<String>,
    md5_checksum: Option<String>,
    head_revision_id: Option<String>,
    drive_version: Option<String>,
    drive_modified_time: Option<DateTime<Utc>>,
    core_object_id: Option<String>,
    core_revision_id: Option<RevisionId>,
    core_seq: Option<i64>,
    echo_state: GDriveEchoState,
    echo_operation_id: Option<OperationId>,
    echo_provider_version: Option<String>,
    delete_candidate_first_seen_at: Option<DateTime<Utc>>,
    delete_candidate_last_seen_at: Option<DateTime<Utc>>,
    delete_candidate_generation: Option<i64>,
    delete_candidate_blocked: bool,
    delete_confirmation_audit_id: Option<String>,
    last_imported_at: Option<DateTime<Utc>>,
    last_exported_at: Option<DateTime<Utc>>,
    last_seen_at: Option<DateTime<Utc>>,
}

impl PreparedCommit {
    fn try_from_request(
        adapter_id: AdapterId,
        body: &GDriveStateCommitRequest,
    ) -> Result<Self, GDriveHttpError> {
        let cursor = body
            .cursor
            .advance
            .as_ref()
            .map(|advance| {
                GDriveCursor::parse(advance.cursor.expose_for_private_commit().to_owned())
            })
            .transpose()
            .map_err(repository_error)?;
        let item = body
            .mapping
            .as_ref()
            .map(PreparedItem::try_from)
            .transpose()?;
        Ok(Self {
            adapter_id,
            expected_state_version: to_i64(body.expected_state_version)?,
            cursor,
            cursor_expected_generation: to_i64(body.cursor.expected_generation)?,
            cursor_next_generation: body
                .cursor
                .advance
                .as_ref()
                .map(|advance| to_i64(advance.next_generation))
                .transpose()?,
            core_export_seq: body.core_export_checkpoint.map(to_i64).transpose()?,
            item,
            operation_id: OperationId::try_from(&body.operation.operation_id)
                .map_err(|_| GDriveHttpError::validation())?,
            operation_kind: match body.operation.kind {
                GDriveOperationKindDto::Import => GDriveOperationKind::Import,
                GDriveOperationKindDto::Export => GDriveOperationKind::Export,
                GDriveOperationKindDto::ProviderMutation => GDriveOperationKind::ProviderMutation,
                GDriveOperationKindDto::CursorCheckpoint => GDriveOperationKind::CursorCheckpoint,
                GDriveOperationKindDto::DeleteCandidate => GDriveOperationKind::DeleteCandidate,
            },
            facts_fingerprint: GDriveOperationFingerprint::parse(
                body.operation
                    .facts_fingerprint
                    .expose_for_private_commit()
                    .to_owned(),
            )
            .map_err(repository_error)?,
            operation_mapping_path: body
                .operation
                .mapping_path
                .as_ref()
                .map(VaultPath::try_from)
                .transpose()
                .map_err(|_| GDriveHttpError::validation())?,
            operation_core_seq: body.operation.core_seq.map(to_i64).transpose()?,
            operation_drive_version: body
                .operation
                .drive_version
                .as_ref()
                .map(|value| value.expose_for_private_transport().to_owned()),
        })
    }

    fn as_storage_input(&self) -> GDriveStateCommit<'_> {
        let cursor_advance = match (self.cursor.as_ref(), self.cursor_next_generation) {
            (Some(cursor), Some(next_generation)) => Some(GDriveCursorAdvance {
                cursor,
                expected_generation: self.cursor_expected_generation,
                next_generation,
            }),
            _ => None,
        };
        GDriveStateCommit {
            adapter_id: &self.adapter_id,
            expected_state_version: self.expected_state_version,
            cursor_advance,
            core_export_seq: self.core_export_seq,
            item: self.item.as_ref().map(PreparedItem::as_storage_input),
            operation: GDriveOperationInput {
                operation_id: &self.operation_id,
                kind: self.operation_kind,
                facts_fingerprint: &self.facts_fingerprint,
                mapping_path: self.operation_mapping_path.as_ref(),
                core_seq: self.operation_core_seq,
                drive_version: self.operation_drive_version.as_deref(),
            },
        }
    }
}

impl TryFrom<&GDriveMappingFactsDto> for PreparedItem {
    type Error = GDriveHttpError;

    fn try_from(value: &GDriveMappingFactsDto) -> Result<Self, Self::Error> {
        let candidate = value.delete_candidate.as_ref();
        Ok(Self {
            path: VaultPath::try_from(&value.path).map_err(|_| GDriveHttpError::validation())?,
            drive_file_id: private_id(value.drive_file_id.as_ref()),
            drive_parent_id: private_id(value.drive_parent_id.as_ref()),
            drive_name: value.drive_name.clone(),
            mime_type: value.mime_type.clone(),
            md5_checksum: value.md5_checksum.clone(),
            head_revision_id: private_id(value.head_revision_id.as_ref()),
            drive_version: private_id(value.drive_version.as_ref()),
            drive_modified_time: parse_timestamp(value.drive_modified_time.as_ref())?,
            core_object_id: value.core_object_id.clone(),
            core_revision_id: value
                .core_revision_id
                .as_ref()
                .map(RevisionId::try_from)
                .transpose()
                .map_err(|_| GDriveHttpError::validation())?,
            core_seq: value.core_seq.map(to_i64).transpose()?,
            echo_state: match value.echo.state {
                GDriveEchoStateDto::None => GDriveEchoState::None,
                GDriveEchoStateDto::Pending => GDriveEchoState::Pending,
                GDriveEchoStateDto::Confirmed => GDriveEchoState::Confirmed,
            },
            echo_operation_id: value
                .echo
                .operation_id
                .as_ref()
                .map(OperationId::try_from)
                .transpose()
                .map_err(|_| GDriveHttpError::validation())?,
            echo_provider_version: private_id(value.echo.provider_version.as_ref()),
            delete_candidate_first_seen_at: parse_timestamp(
                candidate.map(|item| &item.first_seen_at),
            )?,
            delete_candidate_last_seen_at: parse_timestamp(
                candidate.map(|item| &item.last_seen_at),
            )?,
            delete_candidate_generation: candidate
                .map(|item| to_i64(item.generation))
                .transpose()?,
            delete_candidate_blocked: candidate.is_some_and(|item| item.blocked),
            delete_confirmation_audit_id: candidate
                .and_then(|item| item.confirmation_audit_id.as_ref())
                .map(|value| value.expose_for_private_transport().to_owned()),
            last_imported_at: parse_timestamp(value.last_imported_at.as_ref())?,
            last_exported_at: parse_timestamp(value.last_exported_at.as_ref())?,
            last_seen_at: parse_timestamp(value.last_seen_at.as_ref())?,
        })
    }
}

impl PreparedItem {
    fn as_storage_input(&self) -> GDriveItemUpsert<'_> {
        GDriveItemUpsert {
            path: &self.path,
            drive_file_id: self.drive_file_id.as_deref(),
            drive_parent_id: self.drive_parent_id.as_deref(),
            drive_name: self.drive_name.as_deref(),
            mime_type: self.mime_type.as_deref(),
            md5_checksum: self.md5_checksum.as_deref(),
            head_revision_id: self.head_revision_id.as_deref(),
            drive_version: self.drive_version.as_deref(),
            drive_modified_time: self.drive_modified_time,
            core_object_id: self.core_object_id.as_deref(),
            core_revision_id: self.core_revision_id.as_ref(),
            core_seq: self.core_seq,
            echo_state: self.echo_state,
            echo_operation_id: self.echo_operation_id.as_ref(),
            echo_provider_version: self.echo_provider_version.as_deref(),
            delete_candidate_first_seen_at: self.delete_candidate_first_seen_at,
            delete_candidate_last_seen_at: self.delete_candidate_last_seen_at,
            delete_candidate_generation: self.delete_candidate_generation,
            delete_candidate_blocked: self.delete_candidate_blocked,
            delete_confirmation_audit_id: self.delete_confirmation_audit_id.as_deref(),
            last_imported_at: self.last_imported_at,
            last_exported_at: self.last_exported_at,
            last_seen_at: self.last_seen_at,
        }
    }
}

fn private_id(value: Option<&GDriveProviderIdentifierDto>) -> Option<String> {
    value.map(|value| value.expose_for_private_transport().to_owned())
}

fn parse_timestamp(value: Option<&TimestampDto>) -> Result<Option<DateTime<Utc>>, GDriveHttpError> {
    value
        .map(|value| {
            DateTime::parse_from_rfc3339(value.as_str())
                .map(|value| value.with_timezone(&Utc))
                .map_err(|_| GDriveHttpError::validation())
        })
        .transpose()
}

fn to_i64(value: u64) -> Result<i64, GDriveHttpError> {
    i64::try_from(value).map_err(|_| GDriveHttpError::validation())
}

fn commit_error_response(
    error: RepositoryError,
) -> Result<(StatusCode, GDriveStateCommitResponse), GDriveHttpError> {
    match error {
        RepositoryError::GDriveStateStaleExpected => {
            Ok((StatusCode::CONFLICT, GDriveStateCommitResponse::StaleState))
        }
        RepositoryError::CursorRegression | RepositoryError::CheckpointRegression => Ok((
            StatusCode::CONFLICT,
            GDriveStateCommitResponse::CursorRegression,
        )),
        RepositoryError::CursorGap => {
            Ok((StatusCode::CONFLICT, GDriveStateCommitResponse::CursorGap))
        }
        RepositoryError::GDriveCursorGenerationMismatch => Err(GDriveHttpError::from_route(
            GDriveStateRouteError::InvalidCursorState,
        )),
        RepositoryError::GDriveOperationConflict => Ok((
            StatusCode::CONFLICT,
            GDriveStateCommitResponse::IdempotencyConflict,
        )),
        RepositoryError::InvalidPath
        | RepositoryError::InvalidIdentifier
        | RepositoryError::InvalidHash
        | RepositoryError::InvalidProviderMetadata
        | RepositoryError::InvalidOperationKind
        | RepositoryError::InvalidSequence => Ok((
            StatusCode::UNPROCESSABLE_ENTITY,
            GDriveStateCommitResponse::ValidationFailed,
        )),
        _ => Err(GDriveHttpError::internal()),
    }
}

fn repository_error(error: RepositoryError) -> GDriveHttpError {
    match error {
        RepositoryError::GDriveStateMissing => {
            GDriveHttpError::from_route(GDriveStateRouteError::AdapterNotFound)
        }
        RepositoryError::GDriveStateStaleExpected => {
            GDriveHttpError::from_route(GDriveStateRouteError::StaleState)
        }
        RepositoryError::GDriveCursorGenerationMismatch => {
            GDriveHttpError::from_route(GDriveStateRouteError::InvalidCursorState)
        }
        RepositoryError::CursorRegression | RepositoryError::CheckpointRegression => {
            GDriveHttpError::from_route(GDriveStateRouteError::CursorRegression)
        }
        RepositoryError::CursorGap => GDriveHttpError::from_route(GDriveStateRouteError::CursorGap),
        RepositoryError::GDriveOperationConflict => {
            GDriveHttpError::from_route(GDriveStateRouteError::IdempotencyConflict)
        }
        RepositoryError::DatabaseOperationFailed => GDriveHttpError::internal(),
        _ => GDriveHttpError::validation(),
    }
}

#[derive(Debug)]
struct GDriveHttpError {
    status: StatusCode,
    body: GDriveStateErrorResponse,
}

impl GDriveHttpError {
    fn from_auth(error: AuthFailure) -> Self {
        match error {
            AuthFailure::MissingToken => Self::from_route(GDriveStateRouteError::Unauthorized),
            AuthFailure::InvalidToken => Self::from_route(GDriveStateRouteError::Unauthorized),
            AuthFailure::Internal => Self::internal(),
        }
    }

    fn from_route(error: GDriveStateRouteError) -> Self {
        let status = StatusCode::from_u16(error.http_status_code())
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        Self {
            status,
            body: error.to_error_response(),
        }
    }

    fn validation() -> Self {
        Self::from_route(GDriveStateRouteError::ValidationError)
    }

    fn internal() -> Self {
        Self::from_route(GDriveStateRouteError::Internal)
    }
}

impl IntoResponse for GDriveHttpError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}
