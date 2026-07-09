//! Passive route-level helpers for PUT and GET file route contracts.

use std::{collections::BTreeMap, fmt};

use haze_sync_common::{ConflictId, ContentHash, RevisionId, VaultPath};

use crate::{
    auth::AdapterPrincipal,
    contracts::{
        errors::{ErrorResponse, PublicError, PublicErrorCode, SafeErrorDetails},
        headers::{
            BaseRevisionIdHeader, ContentSha256Header, IdempotencyKey, IDEMPOTENCY_KEY_HEADER,
            X_BASE_REVISION_ID_HEADER, X_CONTENT_SHA256_HEADER,
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

pub const X_REVISION_ID_HEADER: &str = "X-Revision-Id";
pub const X_SIZE_BYTES_HEADER: &str = "X-Size-Bytes";
pub const CONTENT_TYPE_HEADER: &str = "Content-Type";
pub const APPLICATION_OCTET_STREAM: &str = "application/octet-stream";

pub struct PutFileRouteRequestParts<'a> {
    pub route_path: &'a str,
    pub adapter_principal: Option<&'a AdapterPrincipal>,
    pub idempotency_key: Option<&'a str>,
    pub content_sha256: Option<&'a str>,
    pub base_revision_id: Option<&'a str>,
    pub body: Vec<u8>,
    pub max_upload_bytes: Option<u64>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct PutFileRouteRequest {
    path: VaultPath,
    adapter_principal: AdapterPrincipal,
    idempotency_key: IdempotencyKey,
    base_revision_id: Option<RevisionId>,
    content_sha256: ContentHash,
    size_bytes: u64,
    body: Vec<u8>,
}

impl PutFileRouteRequest {
    #[must_use]
    pub const fn path(&self) -> &VaultPath {
        &self.path
    }

    #[must_use]
    pub fn adapter_principal(&self) -> &AdapterPrincipal {
        &self.adapter_principal
    }

    #[must_use]
    pub const fn idempotency_key(&self) -> &IdempotencyKey {
        &self.idempotency_key
    }

    #[must_use]
    pub const fn base_revision_id(&self) -> Option<&RevisionId> {
        self.base_revision_id.as_ref()
    }

    #[must_use]
    pub const fn content_sha256(&self) -> ContentHash {
        self.content_sha256
    }

    #[must_use]
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    #[must_use]
    pub fn into_body(self) -> Vec<u8> {
        self.body
    }

    #[must_use]
    pub const fn size_bytes(&self) -> u64 {
        self.size_bytes
    }

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
            .field("adapter_principal", &self.adapter_principal)
            .field("idempotency_key", &"<validated>")
            .field("base_revision_id", &self.base_revision_id)
            .field("content_sha256", &self.content_sha256)
            .field("size_bytes", &self.size_bytes)
            .field("body", &"<raw-bytes-redacted>")
            .finish()
    }
}

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
    let adapter_principal = parse_required_adapter_principal(parts.adapter_principal)?;
    let idempotency_key = parse_required_idempotency_key(parts.idempotency_key)?;
    let content_sha256 = parse_required_content_hash(parts.content_sha256)?;
    let base_revision_id = parse_required_base_revision(parts.base_revision_id)?;

    Ok(PutFileRouteRequest {
        path,
        adapter_principal,
        idempotency_key,
        base_revision_id,
        content_sha256,
        size_bytes,
        body: parts.body,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GetFileRouteRequestParts<'a> {
    pub route_path: &'a str,
    pub revision_id: Option<&'a str>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GetFileRouteRequest {
    path: VaultPath,
    revision_id: Option<RevisionId>,
}

impl GetFileRouteRequest {
    #[must_use]
    pub const fn path(&self) -> &VaultPath {
        &self.path
    }

    #[must_use]
    pub const fn revision_id(&self) -> Option<&RevisionId> {
        self.revision_id.as_ref()
    }

    #[must_use]
    pub fn to_query_dto(&self) -> FileQuery {
        FileQuery {
            revision_id: self.revision_id.as_ref().cloned().map(RevisionIdDto::from),
        }
    }
}

pub fn parse_get_file_request(
    parts: GetFileRouteRequestParts<'_>,
) -> Result<GetFileRouteRequest, FileRouteError> {
    let path = parse_vault_path(parts.route_path)?;
    let revision_id = parts.revision_id.map(parse_revision_query).transpose()?;

    Ok(GetFileRouteRequest { path, revision_id })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileDownloadRouteHeaders {
    revision_id: RevisionId,
    content_sha256: ContentHash,
    size_bytes: u64,
}

impl FileDownloadRouteHeaders {
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

    #[must_use]
    pub const fn revision_id(&self) -> &RevisionId {
        &self.revision_id
    }

    #[must_use]
    pub const fn content_sha256(&self) -> ContentHash {
        self.content_sha256
    }

    #[must_use]
    pub const fn size_bytes(&self) -> u64 {
        self.size_bytes
    }

    #[must_use]
    pub const fn content_type(&self) -> &'static str {
        APPLICATION_OCTET_STREAM
    }

    #[must_use]
    pub fn to_header_map(&self) -> BTreeMap<&'static str, String> {
        BTreeMap::from([
            (CONTENT_TYPE_HEADER, APPLICATION_OCTET_STREAM.to_owned()),
            (X_CONTENT_SHA256_HEADER, self.content_sha256.to_string()),
            (X_REVISION_ID_HEADER, self.revision_id.to_string()),
            (X_SIZE_BYTES_HEADER, self.size_bytes.to_string()),
        ])
    }

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileRouteError {
    InvalidPath,
    MissingAdapterPrincipal,
    MissingRequiredHeader { header: &'static str },
    InvalidIdempotencyKey,
    InvalidContentSha256,
    InvalidBaseRevision,
    InvalidRevisionQuery,
    PayloadTooLarge { max_upload_bytes: u64 },
    NotFound,
    Conflict,
    IdempotencyMismatch,
}

impl FileRouteError {
    #[must_use]
    pub const fn http_status_code(&self) -> u16 {
        match self {
            Self::InvalidPath
            | Self::MissingRequiredHeader { .. }
            | Self::InvalidIdempotencyKey
            | Self::InvalidContentSha256
            | Self::InvalidBaseRevision
            | Self::InvalidRevisionQuery => 400,
            Self::MissingAdapterPrincipal => 401,
            Self::NotFound => 404,
            Self::PayloadTooLarge { .. } => 413,
            Self::Conflict | Self::IdempotencyMismatch => 409,
        }
    }

    #[must_use]
    pub const fn public_code(&self) -> PublicErrorCode {
        match self {
            Self::InvalidPath => PublicErrorCode::InvalidPath,
            Self::MissingAdapterPrincipal => PublicErrorCode::MissingToken,
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

    #[must_use]
    pub fn to_public_error(&self) -> PublicError {
        let mut error = PublicError::new(self.public_code(), self.safe_message());
        if let Some(details) = self.safe_details() {
            error = error.with_details(details);
        }
        error
    }

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
            Self::MissingAdapterPrincipal => Some(detail("auth", "adapter principal required")),
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

#[must_use]
pub fn ignored_same_content_response(path: VaultPath) -> PutFileResponse {
    PutFileResponse::Ignored {
        reason: FileIgnoredReasonDto::SameContent,
        path: VaultPathDto::from(path),
    }
}

#[must_use]
pub fn rejected_upload_response(path: VaultPath, reason: FileRejectedReasonDto) -> PutFileResponse {
    PutFileResponse::Rejected {
        reason,
        path: VaultPathDto::from(path),
    }
}

#[must_use]
pub fn hash_mismatch_upload_response(path: VaultPath) -> PutFileResponse {
    rejected_upload_response(path, FileRejectedReasonDto::HashMismatch)
}

#[must_use]
pub fn stale_base_upload_response(path: VaultPath) -> PutFileResponse {
    rejected_upload_response(path, FileRejectedReasonDto::StaleBaseRevision)
}

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

fn parse_required_adapter_principal(
    value: Option<&AdapterPrincipal>,
) -> Result<AdapterPrincipal, FileRouteError> {
    value.cloned().ok_or(FileRouteError::MissingAdapterPrincipal)
}

fn parse_required_idempotency_key(value: Option<&str>) -> Result<IdempotencyKey, FileRouteError> {
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
    header
        .to_common_hash()
        .map_err(|_| FileRouteError::InvalidContentSha256)
}

fn parse_required_base_revision(value: Option<&str>) -> Result<Option<RevisionId>, FileRouteError> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{AdapterRole, AdapterPrincipal};

    fn principal() -> AdapterPrincipal {
        AdapterPrincipal::new("obsidian-plugin", AdapterRole::ObsidianPlugin).unwrap()
    }

    fn hash_with(hex_char: &str) -> String {
        format!("sha256:{}", hex_char.repeat(64))
    }

    fn path(value: &str) -> VaultPath {
        VaultPath::parse(value).unwrap()
    }

    fn revision(value: &str) -> RevisionId {
        RevisionId::parse(value).unwrap()
    }

    fn conflict(value: &str) -> ConflictId {
        ConflictId::parse(value).unwrap()
    }

    #[test]
    fn put_file_request_extracts_safe_metadata_and_redacts_sensitive_inputs() {
        let principal = principal();
        let raw_hash = hash_with("A");
        let canonical_hash = hash_with("a");
        let request = parse_put_file_request(PutFileRouteRequestParts {
            route_path: "./Notes//daily.md",
            adapter_principal: Some(&principal),
            idempotency_key: Some("idem-01"),
            content_sha256: Some(&raw_hash),
            base_revision_id: Some("null"),
            body: b"hello".to_vec(),
            max_upload_bytes: Some(16),
        })
        .unwrap();

        assert_eq!(request.path().as_str(), "Notes/daily.md");
        assert_eq!(request.adapter_principal(), &principal);
        assert_eq!(request.idempotency_key().as_str(), "idem-01");
        assert_eq!(request.base_revision_id(), None);
        assert_eq!(request.content_sha256().to_string(), canonical_hash);
        assert_eq!(request.size_bytes(), 5);
        assert_eq!(request.body(), b"hello");

        let metadata = request.to_metadata_dto();
        assert_eq!(metadata.path.as_str(), "Notes/daily.md");
        assert_eq!(metadata.base_revision_id, None);
        assert_eq!(metadata.content_sha256.as_str(), canonical_hash);
        assert_eq!(metadata.size_bytes, Some(5));

        let debug = format!("{request:?}");
        assert!(!debug.contains("idem-01"));
        assert!(!debug.contains("hello"));
        assert!(debug.contains("<raw-bytes-redacted>"));
    }

    #[test]
    fn put_file_request_requires_verified_adapter_principal() {
        let error = parse_put_file_request(PutFileRouteRequestParts {
            route_path: "Notes/daily.md",
            adapter_principal: None,
            idempotency_key: Some("idem-01"),
            content_sha256: Some(&hash_with("b")),
            base_revision_id: Some("rev_01JBASE"),
            body: Vec::new(),
            max_upload_bytes: None,
        })
        .unwrap_err();

        assert_eq!(error, FileRouteError::MissingAdapterPrincipal);
        assert_eq!(error.http_status_code(), 401);
        assert_eq!(error.public_code(), PublicErrorCode::MissingToken);

        let json = serde_json::to_string(&error.to_error_response()).unwrap();
        assert!(json.contains("missing_token"));
        assert!(!json.contains("idem-01"));
        assert!(!json.contains("fixture-token"));
    }

    #[test]
    fn put_file_request_parses_known_base_revision_and_size_limit() {
        let principal = principal();
        let request = parse_put_file_request(PutFileRouteRequestParts {
            route_path: "Notes/daily.md",
            adapter_principal: Some(&principal),
            idempotency_key: Some("idem-02"),
            content_sha256: Some(&hash_with("c")),
            base_revision_id: Some("rev_01JBASE"),
            body: vec![1, 2, 3],
            max_upload_bytes: Some(3),
        })
        .unwrap();

        assert_eq!(request.base_revision_id().unwrap().as_str(), "rev_01JBASE");
        assert_eq!(request.size_bytes(), 3);

        assert_eq!(
            parse_put_file_request(PutFileRouteRequestParts {
                route_path: "Notes/daily.md",
                adapter_principal: Some(&principal),
                idempotency_key: Some("idem-02"),
                content_sha256: Some(&hash_with("c")),
                base_revision_id: Some("rev_01JBASE"),
                body: vec![1, 2, 3, 4],
                max_upload_bytes: Some(3),
            }),
            Err(FileRouteError::PayloadTooLarge {
                max_upload_bytes: 3
            })
        );
    }

    #[test]
    fn get_file_request_uses_vault_path_and_optional_revision_query() {
        let request = parse_get_file_request(GetFileRouteRequestParts {
            route_path: "./Notes//daily.md",
            revision_id: Some("rev_01JREAD"),
        })
        .unwrap();

        assert_eq!(request.path().as_str(), "Notes/daily.md");
        assert_eq!(request.revision_id().unwrap().as_str(), "rev_01JREAD");
        assert_eq!(
            request.to_query_dto().revision_id.unwrap().as_str(),
            "rev_01JREAD"
        );
        assert_eq!(
            parse_get_file_request(GetFileRouteRequestParts {
                route_path: "../secret.md",
                revision_id: None,
            }),
            Err(FileRouteError::InvalidPath)
        );
        assert_eq!(
            parse_get_file_request(GetFileRouteRequestParts {
                route_path: "Notes/daily.md",
                revision_id: Some("bad-revision"),
            }),
            Err(FileRouteError::InvalidRevisionQuery)
        );
    }

    #[test]
    fn upload_response_helpers_preserve_public_outcome_vocabulary() {
        let accepted = accepted_upload_response(path("Notes/daily.md"), revision("rev_01JNEW"), 42);
        let same_content = ignored_same_content_response(path("Notes/daily.md"));
        let conflict_saved = conflict_saved_upload_response(
            path("Notes/daily.md"),
            conflict("conf_01J"),
            path("_haze_conflicts/open/Notes/daily.conflict.gdrive.md"),
            ConflictPolicyDto::PreserveBoth,
            43,
        );
        let hash_mismatch = hash_mismatch_upload_response(path("Notes/daily.md"));
        let stale = stale_base_upload_response(path("Notes/daily.md"));

        assert!(matches!(accepted, PutFileResponse::Accepted { seq: 42, .. }));
        assert_eq!(
            same_content,
            PutFileResponse::Ignored {
                reason: FileIgnoredReasonDto::SameContent,
                path: VaultPathDto::from("Notes/daily.md"),
            }
        );
        assert!(matches!(
            conflict_saved,
            PutFileResponse::ConflictSaved { seq: 43, .. }
        ));
        assert_eq!(
            hash_mismatch,
            PutFileResponse::Rejected {
                reason: FileRejectedReasonDto::HashMismatch,
                path: VaultPathDto::from("Notes/daily.md"),
            }
        );
        assert_eq!(
            stale,
            PutFileResponse::Rejected {
                reason: FileRejectedReasonDto::StaleBaseRevision,
                path: VaultPathDto::from("Notes/daily.md"),
            }
        );
    }

    #[test]
    fn file_download_headers_preserve_metadata_without_body_bytes() {
        let headers = FileDownloadRouteHeaders::new(
            revision("rev_01JREAD"),
            ContentHash::parse(&hash_with("d")).unwrap(),
            128,
        );

        let header_map = headers.to_header_map();
        assert_eq!(header_map[CONTENT_TYPE_HEADER], APPLICATION_OCTET_STREAM);
        assert_eq!(header_map[X_REVISION_ID_HEADER], "rev_01JREAD");
        assert_eq!(header_map[X_SIZE_BYTES_HEADER], "128");

        let metadata = headers.to_metadata_dto();
        assert_eq!(metadata.revision_id.as_str(), "rev_01JREAD");
        assert_eq!(metadata.content_sha256.as_str(), hash_with("d"));
        assert_eq!(metadata.size_bytes, 128);
        assert_eq!(metadata.content_type, APPLICATION_OCTET_STREAM);
    }
}
