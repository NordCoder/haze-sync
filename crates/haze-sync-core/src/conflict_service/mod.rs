//! Pure conflict preservation and resolution value types and planners.

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

/// Pure conflict resolution actions accepted by Core.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolutionAction {
    /// Keep the current revision authoritative and close the conflict.
    AcceptCurrent,
    /// Promote the preserved incoming conflict content to a new current revision.
    AcceptConflict,
    /// Keep the current revision and keep the materialized conflict copy as the other side.
    KeepBoth,
    /// Close conflict metadata without changing current revision or conflict-copy materialization.
    MarkResolved,
}

impl ConflictResolutionAction {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AcceptCurrent => "accept_current",
            Self::AcceptConflict => "accept_conflict",
            Self::KeepBoth => "keep_both",
            Self::MarkResolved => "mark_resolved",
        }
    }

    /// Whether this action requires downstream storage to create a new current revision.
    #[must_use]
    pub const fn requires_new_current_revision(self) -> bool {
        match self {
            Self::AcceptConflict => true,
            Self::AcceptCurrent | Self::KeepBoth | Self::MarkResolved => false,
        }
    }
}

impl fmt::Display for ConflictResolutionAction {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Request accepted by the pure conflict resolution planner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConflictResolutionRequest {
    pub conflict: ConflictRecord,
    pub action: ConflictResolutionAction,
    pub resolved_at: DateTime<Utc>,
}

impl ConflictResolutionRequest {
    pub fn new(
        conflict: ConflictRecord,
        action: ConflictResolutionAction,
        resolved_at: DateTime<Utc>,
    ) -> Result<Self, ConflictResolutionError> {
        validate_resolution_conflict_record(&conflict)?;
        Ok(Self {
            conflict,
            action,
            resolved_at,
        })
    }
}

/// Metadata-only conflict status transition for downstream persistence fan-in.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConflictStatusUpdatePlan {
    pub conflict_id: ConflictId,
    pub from_status: ConflictRecordStatus,
    pub to_status: ConflictRecordStatus,
    pub resolved_at: DateTime<Utc>,
}

/// Storage-neutral request to create a new current revision from preserved incoming content.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NewCurrentRevisionPlan {
    pub path: VaultPath,
    pub parent_revision_id: RevisionId,
    pub content_hash: ContentHash,
    pub size_bytes: u64,
    pub created_by: AdapterId,
}

/// Revision effect of a conflict resolution action. This is a plan, not a write.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "effect")]
pub enum ConflictResolutionRevisionEffect {
    /// No new current revision is required; downstream only updates metadata.
    MetadataOnlyCurrentUnchanged { current_revision_id: RevisionId },
    /// Downstream storage must create a new current revision from the conflict copy content.
    CreateCurrentRevisionFromConflict {
        new_revision: NewCurrentRevisionPlan,
    },
}

impl ConflictResolutionRevisionEffect {
    #[must_use]
    pub const fn requires_new_current_revision(&self) -> bool {
        match self {
            Self::MetadataOnlyCurrentUnchanged { .. } => false,
            Self::CreateCurrentRevisionFromConflict { .. } => true,
        }
    }
}

/// Conflict-copy handling requested by a resolution plan.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictCopyDisposition {
    /// The materialized conflict copy should remain available as the kept second copy.
    KeepMaterializedConflictCopy,
    /// The conflict copy can be marked resolved/superseded after metadata update.
    MarkConflictCopyResolved,
    /// The resolution action makes no request about conflict-copy materialization.
    NoConflictCopyChange,
}

/// Storage/API-neutral plan for resolving an open conflict.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConflictResolutionPlan {
    pub conflict_id: ConflictId,
    pub action: ConflictResolutionAction,
    pub original_path: VaultPath,
    pub conflict_path: VaultPath,
    pub status_update: ConflictStatusUpdatePlan,
    pub revision_effect: ConflictResolutionRevisionEffect,
    pub conflict_copy_disposition: ConflictCopyDisposition,
}

impl ConflictResolutionPlan {
    #[must_use]
    pub fn requires_new_current_revision(&self) -> bool {
        self.revision_effect.requires_new_current_revision()
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

/// Safe conflict resolution errors. No raw bytes, provider payloads, paths outside safe value
/// types, stack traces, or credentials are exposed.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "error")]
pub enum ConflictResolutionError {
    ConflictPolicy { reason: ConflictPolicyError },
    ConflictNotOpen { status: ConflictRecordStatus },
    InvalidConflictMaterializationPath,
}

impl ConflictResolutionError {
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::ConflictPolicy { reason } => reason.code(),
            Self::ConflictNotOpen { .. } => "conflict_not_open",
            Self::InvalidConflictMaterializationPath => "invalid_conflict_materialization_path",
        }
    }

    #[must_use]
    pub const fn message(&self) -> &'static str {
        match self {
            Self::ConflictPolicy { reason } => reason.message(),
            Self::ConflictNotOpen { .. } => "only open conflicts can be resolved",
            Self::InvalidConflictMaterializationPath => {
                "conflict materialization path must be under _haze_conflicts/open"
            }
        }
    }
}

impl fmt::Display for ConflictResolutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message())
    }
}

impl Error for ConflictResolutionError {}

impl From<ConflictPolicyError> for ConflictResolutionError {
    fn from(reason: ConflictPolicyError) -> Self {
        Self::ConflictPolicy { reason }
    }
}

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

#[must_use]
pub fn is_open_conflict_area_path(path: &VaultPath) -> bool {
    let mut segments = path.segments();
    segments.next() == Some(CONFLICT_ROOT_SEGMENT)
        && segments.next() == Some(CONFLICT_OPEN_SEGMENT)
        && segments.next().is_some()
}

pub fn validate_original_path(path: &VaultPath) -> Result<(), ConflictPolicyError> {
    if is_conflict_area_path(path) {
        return Err(ConflictPolicyError::RecursiveConflictPath);
    }
    Ok(())
}

/// Build a storage/API-neutral plan for a conflict resolution action.
pub fn plan_conflict_resolution(
    request: ConflictResolutionRequest,
) -> Result<ConflictResolutionPlan, ConflictResolutionError> {
    validate_resolution_conflict_record(&request.conflict)?;

    let ConflictResolutionRequest {
        conflict,
        action,
        resolved_at,
    } = request;

    let revision_effect = match action {
        ConflictResolutionAction::AcceptConflict => {
            ConflictResolutionRevisionEffect::CreateCurrentRevisionFromConflict {
                new_revision: NewCurrentRevisionPlan {
                    path: conflict.original_path.clone(),
                    parent_revision_id: conflict.current_revision_id.clone(),
                    content_hash: conflict.incoming_content_hash,
                    size_bytes: conflict.incoming_size_bytes,
                    created_by: conflict.adapter_id.clone(),
                },
            }
        }
        ConflictResolutionAction::AcceptCurrent
        | ConflictResolutionAction::KeepBoth
        | ConflictResolutionAction::MarkResolved => {
            ConflictResolutionRevisionEffect::MetadataOnlyCurrentUnchanged {
                current_revision_id: conflict.current_revision_id.clone(),
            }
        }
    };

    let conflict_copy_disposition = match action {
        ConflictResolutionAction::AcceptCurrent | ConflictResolutionAction::AcceptConflict => {
            ConflictCopyDisposition::MarkConflictCopyResolved
        }
        ConflictResolutionAction::KeepBoth => ConflictCopyDisposition::KeepMaterializedConflictCopy,
        ConflictResolutionAction::MarkResolved => ConflictCopyDisposition::NoConflictCopyChange,
    };

    Ok(ConflictResolutionPlan {
        conflict_id: conflict.conflict_id.clone(),
        action,
        original_path: conflict.original_path.clone(),
        conflict_path: conflict.conflict_path.clone(),
        status_update: ConflictStatusUpdatePlan {
            conflict_id: conflict.conflict_id,
            from_status: ConflictRecordStatus::Open,
            to_status: ConflictRecordStatus::Resolved,
            resolved_at,
        },
        revision_effect,
        conflict_copy_disposition,
    })
}

fn validate_resolution_conflict_record(
    conflict: &ConflictRecord,
) -> Result<(), ConflictResolutionError> {
    if conflict.status != ConflictRecordStatus::Open {
        return Err(ConflictResolutionError::ConflictNotOpen {
            status: conflict.status,
        });
    }
    validate_original_path(&conflict.original_path)?;
    if !is_open_conflict_area_path(&conflict.conflict_path) {
        return Err(ConflictResolutionError::InvalidConflictMaterializationPath);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn timestamp() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-07-09T10:30:45Z")
            .expect("fixture timestamp should parse")
            .with_timezone(&Utc)
    }

    fn resolved_at() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-07-09T11:00:00Z")
            .expect("fixture timestamp should parse")
            .with_timezone(&Utc)
    }

    fn vault_path(value: &str) -> VaultPath {
        VaultPath::parse(value).expect("fixture path should parse")
    }

    fn adapter_id() -> AdapterId {
        AdapterId::parse("gdrive-adapter").expect("fixture adapter id should parse")
    }

    fn revision_id(value: &str) -> RevisionId {
        RevisionId::parse(value).expect("fixture revision id should parse")
    }

    fn conflict_id() -> ConflictId {
        ConflictId::parse("conf_01JTEST").expect("fixture conflict id should parse")
    }

    fn content_hash(byte: u8) -> ContentHash {
        ContentHash::from_bytes([byte; 32])
    }

    fn conflict_record(status: ConflictRecordStatus) -> ConflictRecord {
        ConflictRecord {
            conflict_id: conflict_id(),
            original_path: vault_path("Notes/a.md"),
            conflict_path: vault_path(
                "_haze_conflicts/open/Notes/a.conflict.gdrive-adapter.2026-07-09-103045.md",
            ),
            base_revision_id: Some(revision_id("rev_01JOLD")),
            current_revision_id: revision_id("rev_01JCURRENT"),
            incoming_content_hash: content_hash(7),
            incoming_size_bytes: 42,
            adapter_id: adapter_id(),
            policy_applied: ConflictPolicy::PreserveBoth,
            status,
            created_at: timestamp(),
        }
    }

    #[test]
    fn conflict_path_generation_preserves_nested_path_extension_and_timestamp() {
        let request =
            ConflictPathRequest::parse("Folder/Sub/archive.tar.gz", "gdrive-adapter", timestamp())
                .expect("request should parse");

        let conflict_path = generate_conflict_path(&request).expect("path should generate");

        assert_eq!(
            conflict_path.as_str(),
            "_haze_conflicts/open/Folder/Sub/archive.tar.conflict.gdrive-adapter.2026-07-09-103045.gz"
        );
        assert!(is_open_conflict_area_path(&conflict_path));
        assert_eq!(
            conflict_path.segments().take(2).collect::<Vec<_>>(),
            vec![CONFLICT_ROOT_SEGMENT, CONFLICT_OPEN_SEGMENT]
        );
    }

    #[test]
    fn conflict_path_generation_handles_files_without_extensions() {
        let request = ConflictPathRequest::parse("Notes/README", "worktree-adapter", timestamp())
            .expect("request should parse");

        let conflict_path = generate_conflict_path(&request).expect("path should generate");

        assert_eq!(
            conflict_path.as_str(),
            "_haze_conflicts/open/Notes/README.conflict.worktree-adapter.2026-07-09-103045"
        );
        assert!(VaultPath::parse(conflict_path.as_str()).is_ok());
    }

    #[test]
    fn conflict_path_generation_rejects_unsafe_adapter_ids() {
        let error = ConflictPathRequest::parse("Notes/a.md", "gdrive.adapter", timestamp())
            .expect_err("unsafe adapter id should reject");

        assert_eq!(error, ConflictPolicyError::UnsafeAdapterId);
    }

    #[test]
    fn conflict_path_generation_rejects_recursive_conflict_area_input() {
        let error = ConflictPathRequest::parse(
            "_haze_conflicts/open/Notes/a.md",
            "gdrive-adapter",
            timestamp(),
        )
        .expect_err("recursive conflict-area input should reject");

        assert_eq!(error, ConflictPolicyError::RecursiveConflictPath);
    }

    #[test]
    fn open_conflict_area_requires_a_materialized_path_segment() {
        assert!(!is_open_conflict_area_path(&vault_path(
            "_haze_conflicts/open"
        )));
        assert!(is_open_conflict_area_path(&vault_path(
            "_haze_conflicts/open/a.md"
        )));
    }

    #[test]
    fn conflict_resolution_actions_have_stable_serde_names() {
        for (action, expected_name) in [
            (ConflictResolutionAction::AcceptCurrent, "accept_current"),
            (ConflictResolutionAction::AcceptConflict, "accept_conflict"),
            (ConflictResolutionAction::KeepBoth, "keep_both"),
            (ConflictResolutionAction::MarkResolved, "mark_resolved"),
        ] {
            let serialized = serde_json::to_string(&action).expect("action should serialize");
            assert_eq!(serialized, format!("\"{expected_name}\""));
            assert_eq!(
                serde_json::from_str::<ConflictResolutionAction>(&serialized)
                    .expect("action should deserialize"),
                action
            );
        }
    }

    #[test]
    fn accept_conflict_resolution_requires_new_current_revision_plan() {
        let request = ConflictResolutionRequest::new(
            conflict_record(ConflictRecordStatus::Open),
            ConflictResolutionAction::AcceptConflict,
            resolved_at(),
        )
        .expect("resolution request should be valid");

        let plan = plan_conflict_resolution(request).expect("resolution should plan");

        assert_eq!(plan.action, ConflictResolutionAction::AcceptConflict);
        assert!(plan.requires_new_current_revision());
        assert_eq!(
            plan.conflict_copy_disposition,
            ConflictCopyDisposition::MarkConflictCopyResolved
        );
        assert_eq!(plan.status_update.from_status, ConflictRecordStatus::Open);
        assert_eq!(plan.status_update.to_status, ConflictRecordStatus::Resolved);
        let ConflictResolutionRevisionEffect::CreateCurrentRevisionFromConflict { new_revision } =
            plan.revision_effect
        else {
            panic!("accept_conflict should create a new current revision plan");
        };
        assert_eq!(new_revision.path, vault_path("Notes/a.md"));
        assert_eq!(
            new_revision.parent_revision_id,
            revision_id("rev_01JCURRENT")
        );
        assert_eq!(new_revision.content_hash, content_hash(7));
        assert_eq!(new_revision.size_bytes, 42);
        assert_eq!(new_revision.created_by, adapter_id());
    }

    #[test]
    fn metadata_only_resolution_actions_do_not_create_new_current_revision() {
        for (action, expected_copy_disposition) in [
            (
                ConflictResolutionAction::AcceptCurrent,
                ConflictCopyDisposition::MarkConflictCopyResolved,
            ),
            (
                ConflictResolutionAction::KeepBoth,
                ConflictCopyDisposition::KeepMaterializedConflictCopy,
            ),
            (
                ConflictResolutionAction::MarkResolved,
                ConflictCopyDisposition::NoConflictCopyChange,
            ),
        ] {
            let request = ConflictResolutionRequest::new(
                conflict_record(ConflictRecordStatus::Open),
                action,
                resolved_at(),
            )
            .expect("resolution request should be valid");

            let plan = plan_conflict_resolution(request).expect("resolution should plan");

            assert_eq!(plan.action, action);
            assert!(!action.requires_new_current_revision());
            assert!(!plan.requires_new_current_revision());
            assert_eq!(plan.conflict_copy_disposition, expected_copy_disposition);
            assert_eq!(plan.status_update.to_status, ConflictRecordStatus::Resolved);
            let ConflictResolutionRevisionEffect::MetadataOnlyCurrentUnchanged {
                current_revision_id,
            } = plan.revision_effect
            else {
                panic!("metadata-only actions should not create current revisions");
            };
            assert_eq!(current_revision_id, revision_id("rev_01JCURRENT"));
        }
    }

    #[test]
    fn resolution_rejects_non_open_conflicts() {
        let error = ConflictResolutionRequest::new(
            conflict_record(ConflictRecordStatus::Resolved),
            ConflictResolutionAction::MarkResolved,
            resolved_at(),
        )
        .expect_err("already resolved conflicts should reject");

        assert_eq!(
            error,
            ConflictResolutionError::ConflictNotOpen {
                status: ConflictRecordStatus::Resolved
            }
        );
    }

    #[test]
    fn resolution_rejects_conflict_paths_outside_open_area() {
        let mut conflict = conflict_record(ConflictRecordStatus::Open);
        conflict.conflict_path = vault_path("_haze_conflicts/closed/Notes/a.md");

        let error = ConflictResolutionRequest::new(
            conflict,
            ConflictResolutionAction::MarkResolved,
            resolved_at(),
        )
        .expect_err("invalid conflict materialization path should reject");

        assert_eq!(
            error,
            ConflictResolutionError::InvalidConflictMaterializationPath
        );
    }

    #[test]
    fn resolution_rejects_open_conflict_area_root_without_materialized_copy() {
        let mut conflict = conflict_record(ConflictRecordStatus::Open);
        conflict.conflict_path = vault_path("_haze_conflicts/open");

        let error = ConflictResolutionRequest::new(
            conflict,
            ConflictResolutionAction::MarkResolved,
            resolved_at(),
        )
        .expect_err("open conflict-area root should not count as a conflict copy");

        assert_eq!(
            error,
            ConflictResolutionError::InvalidConflictMaterializationPath
        );
    }
}
