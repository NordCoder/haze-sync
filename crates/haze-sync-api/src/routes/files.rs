//! Passive route-level helpers for `PUT /v1/files/{path}` and
//! `GET /v1/files/{path}`.
//!
//! These helpers validate route path, query, and header contract values and build
//! typed request/response metadata for future server-to-Core wiring. They do not
//! register Axum routes, authenticate tokens, persist content, call storage,
//! append operation-log entries, perform idempotency lookup, or apply conflict
//! policy.

use std::{collections::BTreeMap, fmt};

use haze_sync_common::{ConflictId, ContentHash, RevisionId, VaultPath};

use crate::{
    contracts::{
        errors::{ErrorResponse, PublicError, PublicErrorCode, SafeErrorDetails},
        headers::{
            BaseRevisionIdHeader, ContentSha256Header, IdempotencyKey,
            IDEMPOTENCY_KEY_HEADER, X_BASE_REVISION_ID_HEADER, X_CONTENT_SHA256_HEADER,
        },
    },
    dto::{
        common::ConflictPolicyDto,
        files::{
            FileDownloadMetadata, FileIgnoredReasonDto, FileQuery, FileRejectedReasonDto,
            PutFileRequestMetadata, PutFileResponse,
        },
        primitives::{ConflictIdDto, ContentSha256Dto, RevisionIdDto, VaultPathDto},
    },
};

/// Download response header name for selected revision metadata.
pub const X_REVISION_ID_HEADER: &str = "X-Revision-Id";

/// Download response header name for selected content size.
pub const X_SIZE_BYTES_HEADER: &str = "X-Size-Bytes";

/// Standard HTTP content type header name.
pub const CONTENT_TYPE_HEADER: &str = "Content-Type";

/// V1 file body content type.
pub const APPLICATION_OCTET_STREAM: &str = "application/octet-stream";

/// Caller-supplied parts for validating a future `PUT /v1/files/{path}` request.
pub struct PutFileRouteRequestParts<'a> {
    /// Raw route path capture. It may still be percent-encoded.
    pub route_path: &'a str,
    /// Raw `Idempotency-Key` header value.
    pub idempotency_key: Option<&'a str>,
    /// Raw `X-Content-SHA256` header value.
    pub content_sha256: Option<&'a str>,
    /// Raw `X-Base-Revision-Id` header value.
    pub base_revision_id: Option<&'a str>,
    /// Raw request body bytes supplied by the HTTP layer.
    pub body: Vec<u8>,
    /// Optional route-layer upload limit. When present, oversized payloads are
    /// rejected before future Core wiring receives the bytes.
    pub max_upload_bytes: Option<u64>,
}

/// Validated passive request object for future `PUT /v1/files/{path}` service
/// wiring.
#[derive(Clone, PartialEq, Eq)]
pub struct PutFileRouteRequest {
    path: VaultPath,
    idempotency_key: IdempotencyKey,
    base_revision_id: Option<RevisionId>,
    content_sha256: ContentHash,
    size_bytes: u64,
    body: Vec<u8>,
}

impl PutFileRouteRequest {
    /// Normalized vault path selected by the route capture.
    #[must_use]
    pub const fn path(&self) -> &VaultPath {
        &self.path
    }

    /// Validated idempotency key required for writes.
    #[must_use]
    pub const fn idempotency_key(&self) -> &IdempotencyKey {
        &self.idempotency_key
    }

    /// Explicit base revision, or `None` when the header was the literal
    /// `null` value.
    #[must_use]
    pub const fn base_revision_id(&self) -> Option<&RevisionId> {
        self.base_revision_id.as_ref()
    }

    /// Expected content hash from `X-Content-SHA256`.
    #[must_use]
    pub const fn content_sha256(&self) -> ContentHash {
        self.content_sha256
    }

    /// Raw request body bytes supplied by the caller.
    #[must_use]
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// Consumes the request and returns the caller-supplied raw body bytes.
    #[must_use]
    pub fn into_body(self) -> Vec<u8> {
        self.body
    }

    /// Number of raw body bytes supplied by the caller.
    #[must_use]
    pub const fn size_bytes(&self) -> u64 {
        self.size_bytes
    }

    /// Builds the existing JSON metadata DTO shape for future handler wiring.
    #[must_use]
    pub fn to_metadata_dto(&self) -> PutFileRequestMetadata {
        PutFileRequestMetadata {
            path: VaultPathDto::from(self.path.clone()),
            base_revision_id: self
                .base_revision_id
                .as_ref()
                .cloned()
                .map(RevisionIdDto::from),
            content_sha256: ContentSha256Dto::from(self.content_sha256),
            size_bytes: Some(self.size_bytes),
        }
    }
}

impl fmt::Debug for PutFileRouteRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PutFileRouteRequest")
            .field("path", &self.path)
            .field("idempotency_key", &"<validated>")
            .field("base_revision_id", &self.base_revision_id)
            .field("content_sha256", &self.content_sha256)
            .field("size_bytes", &self.size_bytes)
            .field("body", &"<raw-bytes-redacted>")
            .finish()
    }
}

/// Validate route path, required write headers, optional upload limit, and raw
/// body bytes for `PUT /v1/files/{path}`.
pub fn parse_put_file_request(
    parts: PutFileRouteRequestParts<'_>,
) -> Result<PutFileRouteRequest, FileRouteError> {
    let size_bytes = body_len_u64(parts.body.len());
    if let Some(max_upload_bytes) = parts.max_upload_bytes {
        if size_bytes > max_upload_bytes {
            return Err(FileRouteError::PayloadTooLarge { max_upload_bytes });
        }
    }

    let path = parse_vault_path(parts.route_path)?;
    let idempotency_key = parse_required_idempotency_key(parts.idempotency_key)?;
    let content_sha256 = parse_required_content_hash(parts.content_sha256)?;
    let base_revision_id = parse_required_base_revision(parts.base_revision_id)?;

    Ok(PutFileRouteRequest {
        path,
        idempotency_key,
        base_revision_id,
        content_sha256,
        size_bytes,
        body: parts.body,
    })
}

/// Caller-supplied parts for validating a future `GET /v1/files/{path}` request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GetFileRouteRequestParts<'a> {
    /// Raw route path capture. It may still be percent-encoded.
    pub route_path: &'a str,
    /// Optional raw `revision_id` query parameter.
    pub revision_id: Option<&'a str>,
}

/// Validated passive request object for future `GET /v1/files/{path}` service
/// wiring.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GetFileRouteRequest {
    path: VaultPath,
    revision_id: Option<RevisionId>,
}

impl GetFileRouteRequest {
    /// Normalized vault path selected by the route capture.
    #[must_use]
    pub const fn path(&self) -> &VaultPath {
        &self.path
    }

    /// Optional selected revision from query parameters.
    #[must_use]
    pub const fn revision_id(&self) -> Option<&RevisionId> {
        self.revision_id.as_ref()
    }

    /// Builds the existing JSON query DTO shape for future handler wiring.
    #[must_use]
    pub fn to_query_dto(&self) -> FileQuery {
        FileQuery {
            revision_id: self.revision_id.as_ref().cloned().map(RevisionIdDto::from),
        }
    }
}

/// Validate route path and optional `revision_id` query parameter for
/// `GET /v1/files/{path}`.
pub fn parse_get_file_request(
    parts: GetFileRouteRequestParts<'_>,
) -> Result<GetFileRouteRequest, FileRouteError> {
    let path = parse_vault_path(parts.route_path)?;
    let revision_id = parts.revision_id.map(parse_revision_query).transpose()?;

    Ok(GetFileRouteRequest { path, revision_id })
}

/// Safe deterministic download response metadata for `GET /v1/files/{path}`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileDownloadRouteHeaders {
    revision_id: RevisionId,
    content_sha256: ContentHash,
    size_bytes: u64,
}

impl FileDownloadRouteHeaders {
    /// Build validated download metadata from already-typed Core/service output.
    #[must_use]
    pub const fn new(
        revision_id: RevisionId,
        content_sha256: ContentHash,
        size_bytes: u64,
    ) -> Self {
        Self {
            revision_id,
            content_sha256,
            size_bytes,
        }
    }

    /// Selected revision sent as `X-Revision-Id`.
    #[must_use]
    pub const fn revision_id(&self) -> &RevisionId {
        &self.revision_id
    }

    /// Selected content hash sent as `X-Content-SHA256`.
    #[must_use]
    pub const fn content_sha256(&self) -> ContentHash {
        self.content_sha256
    }

    /// Selected content size sent as `X-Size-Bytes`.
    #[must_use]
    pub const fn size_bytes(&self) -> u64 {
        self.size_bytes
    }

    /// Content type sent for file byte downloads.
    #[must_use]
    pub const fn content_type(&self) -> &'static str {
        APPLICATION_OCTET_STREAM
    }

    /// Returns deterministic, safe header metadata. Values are selected metadata
    /// only; raw body bytes and caller headers are not included.
    #[must_use]
    pub fn to_header_map(&self) -> BTreeMap<&'static str, String> {
        BTreeMap::from([
            (CONTENT_TYPE_HEADER, APPLICATION_OCTET_STREAM.to_owned()),
            (X_CONTENT_SHA256_HEADER, self.content_sha256.to_string()),
            (X_REVISION_ID_HEADER, self.revision_id.to_string()),
            (X_SIZE_BYTES_HEADER, self.size_bytes.to_string()),
        ])
    }

    /// Builds the existing JSON metadata DTO shape for tests or future handler
    /// wiring that needs to serialize download metadata separately.
    #[must_use]
    pub fn to_metadata_dto(&self) -> FileDownloadMetadata {
        FileDownloadMetadata {
            revision_id: RevisionIdDto::from(self.revision_id.clone()),
            content_sha256: ContentSha256Dto::from(self.content_sha256),
            size_bytes: self.size_bytes,
            content_type: APPLICATION_OCTET_STREAM.to_owned(),
        }
    }
}

/// Safe route-level errors for future file handlers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileRouteError {
    /// Route path failed vault path validation.
    InvalidPath,
    /// A required header was absent.
    MissingRequiredHeader { header: &'static str },
    /// `Idempotency-Key` was present but malformed.
    InvalidIdempotencyKey,
    /// `X-Content-SHA256` was absent or malformed.
    InvalidContentSha256,
    /// `X-Base-Revision-Id` was present but was neither `null` nor a valid
    /// revision id.
    InvalidBaseRevision,
    /// Optional `revision_id` query parameter was malformed.
    InvalidRevisionQuery,
    /// Raw request body exceeded the configured upload limit.
    PayloadTooLarge { max_upload_bytes: u64 },
    /// Later service wiring could not find the requested file or revision.
    NotFound,
    /// Later service wiring reported a safe conflict.
    Conflict,
    /// Later idempotency wiring detected key reuse with a different request.
    IdempotencyMismatch,
}

impl FileRouteError {
    /// HTTP status code that future handlers can use for this public error.
    #[must_use]
    pub const fn http_status_code(&self) -> u16 {
        match self {
            Self::InvalidPath
            | Self::MissingRequiredHeader { .. }
            | Self::InvalidIdempotencyKey
            | Self::InvalidContentSha256
            | Self::InvalidBaseRevision
            | Self::InvalidRevisionQuery => 400,
            Self::NotFound => 404,
            Self::PayloadTooLarge { .. } => 413,
            Self::Conflict | Self::IdempotencyMismatch => 409,
        }
    }

    /// Stable public API error code.
    #[must_use]
    pub const fn public_code(&self) -> PublicErrorCode {
        match self {
            Self::InvalidPath => PublicErrorCode::InvalidPath,
            Self::MissingRequiredHeader { .. } => PublicErrorCode::InvalidRequest,
            Self::InvalidIdempotencyKey
            | Self::InvalidContentSha256
            | Self::InvalidBaseRevision
            | Self::InvalidRevisionQuery => PublicErrorCode::ValidationError,
            Self::PayloadTooLarge { .. } => PublicErrorCode::PayloadTooLarge,
            Self::NotFound => PublicErrorCode::NotFound,
            Self::Conflict => PublicErrorCode::Conflict,
            Self::IdempotencyMismatch => PublicErrorCode::IdempotencyConflict,
        }
    }

    /// Sanitized public error payload with no raw header values, body bytes,
    /// storage errors, local paths, provider payloads, stack traces, or secrets.
    #[must_use]
    pub fn to_public_error(&self) -> PublicError {
        let mut error = PublicError::new(self.public_code(), self.safe_message());
        if let Some(details) = self.safe_details() {
            error = error.with_details(details);
        }
        error
    }

    /// Sanitized public error response wrapper.
    #[must_use]
    pub fn to_error_response(&self) -> ErrorResponse {
        ErrorResponse {
            error: self.to_public_error(),
        }
    }

    fn safe_message(&self) -> &'static str {
        match self {
            Self::InvalidPath => "Invalid vault path",
            Self::MissingRequiredHeader { .. } => "Missing required header",
            Self::InvalidIdempotencyKey => "Invalid Idempotency-Key header",
            Self::InvalidContentSha256 => "Invalid X-Content-SHA256 header",
            Self::InvalidBaseRevision => "Invalid X-Base-Revision-Id header",
            Self::InvalidRevisionQuery => "Invalid revision_id query parameter",
            Self::PayloadTooLarge { .. } => "Payload too large",
            Self::NotFound => "File or revision not found",
            Self::Conflict => "Write conflicted with current state",
            Self::IdempotencyMismatch => "Idempotency key reused with a different request",
        }
    }

    fn safe_details(&self) -> Option<SafeErrorDetails> {
        match self {
            Self::InvalidPath => Some(detail("path", "failed validation")),
            Self::MissingRequiredHeader { header } => Some(detail("header", *header)),
            Self::InvalidIdempotencyKey => Some(detail("header", IDEMPOTENCY_KEY_HEADER)),
            Self::InvalidContentSha256 => Some(detail("header", X_CONTENT_SHA256_HEADER)),
            Self::InvalidBaseRevision => Some(detail("header", X_BASE_REVISION_ID_HEADER)),
            Self::InvalidRevisionQuery => Some(detail("query", "revision_id")),
            Self::PayloadTooLarge { max_upload_bytes } => {
                Some(detail("max_upload_bytes", max_upload_bytes.to_string()))
            }
            Self::NotFound | Self::Conflict | Self::IdempotencyMismatch => None,
        }
    }
}

impl fmt::Display for FileRouteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.safe_message())
    }
}

impl std::error::Error for FileRouteError {}

/// Builds an accepted upload DTO from future service-layer output.
#[must_use]
pub fn accepted_upload_response(
    path: VaultPath,
    revision_id: RevisionId,
    seq: i64,
) -> PutFileResponse {
    PutFileResponse::Accepted {
        path: VaultPathDto::from(path),
        revision_id: RevisionIdDto::from(revision_id),
        seq,
    }
}

/// Builds a same-content ignored upload DTO from future service-layer output.
#[must_use]
pub fn ignored_same_content_response(path: VaultPath) -> PutFileResponse {
    PutFileResponse::Ignored {
        reason: FileIgnoredReasonDto::SameContent,
        path: VaultPathDto::from(path),
    }
}

/// Builds a rejected upload DTO without applying any route-layer policy.
#[must_use]
pub fn rejected_upload_response(
    path: VaultPath,
    reason: FileRejectedReasonDto,
) -> PutFileResponse {
    PutFileResponse::Rejected {
        reason,
        path: VaultPathDto::from(path),
    }
}

/// Builds a conflict-saved upload DTO from future conflict-policy output. This
/// function is a shape mapper only and does not decide whether a conflict exists.
#[must_use]
pub fn conflict_saved_upload_response(
    path: VaultPath,
    conflict_id: ConflictId,
    materialized_path: VaultPath,
    policy_applied: ConflictPolicyDto,
    seq: i64,
) -> PutFileResponse {
    PutFileResponse::ConflictSaved {
        path: VaultPathDto::from(path),
        conflict_id: ConflictIdDto::from(conflict_id),
        materialized_path: VaultPathDto::from(materialized_path),
        policy_applied,
        seq,
    }
}

fn parse_vault_path(value: &str) -> Result<VaultPath, FileRouteError> {
    VaultPath::parse(value).map_err(|_| FileRouteError::InvalidPath)
}

fn parse_required_idempotency_key(
    value: Option<&str>,
) -> Result<IdempotencyKey, FileRouteError> {
    let value = value.ok_or(FileRouteError::MissingRequiredHeader {
        header: IDEMPOTENCY_KEY_HEADER,
    })?;
    IdempotencyKey::new(value).map_err(|_| FileRouteError::InvalidIdempotencyKey)
}

fn parse_required_content_hash(value: Option<&str>) -> Result<ContentHash, FileRouteError> {
    let value = value.ok_or(FileRouteError::MissingRequiredHeader {
        header: X_CONTENT_SHA256_HEADER,
    })?;
    let header =
        ContentSha256Header::parse(value).map_err(|_| FileRouteError::InvalidContentSha256)?;
    ContentHash::parse(header.as_str()).map_err(|_| FileRouteError::InvalidContentSha256)
}

fn parse_required_base_revision(
    value: Option<&str>,
) -> Result<Option<RevisionId>, FileRouteError> {
    let value = value.ok_or(FileRouteError::MissingRequiredHeader {
        header: X_BASE_REVISION_ID_HEADER,
    })?;
    match BaseRevisionIdHeader::parse(value).map_err(|_| FileRouteError::InvalidBaseRevision)? {
        BaseRevisionIdHeader::Null => Ok(None),
        BaseRevisionIdHeader::Revision(revision_id) => RevisionId::parse(&revision_id)
            .map(Some)
            .map_err(|_| FileRouteError::InvalidBaseRevision),
    }
}

fn parse_revision_query(value: &str) -> Result<RevisionId, FileRouteError> {
    RevisionId::parse(value).map_err(|_| FileRouteError::InvalidRevisionQuery)
}

fn body_len_u64(len: usize) -> u64 {
    u64::try_from(len).unwrap_or(u64::MAX)
}

fn detail(field: impl Into<String>, value: impl Into<String>) -> SafeErrorDetails {
    let mut fields = BTreeMap::new();
    fields.insert(field.into(), vec![value.into()]);
    SafeErrorDetails::Map(fields)
}
