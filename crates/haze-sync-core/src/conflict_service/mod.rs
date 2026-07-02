//! Pure conflict-service value types and materialized conflict path generation.
//!
//! This module contains no database, object-store, route, adapter, provider, or
//! filesystem behavior. It only validates already-normalized Core inputs and
//! prepares deterministic conflict records and vault-relative paths for later
//! storage/API fan-in.

use chrono::{DateTime, Utc};
use haze_sync_common::{AdapterId, ConflictId, ContentHash, RevisionId, ValidationError, VaultPath};
use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};

const CONFLICT_ROOT_SEGMENT: &str = "_haze_conflicts";
const CONFLICT_OPEN_SEGMENT: &str = "open";
const CONFLICT_FILENAME_MARKER: &str = "conflict";
const CONFLICT_TIMESTAMP_FORMAT: &str = "%Y-%m-%d-%H%M%S";

/// V1 Core conflict policy names.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictPolicy {
    /// Preserve current content and save incoming content as a materialized copy.
    PreserveBoth,
    /// Keep current content as winner and save incoming content as a backup copy.
    CurrentWinsWithIncomingBackup,
}

impl ConflictPolicy {
    /// Stable machine-readable policy name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PreserveBoth => "preserve_both",
            Self::CurrentWinsWithIncomingBackup => "current_wins_with_incoming_backup",
        }
    }
}

impl fmt::Display for ConflictPolicy {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Core conflict lifecycle status values.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictRecordStatus {
    /// Conflict is unresolved and visible to later Conflict Center surfaces.
    Open,
    /// Conflict was resolved by a later explicit resolution action.
    Resolved,
    /// Conflict was intentionally ignored by a later explicit resolution action.
    Ignored,
}

impl ConflictRecordStatus {
    /// Stable machine-readable status name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Resolved => "resolved",
            Self::Ignored => "ignored",
        }
    }
}

/// Current Core revision snapshot used when a conflict decision is made.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CurrentRevision {
    /// Current immutable revision identifier at the original path.
    pub revision_id: RevisionId,
    /// Original vault path whose current revision would otherwise be overwritten.
    pub path: VaultPath,
    /// Current revision content hash.
    pub content_hash: ContentHash,
    /// Current revision size in bytes.
    pub size_bytes: u64,
}

impl CurrentRevision {
    /// Build a current revision snapshot from validated Core primitives.
    #[must_use]
    pub const fn new(
        revision_id: RevisionId,
        path: VaultPath,
        content_hash: ContentHash,
        size_bytes: u64,
    ) -> Self {
        Self {
            revision_id,
            path,
            content_hash,
            size_bytes,
        }
    }
}

/// Incoming content candidate that must be preserved instead of overwriting.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IncomingConflictCandidate {
    /// Original path submitted by the source adapter.
    pub original_path: VaultPath,
    /// Base revision claimed by the source adapter, or null when unknown.
    pub base_revision_id: Option<RevisionId>,
    /// Adapter that submitted the incoming content.
    pub adapter_id: AdapterId,
    /// Incoming content hash.
    pub content_hash: ContentHash,
    /// Incoming content size in bytes.
    pub size_bytes: u64,
}

impl IncomingConflictCandidate {
    /// Build an incoming candidate from validated Core primitives.
    #[must_use]
    pub const fn new(
        original_path: VaultPath,
        base_revision_id: Option<RevisionId>,
        adapter_id: AdapterId,
        content_hash: ContentHash,
        size_bytes: u64,
    ) -> Self {
        Self {
            original_path,
            base_revision_id,
            adapter_id,
            content_hash,
            size_bytes,
        }
    }
}

/// Request for deterministic materialized conflict path generation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConflictPathRequest {
    /// Original non-conflict vault path.
    pub original_path: VaultPath,
    /// Source adapter identifier used in the conflict filename.
    pub adapter_id: AdapterId,
    /// UTC timestamp used in the conflict filename.
    pub timestamp: DateTime<Utc>,
}

impl ConflictPathRequest {
    /// Build a path generation request from validated Core primitives.
    pub fn new(
        original_path: VaultPath,
        adapter_id: AdapterId,
        timestamp: DateTime<Utc>,
    ) -> Result<Self, ConflictPolicyError> {
        validate_original_path(&original_path)?;
        validate_conflict_adapter_id(&adapter_id)?;
        Ok(Self {
            original_path,
            adapter_id,
            timestamp,
        })
    }

    /// Parse raw public request parts through safe domain validators.
    pub fn parse(
        original_path: &str,
        adapter_id: &str,
        timestamp: DateTime<Utc>,
    ) -> Result<Self, ConflictPolicyError> {
        let original_path = VaultPath::parse(original_path)
            .map_err(|reason| ConflictPolicyError::InvalidPath { reason })?;
        let adapter_id = AdapterId::parse(adapter_id).map_err(|_reason| ConflictPolicyError::UnsafeAdapterId)?;
        Self::new(original_path, adapter_id, timestamp)
    }
}

/// Planned materialized copy for incoming conflict content.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IncomingBackupPlan {
    /// Original path whose incoming content is being preserved.
    pub original_path: VaultPath,
    /// Generated materialized conflict-copy path.
    pub conflict_path: VaultPath,
    /// Adapter that submitted the incoming content.
    pub adapter_id: AdapterId,
    /// Incoming content hash to be stored at `conflict_path` by a later phase.
    pub content_hash: ContentHash,
    /// Incoming content size in bytes.
    pub size_bytes: u64,
}

/// Pure Core conflict record input suitable for later repository insertion.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConflictRecordInput {
    /// Original path where the conflict was detected.
    pub original_path: VaultPath,
    /// Generated materialized conflict-copy path.
    pub conflict_path: VaultPath,
    /// Base revision claimed by the incoming adapter, or null when unknown.
    pub base_revision_id: Option<RevisionId>,
    /// Current winning revision at `original_path`.
    pub current_revision_id: RevisionId,
    /// Incoming content hash preserved as a conflict copy.
    pub incoming_content_hash: ContentHash,
    /// Incoming content size in bytes.
    pub incoming_size_bytes: u64,
    /// Adapter that submitted the incoming content.
    pub adapter_id: AdapterId,
    /// Conflict policy applied by Core.
    pub policy_applied: ConflictPolicy,
    /// Initial lifecycle marker for later Conflict Center surfaces.
    pub status: ConflictRecordStatus,
    /// Timestamp of the Core conflict decision.
    pub created_at: DateTime<Utc>,
}

/// Pure Core conflict record shape after a later repository assigns an id.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConflictRecord {
    /// Public conflict identifier.
    pub conflict_id: ConflictId,
    /// Original path where the conflict was detected.
    pub original_path: VaultPath,
    /// Generated materialized conflict-copy path.
    pub conflict_path: VaultPath,
    /// Base revision claimed by the incoming adapter, or null when unknown.
    pub base_revision_id: Option<RevisionId>,
    /// Current winning revision at `original_path`.
    pub current_revision_id: RevisionId,
    /// Incoming content hash preserved as a conflict copy.
    pub incoming_content_hash: ContentHash,
    /// Incoming content size in bytes.
    pub incoming_size_bytes: u64,
    /// Adapter that submitted the incoming content.
    pub adapter_id: AdapterId,
    /// Conflict policy applied by Core.
    pub policy_applied: ConflictPolicy,
    /// Conflict lifecycle marker.
    pub status: ConflictRecordStatus,
    /// Timestamp of the Core conflict decision.
    pub created_at: DateTime<Utc>,
}

impl ConflictRecord {
    /// Attach an assigned conflict id to a pure conflict record input.
    #[must_use]
    pub fn from_input(conflict_id: ConflictId, input: ConflictRecordInput) -> Self {
        Self {
            conflict_id,
            original_path: input.original_path,
            conflict_path: input.conflict_path,
            base_revision_id: input.base_revision_id,
            current_revision_id: input.current_revision_id,
            incoming_content_hash: input.incoming_content_hash,
            incoming_size_bytes: input.incoming_size_bytes,
            adapter_id: input.adapter_id,
            policy_applied: input.policy_applied,
            status: input.status,
            created_at: input.created_at,
        }
    }
}

/// Safe, path-free conflict policy validation and planning errors.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "error")]
pub enum ConflictPolicyError {
    /// A public path failed safe VaultPath validation.
    InvalidPath { reason: ValidationError },
    /// Conflict inputs under `_haze_conflicts/**` are rejected to avoid recursion.
    RecursiveConflictPath,
    /// Adapter id is valid generally but unsafe inside materialized conflict names.
    UnsafeAdapterId,
    /// Current and incoming snapshots do not describe the same original path.
    PathMismatch,
}

impl ConflictPolicyError {
    /// Stable machine-readable error code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidPath { .. } => "invalid_path",
            Self::RecursiveConflictPath => "recursive_conflict_path",
            Self::UnsafeAdapterId => "unsafe_adapter_id",
            Self::PathMismatch => "path_mismatch",
        }
    }

    /// Stable public message with no raw paths, filesystem details, tokens, or provider data.
    #[must_use]
    pub const fn message(&self) -> &'static str {
        match self {
            Self::InvalidPath { .. } => "conflict path input is invalid",
            Self::RecursiveConflictPath => "conflicts under the conflict area are not materialized recursively",
            Self::UnsafeAdapterId => "adapter id is not safe for conflict path generation",
            Self::PathMismatch => "current and incoming conflict paths do not match",
        }
    }
}

impl fmt::Display for ConflictPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message())
    }
}

impl Error for ConflictPolicyError {}

/// Generate a deterministic materialized conflict path under `_haze_conflicts/open`.
pub fn generate_conflict_path(request: &ConflictPathRequest) -> Result<VaultPath, ConflictPolicyError> {
    validate_original_path(&request.original_path)?;
    validate_conflict_adapter_id(&request.adapter_id)?;

    let original = request.original_path.as_str();
    let mut segments: Vec<&str> = original.split('/').collect();
    let filename = segments.pop().ok_or(ConflictPolicyError::InvalidPath {
        reason: ValidationError::EmptyPath,
    })?;
    let (stem, extension) = split_filename(filename);
    let timestamp = request.timestamp.format(CONFLICT_TIMESTAMP_FORMAT);

    let mut generated = String::from(CONFLICT_ROOT_SEGMENT);
    generated.push('/');
    generated.push_str(CONFLICT_OPEN_SEGMENT);
    for segment in segments {
        generated.push('/');
        generated.push_str(segment);
    }
    generated.push('/');
    generated.push_str(stem);
    generated.push('.');
    generated.push_str(CONFLICT_FILENAME_MARKER);
    generated.push('.');
    generated.push_str(request.adapter_id.as_str());
    generated.push('.');
    generated.push_str(&timestamp.to_string());
    generated.push_str(extension);

    VaultPath::parse(&generated).map_err(|reason| ConflictPolicyError::InvalidPath { reason })
}

/// Return true when a validated path is inside `_haze_conflicts/**`.
#[must_use]
pub fn is_conflict_area_path(path: &VaultPath) -> bool {
    path.segments().next() == Some(CONFLICT_ROOT_SEGMENT)
}

/// Validate that a path can be used as a non-recursive original conflict source.
pub fn validate_original_path(path: &VaultPath) -> Result<(), ConflictPolicyError> {
    if is_conflict_area_path(path) {
        return Err(ConflictPolicyError::RecursiveConflictPath);
    }
    Ok(())
}

fn validate_conflict_adapter_id(adapter_id: &AdapterId) -> Result<(), ConflictPolicyError> {
    if adapter_id
        .as_str()
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        Ok(())
    } else {
        Err(ConflictPolicyError::UnsafeAdapterId)
    }
}

fn split_filename(filename: &str) -> (&str, &str) {
    match filename.rfind('.') {
        Some(dot_index) if dot_index > 0 && dot_index + 1 < filename.len() => {
            filename.split_at(dot_index)
        }
        _ => (filename, ""),
    }
}
