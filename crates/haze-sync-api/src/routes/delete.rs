//! Passive route-level helpers for `DELETE /v1/files/{path}`.
//!
//! This module owns API-layer parsing, typed authorization metadata, delete-guard
//! request metadata, deterministic response builders, and sanitized error mapping
//! for the future DELETE file route. It deliberately does not register Axum
//! handlers, perform authentication, access storage, call Core tombstone/delete
//! services, create tombstones, append operation-log rows, or contact adapters.

use std::{collections::BTreeMap, error::Error, fmt};

use haze_sync_common::{RevisionId, VaultPath};
use serde::{Deserialize, Serialize};

use crate::{
    auth::{AdapterPrincipal, AdapterRole},
    contracts::{
        errors::{ErrorResponse, PublicError, PublicErrorCode, SafeErrorDetails},
        headers::{
            BaseRevisionIdHeader, IdempotencyKey, IDEMPOTENCY_KEY_HEADER, X_BASE_REVISION_ID_HEADER,
        },
    },
    dto::{
        files::{DeleteFileRequestMetadata, DeleteFileResponse, DeleteRejectedReasonDto},
        primitives::{RevisionIdDto, TimestampDto, TombstoneIdDto, VaultPathDto},
    },
};

/// Default single-path delete count used by `DELETE /v1/files/{path}`.
pub const DEFAULT_DELETE_REQUESTED_DELETE_COUNT: u32 = 1;

/// Roles that may request file deletes once server auth middleware is wired.
pub const DELETE_FILE_ALLOWED_ROLES: [AdapterRole; 4] = [
    AdapterRole::ObsidianPlugin,
    AdapterRole::GdriveAdapter,
    AdapterRole::WorktreeAdapter,
    AdapterRole::Admin,
];

/// HTTP status code for malformed DELETE route requests.
pub const HTTP_STATUS_BAD_REQUEST: u16 = 400;

/// HTTP status code for a missing authenticated adapter principal.
pub const HTTP_STATUS_UNAUTHORIZED: u16 = 401;

/// HTTP status code for forbidden authenticated roles.
pub const HTTP_STATUS_FORBIDDEN: u16 = 403;

/// HTTP status code for missing current files.
pub const HTTP_STATUS_NOT_FOUND: u16 = 404;

/// HTTP status code for safe delete conflicts and delete guard rejections.
pub const HTTP_STATUS_CONFLICT: u16 = 409;

/// Raw request-shaped DELETE route parts supplied by a future HTTP handler.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct DeleteFileRouteRequestParts<'a> {
    /// Raw `{path}` route capture before vault path normalization.
    pub route_path: &'a str,
    /// Raw `Idempotency-Key` header value.
    pub idempotency_key: Option<&'a str>,
    /// Raw `X-Base-Revision-Id` header value. The V1 contract requires this
    /// header on writes and uses the literal `null` for an unknown base.
    pub base_revision_id: Option<&'a str>,
    /// Caller-supplied delete count for this request/run. Missing defaults to 1.
    pub requested_delete_count: Option<u32>,
}

/// Parsed passive DELETE route request contract.
#[derive(Clone, PartialEq, Eq)]
pub struct DeleteFileRouteRequest {
    path: VaultPath,
    idempotency_key: IdempotencyKey,
    base_revision_id: Option<RevisionId>,
    auth_requirement: DeleteFileAuthRequirement,
    delete_guard: DeleteGuardRequestMetadata,
}

impl DeleteFileRouteRequest {
    /// Normalized vault path targeted by the delete request.
    #[must_use]
    pub const fn path(&self) -> &VaultPath {
        &self.path
    }

    /// Validated idempotency key. This value must never be serialized or logged.
    #[must_use]
    pub const fn idempotency_key(&self) -> &IdempotencyKey {
        &self.idempotency_key
    }

    /// Parsed base revision, or `None` for explicit `X-Base-Revision-Id: null`.
    #[must_use]
    pub const fn base_revision_id(&self) -> Option<&RevisionId> {
        self.base_revision_id.as_ref()
    }

    /// Typed role requirement for future server auth wiring.
    #[must_use]
    pub const fn auth_requirement(&self) -> DeleteFileAuthRequirement {
        self.auth_requirement
    }

    /// Delete guard metadata supplied to future service-layer delete checks.
    #[must_use]
    pub const fn delete_guard(&self) -> DeleteGuardRequestMetadata {
        self.delete_guard
    }

    /// Convert the public request metadata into the existing DELETE DTO shape.
    #[must_use]
    pub fn to_metadata_dto(&self) -> DeleteFileRequestMetadata {
        DeleteFileRequestMetadata {
            path: VaultPathDto::from(self.path.clone()),
            base_revision_id: self
                .base_revision_id
                .as_ref()
                .cloned()
                .map(RevisionIdDto::from),
        }
    }
}

impl fmt::Debug for DeleteFileRouteRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DeleteFileRouteRequest")
            .field("path", &self.path)
            .field("idempotency_key", &"<validated>")
            .field("base_revision_id", &self.base_revision_id)
            .field("auth_requirement", &self.auth_requirement)
            .field("delete_guard", &self.delete_guard)
            .finish()
    }
}

/// DELETE request paired with a principal already verified by server middleware.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthenticatedDeleteFileRouteRequest {
    request: DeleteFileRouteRequest,
    adapter_principal: AdapterPrincipal,
}

impl AuthenticatedDeleteFileRouteRequest {
    /// Parsed delete request metadata.
    #[must_use]
    pub const fn request(&self) -> &DeleteFileRouteRequest {
        &self.request
    }

    /// Verified adapter principal supplied by the runtime layer.
    #[must_use]
    pub const fn adapter_principal(&self) -> &AdapterPrincipal {
        &self.adapter_principal
    }

    /// Consume the authenticated wrapper and return the passive request metadata.
    #[must_use]
    pub fn into_request(self) -> DeleteFileRouteRequest {
        self.request
    }
}

/// Role metadata for the future DELETE handler. No authentication happens here.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DeleteFileAuthRequirement;

impl DeleteFileAuthRequirement {
    /// Roles allowed to request file deletion under the V1 security contract.
    #[must_use]
    pub const fn allowed_roles(self) -> &'static [AdapterRole; 4] {
        &DELETE_FILE_ALLOWED_ROLES
    }

    /// Validate an already authenticated role without performing token checks.
    pub fn validate_role(self, role: AdapterRole) -> Result<(), DeleteRouteError> {
        if DELETE_FILE_ALLOWED_ROLES.contains(&role) {
            Ok(())
        } else {
            Err(DeleteRouteError::ForbiddenRole)
        }
    }

    /// Validate the role carried by an already verified adapter principal.
    pub fn validate_principal(
        self,
        principal: &AdapterPrincipal,
    ) -> Result<(), DeleteRouteError> {
        self.validate_role(principal.role())
    }
}

/// Passive metadata for future delete-guard evaluation.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct DeleteGuardRequestMetadata {
    /// Number of delete candidates represented by this request/run.
    pub requested_delete_count: u32,
}

impl DeleteGuardRequestMetadata {
    /// Creates delete-guard request metadata. The count must be positive so a
    /// missing or malformed future bulk context cannot silently bypass the guard.
    pub fn new(requested_delete_count: u32) -> Result<Self, DeleteRouteError> {
        if requested_delete_count == 0 {
            return Err(DeleteRouteError::InvalidDeleteGuardMetadata);
        }

        Ok(Self {
            requested_delete_count,
        })
    }
}

impl Default for DeleteGuardRequestMetadata {
    fn default() -> Self {
        Self {
            requested_delete_count: DEFAULT_DELETE_REQUESTED_DELETE_COUNT,
        }
    }
}

/// Parse DELETE route inputs into safe typed request metadata.
pub fn parse_delete_file_request(
    parts: DeleteFileRouteRequestParts<'_>,
) -> Result<DeleteFileRouteRequest, DeleteRouteError> {
    let path = parse_vault_path(parts.route_path)?;
    let idempotency_key = parse_required_idempotency_key(parts.idempotency_key)?;
    let base_revision_id = parse_required_base_revision(parts.base_revision_id)?;
    let delete_guard = DeleteGuardRequestMetadata::new(
        parts
            .requested_delete_count
            .unwrap_or(DEFAULT_DELETE_REQUESTED_DELETE_COUNT),
    )?;

    Ok(DeleteFileRouteRequest {
        path,
        idempotency_key,
        base_revision_id,
        auth_requirement: DeleteFileAuthRequirement,
        delete_guard,
    })
}

/// Parse DELETE metadata and require an already verified adapter principal.
///
/// This helper does not parse bearer tokens or perform runtime authentication.
pub fn parse_authenticated_delete_file_request(
    parts: DeleteFileRouteRequestParts<'_>,
    adapter_principal: Option<&AdapterPrincipal>,
) -> Result<AuthenticatedDeleteFileRouteRequest, DeleteRouteError> {
    let adapter_principal = adapter_principal
        .cloned()
        .ok_or(DeleteRouteError::MissingAdapterPrincipal)?;
    let request = parse_delete_file_request(parts)?;
    request
        .auth_requirement()
        .validate_principal(&adapter_principal)?;

    Ok(AuthenticatedDeleteFileRouteRequest {
        request,
        adapter_principal,
    })
}

/// Sanitized error category for DELETE route contract helpers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeleteRouteError {
    /// Route path failed vault path validation.
    InvalidPath,
    /// A verified adapter principal was not supplied by runtime auth wiring.
    MissingAdapterPrincipal,
    /// A required contract header was absent.
    MissingRequiredHeader { header: &'static str },
    /// `Idempotency-Key` was empty, unsafe, or too long.
    InvalidIdempotencyKey,
    /// `X-Base-Revision-Id` was absent, malformed, or not a revision/null value.
    InvalidBaseRevision,
    /// Delete guard metadata from a future caller was malformed.
    InvalidDeleteGuardMetadata,
    /// Authenticated role is not allowed to request file deletion.
    ForbiddenRole,
    /// Future service/storage layer found no current file to delete.
    NotFound,
    /// Future service layer rejected a stale or conflicting base revision.
    StaleBaseConflict,
    /// Future delete guard blocked a potentially unsafe delete.
    DeleteGuardBlocked,
    /// Same idempotency key was reused for a different request fingerprint.
    IdempotencyMismatch,
}

impl DeleteRouteError {
    /// HTTP status code intended for future DELETE server handlers.
    #[must_use]
    pub const fn http_status_code(&self) -> u16 {
        match self {
            Self::InvalidPath
            | Self::MissingRequiredHeader { .. }
            | Self::InvalidIdempotencyKey
            | Self::InvalidBaseRevision
            | Self::InvalidDeleteGuardMetadata => HTTP_STATUS_BAD_REQUEST,
            Self::MissingAdapterPrincipal => HTTP_STATUS_UNAUTHORIZED,
            Self::ForbiddenRole => HTTP_STATUS_FORBIDDEN,
            Self::NotFound => HTTP_STATUS_NOT_FOUND,
            Self::StaleBaseConflict | Self::DeleteGuardBlocked | Self::IdempotencyMismatch => {
                HTTP_STATUS_CONFLICT
            }
        }
    }

    /// Stable safe public error code.
    #[must_use]
    pub const fn public_code(&self) -> PublicErrorCode {
        match self {
            Self::InvalidPath => PublicErrorCode::InvalidPath,
            Self::MissingAdapterPrincipal => PublicErrorCode::MissingToken,
            Self::MissingRequiredHeader { .. } => PublicErrorCode::InvalidRequest,
            Self::InvalidIdempotencyKey
            | Self::InvalidBaseRevision
            | Self::InvalidDeleteGuardMetadata => PublicErrorCode::ValidationError,
            Self::ForbiddenRole => PublicErrorCode::ForbiddenRole,
            Self::NotFound => PublicErrorCode::NotFound,
            Self::StaleBaseConflict => PublicErrorCode::Conflict,
            Self::DeleteGuardBlocked => PublicErrorCode::UnsafeDelete,
            Self::IdempotencyMismatch => PublicErrorCode::IdempotencyConflict,
        }
    }

    /// Convert the error into the sanitized public API error contract.
    #[must_use]
    pub fn to_public_error(&self) -> PublicError {
        let mut error = PublicError::new(self.public_code(), self.safe_message());
        if let Some(details) = self.safe_details() {
            error = error.with_details(details);
        }
        error
    }

    /// Convert the error into the sanitized JSON error response contract.
    #[must_use]
    pub fn to_error_response(&self) -> ErrorResponse {
        ErrorResponse {
            error: self.to_public_error(),
        }
    }

    fn safe_message(&self) -> &'static str {
        match self {
            Self::InvalidPath => "Invalid vault path",
            Self::MissingAdapterPrincipal => "Missing authenticated adapter principal",
            Self::MissingRequiredHeader { .. } => "Missing required header",
            Self::InvalidIdempotencyKey => "Invalid Idempotency-Key header",
            Self::InvalidBaseRevision => "Invalid X-Base-Revision-Id header",
            Self::InvalidDeleteGuardMetadata => "Invalid delete guard metadata",
            Self::ForbiddenRole => "Adapter role is not allowed to delete files",
            Self::NotFound => "File not found",
            Self::StaleBaseConflict => "Delete conflicted with current state",
            Self::DeleteGuardBlocked => "Delete rejected by safety guard",
            Self::IdempotencyMismatch => "Idempotency key reused with a different delete request",
        }
    }

    fn safe_details(&self) -> Option<SafeErrorDetails> {
        match self {
            Self::InvalidPath => Some(detail("path", "failed validation")),
            Self::MissingAdapterPrincipal => Some(detail("auth", "adapter principal required")),
            Self::MissingRequiredHeader { header } => Some(detail("header", *header)),
            Self::InvalidIdempotencyKey => Some(detail("header", IDEMPOTENCY_KEY_HEADER)),
            Self::InvalidBaseRevision => Some(detail("header", X_BASE_REVISION_ID_HEADER)),
            Self::InvalidDeleteGuardMetadata => {
                Some(detail("requested_delete_count", "must be positive"))
            }
            Self::ForbiddenRole => Some(detail("role", "not allowed for file delete")),
            Self::NotFound
            | Self::StaleBaseConflict
            | Self::DeleteGuardBlocked
            | Self::IdempotencyMismatch => None,
        }
    }
}

impl fmt::Display for DeleteRouteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.safe_message())
    }
}

impl Error for DeleteRouteError {}

/// Build a deterministic tombstoned DELETE response from future service output.
#[must_use]
pub fn tombstoned_delete_response(
    path: VaultPath,
    tombstone_id: impl Into<String>,
    seq: i64,
    retention_until: impl Into<String>,
) -> DeleteFileResponse {
    DeleteFileResponse::Tombstoned {
        path: VaultPathDto::from(path),
        tombstone_id: TombstoneIdDto::from(tombstone_id.into()),
        seq,
        retention_until: TimestampDto::from(retention_until.into()),
    }
}

/// Build a deterministic DELETE not-found placeholder response.
#[must_use]
pub fn not_found_delete_response(path: VaultPath) -> DeleteFileResponse {
    DeleteFileResponse::NotFound {
        path: VaultPathDto::from(path),
    }
}

/// Build a deterministic public rejected DELETE response.
#[must_use]
pub fn rejected_delete_response(
    path: VaultPath,
    reason: DeleteRejectedReasonDto,
) -> DeleteFileResponse {
    DeleteFileResponse::Rejected {
        reason,
        path: VaultPathDto::from(path),
    }
}

/// Build a stale-base rejection response for future Core delete semantics.
#[must_use]
pub fn stale_base_delete_response(path: VaultPath) -> DeleteFileResponse {
    rejected_delete_response(path, DeleteRejectedReasonDto::StaleBaseRevision)
}

/// Build a delete-guard rejection response for future Core delete semantics.
#[must_use]
pub fn unsafe_delete_response(path: VaultPath) -> DeleteFileResponse {
    rejected_delete_response(path, DeleteRejectedReasonDto::UnsafeDelete)
}

fn parse_vault_path(value: &str) -> Result<VaultPath, DeleteRouteError> {
    VaultPath::parse(value).map_err(|_| DeleteRouteError::InvalidPath)
}

fn parse_required_idempotency_key(value: Option<&str>) -> Result<IdempotencyKey, DeleteRouteError> {
    let value = value.ok_or(DeleteRouteError::MissingRequiredHeader {
        header: IDEMPOTENCY_KEY_HEADER,
    })?;
    IdempotencyKey::new(value).map_err(|_| DeleteRouteError::InvalidIdempotencyKey)
}

fn parse_required_base_revision(
    value: Option<&str>,
) -> Result<Option<RevisionId>, DeleteRouteError> {
    let value = value.ok_or(DeleteRouteError::MissingRequiredHeader {
        header: X_BASE_REVISION_ID_HEADER,
    })?;
    match BaseRevisionIdHeader::parse(value).map_err(|_| DeleteRouteError::InvalidBaseRevision)? {
        BaseRevisionIdHeader::Null => Ok(None),
        BaseRevisionIdHeader::Revision(revision_id) => RevisionId::parse(&revision_id)
            .map(Some)
            .map_err(|_| DeleteRouteError::InvalidBaseRevision),
    }
}

fn detail(field: impl Into<String>, value: impl Into<String>) -> SafeErrorDetails {
    let mut fields = BTreeMap::new();
    fields.insert(field.into(), vec![value.into()]);
    SafeErrorDetails::Map(fields)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_parts() -> DeleteFileRouteRequestParts<'static> {
        DeleteFileRouteRequestParts {
            route_path: "Projects/Haze/old.md",
            idempotency_key: Some("iphone-anna:delete-001"),
            base_revision_id: Some("rev_123"),
            requested_delete_count: None,
        }
    }

    fn principal() -> AdapterPrincipal {
        AdapterPrincipal::new("obsidian-plugin", AdapterRole::ObsidianPlugin).unwrap()
    }

    fn path(value: &str) -> VaultPath {
        VaultPath::parse(value).unwrap()
    }

    #[test]
    fn valid_delete_route_request_parses() {
        let request = parse_delete_file_request(valid_parts()).expect("request should parse");

        assert_eq!(request.path().as_str(), "Projects/Haze/old.md");
        assert_eq!(request.idempotency_key().as_str(), "iphone-anna:delete-001");
        assert_eq!(
            request.base_revision_id().map(RevisionId::as_str),
            Some("rev_123")
        );
        assert_eq!(
            request.auth_requirement().allowed_roles(),
            &DELETE_FILE_ALLOWED_ROLES
        );
        assert_eq!(request.delete_guard().requested_delete_count, 1);
        assert_eq!(
            request.to_metadata_dto(),
            DeleteFileRequestMetadata {
                path: VaultPathDto::from("Projects/Haze/old.md"),
                base_revision_id: Some(RevisionIdDto::from("rev_123")),
            }
        );
        assert!(!format!("{request:?}").contains("iphone-anna:delete-001"));
    }

    #[test]
    fn authenticated_delete_requires_verified_adapter_principal() {
        let error = parse_authenticated_delete_file_request(valid_parts(), None).unwrap_err();
        assert_eq!(error, DeleteRouteError::MissingAdapterPrincipal);
        assert_eq!(error.http_status_code(), HTTP_STATUS_UNAUTHORIZED);
        assert_eq!(error.public_code(), PublicErrorCode::MissingToken);

        let json = serde_json::to_string(&error.to_error_response()).unwrap();
        assert!(json.contains("missing_token"));
        assert!(!json.contains("delete-001"));
        assert!(!json.contains("Bearer"));

        let principal = principal();
        let authenticated =
            parse_authenticated_delete_file_request(valid_parts(), Some(&principal)).unwrap();
        assert_eq!(authenticated.adapter_principal(), &principal);
        assert_eq!(
            authenticated.request().path().as_str(),
            "Projects/Haze/old.md"
        );
        assert_eq!(
            authenticated.request().base_revision_id().map(RevisionId::as_str),
            Some("rev_123")
        );
    }

    #[test]
    fn missing_idempotency_key_is_rejected_safely() {
        let error = parse_delete_file_request(DeleteFileRouteRequestParts {
            idempotency_key: None,
            ..valid_parts()
        })
        .expect_err("missing idempotency key should fail");

        assert_eq!(error.http_status_code(), HTTP_STATUS_BAD_REQUEST);
        assert_eq!(error.public_code(), PublicErrorCode::InvalidRequest);
        let json = serde_json::to_string(&error.to_error_response()).unwrap();
        assert!(json.contains(IDEMPOTENCY_KEY_HEADER));
        assert!(!json.contains("delete-001"));
    }

    #[test]
    fn invalid_path_is_rejected_safely() {
        let error = parse_delete_file_request(DeleteFileRouteRequestParts {
            route_path: "../secrets.md",
            ..valid_parts()
        })
        .expect_err("invalid path should fail");

        assert_eq!(error.http_status_code(), HTTP_STATUS_BAD_REQUEST);
        assert_eq!(error.public_code(), PublicErrorCode::InvalidPath);
        let json = serde_json::to_string(&error.to_error_response()).unwrap();
        assert!(!json.contains("../secrets.md"));
    }

    #[test]
    fn base_revision_parses_known_and_explicit_null() {
        let known = parse_delete_file_request(valid_parts()).unwrap();
        assert_eq!(
            known.base_revision_id().map(RevisionId::as_str),
            Some("rev_123")
        );

        let explicit_null = parse_delete_file_request(DeleteFileRouteRequestParts {
            base_revision_id: Some("null"),
            ..valid_parts()
        })
        .unwrap();
        assert_eq!(explicit_null.base_revision_id(), None);
        assert_eq!(explicit_null.to_metadata_dto().base_revision_id, None);
    }

    #[test]
    fn invalid_base_revision_is_rejected_safely() {
        let error = parse_delete_file_request(DeleteFileRouteRequestParts {
            base_revision_id: Some("123"),
            ..valid_parts()
        })
        .expect_err("invalid base revision should fail");

        assert_eq!(error.http_status_code(), HTTP_STATUS_BAD_REQUEST);
        assert_eq!(error.public_code(), PublicErrorCode::ValidationError);
        let json = serde_json::to_string(&error.to_error_response()).unwrap();
        assert!(json.contains(X_BASE_REVISION_ID_HEADER));
        assert!(!json.contains("123"));
    }

    #[test]
    fn delete_guard_metadata_rejects_zero_and_preserves_positive_counts() {
        assert_eq!(
            DeleteGuardRequestMetadata::new(0),
            Err(DeleteRouteError::InvalidDeleteGuardMetadata)
        );
        assert_eq!(
            DeleteGuardRequestMetadata::new(25).unwrap(),
            DeleteGuardRequestMetadata {
                requested_delete_count: 25
            }
        );
    }

    #[test]
    fn delete_response_vocabulary_is_deterministic_and_passive() {
        let tombstoned = tombstoned_delete_response(
            path("Projects/Haze/old.md"),
            "tmb_01JDELETE",
            12_382,
            "2026-08-01T00:00:00Z",
        );
        let not_found = not_found_delete_response(path("Projects/Haze/missing.md"));
        let stale = stale_base_delete_response(path("Projects/Haze/old.md"));
        let guard_blocked = unsafe_delete_response(path("Projects/Haze/old.md"));

        assert_eq!(
            serde_json::to_string(&tombstoned).unwrap(),
            "{\"status\":\"tombstoned\",\"path\":\"Projects/Haze/old.md\",\"tombstone_id\":\"tmb_01JDELETE\",\"seq\":12382,\"retention_until\":\"2026-08-01T00:00:00Z\"}"
        );
        assert!(serde_json::to_string(&not_found)
            .unwrap()
            .contains("\"status\":\"not_found\""));
        assert!(serde_json::to_string(&stale)
            .unwrap()
            .contains("stale_base_revision"));
        assert!(serde_json::to_string(&guard_blocked)
            .unwrap()
            .contains("unsafe_delete"));
    }

    #[test]
    fn delete_guard_blocked_error_maps_safely() {
        let error = DeleteRouteError::DeleteGuardBlocked;

        assert_eq!(error.http_status_code(), HTTP_STATUS_CONFLICT);
        assert_eq!(error.public_code(), PublicErrorCode::UnsafeDelete);
        let json = serde_json::to_string(&error.to_error_response()).unwrap();
        assert!(json.contains("unsafe_delete"));
        assert!(!json.contains("Idempotency-Key"));
        assert!(!json.contains("stack"));
        assert!(!json.contains("token"));
    }

    #[test]
    fn not_found_maps_safely() {
        let error = DeleteRouteError::NotFound;

        assert_eq!(error.http_status_code(), HTTP_STATUS_NOT_FOUND);
        assert_eq!(error.public_code(), PublicErrorCode::NotFound);
        assert_eq!(
            not_found_delete_response(path("Projects/Haze/missing.md")),
            DeleteFileResponse::NotFound {
                path: VaultPathDto::from("Projects/Haze/missing.md"),
            }
        );
    }
}
