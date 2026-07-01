//! Pure Core-facing operation log and cursor types.
//!
//! This module defines deterministic value types for the future changes feed. It
//! does not apply file revisions, perform conflict/delete policy, update storage,
//! call providers, or implement route handlers.

use chrono::{DateTime, Utc};
use haze_sync_common::{
    AdapterId, ConflictId, ContentHash, OperationId, RevisionId, VaultPath,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{error::Error, fmt, str::FromStr};

/// Default page size used by future changes-feed callers when no explicit limit
/// is supplied by a route layer.
pub const DEFAULT_CHANGES_LIMIT: u32 = 500;

/// Maximum operation-log entries returned by a single changes page.
pub const MAX_CHANGES_LIMIT: u32 = 1_000;

const TOMBSTONE_PREFIX: &str = "tmb_";
const MAX_TOMBSTONE_ID_LEN: usize = 128;

/// Safe validation errors for operation-log and cursor value types.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum OperationLogError {
    /// A sequence value must be zero or greater.
    NegativeSequence,
    /// A page limit must be at least one.
    ZeroLimit,
    /// A page limit exceeded the contract maximum.
    LimitTooLarge { max: u32 },
    /// Operation rows must be strictly ordered by ascending sequence.
    NonMonotonicSequence,
    /// An operation kind string is not part of the V1 public contract.
    InvalidOperationKind,
    /// Tombstone identifiers must be safe public identifiers with the V1 prefix.
    InvalidTombstoneId,
}

impl fmt::Display for OperationLogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NegativeSequence => formatter.write_str("operation sequence must be non-negative"),
            Self::ZeroLimit => formatter.write_str("changes limit must be at least one"),
            Self::LimitTooLarge { max } => {
                write!(formatter, "changes limit must not exceed {max}")
            }
            Self::NonMonotonicSequence => formatter.write_str(
                "operation log entries must be ordered by strictly increasing sequence",
            ),
            Self::InvalidOperationKind => formatter.write_str("operation kind is not supported"),
            Self::InvalidTombstoneId => formatter.write_str("tombstone id is invalid"),
        }
    }
}

impl Error for OperationLogError {}

/// Monotonic operation-log sequence value.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OperationSequence(i64);

impl OperationSequence {
    /// Sequence zero is the initial cursor before any operation has been read.
    pub const ZERO: Self = Self(0);

    /// Validate and construct a sequence value.
    pub fn new(value: i64) -> Result<Self, OperationLogError> {
        if value < 0 {
            return Err(OperationLogError::NegativeSequence);
        }

        Ok(Self(value))
    }

    /// Return the raw database sequence value.
    #[must_use]
    pub const fn value(self) -> i64 {
        self.0
    }
}

impl fmt::Display for OperationSequence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// Bounded changes-feed page limit.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ChangesLimit(u32);

impl ChangesLimit {
    /// Validate and construct a bounded limit.
    pub fn new(value: u32) -> Result<Self, OperationLogError> {
        if value == 0 {
            return Err(OperationLogError::ZeroLimit);
        }

        if value > MAX_CHANGES_LIMIT {
            return Err(OperationLogError::LimitTooLarge {
                max: MAX_CHANGES_LIMIT,
            });
        }

        Ok(Self(value))
    }

    /// Contract default limit.
    #[must_use]
    pub const fn default_limit() -> Self {
        Self(DEFAULT_CHANGES_LIMIT)
    }

    /// Return the raw limit value.
    #[must_use]
    pub const fn value(self) -> u32 {
        self.0
    }

    /// Return a SQL-friendly limit including one sentinel row for has_more.
    #[must_use]
    pub const fn value_with_sentinel(self) -> i64 {
        self.0 as i64 + 1
    }
}

impl Default for ChangesLimit {
    fn default() -> Self {
        Self::default_limit()
    }
}

/// Query parameters for reading operation-log changes.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ChangesQuery {
    /// Return operations strictly after this sequence.
    pub since: OperationSequence,
    /// Maximum number of operations returned to the caller.
    pub limit: ChangesLimit,
}

impl ChangesQuery {
    /// Validate and construct a changes query.
    pub fn new(since: i64, limit: u32) -> Result<Self, OperationLogError> {
        Ok(Self {
            since: OperationSequence::new(since)?,
            limit: ChangesLimit::new(limit)?,
        })
    }
}

/// Operation kinds exposed by the V1 changes-feed contract.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationKind {
    /// File was created or updated.
    UpsertFile,
    /// File was tombstoned.
    DeleteFile,
    /// Tombstoned file was restored.
    RestoreFile,
    /// Conflict record was created.
    ConflictCreated,
    /// Conflict record was resolved.
    ConflictResolved,
    /// Backup copy was created by a conflict policy.
    BackupCreated,
}

impl OperationKind {
    /// Return the contract string stored in the operation log.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UpsertFile => "upsert_file",
            Self::DeleteFile => "delete_file",
            Self::RestoreFile => "restore_file",
            Self::ConflictCreated => "conflict_created",
            Self::ConflictResolved => "conflict_resolved",
            Self::BackupCreated => "backup_created",
        }
    }
}

impl fmt::Display for OperationKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for OperationKind {
    type Err = OperationLogError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "upsert_file" => Ok(Self::UpsertFile),
            "delete_file" => Ok(Self::DeleteFile),
            "restore_file" => Ok(Self::RestoreFile),
            "conflict_created" => Ok(Self::ConflictCreated),
            "conflict_resolved" => Ok(Self::ConflictResolved),
            "backup_created" => Ok(Self::BackupCreated),
            _ => Err(OperationLogError::InvalidOperationKind),
        }
    }
}

/// Public tombstone identifier used by delete-file operations.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TombstoneId(String);

impl TombstoneId {
    /// Validate and construct a tombstone identifier.
    pub fn parse(input: &str) -> Result<Self, OperationLogError> {
        if input.len() <= TOMBSTONE_PREFIX.len()
            || input.len() > MAX_TOMBSTONE_ID_LEN
            || !input.starts_with(TOMBSTONE_PREFIX)
            || input.as_bytes().contains(&0)
            || !input.chars().all(is_identifier_char)
        {
            return Err(OperationLogError::InvalidTombstoneId);
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
    type Err = OperationLogError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Self::parse(input)
    }
}

fn is_identifier_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.')
}

/// Append-only operation log entry consumed by adapters.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SyncOperation {
    /// Global database-generated sequence number.
    pub seq: OperationSequence,
    /// Unique public operation identifier.
    pub op_id: OperationId,
    /// Adapter that caused the operation.
    pub adapter_id: AdapterId,
    /// Contract operation kind.
    pub kind: OperationKind,
    /// Vault path affected by the operation.
    pub path: VaultPath,
    /// Revision associated with file-like operations when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision_id: Option<RevisionId>,
    /// Tombstone associated with delete operations when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tombstone_id: Option<TombstoneId>,
    /// Conflict associated with conflict operations when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflict_id: Option<ConflictId>,
    /// Time when storage appended the operation.
    pub created_at: DateTime<Utc>,
}

/// Changes-feed entry enriched with immutable file metadata when available.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ChangeFeedEntry {
    /// Append-only operation metadata.
    pub operation: SyncOperation,
    /// Content hash for upsert-like operations when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_sha256: Option<ContentHash>,
    /// Content size for upsert-like operations when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
}

/// Validated ordered operation page.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ChangesPage {
    /// Sequence supplied by the adapter request.
    pub from_seq: OperationSequence,
    /// Highest sequence included in this page, or from_seq when empty.
    pub to_seq: OperationSequence,
    /// True when another request is needed to continue pagination.
    pub has_more: bool,
    /// Ordered operations after from_seq.
    pub changes: Vec<ChangeFeedEntry>,
}

impl ChangesPage {
    /// Build a changes page after validating strict sequence ordering.
    pub fn new(
        query: ChangesQuery,
        changes: Vec<ChangeFeedEntry>,
        has_more: bool,
    ) -> Result<Self, OperationLogError> {
        let mut previous = query.since;
        for change in &changes {
            if change.operation.seq <= previous {
                return Err(OperationLogError::NonMonotonicSequence);
            }
            previous = change.operation.seq;
        }

        let to_seq = changes
            .last()
            .map_or(query.since, |change| change.operation.seq);

        Ok(Self {
            from_seq: query.since,
            to_seq,
            has_more,
            changes,
        })
    }
}

/// Adapter progress cursor.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AdapterCursor {
    /// Adapter that owns this cursor.
    pub adapter_id: AdapterId,
    /// Highest Core operation sequence successfully processed by the adapter.
    pub last_core_seq: OperationSequence,
    /// Provider/local cursor metadata. This must not contain secrets.
    pub external_cursor_json: Value,
    /// Time of the last successful cursor advancement when known.
    pub last_success_at: Option<DateTime<Utc>>,
}

/// Safe outcome for a requested adapter cursor update.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CursorUpdateOutcome {
    /// Cursor moved to a higher sequence.
    Advanced,
    /// Cursor was already at the requested sequence.
    Unchanged,
    /// Requested sequence was below the current cursor and must not be applied.
    RejectedRegression,
}

/// Classify a cursor update without mutating storage.
pub fn classify_cursor_update(
    current: OperationSequence,
    requested: OperationSequence,
) -> CursorUpdateOutcome {
    if requested > current {
        CursorUpdateOutcome::Advanced
    } else if requested == current {
        CursorUpdateOutcome::Unchanged
    } else {
        CursorUpdateOutcome::RejectedRegression
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_change(seq: i64) -> ChangeFeedEntry {
        ChangeFeedEntry {
            operation: SyncOperation {
                seq: OperationSequence::new(seq).unwrap(),
                op_id: OperationId::parse(&format!("op_{seq}")).unwrap(),
                adapter_id: AdapterId::parse("worktree-adapter").unwrap(),
                kind: OperationKind::UpsertFile,
                path: VaultPath::parse("Notes/a.md").unwrap(),
                revision_id: Some(RevisionId::parse(&format!("rev_{seq}")).unwrap()),
                tombstone_id: None,
                conflict_id: None,
                created_at: DateTime::<Utc>::from(std::time::UNIX_EPOCH),
            },
            content_sha256: None,
            size_bytes: Some(1),
        }
    }

    #[test]
    fn changes_limit_rejects_unsafe_values() {
        assert_eq!(ChangesLimit::new(0), Err(OperationLogError::ZeroLimit));
        assert_eq!(
            ChangesLimit::new(MAX_CHANGES_LIMIT + 1),
            Err(OperationLogError::LimitTooLarge {
                max: MAX_CHANGES_LIMIT
            })
        );
        assert_eq!(ChangesLimit::new(MAX_CHANGES_LIMIT).unwrap().value(), 1_000);
    }

    #[test]
    fn sequence_rejects_negative_values() {
        assert_eq!(
            OperationSequence::new(-1),
            Err(OperationLogError::NegativeSequence)
        );
        assert_eq!(OperationSequence::new(0).unwrap(), OperationSequence::ZERO);
    }

    #[test]
    fn operation_kind_matches_contract_strings() {
        assert_eq!(OperationKind::UpsertFile.as_str(), "upsert_file");
        assert_eq!(
            OperationKind::from_str("conflict_resolved").unwrap(),
            OperationKind::ConflictResolved
        );
        assert_eq!(
            serde_json::to_string(&OperationKind::BackupCreated).unwrap(),
            "\"backup_created\""
        );
        assert_eq!(
            OperationKind::from_str("overwrite_file"),
            Err(OperationLogError::InvalidOperationKind)
        );
    }

    #[test]
    fn tombstone_id_requires_public_prefix() {
        assert_eq!(TombstoneId::parse("tmb_01JTEST").unwrap().as_str(), "tmb_01JTEST");
        assert_eq!(
            TombstoneId::parse("../secret"),
            Err(OperationLogError::InvalidTombstoneId)
        );
        assert_eq!(
            TombstoneId::parse("tmb_"),
            Err(OperationLogError::InvalidTombstoneId)
        );
    }

    #[test]
    fn changes_page_preserves_ordering_and_to_seq() {
        let query = ChangesQuery::new(10, 100).unwrap();
        let page = ChangesPage::new(query, vec![sample_change(11), sample_change(12)], false).unwrap();

        assert_eq!(page.from_seq.value(), 10);
        assert_eq!(page.to_seq.value(), 12);
        assert!(!page.has_more);
    }

    #[test]
    fn changes_page_rejects_non_monotonic_rows() {
        let query = ChangesQuery::new(10, 100).unwrap();
        assert_eq!(
            ChangesPage::new(query, vec![sample_change(12), sample_change(12)], false),
            Err(OperationLogError::NonMonotonicSequence)
        );
    }

    #[test]
    fn cursor_update_classification_is_monotonic() {
        let current = OperationSequence::new(5).unwrap();
        assert_eq!(
            classify_cursor_update(current, OperationSequence::new(8).unwrap()),
            CursorUpdateOutcome::Advanced
        );
        assert_eq!(
            classify_cursor_update(current, OperationSequence::new(5).unwrap()),
            CursorUpdateOutcome::Unchanged
        );
        assert_eq!(
            classify_cursor_update(current, OperationSequence::new(4).unwrap()),
            CursorUpdateOutcome::RejectedRegression
        );
    }
}
