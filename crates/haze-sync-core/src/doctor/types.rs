use haze_sync_common::ContentHash;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, ops::Deref};

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
            Self::ObjectStoreExistsWritable(_) => Some(DoctorCheckId::ObjectStoreExistsWritable),
            Self::MissingBlobs(_) => Some(DoctorCheckId::MissingBlobs),
            Self::AdapterCursors(_) => Some(DoctorCheckId::AdapterCursors),
            Self::GdriveMapping(_) => Some(DoctorCheckId::GdriveMapping),
            Self::AdapterTokenSanity(_) => Some(DoctorCheckId::AdapterTokenSanity),
            Self::WorktreeDrift(_) => Some(DoctorCheckId::WorktreeDrift),
            Self::NotRun(_) | Self::Placeholder(_) => None,
        }
    }

    fn validate_for(
        &self,
        check_id: DoctorCheckId,
        status: DoctorCheckStatus,
        message: DoctorCheckMessage,
    ) -> Result<(), &'static str> {
        if let Some(details_id) = self.fixed_check_id() {
            if details_id != check_id {
                return Err("doctor details do not match check identifier");
            }
        }

        let valid = match self {
            Self::DbConnectivity(details) => {
                db_connectivity_details_match(details, status, message)
            }
            Self::ObjectStoreExistsWritable(details) => {
                classification_matches(object_store_classification(details), status, message)
            }
            Self::MissingBlobs(details) => {
                classification_matches(missing_blob_classification(details), status, message)
            }
            Self::AdapterCursors(details) => {
                classification_matches(adapter_cursor_classification(details), status, message)
            }
            Self::GdriveMapping(details) => {
                classification_matches(gdrive_mapping_classification(details), status, message)
            }
            Self::AdapterTokenSanity(details) => {
                classification_matches(adapter_token_classification(details), status, message)
            }
            Self::WorktreeDrift(details) => {
                classification_matches(worktree_drift_classification(details), status, message)
            }
            Self::NotRun(_) => {
                status == DoctorCheckStatus::NotRun && message == DoctorCheckMessage::CheckNotRun
            }
            Self::Placeholder(_) => {
                status == DoctorCheckStatus::Placeholder
                    && message == DoctorCheckMessage::CheckPlaceholder
            }
        };

        if valid {
            Ok(())
        } else {
            Err("doctor details do not match status and message")
        }
    }
}

macro_rules! define_doctor_messages {
    ($( $variant:ident => ($wire:literal, $status:ident, $check_id:expr), )+) => {
        /// Fixed, redacted doctor message vocabulary.
        #[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
        pub enum DoctorCheckMessage {
            $(
                #[serde(rename = $wire)]
                $variant,
            )+
        }

        impl DoctorCheckMessage {
            /// Stable redacted message text.
            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $wire,)+
                }
            }

            const fn status(self) -> DoctorCheckStatus {
                match self {
                    $(Self::$variant => DoctorCheckStatus::$status,)+
                }
            }

            const fn fixed_check_id(self) -> Option<DoctorCheckId> {
                match self {
                    $(Self::$variant => $check_id,)+
                }
            }

            fn is_valid_for(self, check_id: DoctorCheckId, status: DoctorCheckStatus) -> bool {
                self.status() == status
                    && match self.fixed_check_id() {
                        Some(expected_check_id) => expected_check_id == check_id,
                        None => true,
                    }
            }
        }
    };
}

define_doctor_messages! {
    DatabaseMetadataNotConfigured => (
        "database metadata is not configured",
        Warning,
        Some(DoctorCheckId::DbConnectivity)
    ),
    DatabaseConnectivitySkippedOffline => (
        "database connectivity check skipped in offline mode",
        Skipped,
        Some(DoctorCheckId::DbConnectivity)
    ),
    DatabaseConnectivityVerified => (
        "database connectivity verified",
        Ok,
        Some(DoctorCheckId::DbConnectivity)
    ),
    DatabaseConnectivityFailed => (
        "database connectivity check failed",
        Failed,
        Some(DoctorCheckId::DbConnectivity)
    ),
    DatabaseConnectivityNotRun => (
        "database connectivity result was not provided",
        NotRun,
        Some(DoctorCheckId::DbConnectivity)
    ),
    ObjectStoreNotConfigured => (
        "object store root is not configured",
        Warning,
        Some(DoctorCheckId::ObjectStoreExistsWritable)
    ),
    ObjectStoreSkippedOffline => (
        "object store filesystem check skipped in offline mode",
        Skipped,
        Some(DoctorCheckId::ObjectStoreExistsWritable)
    ),
    ObjectStoreMissing => (
        "object store root does not exist",
        Failed,
        Some(DoctorCheckId::ObjectStoreExistsWritable)
    ),
    ObjectStoreNotWritable => (
        "object store root is not writable",
        Failed,
        Some(DoctorCheckId::ObjectStoreExistsWritable)
    ),
    ObjectStoreAccessible => (
        "object store root is accessible",
        Ok,
        Some(DoctorCheckId::ObjectStoreExistsWritable)
    ),
    ObjectStoreNotRun => (
        "object store filesystem result was incomplete",
        NotRun,
        Some(DoctorCheckId::ObjectStoreExistsWritable)
    ),
    NoMissingBlobs => (
        "no missing blobs detected",
        Ok,
        Some(DoctorCheckId::MissingBlobs)
    ),
    MissingBlobsDetected => (
        "missing required blobs detected",
        Failed,
        Some(DoctorCheckId::MissingBlobs)
    ),
    NoAdaptersForCursorValidation => (
        "no adapters require cursor validation",
        Skipped,
        Some(DoctorCheckId::AdapterCursors)
    ),
    InvalidAdapterCursorDetected => (
        "invalid adapter cursor detected",
        Failed,
        Some(DoctorCheckId::AdapterCursors)
    ),
    MissingAdapterCursorDetected => (
        "required adapter cursor is missing",
        Warning,
        Some(DoctorCheckId::AdapterCursors)
    ),
    OrphanedAdapterCursorDetected => (
        "orphaned adapter cursor detected",
        Warning,
        Some(DoctorCheckId::AdapterCursors)
    ),
    StaleAdapterCursorDetected => (
        "adapter cursor appears stale",
        Warning,
        Some(DoctorCheckId::AdapterCursors)
    ),
    AdapterCursorsValid => (
        "adapter cursors are valid",
        Ok,
        Some(DoctorCheckId::AdapterCursors)
    ),
    GdriveMappingSkipped => (
        "Google Drive mapping check skipped because the adapter is not configured",
        Skipped,
        Some(DoctorCheckId::GdriveMapping)
    ),
    GdriveMappingUnknownRevision => (
        "Google Drive mapping references an unknown Core revision",
        Failed,
        Some(DoctorCheckId::GdriveMapping)
    ),
    GdriveMappingDuplicateFileId => (
        "Google Drive mapping contains duplicate file identifiers",
        Failed,
        Some(DoctorCheckId::GdriveMapping)
    ),
    GdriveMappingDriftDetected => (
        "Google Drive mapping drift detected",
        Warning,
        Some(DoctorCheckId::GdriveMapping)
    ),
    GdriveMappingValid => (
        "Google Drive mapping is valid",
        Ok,
        Some(DoctorCheckId::GdriveMapping)
    ),
    AdapterInvalidRole => (
        "enabled adapter has invalid role",
        Failed,
        Some(DoctorCheckId::AdapterTokenSanity)
    ),
    AdapterMissingTokenHash => (
        "enabled adapter is missing a token hash",
        Warning,
        Some(DoctorCheckId::AdapterTokenSanity)
    ),
    NoEnabledAdaptersForTokenSanity => (
        "no enabled adapters provided for token sanity check",
        Skipped,
        Some(DoctorCheckId::AdapterTokenSanity)
    ),
    AdapterTokenMetadataSane => (
        "adapter token metadata is sane",
        Ok,
        Some(DoctorCheckId::AdapterTokenSanity)
    ),
    WorktreeDriftSkipped => (
        "worktree drift check skipped because the worktree is not configured",
        Skipped,
        Some(DoctorCheckId::WorktreeDrift)
    ),
    WorktreeUnknownRevision => (
        "worktree state references an unknown Core revision",
        Failed,
        Some(DoctorCheckId::WorktreeDrift)
    ),
    WorktreeDriftDetected => (
        "worktree drift detected",
        Warning,
        Some(DoctorCheckId::WorktreeDrift)
    ),
    WorktreeStateConsistent => (
        "worktree state is consistent",
        Ok,
        Some(DoctorCheckId::WorktreeDrift)
    ),
    CheckNotRun => ("doctor check was not run", NotRun, None),
    CheckPlaceholder => ("doctor check integration is pending", Placeholder, None),
}

impl fmt::Display for DoctorCheckMessage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl Deref for DoctorCheckMessage {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

/// JSON-safe doctor check result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DoctorCheckResult {
    pub check_id: DoctorCheckId,
    status: DoctorCheckStatus,
    pub message: DoctorCheckMessage,
    details: DoctorCheckDetails,
}

impl DoctorCheckResult {
    pub(super) fn classified(
        check_id: DoctorCheckId,
        status: DoctorCheckStatus,
        message: DoctorCheckMessage,
        details: DoctorCheckDetails,
    ) -> Self {
        let result = Self {
            check_id,
            status,
            message,
            details,
        };
        debug_assert!(result.validate().is_ok());
        result
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

    pub(super) fn validate(&self) -> Result<(), &'static str> {
        if !self.message.is_valid_for(self.check_id, self.status) {
            return Err("doctor message does not match check and status");
        }
        self.details
            .validate_for(self.check_id, self.status, self.message)
    }
}

#[derive(Serialize)]
struct DoctorCheckResultWireRef<'a> {
    check_id: DoctorCheckId,
    status: DoctorCheckStatus,
    message: DoctorCheckMessage,
    details: &'a DoctorCheckDetails,
}

impl Serialize for DoctorCheckResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        DoctorCheckResultWireRef {
            check_id: self.check_id,
            status: self.status,
            message: self.message,
            details: &self.details,
        }
        .serialize(serializer)
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
        let result = Self {
            check_id: wire.check_id,
            status: wire.status,
            message: wire.message,
            details: wire.details,
        };
        result.validate().map_err(serde::de::Error::custom)?;
        Ok(result)
    }
}

type Classification = (DoctorCheckStatus, DoctorCheckMessage);

fn classification_matches(
    expected: Option<Classification>,
    status: DoctorCheckStatus,
    message: DoctorCheckMessage,
) -> bool {
    expected == Some((status, message))
}

fn db_connectivity_details_match(
    details: &DbConnectivityDetails,
    status: DoctorCheckStatus,
    message: DoctorCheckMessage,
) -> bool {
    if details.live_check_performed != details.connectivity_verified.is_some() {
        return false;
    }

    match (status, message) {
        (
            DoctorCheckStatus::Warning,
            DoctorCheckMessage::DatabaseMetadataNotConfigured,
        ) => {
            !details.metadata_configured
                && !details.live_check_performed
                && details.connectivity_verified.is_none()
        }
        (
            DoctorCheckStatus::Skipped,
            DoctorCheckMessage::DatabaseConnectivitySkippedOffline,
        )
        | (
            DoctorCheckStatus::NotRun,
            DoctorCheckMessage::DatabaseConnectivityNotRun,
        ) => {
            details.metadata_configured
                && !details.live_check_performed
                && details.connectivity_verified.is_none()
        }
        (
            DoctorCheckStatus::Ok,
            DoctorCheckMessage::DatabaseConnectivityVerified,
        ) => {
            details.metadata_configured
                && details.live_check_performed
                && details.connectivity_verified == Some(true)
        }
        (
            DoctorCheckStatus::Failed,
            DoctorCheckMessage::DatabaseConnectivityFailed,
        ) => {
            details.metadata_configured
                && details.live_check_performed
                && details.connectivity_verified == Some(false)
        }
        _ => false,
    }
}

fn object_store_classification(details: &ObjectStoreExistsWritableDetails) -> Option<Classification> {
    if !details.configured {
        return (details.exists.is_none() && details.writable.is_none()).then_some((
            DoctorCheckStatus::Warning,
            DoctorCheckMessage::ObjectStoreNotConfigured,
        ));
    }
    if details.exists == Some(false) {
        return details.writable.is_none().then_some((
            DoctorCheckStatus::Failed,
            DoctorCheckMessage::ObjectStoreMissing,
        ));
    }
    if details.writable == Some(false) {
        return Some((
            DoctorCheckStatus::Failed,
            DoctorCheckMessage::ObjectStoreNotWritable,
        ));
    }
    if details.exists == Some(true) && details.writable == Some(true) {
        return Some((
            DoctorCheckStatus::Ok,
            DoctorCheckMessage::ObjectStoreAccessible,
        ));
    }
    if details.exists.is_none() && details.writable.is_none() {
        return Some((
            DoctorCheckStatus::Skipped,
            DoctorCheckMessage::ObjectStoreSkippedOffline,
        ));
    }
    Some((
        DoctorCheckStatus::NotRun,
        DoctorCheckMessage::ObjectStoreNotRun,
    ))
}

fn missing_blob_classification(details: &MissingBlobDetectionDetails) -> Option<Classification> {
    let sample_count = u64::try_from(details.sample_hashes.len()).unwrap_or(u64::MAX);
    let sorted_unique = details
        .sample_hashes
        .windows(2)
        .all(|window| window[0] < window[1]);
    if !sorted_unique
        || sample_count > details.missing_count
        || (details.missing_count == 0 && !details.sample_hashes.is_empty())
    {
        return None;
    }

    if details.missing_count == 0 {
        Some((DoctorCheckStatus::Ok, DoctorCheckMessage::NoMissingBlobs))
    } else {
        Some((
            DoctorCheckStatus::Failed,
            DoctorCheckMessage::MissingBlobsDetected,
        ))
    }
}

fn adapter_cursor_classification(details: &AdapterCursorDetails) -> Option<Classification> {
    let expected_missing = details
        .expected_adapter_count
        .saturating_sub(details.cursor_count);
    let expected_orphaned = details
        .cursor_count
        .saturating_sub(details.expected_adapter_count);
    if details.missing_cursor_count != expected_missing
        || details.orphaned_cursor_count != expected_orphaned
    {
        return None;
    }

    if details.invalid_cursor_count > 0 {
        Some((
            DoctorCheckStatus::Failed,
            DoctorCheckMessage::InvalidAdapterCursorDetected,
        ))
    } else if details.missing_cursor_count > 0 {
        Some((
            DoctorCheckStatus::Warning,
            DoctorCheckMessage::MissingAdapterCursorDetected,
        ))
    } else if details.orphaned_cursor_count > 0 {
        Some((
            DoctorCheckStatus::Warning,
            DoctorCheckMessage::OrphanedAdapterCursorDetected,
        ))
    } else if details.stale_cursor_count > 0 {
        Some((
            DoctorCheckStatus::Warning,
            DoctorCheckMessage::StaleAdapterCursorDetected,
        ))
    } else if details.expected_adapter_count == 0 && details.cursor_count == 0 {
        Some((
            DoctorCheckStatus::Skipped,
            DoctorCheckMessage::NoAdaptersForCursorValidation,
        ))
    } else {
        Some((
            DoctorCheckStatus::Ok,
            DoctorCheckMessage::AdapterCursorsValid,
        ))
    }
}

fn gdrive_mapping_classification(details: &GdriveMappingDetails) -> Option<Classification> {
    if !details.configured {
        let no_facts = details.mapping_count == 0
            && details.unknown_revision_count == 0
            && details.duplicate_drive_file_id_count == 0
            && details.unmapped_item_count == 0;
        return no_facts.then_some((
            DoctorCheckStatus::Skipped,
            DoctorCheckMessage::GdriveMappingSkipped,
        ));
    }
    if details.unknown_revision_count > 0 {
        Some((
            DoctorCheckStatus::Failed,
            DoctorCheckMessage::GdriveMappingUnknownRevision,
        ))
    } else if details.duplicate_drive_file_id_count > 0 {
        Some((
            DoctorCheckStatus::Failed,
            DoctorCheckMessage::GdriveMappingDuplicateFileId,
        ))
    } else if details.unmapped_item_count > 0 {
        Some((
            DoctorCheckStatus::Warning,
            DoctorCheckMessage::GdriveMappingDriftDetected,
        ))
    } else {
        Some((
            DoctorCheckStatus::Ok,
            DoctorCheckMessage::GdriveMappingValid,
        ))
    }
}

fn adapter_token_classification(details: &AdapterTokenSanityDetails) -> Option<Classification> {
    if details.missing_token_hash_count > details.enabled_adapter_count
        || details.invalid_role_count > details.enabled_adapter_count
    {
        return None;
    }
    if details.invalid_role_count > 0 {
        Some((
            DoctorCheckStatus::Failed,
            DoctorCheckMessage::AdapterInvalidRole,
        ))
    } else if details.missing_token_hash_count > 0 {
        Some((
            DoctorCheckStatus::Warning,
            DoctorCheckMessage::AdapterMissingTokenHash,
        ))
    } else if details.enabled_adapter_count == 0 {
        Some((
            DoctorCheckStatus::Skipped,
            DoctorCheckMessage::NoEnabledAdaptersForTokenSanity,
        ))
    } else {
        Some((
            DoctorCheckStatus::Ok,
            DoctorCheckMessage::AdapterTokenMetadataSane,
        ))
    }
}

fn worktree_drift_classification(details: &WorktreeDriftDetails) -> Option<Classification> {
    if !details.configured {
        let no_facts = details.tracked_path_count == 0
            && details.unknown_revision_count == 0
            && details.missing_path_count == 0
            && details.content_mismatch_count == 0
            && details.unexpected_path_count == 0;
        return no_facts.then_some((
            DoctorCheckStatus::Skipped,
            DoctorCheckMessage::WorktreeDriftSkipped,
        ));
    }
    if details.unknown_revision_count > 0 {
        Some((
            DoctorCheckStatus::Failed,
            DoctorCheckMessage::WorktreeUnknownRevision,
        ))
    } else if details.missing_path_count > 0
        || details.content_mismatch_count > 0
        || details.unexpected_path_count > 0
    {
        Some((
            DoctorCheckStatus::Warning,
            DoctorCheckMessage::WorktreeDriftDetected,
        ))
    } else {
        Some((
            DoctorCheckStatus::Ok,
            DoctorCheckMessage::WorktreeStateConsistent,
        ))
    }
}
