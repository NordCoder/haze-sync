//! Pure service-level tombstone primitives.
//!
//! Creating a tombstone records safe delete intent and retention metadata only.
//! This module does not hard-delete storage rows, remove blobs, move files to
//! trash, run retention cleanup, call adapters, or implement restore behavior.

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
            || !input
                .chars()
                .all(|character| {
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
        let current_revision_id = current_revision_id
            .unwrap_or_else(|| deleted_revision_id.clone());
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

/// Restore-ready metadata shape. Restore behavior is intentionally not
/// implemented in this phase.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreMetadata {
    pub restored_at: Option<DateTime<Utc>>,
    pub restored_by: Option<AdapterId>,
    pub restore_revision_id: Option<RevisionId>,
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
        let deleted_by =
            AdapterId::parse(deleted_by).map_err(|_error| TombstoneServiceError::InvalidAdapterId)?;

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
        validate_retention(input.created_at, input.retention.retention_until)?;

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
        })
    }
}

impl Error for TombstoneServiceError {}

fn validate_retention(
    created_at: Option<DateTime<Utc>>,
    retention_until: DateTime<Utc>,
) -> Result<(), TombstoneServiceError> {
    if let Some(created_at) = created_at {
        if retention_until <= created_at {
            return Err(TombstoneServiceError::InvalidRetention);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timestamp(value: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(value)
            .unwrap()
            .with_timezone(&Utc)
    }

    fn retention() -> TombstoneRetention {
        TombstoneRetention::new(timestamp("2026-08-01T00:00:00Z"), Some(30))
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
            .unwrap_err(),
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
            .unwrap_err(),
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
            .unwrap_err(),
            TombstoneServiceError::InvalidAdapterId
        );
    }

    #[test]
    fn creates_restore_ready_tombstone_without_restore_behavior() {
        let input = TombstoneCreationInput::try_from_raw(
            "tmb_01JTEST",
            "Notes/old.md",
            "rev_01JDELETE",
            Some("rev_01JCURRENT"),
            "worktree-adapter",
            retention(),
            Some(timestamp("2026-07-01T00:00:00Z")),
        )
        .unwrap();

        let tombstone = TombstoneService::new().create_tombstone(input).unwrap();

        assert_eq!(tombstone.path.as_str(), "Notes/old.md");
        assert_eq!(tombstone.deleted_revision_id().as_str(), "rev_01JDELETE");
        assert_eq!(tombstone.current_revision_id().as_str(), "rev_01JCURRENT");
        assert_eq!(tombstone.deleted_by.as_str(), "worktree-adapter");
        assert_eq!(tombstone.retention.retention_days, Some(30));
        assert_eq!(tombstone.created_at, Some(timestamp("2026-07-01T00:00:00Z")));
        assert_eq!(tombstone.restore, RestoreMetadata::default());
    }

    #[test]
    fn rejects_retention_that_does_not_outlive_creation_timestamp() {
        let input = TombstoneCreationInput::try_from_raw(
            "tmb_01JTEST",
            "Notes/old.md",
            "rev_01JDELETE",
            None,
            "worktree-adapter",
            TombstoneRetention::new(timestamp("2026-07-01T00:00:00Z"), Some(30)),
            Some(timestamp("2026-07-01T00:00:00Z")),
        )
        .unwrap();

        assert_eq!(
            TombstoneService::new().create_tombstone(input).unwrap_err(),
            TombstoneServiceError::InvalidRetention
        );
    }
}
