//! Passive route contracts for Google Drive durable state.
//!
//! This module validates request shape, authenticated adapter identity, bounded
//! values, and safe response construction. It does not register HTTP routes,
//! open transactions, access Storage, call Google, or execute synchronization.

use std::{error::Error, fmt};

use haze_sync_common::{AdapterId, OperationId, RevisionId, VaultPath};

use crate::{
    auth::{AdapterPrincipal, AdapterRole},
    contracts::headers::{HeaderValueError, IdempotencyKey},
    dto::{
        gdrive::{
            GDriveDeleteCandidateCountSummaryDto, GDriveEchoCountSummaryDto, GDriveEchoStateDto,
            GDriveLastOperationPresenceDto, GDriveMappingFactsDto, GDriveStateAdminSummaryResponse,
            GDriveStateCommitRequest, GDriveStateErrorCode, GDriveStateErrorResponse,
            GDriveStatePublicError, GDriveStateSnapshotResponse, MAX_GDRIVE_STATE_ITEMS,
            MAX_GDRIVE_TEXT_BYTES, GDRIVE_MD5_HEX_BYTES,
        },
        primitives::AdapterIdDto,
    },
};

pub const GDRIVE_STATE_ROUTE: &str = "/v1/adapters/{adapter_id}/gdrive/state";
pub const GDRIVE_STATE_COMMIT_ROUTE: &str = "/v1/adapters/{adapter_id}/gdrive/state/commit";
pub const DEFAULT_GDRIVE_STATE_LIMIT: usize = 100;
pub const MAX_GDRIVE_STATE_LIMIT: usize = MAX_GDRIVE_STATE_ITEMS;
pub const HTTP_STATUS_UNAUTHORIZED: u16 = 401;
pub const HTTP_STATUS_FORBIDDEN: u16 = 403;
pub const HTTP_STATUS_NOT_FOUND: u16 = 404;
pub const HTTP_STATUS_CONFLICT: u16 = 409;
pub const HTTP_STATUS_UNPROCESSABLE_ENTITY: u16 = 422;
pub const HTTP_STATUS_INTERNAL_SERVER_ERROR: u16 = 500;
pub const HTTP_STATUS_SERVICE_UNAVAILABLE: u16 = 503;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GDriveStateReadAccess {
    AdapterPrivate,
    AdminSanitized,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GetGDriveStateRouteParts<'a> {
    pub adapter_id: &'a str,
    pub after_path: Option<&'a str>,
    pub limit: Option<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthenticatedGetGDriveStateRequest {
    adapter_id: AdapterId,
    principal: AdapterPrincipal,
    access: GDriveStateReadAccess,
    after_path: Option<VaultPath>,
    limit: usize,
}

impl AuthenticatedGetGDriveStateRequest {
    #[must_use]
    pub const fn adapter_id(&self) -> &AdapterId {
        &self.adapter_id
    }

    #[must_use]
    pub const fn principal(&self) -> &AdapterPrincipal {
        &self.principal
    }

    #[must_use]
    pub const fn access(&self) -> GDriveStateReadAccess {
        self.access
    }

    #[must_use]
    pub const fn after_path(&self) -> Option<&VaultPath> {
        self.after_path.as_ref()
    }

    #[must_use]
    pub const fn limit(&self) -> usize {
        self.limit
    }
}

pub fn parse_authenticated_get_gdrive_state_request(
    parts: GetGDriveStateRouteParts<'_>,
    principal: Option<&AdapterPrincipal>,
) -> Result<AuthenticatedGetGDriveStateRequest, GDriveStateRouteError> {
    let principal = principal
        .cloned()
        .ok_or(GDriveStateRouteError::Unauthorized)?;
    let adapter_id = AdapterId::parse(parts.adapter_id)
        .map_err(|_| GDriveStateRouteError::ValidationError)?;
    let access = match principal.role() {
        AdapterRole::Admin => GDriveStateReadAccess::AdminSanitized,
        AdapterRole::GdriveAdapter if principal.adapter_id() == adapter_id.as_str() => {
            GDriveStateReadAccess::AdapterPrivate
        }
        _ => return Err(GDriveStateRouteError::Forbidden),
    };
    let after_path = parts
        .after_path
        .map(VaultPath::parse)
        .transpose()
        .map_err(|_| GDriveStateRouteError::ValidationError)?;
    let limit = parts.limit.unwrap_or(DEFAULT_GDRIVE_STATE_LIMIT);
    if limit == 0 || limit > MAX_GDRIVE_STATE_LIMIT {
        return Err(GDriveStateRouteError::ValidationError);
    }

    Ok(AuthenticatedGetGDriveStateRequest {
        adapter_id,
        principal,
        access,
        after_path,
        limit,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GDriveStateCommitRouteParts<'a> {
    pub adapter_id: &'a str,
    pub idempotency_key: Option<&'a str>,
    pub body: GDriveStateCommitRequest,
}

#[derive(Clone, Eq, PartialEq)]
pub struct AuthenticatedGDriveStateCommitRequest {
    adapter_id: AdapterId,
    principal: AdapterPrincipal,
    idempotency_key: IdempotencyKey,
    body: GDriveStateCommitRequest,
}

impl AuthenticatedGDriveStateCommitRequest {
    #[must_use]
    pub const fn adapter_id(&self) -> &AdapterId {
        &self.adapter_id
    }

    #[must_use]
    pub const fn principal(&self) -> &AdapterPrincipal {
        &self.principal
    }

    #[must_use]
    pub const fn idempotency_key(&self) -> &IdempotencyKey {
        &self.idempotency_key
    }

    #[must_use]
    pub const fn body(&self) -> &GDriveStateCommitRequest {
        &self.body
    }
}

impl fmt::Debug for AuthenticatedGDriveStateCommitRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthenticatedGDriveStateCommitRequest")
            .field("adapter_id", &self.adapter_id)
            .field("principal", &self.principal)
            .field("idempotency_key", &self.idempotency_key)
            .field("body", &self.body)
            .finish()
    }
}

pub fn parse_authenticated_gdrive_state_commit_request(
    parts: GDriveStateCommitRouteParts<'_>,
    principal: Option<&AdapterPrincipal>,
) -> Result<AuthenticatedGDriveStateCommitRequest, GDriveStateRouteError> {
    let principal = principal
        .cloned()
        .ok_or(GDriveStateRouteError::Unauthorized)?;
    let adapter_id = AdapterId::parse(parts.adapter_id)
        .map_err(|_| GDriveStateRouteError::ValidationError)?;
    if principal.role() != AdapterRole::GdriveAdapter
        || principal.adapter_id() != adapter_id.as_str()
    {
        return Err(GDriveStateRouteError::Forbidden);
    }
    let idempotency_key = parts
        .idempotency_key
        .ok_or(GDriveStateRouteError::ValidationError)
        .and_then(|value| {
            IdempotencyKey::new(value.to_owned()).map_err(GDriveStateRouteError::InvalidHeader)
        })?;
    validate_commit_body(&parts.body)?;

    Ok(AuthenticatedGDriveStateCommitRequest {
        adapter_id,
        principal,
        idempotency_key,
        body: parts.body,
    })
}

pub fn validate_private_snapshot(
    snapshot: &GDriveStateSnapshotResponse,
) -> Result<(), GDriveStateRouteError> {
    AdapterId::try_from(&snapshot.adapter_id)
        .map_err(|_| GDriveStateRouteError::ValidationError)?;
    validate_storage_number(snapshot.state_version)?;
    validate_storage_number(snapshot.cursor.generation)?;
    validate_storage_number(snapshot.core_export_checkpoint)?;
    if snapshot.mappings.len() > MAX_GDRIVE_STATE_ITEMS {
        return Err(GDriveStateRouteError::CollectionTooLarge);
    }
    for mapping in &snapshot.mappings {
        validate_mapping(mapping)?;
    }
    if let Some(path) = snapshot.next_after_path.as_ref() {
        VaultPath::try_from(path).map_err(|_| GDriveStateRouteError::ValidationError)?;
    }
    for operation_id in [
        snapshot.last_operations.import.as_ref(),
        snapshot.last_operations.export.as_ref(),
        snapshot.last_operations.provider_mutation.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        OperationId::try_from(operation_id)
            .map_err(|_| GDriveStateRouteError::ValidationError)?;
    }
    Ok(())
}

pub fn sanitize_snapshot_for_admin(
    snapshot: &GDriveStateSnapshotResponse,
) -> Result<GDriveStateAdminSummaryResponse, GDriveStateRouteError> {
    validate_private_snapshot(snapshot)?;
    let mapping_count = u64::try_from(snapshot.mappings.len())
        .map_err(|_| GDriveStateRouteError::CountOutOfRange)?;
    let mut echo_none = 0_u64;
    let mut echo_pending = 0_u64;
    let mut echo_confirmed = 0_u64;
    let mut delete_total = 0_u64;
    let mut delete_blocked = 0_u64;
    for mapping in &snapshot.mappings {
        match mapping.echo.state {
            GDriveEchoStateDto::None => echo_none += 1,
            GDriveEchoStateDto::Pending => echo_pending += 1,
            GDriveEchoStateDto::Confirmed => echo_confirmed += 1,
        }
        if let Some(candidate) = mapping.delete_candidate.as_ref() {
            delete_total += 1;
            if candidate.blocked {
                delete_blocked += 1;
            }
        }
    }

    Ok(GDriveStateAdminSummaryResponse {
        adapter_id: AdapterIdDto::new(snapshot.adapter_id.as_str()),
        state_format_version: snapshot.state_format_version,
        state_version: snapshot.state_version,
        cursor: snapshot.cursor.clone(),
        core_export_checkpoint: snapshot.core_export_checkpoint,
        mapping_count,
        echo_counts: GDriveEchoCountSummaryDto {
            none: echo_none,
            pending: echo_pending,
            confirmed: echo_confirmed,
        },
        delete_candidates: GDriveDeleteCandidateCountSummaryDto {
            total: delete_total,
            blocked: delete_blocked,
        },
        last_operation_presence: GDriveLastOperationPresenceDto {
            import: snapshot.last_operations.import.is_some(),
            export: snapshot.last_operations.export.is_some(),
            provider_mutation: snapshot.last_operations.provider_mutation.is_some(),
        },
    })
}

fn validate_commit_body(body: &GDriveStateCommitRequest) -> Result<(), GDriveStateRouteError> {
    validate_storage_number(body.expected_state_version)?;
    validate_storage_number(body.cursor.expected_generation)?;
    if let Some(advance) = body.cursor.advance.as_ref() {
        validate_storage_number(advance.next_generation)?;
        if advance.next_generation <= body.cursor.expected_generation {
            return Err(GDriveStateRouteError::CursorRegression);
        }
        let expected_next = body
            .cursor
            .expected_generation
            .checked_add(1)
            .ok_or(GDriveStateRouteError::CursorGap)?;
        if advance.next_generation != expected_next {
            return Err(GDriveStateRouteError::CursorGap);
        }
    }
    if let Some(checkpoint) = body.core_export_checkpoint {
        validate_storage_number(checkpoint)?;
    }
    OperationId::try_from(&body.operation.operation_id)
        .map_err(|_| GDriveStateRouteError::ValidationError)?;
    if let Some(path) = body.operation.mapping_path.as_ref() {
        VaultPath::try_from(path).map_err(|_| GDriveStateRouteError::ValidationError)?;
    }
    if let Some(core_seq) = body.operation.core_seq {
        validate_storage_number(core_seq)?;
    }
    if let Some(mapping) = body.mapping.as_ref() {
        validate_mapping(mapping)?;
        if let Some(operation_path) = body.operation.mapping_path.as_ref() {
            if operation_path.as_str() != mapping.path.as_str() {
                return Err(GDriveStateRouteError::MappingConflict);
            }
        }
    }
    Ok(())
}

fn validate_mapping(mapping: &GDriveMappingFactsDto) -> Result<(), GDriveStateRouteError> {
    VaultPath::try_from(&mapping.path).map_err(|_| GDriveStateRouteError::ValidationError)?;
    validate_optional_text(mapping.drive_name.as_deref())?;
    validate_optional_text(mapping.mime_type.as_deref())?;
    validate_optional_atom(mapping.core_object_id.as_deref())?;
    if let Some(checksum) = mapping.md5_checksum.as_deref() {
        if checksum.len() != GDRIVE_MD5_HEX_BYTES
            || !checksum.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(GDriveStateRouteError::ValidationError);
        }
    }
    if let Some(revision_id) = mapping.core_revision_id.as_ref() {
        RevisionId::try_from(revision_id)
            .map_err(|_| GDriveStateRouteError::ValidationError)?;
    }
    if let Some(core_seq) = mapping.core_seq {
        validate_storage_number(core_seq)?;
    }
    validate_timestamp(mapping.drive_modified_time.as_ref())?;
    validate_timestamp(mapping.last_imported_at.as_ref())?;
    validate_timestamp(mapping.last_exported_at.as_ref())?;
    validate_timestamp(mapping.last_seen_at.as_ref())?;
    match mapping.echo.state {
        GDriveEchoStateDto::None => {
            if mapping.echo.operation_id.is_some() || mapping.echo.provider_version.is_some() {
                return Err(GDriveStateRouteError::ValidationError);
            }
        }
        GDriveEchoStateDto::Pending => {
            if mapping.echo.operation_id.is_none() || mapping.echo.provider_version.is_some() {
                return Err(GDriveStateRouteError::ValidationError);
            }
        }
        GDriveEchoStateDto::Confirmed => {
            if mapping.echo.operation_id.is_none() || mapping.echo.provider_version.is_none() {
                return Err(GDriveStateRouteError::ValidationError);
            }
        }
    }
    if let Some(operation_id) = mapping.echo.operation_id.as_ref() {
        OperationId::try_from(operation_id)
            .map_err(|_| GDriveStateRouteError::ValidationError)?;
    }
    if let Some(candidate) = mapping.delete_candidate.as_ref() {
        validate_storage_number(candidate.generation)?;
        validate_timestamp(Some(&candidate.first_seen_at))?;
        validate_timestamp(Some(&candidate.last_seen_at))?;
    }
    Ok(())
}

fn validate_storage_number(value: u64) -> Result<(), GDriveStateRouteError> {
    i64::try_from(value)
        .map(|_| ())
        .map_err(|_| GDriveStateRouteError::ValidationError)
}

fn validate_optional_text(value: Option<&str>) -> Result<(), GDriveStateRouteError> {
    let Some(value) = value else {
        return Ok(());
    };
    if value.is_empty()
        || value.len() > MAX_GDRIVE_TEXT_BYTES
        || value.chars().any(char::is_control)
    {
        return Err(GDriveStateRouteError::ValidationError);
    }
    Ok(())
}

fn validate_optional_atom(value: Option<&str>) -> Result<(), GDriveStateRouteError> {
    let Some(value) = value else {
        return Ok(());
    };
    if value.is_empty()
        || value.len() > MAX_GDRIVE_TEXT_BYTES
        || value
            .chars()
            .any(|character| character.is_control() || character.is_whitespace())
    {
        return Err(GDriveStateRouteError::ValidationError);
    }
    Ok(())
}

fn validate_timestamp(
    value: Option<&crate::dto::primitives::TimestampDto>,
) -> Result<(), GDriveStateRouteError> {
    let Some(value) = value else {
        return Ok(());
    };
    let value = value.as_str();
    if value.is_empty() || value.len() > 64 || value.chars().any(char::is_control) {
        return Err(GDriveStateRouteError::ValidationError);
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GDriveStateRouteError {
    Unauthorized,
    Forbidden,
    AdapterNotFound,
    StateVersionMismatch,
    InvalidCursorState,
    StaleState,
    CursorRegression,
    CursorGap,
    MappingConflict,
    IdempotencyConflict,
    ValidationError,
    Unavailable,
    Internal,
    CollectionTooLarge,
    CountOutOfRange,
    InvalidHeader(HeaderValueError),
}

impl GDriveStateRouteError {
    #[must_use]
    pub const fn public_code(&self) -> GDriveStateErrorCode {
        match self {
            Self::Unauthorized => GDriveStateErrorCode::Unauthorized,
            Self::Forbidden => GDriveStateErrorCode::Forbidden,
            Self::AdapterNotFound => GDriveStateErrorCode::AdapterNotFound,
            Self::StateVersionMismatch => GDriveStateErrorCode::StateVersionMismatch,
            Self::InvalidCursorState => GDriveStateErrorCode::InvalidCursorState,
            Self::StaleState => GDriveStateErrorCode::StaleState,
            Self::CursorRegression => GDriveStateErrorCode::CursorRegression,
            Self::CursorGap => GDriveStateErrorCode::CursorGap,
            Self::MappingConflict => GDriveStateErrorCode::MappingConflict,
            Self::IdempotencyConflict => GDriveStateErrorCode::IdempotencyConflict,
            Self::ValidationError
            | Self::CollectionTooLarge
            | Self::CountOutOfRange
            | Self::InvalidHeader(_) => GDriveStateErrorCode::ValidationError,
            Self::Unavailable => GDriveStateErrorCode::Unavailable,
            Self::Internal => GDriveStateErrorCode::Internal,
        }
    }

    #[must_use]
    pub const fn http_status_code(&self) -> u16 {
        match self {
            Self::Unauthorized => HTTP_STATUS_UNAUTHORIZED,
            Self::Forbidden => HTTP_STATUS_FORBIDDEN,
            Self::AdapterNotFound => HTTP_STATUS_NOT_FOUND,
            Self::StateVersionMismatch
            | Self::InvalidCursorState
            | Self::StaleState
            | Self::CursorRegression
            | Self::CursorGap
            | Self::MappingConflict
            | Self::IdempotencyConflict => HTTP_STATUS_CONFLICT,
            Self::ValidationError
            | Self::CollectionTooLarge
            | Self::CountOutOfRange
            | Self::InvalidHeader(_) => HTTP_STATUS_UNPROCESSABLE_ENTITY,
            Self::Unavailable => HTTP_STATUS_SERVICE_UNAVAILABLE,
            Self::Internal => HTTP_STATUS_INTERNAL_SERVER_ERROR,
        }
    }

    #[must_use]
    pub fn to_error_response(&self) -> GDriveStateErrorResponse {
        GDriveStateErrorResponse {
            error: GDriveStatePublicError {
                code: self.public_code(),
                message: self.safe_message().to_owned(),
            },
        }
    }

    const fn safe_message(&self) -> &'static str {
        match self {
            Self::Unauthorized => "Authentication is required",
            Self::Forbidden => "Principal is not allowed to access this adapter state",
            Self::AdapterNotFound => "Adapter state was not found",
            Self::StateVersionMismatch => "Adapter state version does not match",
            Self::InvalidCursorState => "Drive cursor state is invalid",
            Self::StaleState => "Adapter state is stale",
            Self::CursorRegression => "Drive cursor generation would regress",
            Self::CursorGap => "Drive cursor generation must advance exactly once",
            Self::MappingConflict => "Drive mapping facts conflict with durable state",
            Self::IdempotencyConflict => "Idempotency metadata conflicts with a prior operation",
            Self::ValidationError
            | Self::CollectionTooLarge
            | Self::CountOutOfRange
            | Self::InvalidHeader(_) => "Google Drive state request failed validation",
            Self::Unavailable => "Google Drive durable state is unavailable",
            Self::Internal => "Google Drive state request failed",
        }
    }
}

impl fmt::Display for GDriveStateRouteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.safe_message())
    }
}

impl Error for GDriveStateRouteError {}

#[cfg(test)]
mod tests {
    use crate::dto::{
        gdrive::{
            GDriveCursorCommitDto, GDriveFactsFingerprintDto, GDriveOperationFactsDto,
            GDriveOperationKindDto,
        },
        primitives::OperationIdDto,
    };

    use super::*;

    fn principal(adapter_id: &str, role: AdapterRole) -> AdapterPrincipal {
        AdapterPrincipal::new(adapter_id, role).unwrap()
    }

    fn minimal_commit() -> GDriveStateCommitRequest {
        GDriveStateCommitRequest {
            expected_state_version: 7,
            cursor: GDriveCursorCommitDto {
                expected_generation: 3,
                advance: None,
            },
            core_export_checkpoint: Some(19),
            mapping: None,
            operation: GDriveOperationFactsDto {
                operation_id: OperationIdDto::from("op_gdrive_fixture_01"),
                kind: GDriveOperationKindDto::CursorCheckpoint,
                facts_fingerprint: GDriveFactsFingerprintDto::parse("a".repeat(64)).unwrap(),
                mapping_path: None,
                core_seq: None,
                drive_version: None,
            },
        }
    }

    #[test]
    fn read_requires_matching_gdrive_adapter_or_admin() {
        let adapter = principal("gdrive-main", AdapterRole::GdriveAdapter);
        let request = parse_authenticated_get_gdrive_state_request(
            GetGDriveStateRouteParts {
                adapter_id: "gdrive-main",
                after_path: None,
                limit: None,
            },
            Some(&adapter),
        )
        .unwrap();
        assert_eq!(request.access(), GDriveStateReadAccess::AdapterPrivate);

        let admin = principal("admin-main", AdapterRole::Admin);
        let request = parse_authenticated_get_gdrive_state_request(
            GetGDriveStateRouteParts {
                adapter_id: "gdrive-main",
                after_path: None,
                limit: Some(10),
            },
            Some(&admin),
        )
        .unwrap();
        assert_eq!(request.access(), GDriveStateReadAccess::AdminSanitized);
    }

    #[test]
    fn commit_requires_matching_gdrive_adapter_and_idempotency_header() {
        let adapter = principal("gdrive-main", AdapterRole::GdriveAdapter);
        let request = parse_authenticated_gdrive_state_commit_request(
            GDriveStateCommitRouteParts {
                adapter_id: "gdrive-main",
                idempotency_key: Some("gdrive-commit-fixture-01"),
                body: minimal_commit(),
            },
            Some(&adapter),
        )
        .unwrap();
        assert_eq!(request.adapter_id().as_str(), "gdrive-main");
        assert_eq!(request.idempotency_key().as_str(), "gdrive-commit-fixture-01");

        assert_eq!(
            parse_authenticated_gdrive_state_commit_request(
                GDriveStateCommitRouteParts {
                    adapter_id: "gdrive-other",
                    idempotency_key: Some("gdrive-commit-fixture-01"),
                    body: minimal_commit(),
                },
                Some(&adapter),
            )
            .unwrap_err(),
            GDriveStateRouteError::Forbidden
        );
    }

    #[test]
    fn cursor_transition_must_be_exactly_contiguous() {
        let mut request = minimal_commit();
        request.cursor.advance = Some(crate::dto::gdrive::GDriveCursorAdvanceDto {
            next_generation: 5,
            cursor: crate::dto::gdrive::GDriveRawCursorDto::parse("cursor-fixture").unwrap(),
        });
        assert_eq!(
            validate_commit_body(&request).unwrap_err(),
            GDriveStateRouteError::CursorGap
        );
    }

    #[test]
    fn errors_are_stable_and_secret_safe() {
        let error = GDriveStateRouteError::IdempotencyConflict;
        assert_eq!(error.http_status_code(), HTTP_STATUS_CONFLICT);
        assert_eq!(error.public_code(), GDriveStateErrorCode::IdempotencyConflict);
        assert_eq!(
            serde_json::to_string(&error.to_error_response()).unwrap(),
            "{\"error\":{\"code\":\"idempotency_conflict\",\"message\":\"Idempotency metadata conflicts with a prior operation\"}}"
        );
    }
}
