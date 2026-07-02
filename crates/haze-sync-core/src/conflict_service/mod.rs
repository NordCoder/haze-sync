//! Pure conflict-service value types and conflict path generation.

use chrono::{DateTime, Utc};
use haze_sync_common::{
    AdapterId, ConflictId, ContentHash, RevisionId, ValidationError, VaultPath,
};
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
    PreserveBoth,
    CurrentWinsWithIncomingBackup,
}

impl ConflictPolicy {
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
    Open,
    Resolved,
    Ignored,
}

impl ConflictRecordStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Resolved => "resolved",
            Self::Ignored => "ignored",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CurrentRevision {
    pub revision_id: RevisionId,
    pub path: VaultPath,
    pub content_hash: ContentHash,
    pub size_bytes: u64,
}

impl CurrentRevision {
    #[must_use]
    pub fn new(
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IncomingConflictCandidate {
    pub original_path: VaultPath,
    pub base_revision_id: Option<RevisionId>,
    pub adapter_id: AdapterId,
    pub content_hash: ContentHash,
    pub size_bytes: u64,
}

impl IncomingConflictCandidate {
    #[must_use]
    pub fn new(
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConflictPathRequest {
    pub original_path: VaultPath,
    pub adapter_id: AdapterId,
    pub timestamp: DateTime<Utc>,
}

impl ConflictPathRequest {
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

    pub fn parse(
        original_path: &str,
        adapter_id: &str,
        timestamp: DateTime<Utc>,
    ) -> Result<Self, ConflictPolicyError> {
        let original_path = VaultPath::parse(original_path)
            .map_err(|reason| ConflictPolicyError::InvalidPath { reason })?;
        let adapter_id =
            AdapterId::parse(adapter_id).map_err(|_reason| ConflictPolicyError::UnsafeAdapterId)?;
        Self::new(original_path, adapter_id, timestamp)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IncomingBackupPlan {
    pub original_path: VaultPath,
    pub conflict_path: VaultPath,
    pub adapter_id: AdapterId,
    pub content_hash: ContentHash,
    pub size_bytes: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConflictRecordInput {
    pub original_path: VaultPath,
    pub conflict_path: VaultPath,
    pub base_revision_id: Option<RevisionId>,
    pub current_revision_id: RevisionId,
    pub incoming_content_hash: ContentHash,
    pub incoming_size_bytes: u64,
    pub adapter_id: AdapterId,
    pub policy_applied: ConflictPolicy,
    pub status: ConflictRecordStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConflictRecord {
    pub conflict_id: ConflictId,
    pub original_path: VaultPath,
    pub conflict_path: VaultPath,
    pub base_revision_id: Option<RevisionId>,
    pub current_revision_id: RevisionId,
    pub incoming_content_hash: ContentHash,
    pub incoming_size_bytes: u64,
    pub adapter_id: AdapterId,
    pub policy_applied: ConflictPolicy,
    pub status: ConflictRecordStatus,
    pub created_at: DateTime<Utc>,
}

impl ConflictRecord {
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "error")]
pub enum ConflictPolicyError {
    InvalidPath { reason: ValidationError },
    RecursiveConflictPath,
    UnsafeAdapterId,
    PathMismatch,
}

impl ConflictPolicyError {
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidPath { .. } => "invalid_path",
            Self::RecursiveConflictPath => "recursive_conflict_path",
            Self::UnsafeAdapterId => "unsafe_adapter_id",
            Self::PathMismatch => "path_mismatch",
        }
    }

    #[must_use]
    pub const fn message(&self) -> &'static str {
        match self {
            Self::InvalidPath { .. } => "conflict path input is invalid",
            Self::RecursiveConflictPath => {
                "conflicts under the conflict area are not materialized recursively"
            }
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

pub fn generate_conflict_path(
    request: &ConflictPathRequest,
) -> Result<VaultPath, ConflictPolicyError> {
    validate_original_path(&request.original_path)?;
    validate_conflict_adapter_id(&request.adapter_id)?;

    let original = request.original_path.as_str();
    let mut segments = original.split('/').collect::<Vec<_>>();
    let filename = segments.pop().ok_or(ConflictPolicyError::InvalidPath {
        reason: ValidationError::EmptyPath,
    })?;
    let (stem, extension) = split_filename(filename);
    let timestamp = request
        .timestamp
        .format(CONFLICT_TIMESTAMP_FORMAT)
        .to_string();

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
    generated.push_str(&timestamp);
    generated.push_str(extension);

    VaultPath::parse(&generated).map_err(|reason| ConflictPolicyError::InvalidPath { reason })
}

#[must_use]
pub fn is_conflict_area_path(path: &VaultPath) -> bool {
    path.segments().next() == Some(CONFLICT_ROOT_SEGMENT)
}

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
