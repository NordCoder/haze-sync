//! Pure service-level tombstone primitives.
//!
//! Creating a tombstone records safe delete intent and retention metadata only.
//! This module classifies restore and retention-cleanup eligibility but does not
//! hard-delete storage rows, remove blobs, move files to trash, run retention
//! cleanup, call adapters, or perform restore side effects.

use chrono::{DateTime, Utc};
use haze_sync_common::{AdapterId, RevisionId, VaultPath};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{error::Error, fmt, str::FromStr};

const MAX_TOMBSTONE_ID_LEN: usize = 128;
const TOMBSTONE_ID_PREFIX: &str = "tmb_";

/// Safe tombstone identifier, for example `tmb_01J...`.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct TombstoneId(String);

impl TombstoneId {
    /// Validate and construct a tombstone id.
    pub fn parse(input: &str) -> Result<Self, TombstoneServiceError> {
        if input.len() <= TOMBSTONE_ID_PREFIX.len()
            || input.len() > MAX_TOMBSTONE_ID_LEN
            || !input.starts_with(TOMBSTONE_ID_PREFIX)
            || input.as_bytes().contains(&0)
            || !input.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.')
            })
        {
            return Err(TombstoneServiceError::InvalidTombstoneId);
        }

        Ok(Self(input.to_owned()))
    }

    /// Borrow the identifier as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume the identifier and return its string value.
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl AsRef<str> for TombstoneId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for TombstoneId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for TombstoneId {
    type Err = TombstoneServiceError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Self::parse(input)
    }
}

impl Serialize for TombstoneId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for TombstoneId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(serde::de::Error::custom)
    }
}

/// Revision metadata captured at tombstone creation time.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TombstonedRevisionRef {
    pub deleted_revision_id: RevisionId,
    pub current_revision_id: RevisionId,
}

impl TombstonedRevisionRef {
    /// Build a revision reference and default the current revision to the deleted
    /// revision when callers have no separate current snapshot.
    #[must_use]
    pub fn new(deleted_revision_id: RevisionId, current_revision_id: Option<RevisionId>) -> Self {
        let current_revision_id =
            current_revision_id.unwrap_or_else(|| deleted_revision_id.clone());
        Self {
            deleted_revision_id,
            current_revision_id,
        }
    }
}

/// Retention metadata required before any future physical cleanup job may act.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TombstoneRetention {
    pub retention_until: DateTime<Utc>,
    pub retention_days: Option<u16>,
    pub cleanup_after_retention_only: bool,
}

impl TombstoneRetention {
    /// Build retention metadata using an absolute timestamp.
    #[must_use]
    pub fn new(retention_until: DateTime<Utc>, retention_days: Option<u16>) -> Self {
        Self {
            retention_until,
            retention_days,
            cleanup_after_retention_only: true,
        }
    }
}

/// Restore-ready metadata. Classification is pure; restore persistence and
/// revision creation remain downstream responsibilities.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreMetadata {
    pub restored_at: Option<DateTime<Utc>>,
    pub restored_by: Option<AdapterId>,
    pub restore_revision_id: Option<RevisionId>,
}

/// Pure classification of whether a tombstone is ready for restore planning.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "eligibility")]
pub enum RestoreEligibility {
    Eligible {
        deleted_revision_id: RevisionId,
        current_revision_id: RevisionId,
    },
    AlreadyRestored {
        restored_at: DateTime<Utc>,
        restored_by: AdapterId,
        restore_revision_id: RevisionId,
    },
}

/// Pure classification for a future physical retention-cleanup worker.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "eligibility")]
pub enum RetentionCleanupEligibility {
    RetainedUntil {
        retention_until: DateTime<Utc>,
    },
    EligibleAfterRetention {
        retention_until: DateTime<Utc>,
    },
    NotEligibleRestored {
        restored_at: DateTime<Utc>,
        restore_revision_id: RevisionId,
    },
}

/// Request accepted by the tombstone service.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TombstoneCreationInput {
    pub tombstone_id: TombstoneId,
    pub path: VaultPath,
    pub revision: TombstonedRevisionRef,
    pub deleted_by: AdapterId,
    pub retention: TombstoneRetention,
    pub created_at: Option<DateTime<Utc>>,
}

impl TombstoneCreationInput {
    /// Build a request from validated value types.
    #[must_use]
    pub fn new(
        tombstone_id: TombstoneId,
        path: VaultPath,
        deleted_revision_id: RevisionId,
        current_revision_id: Option<RevisionId>,
        deleted_by: AdapterId,
        retention: TombstoneRetention,
        created_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            tombstone_id,
            path,
            revision: TombstonedRevisionRef::new(deleted_revision_id, current_revision_id),
            deleted_by,
            retention,
            created_at,
        }
    }

    /// Build and validate a request from raw boundary strings.
    pub fn try_from_raw(
        tombstone_id: &str,
        path: &str,
        deleted_revision_id: &str,
        current_revision_id: Option<&str>,
        deleted_by: &str,
        retention: TombstoneRetention,
        created_at: Option<DateTime<Utc>>,
    ) -> Result<Self, TombstoneServiceError> {
        let tombstone_id = TombstoneId::parse(tombstone_id)?;
        let path = VaultPath::parse(path).map_err(|_error| TombstoneServiceError::InvalidPath)?;
        let deleted_revision_id = RevisionId::parse(deleted_revision_id)
            .map_err(|_error| TombstoneServiceError::InvalidRevisionId)?;
        let current_revision_id = current_revision_id
            .map(RevisionId::parse)
            .transpose()
            .map_err(|_error| TombstoneServiceError::InvalidRevisionId)?;
        let deleted_by = AdapterId::parse(deleted_by)
            .map_err(|_error| TombstoneServiceError::InvalidAdapterId)?;

        Ok(Self::new(
            tombstone_id,
            path,
            deleted_revision_id,
            current_revision_id,
            deleted_by,
            retention,
            created_at,
        ))
    }
}

/// Tombstone metadata produced by the service and ready for persistence/API
/// mapping by callers.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Tombstone {
    pub tombstone_id: TombstoneId,
    pub path: VaultPath,
    pub revision: TombstonedRevisionRef,
    pub deleted_by: AdapterId,
    pub retention: TombstoneRetention,
    pub created_at: Option<DateTime<Utc>>,
    pub restore: RestoreMetadata,
}

impl Tombstone {
    /// Return the deleted revision id for storage rows that use the schema field
    /// `deleted_revision_id`.
    #[must_use]
    pub fn deleted_revision_id(&self) -> &RevisionId {
        &self.revision.deleted_revision_id
    }

    /// Return the current revision snapshot used when the tombstone was created.
    #[must_use]
    pub fn current_revision_id(&self) -> &RevisionId {
        &self.revision.current_revision_id
    }

    /// Validate retention and restore metadata after persistence/API mapping.
    pub fn validate_metadata(&self) -> Result<(), TombstoneServiceError> {
        validate_retention(self.created_at, &self.retention)?;
        validate_restore_metadata(self.created_at, &self.restore)
    }

    /// Classify restore readiness without creating revisions or mutating storage.
    pub fn classify_restore_eligibility(
        &self,
    ) -> Result<RestoreEligibility, TombstoneServiceError> {
        self.validate_metadata()?;

        match (
            self.restore.restored_at.as_ref(),
            self.restore.restored_by.as_ref(),
            self.restore.restore_revision_id.as_ref(),
        ) {
            (Some(restored_at), Some(restored_by), Some(restore_revision_id)) => {
                Ok(RestoreEligibility::AlreadyRestored {
                    restored_at: *restored_at,
                    restored_by: restored_by.clone(),
                    restore_revision_id: restore_revision_id.clone(),
                })
            }
            _ => Ok(RestoreEligibility::Eligible {
                deleted_revision_id: self.deleted_revision_id().clone(),
                current_revision_id: self.current_revision_id().clone(),
            }),
        }
    }

    /// Classify retention-cleanup eligibility at an explicit UTC timestamp.
    pub fn classify_retention_cleanup(
        &self,
        evaluated_at: DateTime<Utc>,
    ) -> Result<RetentionCleanupEligibility, TombstoneServiceError> {
        self.validate_metadata()?;

        if let (Some(restored_at), Some(restore_revision_id)) = (
            self.restore.restored_at.as_ref(),
            self.restore.restore_revision_id.as_ref(),
        ) {
            return Ok(RetentionCleanupEligibility::NotEligibleRestored {
                restored_at: *restored_at,
                restore_revision_id: restore_revision_id.clone(),
            });
        }

        if evaluated_at < self.retention.retention_until {
            return Ok(RetentionCleanupEligibility::RetainedUntil {
                retention_until: self.retention.retention_until,
            });
        }

        Ok(RetentionCleanupEligibility::EligibleAfterRetention {
            retention_until: self.retention.retention_until,
        })
    }
}

/// Pure tombstone service.
#[derive(Clone, Copy, Debug, Default)]
pub struct TombstoneService;

impl TombstoneService {
    /// Construct a pure tombstone service.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Create safe tombstone metadata without mutating storage or deleting data.
    pub fn create_tombstone(
        &self,
        input: TombstoneCreationInput,
    ) -> Result<Tombstone, TombstoneServiceError> {
        validate_retention(input.created_at, &input.retention)?;

        Ok(Tombstone {
            tombstone_id: input.tombstone_id,
            path: input.path,
            revision: input.revision,
            deleted_by: input.deleted_by,
            retention: input.retention,
            created_at: input.created_at,
            restore: RestoreMetadata::default(),
        })
    }
}

/// Safe tombstone input/service errors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TombstoneServiceError {
    InvalidTombstoneId,
    InvalidPath,
    InvalidRevisionId,
    InvalidAdapterId,
    InvalidRetention,
    InvalidRestoreMetadata,
}

impl TombstoneServiceError {
    /// Stable machine-readable error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidTombstoneId => "invalid_tombstone_id",
            Self::InvalidPath => "invalid_tombstone_path",
            Self::InvalidRevisionId => "invalid_deleted_revision_id",
            Self::InvalidAdapterId => "invalid_deleted_by_adapter_id",
            Self::InvalidRetention => "invalid_tombstone_retention",
            Self::InvalidRestoreMetadata => "invalid_tombstone_restore_metadata",
        }
    }
}

impl fmt::Display for TombstoneServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidTombstoneId => "tombstone id is invalid",
            Self::InvalidPath => "tombstone path is invalid",
            Self::InvalidRevisionId => "tombstone revision id is invalid",
            Self::InvalidAdapterId => "tombstone adapter id is invalid",
            Self::InvalidRetention => "tombstone retention metadata is invalid",
            Self::InvalidRestoreMetadata => "tombstone restore metadata is invalid",
        })
    }
}

impl Error for TombstoneServiceError {}

fn validate_retention(
    created_at: Option<DateTime<Utc>>,
    retention: &TombstoneRetention,
) -> Result<(), TombstoneServiceError> {
    if !retention.cleanup_after_retention_only || retention.retention_days == Some(0) {
        return Err(TombstoneServiceError::InvalidRetention);
    }

    if let Some(created_at) = created_at {
        if retention.retention_until <= created_at {
            return Err(TombstoneServiceError::InvalidRetention);
        }
    }

    Ok(())
}

fn validate_restore_metadata(
    created_at: Option<DateTime<Utc>>,
    restore: &RestoreMetadata,
) -> Result<(), TombstoneServiceError> {
    match (
        restore.restored_at.as_ref(),
        restore.restored_by.as_ref(),
        restore.restore_revision_id.as_ref(),
    ) {
        (None, None, None) => Ok(()),
        (Some(restored_at), Some(_restored_by), Some(_restore_revision_id)) => {
            if let Some(created_at) = created_at {
                if restored_at <= &created_at {
                    return Err(TombstoneServiceError::InvalidRestoreMetadata);
                }
            }
            Ok(())
        }
        _ => Err(TombstoneServiceError::InvalidRestoreMetadata),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timestamp(value: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(value)
            .expect("fixture timestamp should parse")
            .with_timezone(&Utc)
    }

    fn retention() -> TombstoneRetention {
        TombstoneRetention::new(timestamp("2026-08-01T00:00:00Z"), Some(30))
    }

    fn tombstone() -> Tombstone {
        let input = TombstoneCreationInput::try_from_raw(
            "tmb_01JTEST",
            "Notes/old.md",
            "rev_01JDELETE",
            Some("rev_01JCURRENT"),
            "worktree-adapter",
            retention(),
            Some(timestamp("2026-07-01T00:00:00Z")),
        )
        .expect("fixture input should parse");

        TombstoneService::new()
            .create_tombstone(input)
            .expect("fixture tombstone should be valid")
    }

    #[test]
    fn tombstone_id_validation_and_serde_roundtrip_are_stable() {
        assert_eq!(
            TombstoneId::parse("tmb_").expect_err("empty suffix should reject"),
            TombstoneServiceError::InvalidTombstoneId
        );
        assert_eq!(
            TombstoneId::parse("tmb_bad/id").expect_err("slash should reject"),
            TombstoneServiceError::InvalidTombstoneId
        );

        let max_length_id = format!("{TOMBSTONE_ID_PREFIX}{}", "a".repeat(124));
        let parsed = TombstoneId::parse(&max_length_id).expect("maximum length should parse");
        let serialized = serde_json::to_string(&parsed).expect("id should serialize");
        assert_eq!(
            serde_json::from_str::<TombstoneId>(&serialized).expect("id should deserialize"),
            parsed
        );

        let too_long_id = format!("{TOMBSTONE_ID_PREFIX}{}", "a".repeat(125));
        assert_eq!(
            TombstoneId::parse(&too_long_id).expect_err("overlong id should reject"),
            TombstoneServiceError::InvalidTombstoneId
        );
    }

    #[test]
    fn tombstone_input_validates_path_revision_and_adapter() {
        assert_eq!(
            TombstoneCreationInput::try_from_raw(
                "tmb_01JTEST",
                "../bad.md",
                "rev_01JDELETE",
                None,
                "worktree-adapter",
                retention(),
                Some(timestamp("2026-07-01T00:00:00Z")),
            )
            .expect_err("unsafe path should reject"),
            TombstoneServiceError::InvalidPath
        );
        assert_eq!(
            TombstoneCreationInput::try_from_raw(
                "tmb_01JTEST",
                "Notes/old.md",
                "not-a-revision",
                None,
                "worktree-adapter",
                retention(),
                Some(timestamp("2026-07-01T00:00:00Z")),
            )
            .expect_err("invalid revision should reject"),
            TombstoneServiceError::InvalidRevisionId
        );
        assert_eq!(
            TombstoneCreationInput::try_from_raw(
                "tmb_01JTEST",
                "Notes/old.md",
                "rev_01JDELETE",
                None,
                "bad/adapter",
                retention(),
                Some(timestamp("2026-07-01T00:00:00Z")),
            )
            .expect_err("invalid adapter should reject"),
            TombstoneServiceError::InvalidAdapterId
        );
    }

    #[test]
    fn creates_restore_ready_tombstone_without_restore_behavior() {
        let tombstone = tombstone();

        assert_eq!(tombstone.path.as_str(), "Notes/old.md");
        assert_eq!(tombstone.deleted_revision_id().as_str(), "rev_01JDELETE");
        assert_eq!(tombstone.current_revision_id().as_str(), "rev_01JCURRENT");
        assert_eq!(tombstone.deleted_by.as_str(), "worktree-adapter");
        assert_eq!(tombstone.retention.retention_days, Some(30));
        assert_eq!(
            tombstone.created_at,
            Some(timestamp("2026-07-01T00:00:00Z"))
        );
        assert_eq!(tombstone.restore, RestoreMetadata::default());
        assert_eq!(
            tombstone
                .classify_restore_eligibility()
                .expect("metadata should classify"),
            RestoreEligibility::Eligible {
                deleted_revision_id: RevisionId::parse("rev_01JDELETE").unwrap(),
                current_revision_id: RevisionId::parse("rev_01JCURRENT").unwrap(),
            }
        );
    }

    #[test]
    fn rejects_invalid_retention_metadata_and_boundaries() {
        for retention in [
            TombstoneRetention::new(timestamp("2026-07-01T00:00:00Z"), Some(30)),
            TombstoneRetention::new(timestamp("2026-06-30T23:59:59Z"), Some(30)),
            TombstoneRetention::new(timestamp("2026-08-01T00:00:00Z"), Some(0)),
            TombstoneRetention {
                retention_until: timestamp("2026-08-01T00:00:00Z"),
                retention_days: Some(30),
                cleanup_after_retention_only: false,
            },
        ] {
            let input = TombstoneCreationInput::try_from_raw(
                "tmb_01JTEST",
                "Notes/old.md",
                "rev_01JDELETE",
                None,
                "worktree-adapter",
                retention,
                Some(timestamp("2026-07-01T00:00:00Z")),
            )
            .expect("boundary input should parse");

            assert_eq!(
                TombstoneService::new()
                    .create_tombstone(input)
                    .expect_err("invalid retention should reject"),
                TombstoneServiceError::InvalidRetention
            );
        }
    }

    #[test]
    fn retention_cleanup_classifier_uses_exact_boundary_without_side_effects() {
        let tombstone = tombstone();

        assert_eq!(
            tombstone
                .classify_retention_cleanup(timestamp("2026-07-31T23:59:59Z"))
                .expect("metadata should classify"),
            RetentionCleanupEligibility::RetainedUntil {
                retention_until: timestamp("2026-08-01T00:00:00Z"),
            }
        );
        assert_eq!(
            tombstone
                .classify_retention_cleanup(timestamp("2026-08-01T00:00:00Z"))
                .expect("metadata should classify"),
            RetentionCleanupEligibility::EligibleAfterRetention {
                retention_until: timestamp("2026-08-01T00:00:00Z"),
            }
        );
        assert_eq!(
            tombstone
                .classify_retention_cleanup(timestamp("2026-08-02T00:00:00Z"))
                .expect("metadata should classify"),
            RetentionCleanupEligibility::EligibleAfterRetention {
                retention_until: timestamp("2026-08-01T00:00:00Z"),
            }
        );
    }

    #[test]
    fn completed_restore_is_classified_and_not_cleanup_eligible() {
        let mut tombstone = tombstone();
        tombstone.restore = RestoreMetadata {
            restored_at: Some(timestamp("2026-07-15T00:00:00Z")),
            restored_by: Some(AdapterId::parse("worktree-adapter").unwrap()),
            restore_revision_id: Some(RevisionId::parse("rev_01JRESTORE").unwrap()),
        };

        assert_eq!(
            tombstone
                .classify_restore_eligibility()
                .expect("restored metadata should classify"),
            RestoreEligibility::AlreadyRestored {
                restored_at: timestamp("2026-07-15T00:00:00Z"),
                restored_by: AdapterId::parse("worktree-adapter").unwrap(),
                restore_revision_id: RevisionId::parse("rev_01JRESTORE").unwrap(),
            }
        );
        assert_eq!(
            tombstone
                .classify_retention_cleanup(timestamp("2026-09-01T00:00:00Z"))
                .expect("restored metadata should classify"),
            RetentionCleanupEligibility::NotEligibleRestored {
                restored_at: timestamp("2026-07-15T00:00:00Z"),
                restore_revision_id: RevisionId::parse("rev_01JRESTORE").unwrap(),
            }
        );
    }

    #[test]
    fn incomplete_or_precreation_restore_metadata_is_rejected() {
        let mut incomplete = tombstone();
        incomplete.restore.restored_at = Some(timestamp("2026-07-15T00:00:00Z"));
        assert_eq!(
            incomplete
                .classify_restore_eligibility()
                .expect_err("partial restore metadata should reject"),
            TombstoneServiceError::InvalidRestoreMetadata
        );

        let mut before_creation = tombstone();
        before_creation.restore = RestoreMetadata {
            restored_at: Some(timestamp("2026-07-01T00:00:00Z")),
            restored_by: Some(AdapterId::parse("worktree-adapter").unwrap()),
            restore_revision_id: Some(RevisionId::parse("rev_01JRESTORE").unwrap()),
        };
        assert_eq!(
            before_creation
                .validate_metadata()
                .expect_err("restore must follow creation"),
            TombstoneServiceError::InvalidRestoreMetadata
        );
    }
}
