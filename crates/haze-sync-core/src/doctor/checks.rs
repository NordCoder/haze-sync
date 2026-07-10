use super::types::{
    AdapterCursorDetails, AdapterTokenSanityDetails, DbConnectivityDetails,
    DoctorCheckDetails, DoctorCheckId, DoctorCheckMessage, DoctorCheckResult,
    DoctorCheckStatus, GdriveMappingDetails, MissingBlobDetectionDetails,
    ObjectStoreExistsWritableDetails, WorktreeDriftDetails,
};
use haze_sync_common::ContentHash;

/// Input for the passive database connectivity check model.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DbConnectivityCheckInput {
    pub metadata_configured: bool,
    pub live_check_enabled: bool,
    pub connectivity_verified: Option<bool>,
}

impl DbConnectivityCheckInput {
    /// Creates the default offline DB check input.
    #[must_use]
    pub const fn offline(metadata_configured: bool) -> Self {
        Self {
            metadata_configured,
            live_check_enabled: false,
            connectivity_verified: None,
        }
    }
}

/// Builds a passive DB connectivity check result without storing DB URLs.
#[must_use]
pub fn db_connectivity_check(input: DbConnectivityCheckInput) -> DoctorCheckResult {
    let connectivity_verified = if input.live_check_enabled {
        input.connectivity_verified
    } else {
        None
    };
    let details = DbConnectivityDetails {
        metadata_configured: input.metadata_configured,
        live_check_performed: input.live_check_enabled && connectivity_verified.is_some(),
        connectivity_verified,
    };
    let (status, message) = if !input.metadata_configured {
        (
            DoctorCheckStatus::Warning,
            DoctorCheckMessage::DatabaseMetadataNotConfigured,
        )
    } else if !input.live_check_enabled {
        (
            DoctorCheckStatus::Skipped,
            DoctorCheckMessage::DatabaseConnectivitySkippedOffline,
        )
    } else if connectivity_verified == Some(true) {
        (
            DoctorCheckStatus::Ok,
            DoctorCheckMessage::DatabaseConnectivityVerified,
        )
    } else if connectivity_verified == Some(false) {
        (
            DoctorCheckStatus::Failed,
            DoctorCheckMessage::DatabaseConnectivityFailed,
        )
    } else {
        (
            DoctorCheckStatus::NotRun,
            DoctorCheckMessage::DatabaseConnectivityNotRun,
        )
    };

    DoctorCheckResult::classified(
        DoctorCheckId::DbConnectivity,
        status,
        message,
        DoctorCheckDetails::DbConnectivity(details),
    )
}

/// Input for the passive object-store existence/writability check model.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ObjectStoreExistsWritableInput {
    pub configured: bool,
    pub exists: Option<bool>,
    pub writable: Option<bool>,
}

impl ObjectStoreExistsWritableInput {
    /// Creates the default offline object-store check input.
    #[must_use]
    pub const fn offline(configured: bool) -> Self {
        Self {
            configured,
            exists: None,
            writable: None,
        }
    }
}

/// Builds a passive object-store existence/writability check result.
///
/// The result intentionally omits the object-store root path.
#[must_use]
pub fn object_store_exists_writable_check(
    input: ObjectStoreExistsWritableInput,
) -> DoctorCheckResult {
    let details = ObjectStoreExistsWritableDetails {
        configured: input.configured,
        exists: input.exists,
        writable: input.writable,
    };
    let (status, message) = if !input.configured {
        (
            DoctorCheckStatus::Warning,
            DoctorCheckMessage::ObjectStoreNotConfigured,
        )
    } else if input.exists == Some(false) {
        (
            DoctorCheckStatus::Failed,
            DoctorCheckMessage::ObjectStoreMissing,
        )
    } else if input.writable == Some(false) {
        (
            DoctorCheckStatus::Failed,
            DoctorCheckMessage::ObjectStoreNotWritable,
        )
    } else if input.exists == Some(true) && input.writable == Some(true) {
        (
            DoctorCheckStatus::Ok,
            DoctorCheckMessage::ObjectStoreAccessible,
        )
    } else if input.exists.is_none() && input.writable.is_none() {
        (
            DoctorCheckStatus::Skipped,
            DoctorCheckMessage::ObjectStoreSkippedOffline,
        )
    } else {
        (
            DoctorCheckStatus::NotRun,
            DoctorCheckMessage::ObjectStoreNotRun,
        )
    };

    DoctorCheckResult::classified(
        DoctorCheckId::ObjectStoreExistsWritable,
        status,
        message,
        DoctorCheckDetails::ObjectStoreExistsWritable(details),
    )
}

/// Input for passive missing-blob detection summaries.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MissingBlobDetectionInput {
    pub input_count: u64,
    pub missing_hashes: Vec<ContentHash>,
    pub sample_limit: usize,
}

impl MissingBlobDetectionInput {
    /// Creates a missing-blob detection input with a deterministic sample limit.
    #[must_use]
    pub fn new(input_count: u64, missing_hashes: Vec<ContentHash>, sample_limit: usize) -> Self {
        Self {
            input_count,
            missing_hashes,
            sample_limit,
        }
    }
}

/// Builds a missing-blob detection result.
///
/// The result serializes counts and safe content hashes only. It never includes
/// full local paths, provider paths, or object-store roots.
#[must_use]
pub fn missing_blob_detection_check(input: MissingBlobDetectionInput) -> DoctorCheckResult {
    let mut missing_hashes = input.missing_hashes;
    missing_hashes.sort();
    missing_hashes.dedup();
    let missing_count = usize_to_u64(missing_hashes.len());
    let sample_hashes = missing_hashes
        .into_iter()
        .take(input.sample_limit)
        .collect();
    let details = MissingBlobDetectionDetails {
        input_count: input.input_count,
        missing_count,
        sample_hashes,
    };
    let (status, message) = if missing_count == 0 {
        (
            DoctorCheckStatus::Ok,
            DoctorCheckMessage::NoMissingBlobs,
        )
    } else {
        (
            DoctorCheckStatus::Failed,
            DoctorCheckMessage::MissingBlobsDetected,
        )
    };

    DoctorCheckResult::classified(
        DoctorCheckId::MissingBlobs,
        status,
        message,
        DoctorCheckDetails::MissingBlobs(details),
    )
}

/// Input for passive adapter-cursor integrity summaries.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdapterCursorCheckInput {
    pub expected_adapter_count: u64,
    pub cursor_count: u64,
    pub invalid_cursor_count: u64,
    pub stale_cursor_count: u64,
}

impl AdapterCursorCheckInput {
    /// Creates an adapter-cursor summary from already-redacted counts.
    #[must_use]
    pub const fn new(
        expected_adapter_count: u64,
        cursor_count: u64,
        invalid_cursor_count: u64,
        stale_cursor_count: u64,
    ) -> Self {
        Self {
            expected_adapter_count,
            cursor_count,
            invalid_cursor_count,
            stale_cursor_count,
        }
    }
}

/// Builds a passive adapter-cursor integrity result.
#[must_use]
pub fn adapter_cursor_check(input: AdapterCursorCheckInput) -> DoctorCheckResult {
    let missing_cursor_count = input
        .expected_adapter_count
        .saturating_sub(input.cursor_count);
    let orphaned_cursor_count = input
        .cursor_count
        .saturating_sub(input.expected_adapter_count);
    let details = AdapterCursorDetails {
        expected_adapter_count: input.expected_adapter_count,
        cursor_count: input.cursor_count,
        missing_cursor_count,
        orphaned_cursor_count,
        invalid_cursor_count: input.invalid_cursor_count,
        stale_cursor_count: input.stale_cursor_count,
    };
    let (status, message) = if input.expected_adapter_count == 0 && input.cursor_count == 0 {
        (
            DoctorCheckStatus::Skipped,
            DoctorCheckMessage::NoAdaptersForCursorValidation,
        )
    } else if input.invalid_cursor_count > 0 {
        (
            DoctorCheckStatus::Failed,
            DoctorCheckMessage::InvalidAdapterCursorDetected,
        )
    } else if missing_cursor_count > 0 {
        (
            DoctorCheckStatus::Warning,
            DoctorCheckMessage::MissingAdapterCursorDetected,
        )
    } else if orphaned_cursor_count > 0 {
        (
            DoctorCheckStatus::Warning,
            DoctorCheckMessage::OrphanedAdapterCursorDetected,
        )
    } else if input.stale_cursor_count > 0 {
        (
            DoctorCheckStatus::Warning,
            DoctorCheckMessage::StaleAdapterCursorDetected,
        )
    } else {
        (
            DoctorCheckStatus::Ok,
            DoctorCheckMessage::AdapterCursorsValid,
        )
    };

    DoctorCheckResult::classified(
        DoctorCheckId::AdapterCursors,
        status,
        message,
        DoctorCheckDetails::AdapterCursors(details),
    )
}

/// Input for passive Google Drive mapping integrity summaries.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GdriveMappingCheckInput {
    pub configured: bool,
    pub mapping_count: u64,
    pub unknown_revision_count: u64,
    pub duplicate_drive_file_id_count: u64,
    pub unmapped_item_count: u64,
}

impl GdriveMappingCheckInput {
    /// Creates a mapping summary from already-redacted counts.
    #[must_use]
    pub const fn new(
        configured: bool,
        mapping_count: u64,
        unknown_revision_count: u64,
        duplicate_drive_file_id_count: u64,
        unmapped_item_count: u64,
    ) -> Self {
        Self {
            configured,
            mapping_count,
            unknown_revision_count,
            duplicate_drive_file_id_count,
            unmapped_item_count,
        }
    }
}

/// Builds a passive Google Drive mapping integrity result.
#[must_use]
pub fn gdrive_mapping_check(input: GdriveMappingCheckInput) -> DoctorCheckResult {
    let details = GdriveMappingDetails {
        configured: input.configured,
        mapping_count: input.mapping_count,
        unknown_revision_count: input.unknown_revision_count,
        duplicate_drive_file_id_count: input.duplicate_drive_file_id_count,
        unmapped_item_count: input.unmapped_item_count,
    };
    let (status, message) = if !input.configured {
        (
            DoctorCheckStatus::Skipped,
            DoctorCheckMessage::GdriveMappingSkipped,
        )
    } else if input.unknown_revision_count > 0 {
        (
            DoctorCheckStatus::Failed,
            DoctorCheckMessage::GdriveMappingUnknownRevision,
        )
    } else if input.duplicate_drive_file_id_count > 0 {
        (
            DoctorCheckStatus::Failed,
            DoctorCheckMessage::GdriveMappingDuplicateFileId,
        )
    } else if input.unmapped_item_count > 0 {
        (
            DoctorCheckStatus::Warning,
            DoctorCheckMessage::GdriveMappingDriftDetected,
        )
    } else {
        (
            DoctorCheckStatus::Ok,
            DoctorCheckMessage::GdriveMappingValid,
        )
    };

    DoctorCheckResult::classified(
        DoctorCheckId::GdriveMapping,
        status,
        message,
        DoctorCheckDetails::GdriveMapping(details),
    )
}

/// Passive inspection row for one configured adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdapterTokenInspection {
    pub enabled: bool,
    pub token_hash_present: bool,
    pub role_valid: bool,
}

impl AdapterTokenInspection {
    /// Creates an adapter token inspection row from redacted booleans only.
    #[must_use]
    pub const fn new(enabled: bool, token_hash_present: bool, role_valid: bool) -> Self {
        Self {
            enabled,
            token_hash_present,
            role_valid,
        }
    }
}

/// Input for adapter token sanity summaries.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdapterTokenSanityInput {
    pub adapters: Vec<AdapterTokenInspection>,
}

impl AdapterTokenSanityInput {
    /// Creates a token sanity input from already-redacted adapter rows.
    #[must_use]
    pub fn new(adapters: Vec<AdapterTokenInspection>) -> Self {
        Self { adapters }
    }
}

/// Builds an adapter token sanity result without raw token material.
#[must_use]
pub fn adapter_token_sanity_check(input: AdapterTokenSanityInput) -> DoctorCheckResult {
    let enabled_adapter_count = input
        .adapters
        .iter()
        .filter(|adapter| adapter.enabled)
        .count();
    let missing_token_hash_count = input
        .adapters
        .iter()
        .filter(|adapter| adapter.enabled && !adapter.token_hash_present)
        .count();
    let invalid_role_count = input
        .adapters
        .iter()
        .filter(|adapter| adapter.enabled && !adapter.role_valid)
        .count();
    let details = AdapterTokenSanityDetails {
        enabled_adapter_count: usize_to_u64(enabled_adapter_count),
        missing_token_hash_count: usize_to_u64(missing_token_hash_count),
        invalid_role_count: usize_to_u64(invalid_role_count),
    };
    let (status, message) = if invalid_role_count > 0 {
        (
            DoctorCheckStatus::Failed,
            DoctorCheckMessage::AdapterInvalidRole,
        )
    } else if missing_token_hash_count > 0 {
        (
            DoctorCheckStatus::Warning,
            DoctorCheckMessage::AdapterMissingTokenHash,
        )
    } else if enabled_adapter_count == 0 {
        (
            DoctorCheckStatus::Skipped,
            DoctorCheckMessage::NoEnabledAdaptersForTokenSanity,
        )
    } else {
        (
            DoctorCheckStatus::Ok,
            DoctorCheckMessage::AdapterTokenMetadataSane,
        )
    };

    DoctorCheckResult::classified(
        DoctorCheckId::AdapterTokenSanity,
        status,
        message,
        DoctorCheckDetails::AdapterTokenSanity(details),
    )
}

/// Input for passive worktree drift summaries.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WorktreeDriftCheckInput {
    pub configured: bool,
    pub tracked_path_count: u64,
    pub unknown_revision_count: u64,
    pub missing_path_count: u64,
    pub content_mismatch_count: u64,
    pub unexpected_path_count: u64,
}

impl WorktreeDriftCheckInput {
    /// Creates a worktree drift summary from already-redacted counts.
    #[must_use]
    pub const fn new(
        configured: bool,
        tracked_path_count: u64,
        unknown_revision_count: u64,
        missing_path_count: u64,
        content_mismatch_count: u64,
        unexpected_path_count: u64,
    ) -> Self {
        Self {
            configured,
            tracked_path_count,
            unknown_revision_count,
            missing_path_count,
            content_mismatch_count,
            unexpected_path_count,
        }
    }
}

/// Builds a passive worktree drift result without paths or file contents.
#[must_use]
pub fn worktree_drift_check(input: WorktreeDriftCheckInput) -> DoctorCheckResult {
    let details = WorktreeDriftDetails {
        configured: input.configured,
        tracked_path_count: input.tracked_path_count,
        unknown_revision_count: input.unknown_revision_count,
        missing_path_count: input.missing_path_count,
        content_mismatch_count: input.content_mismatch_count,
        unexpected_path_count: input.unexpected_path_count,
    };
    let drift_count = input
        .missing_path_count
        .saturating_add(input.content_mismatch_count)
        .saturating_add(input.unexpected_path_count);
    let (status, message) = if !input.configured {
        (
            DoctorCheckStatus::Skipped,
            DoctorCheckMessage::WorktreeDriftSkipped,
        )
    } else if input.unknown_revision_count > 0 {
        (
            DoctorCheckStatus::Failed,
            DoctorCheckMessage::WorktreeUnknownRevision,
        )
    } else if drift_count > 0 {
        (
            DoctorCheckStatus::Warning,
            DoctorCheckMessage::WorktreeDriftDetected,
        )
    } else {
        (
            DoctorCheckStatus::Ok,
            DoctorCheckMessage::WorktreeStateConsistent,
        )
    };

    DoctorCheckResult::classified(
        DoctorCheckId::WorktreeDrift,
        status,
        message,
        DoctorCheckDetails::WorktreeDrift(details),
    )
}

fn usize_to_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}
