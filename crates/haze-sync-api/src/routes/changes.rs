//! Passive contract helpers for `GET /v1/changes?since=&limit=`.
//!
//! This module owns API-layer query parsing, response-shape helpers, pagination
//! metadata preservation, and sanitized error mapping for the future changes
//! route. It deliberately does not register Axum handlers, authenticate adapter
//! tokens, call storage repositories, update adapter cursors, create database
//! pools, or perform adapter runtime behavior.

use std::{collections::BTreeMap, error::Error, fmt};

use haze_sync_core::operation_log::{
    ChangeFeedEntry as CoreChangeFeedEntry, ChangesLimit, ChangesPage as CoreChangesPage,
    OperationKind as CoreOperationKind, OperationSequence,
    DEFAULT_CHANGES_LIMIT as CORE_DEFAULT_CHANGES_LIMIT,
    MAX_CHANGES_LIMIT as CORE_MAX_CHANGES_LIMIT,
};
use serde::{Deserialize, Serialize};

use crate::{
    contracts::errors::{ErrorResponse, PublicError, PublicErrorCode, SafeErrorDetails},
    dto::{
        changes::{ChangeEntryDto, ChangesQuery as ChangesQueryDto, ChangesResponse},
        common::OperationKindDto,
        primitives::{
            AdapterIdDto, ConflictIdDto, ContentSha256Dto, RevisionIdDto, TimestampDto,
            TombstoneIdDto, VaultPathDto,
        },
    },
};

/// Default changes page size aligned with Core operation-log constants.
pub const DEFAULT_CHANGES_LIMIT: u32 = CORE_DEFAULT_CHANGES_LIMIT;

/// Maximum contract page size for `GET /v1/changes`.
pub const MAX_CHANGES_LIMIT: u32 = CORE_MAX_CHANGES_LIMIT;

/// HTTP status code for invalid changes-route requests.
pub const HTTP_STATUS_BAD_REQUEST: u16 = 400;

/// HTTP status code for a sanitized unavailable storage/Core placeholder.
pub const HTTP_STATUS_INTERNAL_ERROR: u16 = 500;

/// Parsed and bounded changes-route request for later W2-F1 service wiring.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ChangesRequest {
    /// Return operation-log entries strictly after this sequence.
    pub since: OperationSequence,
    /// Maximum number of changes returned in one page.
    pub limit: ChangesLimit,
}

impl ChangesRequest {
    /// Parse raw query values from `GET /v1/changes?since=&limit=`.
    ///
    /// Missing `since` defaults to sequence zero. Missing `limit` defaults to
    /// the Core operation-log default. Present values must be plain integers;
    /// negative `since`, zero `limit`, and limits above the contract maximum are
    /// rejected as safe invalid-request errors.
    pub fn parse(
        since: Option<&str>,
        limit: Option<&str>,
    ) -> Result<Self, ChangesRouteError> {
        Ok(Self {
            since: parse_since_query_value(since)?,
            limit: parse_limit_query_value(limit)?,
        })
    }

    /// Construct a request from already numeric values.
    pub fn new(since: i64, limit: u32) -> Result<Self, ChangesRouteError> {
        Ok(Self {
            since: OperationSequence::new(since).map_err(|_| ChangesRouteError::invalid_since())?,
            limit: ChangesLimit::new(limit).map_err(|_| ChangesRouteError::invalid_limit())?,
        })
    }

    /// Return the raw operation sequence value.
    #[must_use]
    pub const fn since_value(self) -> i64 {
        self.since.value()
    }

    /// Return the raw page limit value.
    #[must_use]
    pub const fn limit_value(self) -> u32 {
        self.limit.value()
    }

    /// Return the SQL-friendly limit plus one sentinel row for `has_more`.
    #[must_use]
    pub const fn limit_value_with_sentinel(self) -> i64 {
        self.limit.value_with_sentinel()
    }

    /// Convert into the existing JSON DTO query shape without changing values.
    #[must_use]
    pub const fn to_dto(self) -> ChangesQueryDto {
        ChangesQueryDto {
            since: self.since.value(),
            limit: self.limit.value(),
        }
    }
}

impl Default for ChangesRequest {
    fn default() -> Self {
        Self {
            since: OperationSequence::ZERO,
            limit: ChangesLimit::default(),
        }
    }
}

/// Sanitized error category for changes-route contract helpers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChangesRouteErrorKind {
    /// `since` was missing a valid non-negative operation sequence when present.
    InvalidSince,
    /// `limit` was zero, non-numeric, or above the public contract maximum.
    InvalidLimit,
    /// Future service/storage code supplied inconsistent page metadata.
    InvalidResponsePage,
    /// Placeholder for future storage/Core unavailability mapping.
    CoreUnavailable,
}

/// Safe changes-route error. It carries no SQL, stack trace, provider payload,
/// secret, database URL, local path, or runtime internals.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChangesRouteError {
    kind: ChangesRouteErrorKind,
    message: &'static str,
    field: Option<&'static str>,
}

impl ChangesRouteError {
    /// Error for invalid `since` query values.
    #[must_use]
    pub const fn invalid_since() -> Self {
        Self {
            kind: ChangesRouteErrorKind::InvalidSince,
            message: "since must be a non-negative integer operation sequence",
            field: Some("since"),
        }
    }

    /// Error for invalid `limit` query values.
    #[must_use]
    pub const fn invalid_limit() -> Self {
        Self {
            kind: ChangesRouteErrorKind::InvalidLimit,
            message: "limit must be an integer between 1 and 1000",
            field: Some("limit"),
        }
    }

    /// Error for inconsistent future Core/storage page metadata.
    #[must_use]
    pub const fn invalid_response_page() -> Self {
        Self {
            kind: ChangesRouteErrorKind::InvalidResponsePage,
            message: "changes page metadata is invalid",
            field: None,
        }
    }

    /// Placeholder mapping for future storage/Core unavailability.
    #[must_use]
    pub const fn core_unavailable() -> Self {
        Self {
            kind: ChangesRouteErrorKind::CoreUnavailable,
            message: "changes feed is temporarily unavailable",
            field: None,
        }
    }

    /// Return the safe error category.
    #[must_use]
    pub const fn kind(&self) -> ChangesRouteErrorKind {
        self.kind
    }

    /// Return the HTTP status code intended for future route handlers.
    #[must_use]
    pub const fn status_code(&self) -> u16 {
        match self.kind {
            ChangesRouteErrorKind::InvalidSince | ChangesRouteErrorKind::InvalidLimit => {
                HTTP_STATUS_BAD_REQUEST
            }
            ChangesRouteErrorKind::InvalidResponsePage | ChangesRouteErrorKind::CoreUnavailable => {
                HTTP_STATUS_INTERNAL_ERROR
            }
        }
    }

    /// Convert the error into the existing sanitized public API error contract.
    #[must_use]
    pub fn error_response(&self) -> ErrorResponse {
        let code = match self.kind {
            ChangesRouteErrorKind::InvalidSince | ChangesRouteErrorKind::InvalidLimit => {
                PublicErrorCode::InvalidRequest
            }
            ChangesRouteErrorKind::InvalidResponsePage | ChangesRouteErrorKind::CoreUnavailable => {
                PublicErrorCode::InternalError
            }
        };

        let mut public_error = PublicError::new(code, self.message);
        if let Some(field) = self.field {
            let mut details = BTreeMap::new();
            details.insert(field.to_owned(), vec![self.message.to_owned()]);
            public_error = public_error.with_details(SafeErrorDetails::Map(details));
        }

        ErrorResponse {
            error: public_error,
        }
    }
}

impl fmt::Display for ChangesRouteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message)
    }
}

impl Error for ChangesRouteError {}

/// Parse changes-route query values into a safe typed request object.
pub fn parse_changes_query(
    since: Option<&str>,
    limit: Option<&str>,
) -> Result<ChangesRequest, ChangesRouteError> {
    ChangesRequest::parse(since, limit)
}

/// Build a response from explicit page metadata and DTO entries.
///
/// `from_seq`, `to_seq`, and `has_more` are preserved as supplied after safe
/// consistency checks. `has_more` is never inferred from storage side effects.
pub fn changes_response_from_parts(
    from_seq: i64,
    to_seq: i64,
    has_more: bool,
    changes: Vec<ChangeEntryDto>,
) -> Result<ChangesResponse, ChangesRouteError> {
    validate_response_page(from_seq, to_seq, &changes)?;

    Ok(ChangesResponse {
        from_seq,
        to_seq,
        has_more,
        changes,
    })
}

/// Build an empty response for a parsed request without guessing `has_more`.
#[must_use]
pub fn empty_changes_response(request: ChangesRequest) -> ChangesResponse {
    ChangesResponse {
        from_seq: request.since_value(),
        to_seq: request.since_value(),
        has_more: false,
        changes: Vec::new(),
    }
}

impl TryFrom<CoreChangesPage> for ChangesResponse {
    type Error = ChangesRouteError;

    fn try_from(page: CoreChangesPage) -> Result<Self, Self::Error> {
        let changes = page.changes.into_iter().map(ChangeEntryDto::from).collect();
        changes_response_from_parts(
            page.from_seq.value(),
            page.to_seq.value(),
            page.has_more,
            changes,
        )
    }
}

impl From<CoreChangeFeedEntry> for ChangeEntryDto {
    fn from(entry: CoreChangeFeedEntry) -> Self {
        let operation = entry.operation;
        Self {
            seq: operation.seq.value(),
            kind: OperationKindDto::from(operation.kind),
            path: VaultPathDto::from(operation.path),
            revision_id: operation.revision_id.map(RevisionIdDto::from),
            content_sha256: entry.content_sha256.map(ContentSha256Dto::from),
            size_bytes: entry.size_bytes,
            tombstone_id: operation
                .tombstone_id
                .map(|tombstone_id| TombstoneIdDto::from(tombstone_id.into_string())),
            conflict_id: operation.conflict_id.map(ConflictIdDto::from),
            updated_by: AdapterIdDto::from(operation.adapter_id),
            updated_at: TimestampDto::from(
                operation
                    .created_at
                    .format("%Y-%m-%dT%H:%M:%SZ")
                    .to_string(),
            ),
        }
    }
}

impl From<CoreOperationKind> for OperationKindDto {
    fn from(kind: CoreOperationKind) -> Self {
        match kind {
            CoreOperationKind::UpsertFile => Self::UpsertFile,
            CoreOperationKind::DeleteFile => Self::DeleteFile,
            CoreOperationKind::RestoreFile => Self::RestoreFile,
            CoreOperationKind::ConflictCreated => Self::ConflictCreated,
            CoreOperationKind::ConflictResolved => Self::ConflictResolved,
            CoreOperationKind::BackupCreated => Self::BackupCreated,
        }
    }
}

fn parse_since_query_value(raw: Option<&str>) -> Result<OperationSequence, ChangesRouteError> {
    let value = match raw {
        Some(raw) => raw
            .parse::<i64>()
            .map_err(|_| ChangesRouteError::invalid_since())?,
        None => OperationSequence::ZERO.value(),
    };

    OperationSequence::new(value).map_err(|_| ChangesRouteError::invalid_since())
}

fn parse_limit_query_value(raw: Option<&str>) -> Result<ChangesLimit, ChangesRouteError> {
    let value = match raw {
        Some(raw) => raw
            .parse::<u32>()
            .map_err(|_| ChangesRouteError::invalid_limit())?,
        None => DEFAULT_CHANGES_LIMIT,
    };

    ChangesLimit::new(value).map_err(|_| ChangesRouteError::invalid_limit())
}

fn validate_response_page(
    from_seq: i64,
    to_seq: i64,
    changes: &[ChangeEntryDto],
) -> Result<(), ChangesRouteError> {
    OperationSequence::new(from_seq).map_err(|_| ChangesRouteError::invalid_response_page())?;
    OperationSequence::new(to_seq).map_err(|_| ChangesRouteError::invalid_response_page())?;

    if to_seq < from_seq {
        return Err(ChangesRouteError::invalid_response_page());
    }

    let mut previous_seq = from_seq;
    for change in changes {
        if change.seq <= previous_seq {
            return Err(ChangesRouteError::invalid_response_page());
        }
        previous_seq = change.seq;
    }

    let expected_to_seq = changes.last().map_or(from_seq, |change| change.seq);
    if to_seq != expected_to_seq {
        return Err(ChangesRouteError::invalid_response_page());
    }

    Ok(())
}
