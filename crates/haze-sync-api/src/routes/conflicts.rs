//! Passive contract helpers for `GET /v1/conflicts?status=open` and
//! `POST /v1/conflicts/{conflict_id}/resolve`.
//!
//! This module owns API-layer parsing, DTO mapping, validation, and sanitized
//! placeholder error mapping for future conflict routes. It does not register
//! Axum handlers, authenticate adapter tokens, call storage repositories, execute
//! Core conflict policy, resolve conflicts, write files, or contact providers.

use std::{collections::BTreeMap, error::Error, fmt};

use haze_sync_common::{AdapterId, ConflictId, RevisionId, VaultPath};
use serde::{Deserialize, Serialize};

use crate::{
    contracts::errors::{ErrorResponse, PublicError, PublicErrorCode, SafeErrorDetails},
    dto::{
        common::{
            ConflictPolicyDto, ConflictResolutionDto, ConflictResolveStatusDto,
            ConflictStatusDto,
        },
        conflicts::{ConflictListQuery, ResolveConflictRequest, ResolveConflictResponse},
        primitives::{AdapterIdDto, ConflictIdDto, RevisionIdDto, TimestampDto, VaultPathDto},
    },
};

/// HTTP status code for invalid conflict-route requests.
pub const HTTP_STATUS_BAD_REQUEST: u16 = 400;

/// HTTP status code for a missing conflict placeholder mapping.
pub const HTTP_STATUS_NOT_FOUND: u16 = 404;

/// HTTP status code for conflict lifecycle state mismatches.
pub const HTTP_STATUS_CONFLICT: u16 = 409;

/// Parsed request for `GET /v1/conflicts?status=open`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictListRequest {
    /// Optional status filter. W3-P4 supports `open` and rejects every unsupported value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ConflictStatusDto>,
}

impl ConflictListRequest {
    /// Parse the optional `status` query parameter.
    pub fn parse(status: Option<&str>) -> Result<Self, ConflictsRouteError> {
        Ok(Self {
            status: parse_status_query_value(status)?,
        })
    }

    /// Convert into the existing JSON DTO query shape without changing values.
    #[must_use]
    pub fn to_dto(&self) -> ConflictListQuery {
        ConflictListQuery {
            status: self.status.clone(),
        }
    }
}

/// Raw shape supplied by a future HTTP layer after extracting query values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConflictListRequestParts<'a> {
    /// Raw `status` query parameter, when present.
    pub status: Option<&'a str>,
}

/// Parse conflict-list query values into a safe typed request object.
pub fn parse_conflicts_query(
    parts: ConflictListRequestParts<'_>,
) -> Result<ConflictListRequest, ConflictsRouteError> {
    ConflictListRequest::parse(parts.status)
}

/// Public JSON response for conflict-list route helpers.
///
/// The field order is intentionally deterministic for stable JSON output.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictListRouteResponse {
    /// Public conflict summaries visible to clients.
    pub conflicts: Vec<ConflictRouteSummaryDto>,
}

/// Public conflict summary for W3-P4 route contract helpers.
///
/// This route-level DTO includes both the original path and materialized conflict
/// path explicitly, while preserving incoming/conflict revision metadata when it
/// is available from future Core/storage fan-in code.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictRouteSummaryDto {
    /// Public conflict identifier.
    pub conflict_id: ConflictIdDto,
    /// Original file path that encountered a conflicting incoming change.
    pub original_path: VaultPathDto,
    /// Materialized conflict copy path.
    pub conflict_path: VaultPathDto,
    /// Current revision at the original path.
    pub current_revision_id: RevisionIdDto,
    /// Preserved conflict-copy revision when future Core/storage code provides it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflict_revision_id: Option<RevisionIdDto>,
    /// Incoming revision identifier when distinct incoming metadata is available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub incoming_revision_id: Option<RevisionIdDto>,
    /// Adapter that submitted the conflicting incoming change.
    pub source_adapter_id: AdapterIdDto,
    /// Policy Core applied to preserve the conflict safely.
    pub policy_applied: ConflictPolicyDto,
    /// Current conflict lifecycle status.
    pub status: ConflictStatusDto,
    /// Conflict creation timestamp when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<TimestampDto>,
    /// Conflict update timestamp when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<TimestampDto>,
}

/// Source values used by future fan-in code to build public conflict-list DTOs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConflictRouteSummaryParts {
    /// Public conflict identifier.
    pub conflict_id: ConflictId,
    /// Original file path that encountered a conflicting incoming change.
    pub original_path: VaultPath,
    /// Materialized conflict copy path.
    pub conflict_path: VaultPath,
    /// Current revision at the original path.
    pub current_revision_id: RevisionId,
    /// Preserved conflict-copy revision when available.
    pub conflict_revision_id: Option<RevisionId>,
    /// Incoming revision identifier when available.
    pub incoming_revision_id: Option<RevisionId>,
    /// Adapter that submitted the conflicting incoming change.
    pub source_adapter_id: AdapterId,
    /// Policy Core applied to preserve the conflict safely.
    pub policy_applied: ConflictPolicyDto,
    /// Current conflict lifecycle status.
    pub status: ConflictStatusDto,
    /// Conflict creation timestamp when known.
    pub created_at: Option<TimestampDto>,
    /// Conflict update timestamp when known.
    pub updated_at: Option<TimestampDto>,
}

impl From<ConflictRouteSummaryParts> for ConflictRouteSummaryDto {
    fn from(parts: ConflictRouteSummaryParts) -> Self {
        Self {
            conflict_id: ConflictIdDto::from(parts.conflict_id),
            original_path: VaultPathDto::from(parts.original_path),
            conflict_path: VaultPathDto::from(parts.conflict_path),
            current_revision_id: RevisionIdDto::from(parts.current_revision_id),
            conflict_revision_id: parts.conflict_revision_id.map(RevisionIdDto::from),
            incoming_revision_id: parts.incoming_revision_id.map(RevisionIdDto::from),
            source_adapter_id: AdapterIdDto::from(parts.source_adapter_id),
            policy_applied: parts.policy_applied,
            status: parts.status,
            created_at: parts.created_at,
            updated_at: parts.updated_at,
        }
    }
}

/// Build a deterministic conflict-list response from already validated public parts.
#[must_use]
pub fn conflict_list_response_from_parts(
    conflicts: Vec<ConflictRouteSummaryParts>,
) -> ConflictListRouteResponse {
    ConflictListRouteResponse {
        conflicts: conflicts.into_iter().map(ConflictRouteSummaryDto::from).collect(),
    }
}

/// Raw shape supplied by a future HTTP layer after extracting path and JSON body fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResolveConflictRequestParts<'a> {
    /// Raw `{conflict_id}` path segment.
    pub conflict_id: &'a str,
    /// Raw `resolution` body value.
    pub resolution: Option<&'a str>,
    /// Body field names other than `resolution`; their values must never be echoed.
    pub extra_fields: &'a [&'a str],
}

/// Parsed request for `POST /v1/conflicts/{conflict_id}/resolve`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolveConflictRouteRequest {
    /// Conflict identifier parsed from the route path.
    pub conflict_id: ConflictId,
    /// Requested safe resolution action.
    pub resolution: ConflictResolutionDto,
}

impl ResolveConflictRouteRequest {
    /// Convert into the existing JSON DTO request body shape without the path id.
    #[must_use]
    pub fn to_dto(&self) -> ResolveConflictRequest {
        ResolveConflictRequest {
            resolution: self.resolution.clone(),
        }
    }
}

/// Parse a resolve-conflict request into a safe typed object.
pub fn parse_resolve_conflict_request(
    parts: ResolveConflictRequestParts<'_>,
) -> Result<ResolveConflictRouteRequest, ConflictsRouteError> {
    let conflict_id = ConflictId::parse(parts.conflict_id)
        .map_err(|_| ConflictsRouteError::invalid_conflict_id())?;

    if !parts.extra_fields.is_empty() {
        return Err(ConflictsRouteError::invalid_resolve_payload());
    }

    let resolution = parts
        .resolution
        .ok_or_else(ConflictsRouteError::invalid_resolve_payload)
        .and_then(parse_resolution_action)?;

    Ok(ResolveConflictRouteRequest {
        conflict_id,
        resolution,
    })
}

/// Build the standard resolved response shape from future service output.
#[must_use]
pub fn resolved_conflict_response(
    conflict_id: ConflictId,
    resolution: ConflictResolutionDto,
    seq: i64,
) -> ResolveConflictResponse {
    ResolveConflictResponse {
        status: ConflictResolveStatusDto::Resolved,
        conflict_id: ConflictIdDto::from(conflict_id),
        resolution,
        seq,
    }
}

/// Sanitized error category for conflict-route contract helpers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConflictsRouteErrorKind {
    /// `{conflict_id}` failed public identifier validation.
    InvalidConflictId,
    /// `status` query value is unsupported by the W3-P4 contract.
    UnsupportedStatus,
    /// Resolve request body did not match the expected public shape.
    InvalidResolvePayload,
    /// Resolve request action is unsupported by the W3-P4 contract.
    UnsupportedResolution,
    /// Future service/storage code did not find the public conflict id.
    NotFound,
    /// Future service/storage code found an already resolved conflict.
    AlreadyResolved,
}

/// Safe conflict-route error. It carries no SQL, stack trace, provider payload,
/// secret, database URL, local path, raw request body, token, or runtime internals.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConflictsRouteError {
    kind: ConflictsRouteErrorKind,
    message: &'static str,
    field: Option<&'static str>,
}

impl ConflictsRouteError {
    /// Error for invalid `{conflict_id}` path segments.
    #[must_use]
    pub const fn invalid_conflict_id() -> Self {
        Self {
            kind: ConflictsRouteErrorKind::InvalidConflictId,
            message: "conflict_id must be a valid conflict identifier",
            field: Some("conflict_id"),
        }
    }

    /// Error for unsupported `status` query values.
    #[must_use]
    pub const fn unsupported_status() -> Self {
        Self {
            kind: ConflictsRouteErrorKind::UnsupportedStatus,
            message: "status must be open when provided",
            field: Some("status"),
        }
    }

    /// Error for invalid resolve request body shape.
    #[must_use]
    pub const fn invalid_resolve_payload() -> Self {
        Self {
            kind: ConflictsRouteErrorKind::InvalidResolvePayload,
            message: "resolve request body must contain only a resolution action",
            field: Some("resolution"),
        }
    }

    /// Error for unsupported resolve actions.
    #[must_use]
    pub const fn unsupported_resolution() -> Self {
        Self {
            kind: ConflictsRouteErrorKind::UnsupportedResolution,
            message: "resolution must be one of the supported conflict actions",
            field: Some("resolution"),
        }
    }

    /// Placeholder mapping for a future missing conflict service result.
    #[must_use]
    pub const fn not_found() -> Self {
        Self {
            kind: ConflictsRouteErrorKind::NotFound,
            message: "conflict not found",
            field: None,
        }
    }

    /// Placeholder mapping for a future already-resolved conflict service result.
    #[must_use]
    pub const fn already_resolved() -> Self {
        Self {
            kind: ConflictsRouteErrorKind::AlreadyResolved,
            message: "conflict is already resolved",
            field: None,
        }
    }

    /// Return the safe error category.
    #[must_use]
    pub const fn kind(&self) -> ConflictsRouteErrorKind {
        self.kind
    }

    /// Return the HTTP status code intended for future route handlers.
    #[must_use]
    pub const fn status_code(&self) -> u16 {
        match self.kind {
            ConflictsRouteErrorKind::InvalidConflictId
            | ConflictsRouteErrorKind::UnsupportedStatus
            | ConflictsRouteErrorKind::InvalidResolvePayload
            | ConflictsRouteErrorKind::UnsupportedResolution => HTTP_STATUS_BAD_REQUEST,
            ConflictsRouteErrorKind::NotFound => HTTP_STATUS_NOT_FOUND,
            ConflictsRouteErrorKind::AlreadyResolved => HTTP_STATUS_CONFLICT,
        }
    }

    /// Convert the error into the existing sanitized public API error contract.
    #[must_use]
    pub fn error_response(&self) -> ErrorResponse {
        let mut public_error = PublicError::new(self.public_code(), self.message);
        if let Some(field) = self.field {
            public_error = public_error.with_details(detail(field, self.safe_detail_value()));
        }

        ErrorResponse {
            error: public_error,
        }
    }

    #[must_use]
    fn public_code(&self) -> PublicErrorCode {
        match self.kind {
            ConflictsRouteErrorKind::InvalidConflictId
            | ConflictsRouteErrorKind::UnsupportedStatus
            | ConflictsRouteErrorKind::InvalidResolvePayload
            | ConflictsRouteErrorKind::UnsupportedResolution => PublicErrorCode::ValidationError,
            ConflictsRouteErrorKind::NotFound => PublicErrorCode::NotFound,
            ConflictsRouteErrorKind::AlreadyResolved => PublicErrorCode::Conflict,
        }
    }

    #[must_use]
    const fn safe_detail_value(&self) -> &'static str {
        match self.kind {
            ConflictsRouteErrorKind::InvalidConflictId => "invalid",
            ConflictsRouteErrorKind::UnsupportedStatus => "unsupported",
            ConflictsRouteErrorKind::InvalidResolvePayload => "invalid_shape",
            ConflictsRouteErrorKind::UnsupportedResolution => "unsupported",
            ConflictsRouteErrorKind::NotFound | ConflictsRouteErrorKind::AlreadyResolved => "state",
        }
    }
}

impl fmt::Display for ConflictsRouteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message)
    }
}

impl Error for ConflictsRouteError {}

fn parse_status_query_value(
    raw: Option<&str>,
) -> Result<Option<ConflictStatusDto>, ConflictsRouteError> {
    match raw {
        None => Ok(None),
        Some("open") => Ok(Some(ConflictStatusDto::Open)),
        Some(_) => Err(ConflictsRouteError::unsupported_status()),
    }
}

fn parse_resolution_action(raw: &str) -> Result<ConflictResolutionDto, ConflictsRouteError> {
    match raw {
        "accept_current" => Ok(ConflictResolutionDto::AcceptCurrent),
        "accept_conflict" => Ok(ConflictResolutionDto::AcceptConflict),
        "keep_both" => Ok(ConflictResolutionDto::KeepBoth),
        "mark_resolved" => Ok(ConflictResolutionDto::MarkResolved),
        _ => Err(ConflictsRouteError::unsupported_resolution()),
    }
}

fn detail(field: impl Into<String>, value: impl Into<String>) -> SafeErrorDetails {
    let mut fields = BTreeMap::new();
    fields.insert(field.into(), vec![value.into()]);
    SafeErrorDetails::Map(fields)
}
