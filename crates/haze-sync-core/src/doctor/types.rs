use haze_sync_common::ContentHash;
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;

/// Stable identifier for a passive doctor check.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DoctorCheckId {
    DbConnectivity,
    ObjectStoreExistsWritable,
    MissingBlobs,
    AdapterCursors,
    GdriveMapping,
    AdapterTokenSanity,
    WorktreeDrift,
}

impl DoctorCheckId {
    /// Stable wire identifier for this check.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DbConnectivity => "db_connectivity",
            Self::ObjectStoreExistsWritable => "object_store_exists_writable",
            Self::MissingBlobs => "missing_blobs",
            Self::AdapterCursors => "adapter_cursors",
            Self::GdriveMapping => "gdrive_mapping",
            Self::AdapterTokenSanity => "adapter_token_sanity",
            Self::WorktreeDrift => "worktree_drift",
        }
    }
}

impl fmt::Display for DoctorCheckId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Public status for one doctor check or an aggregate doctor report.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DoctorCheckStatus {
    /// Check completed successfully.
    Ok,
    /// Check completed but reported a recoverable concern.
    Warning,
    /// Check completed and found a blocking or integrity problem.
    Failed,
    /// Check was intentionally omitted because it was disabled or not applicable.
    Skipped,
    /// Check is supported but was not attempted or did not provide a result.
    NotRun,
    /// Check is part of the accepted contract but downstream integration is pending.
    Placeholder,
}

impl DoctorCheckStatus {
    /// Stable wire identifier for this status.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Warning => "warning",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
            Self::NotRun => "not_run",
            Self::Placeholder => "placeholder",
        }
    }
}

/// Safe reason why a supported doctor check was not run.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DoctorNotRunReason {
    NotRequested,
    ToolingUnavailable,
    DependencyUnavailable,
}

/// Safe reason why a doctor check is represented only as a placeholder.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DoctorPlaceholderReason {
    IntegrationPending,
}

/// Redacted database connectivity check details.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DbConnectivityDetails {
    pub metadata_configured: bool,
    pub live_check_performed: bool,
    pub connectivity_verified: Option<bool>,
}

/// Redacted object-store existence/writability details.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ObjectStoreExistsWritableDetails {
    pub configured: bool,
    pub exists: Option<bool>,
    pub writable: Option<bool>,
}

/// Public missing-blob detection details.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MissingBlobDetectionDetails {
    pub input_count: u64,
    pub missing_count: u64,
    pub sample_hashes: Vec<ContentHash>,
}

/// Redacted adapter cursor integrity details.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AdapterCursorDetails {
    pub expected_adapter_count: u64,
    pub cursor_count: u64,
    pub missing_cursor_count: u64,
    pub orphaned_cursor_count: u64,
    pub invalid_cursor_count: u64,
    pub stale_cursor_count: u64,
}

/// Redacted Google Drive mapping integrity details.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GdriveMappingDetails {
    pub configured: bool,
    pub mapping_count: u64,
    pub unknown_revision_count: u64,
    pub duplicate_drive_file_id_count: u64,
    pub unmapped_item_count: u64,
}

/// Redacted adapter token sanity details.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AdapterTokenSanityDetails {
    pub enabled_adapter_count: u64,
    pub missing_token_hash_count: u64,
    pub invalid_role_count: u64,
}

/// Redacted worktree drift details.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorktreeDriftDetails {
    pub configured: bool,
    pub tracked_path_count: u64,
    pub unknown_revision_count: u64,
    pub missing_path_count: u64,
    pub content_mismatch_count: u64,
    pub unexpected_path_count: u64,
}

/// Safe detail payload for a supported check that was not attempted.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DoctorNotRunDetails {
    pub reason: DoctorNotRunReason,
}

/// Safe detail payload for an accepted check whose integration is pending.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DoctorPlaceholderDetails {
    pub reason: DoctorPlaceholderReason,
}

/// JSON-safe detail payload for a doctor check.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DoctorCheckDetails {
    DbConnectivity(DbConnectivityDetails),
    ObjectStoreExistsWritable(ObjectStoreExistsWritableDetails),
    MissingBlobs(MissingBlobDetectionDetails),
    AdapterCursors(AdapterCursorDetails),
    GdriveMapping(GdriveMappingDetails),
    AdapterTokenSanity(AdapterTokenSanityDetails),
    WorktreeDrift(WorktreeDriftDetails),
    NotRun(DoctorNotRunDetails),
    Placeholder(DoctorPlaceholderDetails),
}

impl DoctorCheckDetails {
    fn fixed_check_id(&self) -> Option<DoctorCheckId> {
        match self {
            Self::DbConnectivity(_) => Some(DoctorCheckId::DbConnectivity),
            Self::ObjectStoreExistsWritable(_) => {
                Some(DoctorCheckId::ObjectStoreExistsWritable)
            }
            Self::MissingBlobs(_) => Some(DoctorCheckId::MissingBlobs),
            Self::AdapterCursors(_) => Some(DoctorCheckId::AdapterCursors),
            Self::GdriveMapping(_) => Some(DoctorCheckId::GdriveMapping),
            Self::AdapterTokenSanity(_) => Some(DoctorCheckId::AdapterTokenSanity),
            Self::WorktreeDrift(_) => Some(DoctorCheckId::WorktreeDrift),
            Self::NotRun(_) | Self::Placeholder(_) => None,
        }
    }
}

/// Fixed, redacted doctor message vocabulary.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum DoctorCheckMessage {
    #[serde(rename = "database metadata is not configured")]
    DatabaseMetadataNotConfigured,
    #[serde(rename = "database connectivity check skipped in offline mode")]
    DatabaseConnectivitySkippedOffline,
    #[serde(rename = "database connectivity verified")]
    DatabaseConnectivityVerified,
    #[serde(rename = "database connectivity check failed")]
    DatabaseConnectivityFailed,
    #[serde(rename = "database connectivity result was not provided")]
    DatabaseConnectivityNotRun,
    #[serde(rename = "object store root is not configured")]
    ObjectStoreNotConfigured,
    #[serde(rename = "object store filesystem check skipped in offline mode")]
    ObjectStoreSkippedOffline,
    #[serde(rename = "object store root does not exist")]
    ObjectStoreMissing,
    #[serde(rename = "object store root is not writable")]
    ObjectStoreNotWritable,
    #[serde(rename = "object store root is accessible")]
    ObjectStoreAccessible,
    #[serde(rename = "object store filesystem result was incomplete")]
    ObjectStoreNotRun,
    #[serde(rename = "no missing blobs detected")]
    NoMissingBlobs,
    #[serde(rename = "missing required blobs detected")]
    MissingBlobsDetected,
    #[serde(rename = "no adapters require cursor validation")]
    NoAdaptersForCursorValidation,
    #[serde(rename = "invalid adapter cursor detected")]
    InvalidAdapterCursorDetected,
    #[serde(rename = "required adapter cursor is missing")]
    MissingAdapterCursorDetected,
    #[serde(rename = "orphaned adapter cursor detected")]
    OrphanedAdapterCursorDetected,
    #[serde(rename = "adapter cursor appears stale")]
    StaleAdapterCursorDetected,
    #[serde(rename = "adapter cursors are valid")]
    AdapterCursorsValid,
    #[serde(rename = "Google Drive mapping check skipped because the adapter is not configured")]
    GdriveMappingSkipped,
    #[serde(rename = "Google Drive mapping references an unknown Core revision")]
    GdriveMappingUnknownRevision,
    #[serde(rename = "Google Drive mapping contains duplicate file identifiers")]
    GdriveMappingDuplicateFileId,
    #[serde(rename = "Google Drive mapping drift detected")]
    GdriveMappingDriftDetected,
    #[serde(rename = "Google Drive mapping is valid")]
    GdriveMappingValid,
    #[serde(rename = "enabled adapter has invalid role")]
    AdapterInvalidRole,
    #[serde(rename = "enabled adapter is missing a token hash")]
    AdapterMissingTokenHash,
    #[serde(rename = "no enabled adapters provided for token sanity check")]
    NoEnabledAdaptersForTokenSanity,
    #[serde(rename = "adapter token metadata is sane")]
    AdapterTokenMetadataSane,
    #[serde(rename = "worktree drift check skipped because the worktree is not configured")]
    WorktreeDriftSkipped,
    #[serde(rename = "worktree state references an unknown Core revision")]
    WorktreeUnknownRevision,
    #[serde(rename = "worktree drift detected")]
    WorktreeDriftDetected,
    #[serde(rename = "worktree state is consistent")]
    WorktreeStateConsistent,
    #[serde(rename = "doctor check was not run")]
    CheckNotRun,
    #[serde(rename = "doctor check integration is pending")]
    CheckPlaceholder,
}

impl DoctorCheckMessage {
    /// Stable redacted message text.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DatabaseMetadataNotConfigured => "database metadata is not configured",
            Self::DatabaseConnectivitySkippedOffline => {
                "database connectivity check skipped in offline mode"
            }
            Self::DatabaseConnectivityVerified => "database connectivity verified",
            Self::DatabaseConnectivityFailed => "database connectivity check failed",
            Self::DatabaseConnectivityNotRun => "database connectivity result was not provided",
            Self::ObjectStoreNotConfigured => "object store root is not configured",
            Self::ObjectStoreSkippedOffline => {
                "object store filesystem check skipped in offline mode"
            }
            Self::ObjectStoreMissing => "object store root does not exist",
            Self::ObjectStoreNotWritable => "object store root is not writable",
            Self::ObjectStoreAccessible => "object store root is accessible",
            Self::ObjectStoreNotRun => "object store filesystem result was incomplete",
            Self::NoMissingBlobs => "no missing blobs detected",
            Self::MissingBlobsDetected => "missing required blobs detected",
            Self::NoAdaptersForCursorValidation => "no adapters require cursor validation",
            Self::InvalidAdapterCursorDetected => "invalid adapter cursor detected",
            Self::MissingAdapterCursorDetected => "required adapter cursor is missing",
            Self::OrphanedAdapterCursorDetected => "orphaned adapter cursor detected",
            Self::StaleAdapterCursorDetected => "adapter cursor appears stale",
            Self::AdapterCursorsValid => "adapter cursors are valid",
            Self::GdriveMappingSkipped => {
                "Google Drive mapping check skipped because the adapter is not configured"
            }
            Self::GdriveMappingUnknownRevision => {
                "Google Drive mapping references an unknown Core revision"
            }
            Self::GdriveMappingDuplicateFileId => {
                "Google Drive mapping contains duplicate file identifiers"
            }
            Self::GdriveMappingDriftDetected => "Google Drive mapping drift detected",
            Self::GdriveMappingValid => "Google Drive mapping is valid",
            Self::AdapterInvalidRole => "enabled adapter has invalid role",
            Self::AdapterMissingTokenHash => "enabled adapter is missing a token hash",
            Self::NoEnabledAdaptersForTokenSanity => {
                "no enabled adapters provided for token sanity check"
            }
            Self::AdapterTokenMetadataSane => "adapter token metadata is sane",
            Self::WorktreeDriftSkipped => {
                "worktree drift check skipped because the worktree is not configured"
            }
            Self::WorktreeUnknownRevision => {
                "worktree state references an unknown Core revision"
            }
            Self::WorktreeDriftDetected => "worktree drift detected",
            Self::WorktreeStateConsistent => "worktree state is consistent",
            Self::CheckNotRun => "doctor check was not run",
            Self::CheckPlaceholder => "doctor check integration is pending",
        }
    }

    fn is_valid_for(self, check_id: DoctorCheckId, status: DoctorCheckStatus) -> bool {
        matches!(
            (self, check_id, status),
            (
                Self::DatabaseMetadataNotConfigured,
                DoctorCheckId::DbConnectivity,
                DoctorCheckStatus::Warning
            ) | (
                Self::DatabaseConnectivitySkippedOffline,
                DoctorCheckId::DbConnectivity,
                DoctorCheckStatus::Skipped
            ) | (
                Self::DatabaseConnectivityVerified,
                DoctorCheckId::DbConnectivity,
                DoctorCheckStatus::Ok
            ) | (
                Self::DatabaseConnectivityFailed,
                DoctorCheckId::DbConnectivity,
                DoctorCheckStatus::Failed
            ) | (
                Self::DatabaseConnectivityNotRun,
                DoctorCheckId::DbConnectivity,
                DoctorCheckStatus::NotRun
            ) | (
                Self::ObjectStoreNotConfigured,
                DoctorCheckId::ObjectStoreExistsWritable,
                DoctorCheckStatus::Warning
            ) | (
                Self::ObjectStoreSkippedOffline,
                DoctorCheckId::ObjectStoreExistsWritable,
                DoctorCheckStatus::Skipped
            ) | (
                Self::ObjectStoreMissing,
                DoctorCheckId::ObjectStoreExistsWritable,
                DoctorCheckStatus::Failed
            ) | (
                Self::ObjectStoreNotWritable,
                DoctorCheckId::ObjectStoreExistsWritable,
                DoctorCheckStatus::Failed
            ) | (
                Self::ObjectStoreAccessible,
                DoctorCheckId::ObjectStoreExistsWritable,
                DoctorCheckStatus::Ok
            ) | (
                Self::ObjectStoreNotRun,
                DoctorCheckId::ObjectStoreExistsWritable,
                DoctorCheckStatus::NotRun
            ) | (
                Self::NoMissingBlobs,
                DoctorCheckId::MissingBlobs,
                DoctorCheckStatus::Ok
            ) | (
                Self::MissingBlobsDetected,
                DoctorCheckId::MissingBlobs,
                DoctorCheckStatus::Failed
            ) | (
                Self::NoAdaptersForCursorValidation,
                DoctorCheckId::AdapterCursors,
                DoctorCheckStatus::Skipped
            ) | (
                Self::InvalidAdapterCursorDetected,
                DoctorCheckId::AdapterCursors,
                DoctorCheckStatus::Failed
            ) | (
                Self::MissingAdapterCursorDetected,
                DoctorCheckId::AdapterCursors,
                DoctorCheckStatus::Warning
            ) | (
                Self::OrphanedAdapterCursorDetected,
                DoctorCheckId::AdapterCursors,
                DoctorCheckStatus::Warning
            ) | (
                Self::StaleAdapterCursorDetected,
                DoctorCheckId::AdapterCursors,
                DoctorCheckStatus::Warning
            ) | (
                Self::AdapterCursorsValid,
                DoctorCheckId::AdapterCursors,
                DoctorCheckStatus::Ok
            ) | (
                Self::GdriveMappingSkipped,
                DoctorCheckId::GdriveMapping,
                DoctorCheckStatus::Skipped
            ) | (
                Self::GdriveMappingUnknownRevision,
                DoctorCheckId::GdriveMapping,
                DoctorCheckStatus::Failed
            ) | (
                Self::GdriveMappingDuplicateFileId,
                DoctorCheckId::GdriveMapping,
                DoctorCheckStatus::Failed
            ) | (
                Self::GdriveMappingDriftDetected,
                DoctorCheckId::GdriveMapping,
                DoctorCheckStatus::Warning
            ) | (
                Self::GdriveMappingValid,
                DoctorCheckId::GdriveMapping,
                DoctorCheckStatus::Ok
            ) | (
                Self::AdapterInvalidRole,
                DoctorCheckId::AdapterTokenSanity,
                DoctorCheckStatus::Failed
            ) | (
                Self::AdapterMissingTokenHash,
                DoctorCheckId::AdapterTokenSanity,
                DoctorCheckStatus::Warning
            ) | (
                Self::NoEnabledAdaptersForTokenSanity,
                DoctorCheckId::AdapterTokenSanity,
                DoctorCheckStatus::Skipped
            ) | (
                Self::AdapterTokenMetadataSane,
                DoctorCheckId::AdapterTokenSanity,
                DoctorCheckStatus::Ok
            ) | (
                Self::WorktreeDriftSkipped,
                DoctorCheckId::WorktreeDrift,
                DoctorCheckStatus::Skipped
            ) | (
                Self::WorktreeUnknownRevision,
                DoctorCheckId::WorktreeDrift,
                DoctorCheckStatus::Failed
            ) | (
                Self::WorktreeDriftDetected,
                DoctorCheckId::WorktreeDrift,
                DoctorCheckStatus::Warning
            ) | (
                Self::WorktreeStateConsistent,
                DoctorCheckId::WorktreeDrift,
                DoctorCheckStatus::Ok
            ) | (Self::CheckNotRun, _, DoctorCheckStatus::NotRun)
                | (Self::CheckPlaceholder, _, DoctorCheckStatus::Placeholder)
        )
    }
}

impl fmt::Display for DoctorCheckMessage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// JSON-safe doctor check result.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DoctorCheckResult {
    check_id: DoctorCheckId,
    status: DoctorCheckStatus,
    message: DoctorCheckMessage,
    details: DoctorCheckDetails,
}

impl DoctorCheckResult {
    pub(crate) fn classified(
        check_id: DoctorCheckId,
        status: DoctorCheckStatus,
        message: DoctorCheckMessage,
        details: DoctorCheckDetails,
    ) -> Self {
        debug_assert!(message.is_valid_for(check_id, status));
        if let Some(details_id) = details.fixed_check_id() {
            debug_assert_eq!(details_id, check_id);
        }

        Self {
            check_id,
            status,
            message,
            details,
        }
    }

    /// Creates a safe result for a supported check that was not attempted.
    #[must_use]
    pub fn not_run(check_id: DoctorCheckId, reason: DoctorNotRunReason) -> Self {
        Self::classified(
            check_id,
            DoctorCheckStatus::NotRun,
            DoctorCheckMessage::CheckNotRun,
            DoctorCheckDetails::NotRun(DoctorNotRunDetails { reason }),
        )
    }

    /// Creates a safe result for an accepted check whose integration is pending.
    #[must_use]
    pub fn placeholder(check_id: DoctorCheckId) -> Self {
        Self::classified(
            check_id,
            DoctorCheckStatus::Placeholder,
            DoctorCheckMessage::CheckPlaceholder,
            DoctorCheckDetails::Placeholder(DoctorPlaceholderDetails {
                reason: DoctorPlaceholderReason::IntegrationPending,
            }),
        )
    }

    #[must_use]
    pub const fn check_id(&self) -> DoctorCheckId {
        self.check_id
    }

    #[must_use]
    pub const fn status(&self) -> DoctorCheckStatus {
        self.status
    }

    #[must_use]
    pub const fn message_code(&self) -> DoctorCheckMessage {
        self.message
    }

    #[must_use]
    pub const fn message(&self) -> &'static str {
        self.message.as_str()
    }

    #[must_use]
    pub const fn details(&self) -> &DoctorCheckDetails {
        &self.details
    }
}

impl<'de> Deserialize<'de> for DoctorCheckResult {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct DoctorCheckResultWire {
            check_id: DoctorCheckId,
            status: DoctorCheckStatus,
            message: DoctorCheckMessage,
            details: DoctorCheckDetails,
        }

        let wire = DoctorCheckResultWire::deserialize(deserializer)?;
        if !wire.message.is_valid_for(wire.check_id, wire.status) {
            return Err(serde::de::Error::custom(
                "doctor message does not match check and status",
            ));
        }
        if let Some(details_id) = wire.details.fixed_check_id() {
            if details_id != wire.check_id {
                return Err(serde::de::Error::custom(
                    "doctor details do not match check identifier",
                ));
            }
        }
        match (&wire.details, wire.status) {
            (DoctorCheckDetails::NotRun(_), DoctorCheckStatus::NotRun)
            | (DoctorCheckDetails::Placeholder(_), DoctorCheckStatus::Placeholder) => {}
            (DoctorCheckDetails::NotRun(_), _) | (DoctorCheckDetails::Placeholder(_), _) => {
                return Err(serde::de::Error::custom(
                    "doctor execution details do not match status",
                ));
            }
            _ => {}
        }

        Ok(Self {
            check_id: wire.check_id,
            status: wire.status,
            message: wire.message,
            details: wire.details,
        })
    }
}
