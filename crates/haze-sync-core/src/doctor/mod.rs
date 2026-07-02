//! Safe doctor-check model primitives.
//!
//! The doctor surface is passive. These models describe diagnostic outcomes and
//! redacted summaries only; they do not open database connections, touch
//! providers, repair state, delete blobs, expose local absolute paths, or
//! serialize credentials.

use haze_sync_common::ContentHash;
use serde::{Deserialize, Serialize};

/// Stable identifier for a passive doctor check.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DoctorCheckId {
    DbConnectivity,
    ObjectStoreExistsWritable,
    MissingBlobs,
    AdapterTokenSanity,
}

impl DoctorCheckId {
    /// Stable wire identifier for this check.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DbConnectivity => "db_connectivity",
            Self::ObjectStoreExistsWritable => "object_store_exists_writable",
            Self::MissingBlobs => "missing_blobs",
            Self::AdapterTokenSanity => "adapter_token_sanity",
        }
    }
}

/// Public status for one doctor check or an aggregate doctor report.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DoctorCheckStatus {
    Ok,
    Warning,
    Failed,
    Skipped,
}

/// JSON-safe doctor check result.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DoctorCheckResult {
    pub check_id: DoctorCheckId,
    pub status: DoctorCheckStatus,
    pub message: String,
    pub details: DoctorCheckDetails,
}

impl DoctorCheckResult {
    /// Creates a JSON-safe doctor check result.
    #[must_use]
    pub fn new(
        check_id: DoctorCheckId,
        status: DoctorCheckStatus,
        message: impl Into<String>,
        details: DoctorCheckDetails,
    ) -> Self {
        Self {
            check_id,
            status,
            message: message.into(),
            details,
        }
    }
}

/// JSON-safe detail payload for a doctor check.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DoctorCheckDetails {
    DbConnectivity(DbConnectivityDetails),
    ObjectStoreExistsWritable(ObjectStoreExistsWritableDetails),
    MissingBlobs(MissingBlobDetectionDetails),
    AdapterTokenSanity(AdapterTokenSanityDetails),
}

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

/// Redacted database connectivity check details.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DbConnectivityDetails {
    pub metadata_configured: bool,
    pub live_check_performed: bool,
    pub connectivity_verified: Option<bool>,
}

/// Builds a passive DB connectivity check result without storing DB URLs.
#[must_use]
pub fn db_connectivity_check(input: DbConnectivityCheckInput) -> DoctorCheckResult {
    let details = DbConnectivityDetails {
        metadata_configured: input.metadata_configured,
        live_check_performed: input.live_check_enabled && input.connectivity_verified.is_some(),
        connectivity_verified: input.connectivity_verified,
    };
    let (status, message) = if !input.metadata_configured {
        (
            DoctorCheckStatus::Warning,
            "database metadata is not configured",
        )
    } else if !input.live_check_enabled {
        (
            DoctorCheckStatus::Skipped,
            "database connectivity check skipped in offline mode",
        )
    } else if input.connectivity_verified == Some(true) {
        (DoctorCheckStatus::Ok, "database connectivity verified")
    } else if input.connectivity_verified == Some(false) {
        (
            DoctorCheckStatus::Failed,
            "database connectivity check failed",
        )
    } else {
        (
            DoctorCheckStatus::Skipped,
            "database connectivity result was not provided",
        )
    };

    DoctorCheckResult::new(
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

/// Redacted object-store existence/writability details.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ObjectStoreExistsWritableDetails {
    pub configured: bool,
    pub exists: Option<bool>,
    pub writable: Option<bool>,
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
            "object store root is not configured",
        )
    } else if input.exists == Some(false) {
        (
            DoctorCheckStatus::Failed,
            "object store root does not exist",
        )
    } else if input.writable == Some(false) {
        (
            DoctorCheckStatus::Failed,
            "object store root is not writable",
        )
    } else if input.exists == Some(true) && input.writable == Some(true) {
        (DoctorCheckStatus::Ok, "object store root is accessible")
    } else {
        (
            DoctorCheckStatus::Skipped,
            "object store filesystem check skipped in offline mode",
        )
    };

    DoctorCheckResult::new(
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

/// Public missing-blob detection details.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MissingBlobDetectionDetails {
    pub input_count: u64,
    pub missing_count: u64,
    pub sample_hashes: Vec<ContentHash>,
}

/// Builds a missing-blob detection skeleton result.
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
        (DoctorCheckStatus::Ok, "no missing blobs detected")
    } else {
        (DoctorCheckStatus::Failed, "missing required blobs detected")
    };

    DoctorCheckResult::new(
        DoctorCheckId::MissingBlobs,
        status,
        message,
        DoctorCheckDetails::MissingBlobs(details),
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

/// Redacted adapter token sanity details.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AdapterTokenSanityDetails {
    pub enabled_adapter_count: u64,
    pub missing_token_hash_count: u64,
    pub invalid_role_count: u64,
}

/// Builds an adapter token sanity check result without raw token material.
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
            "enabled adapter has invalid role",
        )
    } else if missing_token_hash_count > 0 {
        (
            DoctorCheckStatus::Warning,
            "enabled adapter is missing a token hash",
        )
    } else if enabled_adapter_count == 0 {
        (
            DoctorCheckStatus::Skipped,
            "no enabled adapters provided for token sanity check",
        )
    } else {
        (DoctorCheckStatus::Ok, "adapter token metadata is sane")
    };

    DoctorCheckResult::new(
        DoctorCheckId::AdapterTokenSanity,
        status,
        message,
        DoctorCheckDetails::AdapterTokenSanity(details),
    )
}

/// Aggregate doctor report summary.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DoctorReportSummary {
    pub status: DoctorCheckStatus,
    pub total_checks: u64,
    pub ok_count: u64,
    pub warning_count: u64,
    pub failed_count: u64,
    pub skipped_count: u64,
}

impl DoctorReportSummary {
    /// Derives a summary from deterministic check results.
    #[must_use]
    pub fn from_results(results: &[DoctorCheckResult]) -> Self {
        let mut ok_count = 0_u64;
        let mut warning_count = 0_u64;
        let mut failed_count = 0_u64;
        let mut skipped_count = 0_u64;
        for result in results {
            match result.status {
                DoctorCheckStatus::Ok => ok_count += 1,
                DoctorCheckStatus::Warning => warning_count += 1,
                DoctorCheckStatus::Failed => failed_count += 1,
                DoctorCheckStatus::Skipped => skipped_count += 1,
            }
        }
        let status = if failed_count > 0 {
            DoctorCheckStatus::Failed
        } else if warning_count > 0 {
            DoctorCheckStatus::Warning
        } else if skipped_count > 0 {
            DoctorCheckStatus::Skipped
        } else {
            DoctorCheckStatus::Ok
        };
        Self {
            status,
            total_checks: usize_to_u64(results.len()),
            ok_count,
            warning_count,
            failed_count,
            skipped_count,
        }
    }
}

/// Deterministically ordered doctor report.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DoctorReport {
    pub summary: DoctorReportSummary,
    pub checks: Vec<DoctorCheckResult>,
}

impl DoctorReport {
    /// Creates a doctor report with checks ordered by stable check identifier.
    #[must_use]
    pub fn from_results(mut checks: Vec<DoctorCheckResult>) -> Self {
        checks.sort_by_key(|result| result.check_id.as_str());
        let summary = DoctorReportSummary::from_results(&checks);
        Self { summary, checks }
    }
}

fn usize_to_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repeated_hash(ch: char) -> ContentHash {
        ContentHash::parse(&ch.to_string().repeat(64)).expect("test hash should be valid")
    }

    #[test]
    fn check_result_serialization_is_stable() {
        let result = object_store_exists_writable_check(ObjectStoreExistsWritableInput {
            configured: true,
            exists: Some(true),
            writable: Some(false),
        });
        let serialized = serde_json::to_string(&result).expect("doctor result should serialize");
        assert_eq!(
            serialized,
            r#"{"check_id":"object_store_exists_writable","status":"failed","message":"object store root is not writable","details":{"kind":"object_store_exists_writable","configured":true,"exists":true,"writable":false}}"#
        );
    }

    #[test]
    fn summary_status_uses_failed_warning_skipped_ok_precedence() {
        let ok = DoctorCheckResult::new(
            DoctorCheckId::MissingBlobs,
            DoctorCheckStatus::Ok,
            "ok",
            DoctorCheckDetails::MissingBlobs(MissingBlobDetectionDetails {
                input_count: 1,
                missing_count: 0,
                sample_hashes: Vec::new(),
            }),
        );
        let warning = db_connectivity_check(DbConnectivityCheckInput::offline(false));
        let skipped =
            object_store_exists_writable_check(ObjectStoreExistsWritableInput::offline(true));
        let failed = missing_blob_detection_check(MissingBlobDetectionInput::new(
            1,
            vec![repeated_hash('a')],
            1,
        ));

        assert_eq!(
            DoctorReport::from_results(vec![ok.clone()]).summary.status,
            DoctorCheckStatus::Ok
        );
        assert_eq!(
            DoctorReport::from_results(vec![ok.clone(), skipped.clone()])
                .summary
                .status,
            DoctorCheckStatus::Skipped
        );
        assert_eq!(
            DoctorReport::from_results(vec![ok.clone(), warning.clone(), skipped])
                .summary
                .status,
            DoctorCheckStatus::Warning
        );
        assert_eq!(
            DoctorReport::from_results(vec![ok, warning, failed])
                .summary
                .status,
            DoctorCheckStatus::Failed
        );
    }

    #[test]
    fn missing_blob_skeleton_exposes_hash_samples_not_paths() {
        let report = DoctorReport::from_results(vec![missing_blob_detection_check(
            MissingBlobDetectionInput::new(3, vec![repeated_hash('b'), repeated_hash('a')], 1),
        )]);
        let serialized = serde_json::to_string(&report).expect("doctor report should serialize");

        assert!(serialized
            .contains("sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
        assert!(!serialized.contains("/srv/haze-sync/objects"));
        assert!(!serialized.contains("C:\\private"));
        assert!(!serialized.contains("Notes/a.md"));
        assert!(!serialized
            .contains("sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"));
    }

    #[test]
    fn token_sanity_skeleton_exposes_counts_not_token_material() {
        let report = DoctorReport::from_results(vec![adapter_token_sanity_check(
            AdapterTokenSanityInput::new(vec![
                AdapterTokenInspection::new(true, false, true),
                AdapterTokenInspection::new(true, true, false),
                AdapterTokenInspection::new(false, false, false),
            ]),
        )]);
        let serialized = serde_json::to_string(&report).expect("doctor report should serialize");

        assert!(serialized.contains("\"enabled_adapter_count\":2"));
        assert!(serialized.contains("\"missing_token_hash_count\":1"));
        assert!(serialized.contains("\"invalid_role_count\":1"));
        assert!(!serialized.contains("credential_material"));
        assert!(!serialized
            .contains("sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"));
    }
}
