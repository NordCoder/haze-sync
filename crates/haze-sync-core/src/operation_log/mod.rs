//! Pure Core-facing operation log and cursor types.
//!
//! This module defines deterministic value types for the future changes feed. It
//! does not apply file revisions, perform conflict/delete policy, update storage,
//! call providers, or implement route handlers.

use chrono::{DateTime, Utc};
use haze_sync_common::{AdapterId, ConflictId, ContentHash, OperationId, RevisionId, VaultPath};
use serde::{Deserialize, Deserializer, Serialize};
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
    /// A changes page contains more rows than its applicable limit.
    PageTooLarge { max: u32 },
    /// Operation rows must be strictly ordered by ascending sequence.
    NonMonotonicSequence,
    /// A page that advertises more rows must advance its sequence.
    NonAdvancingPage,
    /// Serialized page bounds do not match the ordered operation rows.
    InconsistentPageBounds,
    /// An operation kind string is not part of the V1 public contract.
    InvalidOperationKind,
    /// Tombstone identifiers must be safe public identifiers with the V1 prefix.
    InvalidTombstoneId,
}

impl fmt::Display for OperationLogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NegativeSequence => {
                formatter.write_str("operation sequence must be non-negative")
            }
            Self::ZeroLimit => formatter.write_str("changes limit must be at least one"),
            Self::LimitTooLarge { max } => {
                write!(formatter, "changes limit must not exceed {max}")
            }
            Self::PageTooLarge { max } => {
                write!(formatter, "changes page must not contain more than {max} rows")
            }
            Self::NonMonotonicSequence => formatter
                .write_str("operation log entries must be ordered by strictly increasing sequence"),
            Self::NonAdvancingPage => {
                formatter.write_str("changes page with has_more must advance its sequence")
            }
            Self::InconsistentPageBounds => {
                formatter.write_str("changes page bounds do not match its operation rows")
            }
            Self::InvalidOperationKind => formatter.write_str("operation kind is not supported"),
            Self::InvalidTombstoneId => formatter.write_str("tombstone id is invalid"),
        }
    }
}

impl Error for OperationLogError {}

/// Monotonic operation-log sequence value.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
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

impl<'de> Deserialize<'de> for OperationSequence {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = i64::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

impl fmt::Display for OperationSequence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// Bounded changes-feed page limit.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
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

    /// Return a SQL-friendly limit including one sentinel row for `has_more`.
    #[must_use]
    pub const fn value_with_sentinel(self) -> i64 {
        self.0 as i64 + 1
    }
}

impl<'de> Deserialize<'de> for ChangesLimit {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = u32::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
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
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
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

impl<'de> Deserialize<'de> for TombstoneId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(serde::de::Error::custom)
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
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ChangesPage {
    /// Sequence supplied by the adapter request.
    pub from_seq: OperationSequence,
    /// Highest sequence included in this page, or `from_seq` when empty.
    pub to_seq: OperationSequence,
    /// True when another request is needed to continue pagination.
    pub has_more: bool,
    /// Ordered operations after `from_seq`.
    pub changes: Vec<ChangeFeedEntry>,
}

impl ChangesPage {
    /// Build a changes page after validating its limit, progress, and ordering.
    pub fn new(
        query: ChangesQuery,
        changes: Vec<ChangeFeedEntry>,
        has_more: bool,
    ) -> Result<Self, OperationLogError> {
        let to_seq = validate_page_shape(
            query.since,
            &changes,
            has_more,
            query.limit.value(),
        )?;

        Ok(Self {
            from_seq: query.since,
            to_seq,
            has_more,
            changes,
        })
    }
}

impl<'de> Deserialize<'de> for ChangesPage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct ChangesPageWire {
            from_seq: OperationSequence,
            to_seq: OperationSequence,
            has_more: bool,
            changes: Vec<ChangeFeedEntry>,
        }

        let wire = ChangesPageWire::deserialize(deserializer)?;
        let expected_to_seq = validate_page_shape(
            wire.from_seq,
            &wire.changes,
            wire.has_more,
            MAX_CHANGES_LIMIT,
        )
        .map_err(serde::de::Error::custom)?;
        if wire.to_seq != expected_to_seq {
            return Err(serde::de::Error::custom(
                OperationLogError::InconsistentPageBounds,
            ));
        }

        Ok(Self {
            from_seq: wire.from_seq,
            to_seq: wire.to_seq,
            has_more: wire.has_more,
            changes: wire.changes,
        })
    }
}

fn validate_page_shape(
    from_seq: OperationSequence,
    changes: &[ChangeFeedEntry],
    has_more: bool,
    max_changes: u32,
) -> Result<OperationSequence, OperationLogError> {
    if changes.len() > max_changes as usize {
        return Err(OperationLogError::PageTooLarge { max: max_changes });
    }

    if has_more && changes.is_empty() {
        return Err(OperationLogError::NonAdvancingPage);
    }

    validate_change_order(from_seq, changes)
}

fn validate_change_order(
    from_seq: OperationSequence,
    changes: &[ChangeFeedEntry],
) -> Result<OperationSequence, OperationLogError> {
    let mut previous = from_seq;
    for change in changes {
        if change.operation.seq <= previous {
            return Err(OperationLogError::NonMonotonicSequence);
        }
        previous = change.operation.seq;
    }

    Ok(previous)
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

impl AdapterCursor {
    /// Classify a requested Core sequence without mutating durable cursor state.
    #[must_use]
    pub fn classify_core_sequence_update(
        &self,
        requested: OperationSequence,
    ) -> CursorUpdateOutcome {
        classify_cursor_update(self.last_core_seq, requested)
    }
}

/// Safe outcome for a requested adapter cursor update.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
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
#[must_use]
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
    use serde_json::json;

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

    fn adapter_cursor(last_core_seq: i64) -> AdapterCursor {
        AdapterCursor {
            adapter_id: AdapterId::parse("worktree-adapter").unwrap(),
            last_core_seq: OperationSequence::new(last_core_seq).unwrap(),
            external_cursor_json: json!({"page_token": "public-fixture"}),
            last_success_at: None,
        }
    }

    #[test]
    fn changes_limit_covers_all_boundaries_and_validating_serde() {
        assert_eq!(ChangesLimit::new(0), Err(OperationLogError::ZeroLimit));
        assert_eq!(ChangesLimit::new(1).unwrap().value(), 1);
        assert_eq!(ChangesLimit::default().value(), DEFAULT_CHANGES_LIMIT);
        assert_eq!(ChangesLimit::new(MAX_CHANGES_LIMIT).unwrap().value(), 1_000);
        assert_eq!(
            ChangesLimit::new(MAX_CHANGES_LIMIT)
                .unwrap()
                .value_with_sentinel(),
            1_001
        );
        assert_eq!(
            ChangesLimit::new(MAX_CHANGES_LIMIT + 1),
            Err(OperationLogError::LimitTooLarge {
                max: MAX_CHANGES_LIMIT
            })
        );
        assert!(serde_json::from_str::<ChangesLimit>("0").is_err());
        assert!(serde_json::from_str::<ChangesLimit>("1001").is_err());
        assert_eq!(
            serde_json::from_str::<ChangesLimit>("1").unwrap().value(),
            1
        );
    }

    #[test]
    fn sequence_validation_cannot_be_bypassed_by_serde() {
        assert_eq!(
            OperationSequence::new(-1),
            Err(OperationLogError::NegativeSequence)
        );
        assert_eq!(OperationSequence::new(0).unwrap(), OperationSequence::ZERO);
        assert_eq!(OperationSequence::new(i64::MAX).unwrap().value(), i64::MAX);
        assert!(serde_json::from_str::<OperationSequence>("-1").is_err());
        assert_eq!(
            serde_json::from_str::<OperationSequence>("0").unwrap(),
            OperationSequence::ZERO
        );
    }

    #[test]
    fn changes_query_deserialization_enforces_sequence_and_limit_bounds() {
        assert!(serde_json::from_value::<ChangesQuery>(json!({
            "since": -1,
            "limit": 100
        }))
        .is_err());
        assert!(serde_json::from_value::<ChangesQuery>(json!({
            "since": 0,
            "limit": 0
        }))
        .is_err());
        assert!(serde_json::from_value::<ChangesQuery>(json!({
            "since": 0,
            "limit": 1001
        }))
        .is_err());
        assert_eq!(
            serde_json::from_value::<ChangesQuery>(json!({
                "since": 0,
                "limit": 1
            }))
            .unwrap(),
            ChangesQuery::new(0, 1).unwrap()
        );
    }

    #[test]
    fn operation_kind_parse_display_and_serde_are_stable_for_all_variants() {
        for kind in [
            OperationKind::UpsertFile,
            OperationKind::DeleteFile,
            OperationKind::RestoreFile,
            OperationKind::ConflictCreated,
            OperationKind::ConflictResolved,
            OperationKind::BackupCreated,
        ] {
            assert_eq!(OperationKind::from_str(kind.as_str()).unwrap(), kind);
            assert_eq!(kind.to_string(), kind.as_str());
            let serialized = serde_json::to_string(&kind).unwrap();
            assert_eq!(serialized, format!("\"{}\"", kind.as_str()));
            assert_eq!(
                serde_json::from_str::<OperationKind>(&serialized).unwrap(),
                kind
            );
        }

        for invalid in ["overwrite_file", "UPSERT_FILE", "upsert-file", ""] {
            assert_eq!(
                OperationKind::from_str(invalid),
                Err(OperationLogError::InvalidOperationKind)
            );
            assert!(serde_json::from_str::<OperationKind>(&format!("\"{invalid}\"")).is_err());
        }
    }

    #[test]
    fn tombstone_id_validation_cannot_be_bypassed_by_serde() {
        assert_eq!(
            TombstoneId::parse("tmb_01JTEST").unwrap().as_str(),
            "tmb_01JTEST"
        );
        for invalid in ["../secret", "tmb_", "tmb_bad/id"] {
            assert_eq!(
                TombstoneId::parse(invalid),
                Err(OperationLogError::InvalidTombstoneId)
            );
            assert!(serde_json::from_str::<TombstoneId>(&format!("\"{invalid}\"")).is_err());
        }
    }

    #[test]
    fn changes_page_preserves_ordering_bounds_and_empty_cursor() {
        let query = ChangesQuery::new(10, 100).unwrap();
        let page =
            ChangesPage::new(query, vec![sample_change(11), sample_change(12)], false).unwrap();

        assert_eq!(page.from_seq.value(), 10);
        assert_eq!(page.to_seq.value(), 12);
        assert!(!page.has_more);
        assert_eq!(
            serde_json::from_str::<ChangesPage>(&serde_json::to_string(&page).unwrap()).unwrap(),
            page
        );

        let empty = ChangesPage::new(query, Vec::new(), false).unwrap();
        assert_eq!(empty.from_seq, query.since);
        assert_eq!(empty.to_seq, query.since);
    }

    #[test]
    fn changes_page_rejects_duplicate_regression_and_inconsistent_bounds() {
        let query = ChangesQuery::new(10, 100).unwrap();
        assert_eq!(
            ChangesPage::new(query, vec![sample_change(12), sample_change(12)], false),
            Err(OperationLogError::NonMonotonicSequence)
        );
        assert_eq!(
            ChangesPage::new(query, vec![sample_change(9)], false),
            Err(OperationLogError::NonMonotonicSequence)
        );

        let inconsistent = json!({
            "from_seq": 10,
            "to_seq": 99,
            "has_more": false,
            "changes": [sample_change(11)]
        });
        assert!(serde_json::from_value::<ChangesPage>(inconsistent).is_err());

        let non_monotonic = json!({
            "from_seq": 10,
            "to_seq": 12,
            "has_more": false,
            "changes": [sample_change(12), sample_change(12)]
        });
        assert!(serde_json::from_value::<ChangesPage>(non_monotonic).is_err());
    }

    #[test]
    fn changes_page_enforces_limit_and_progress() {
        let query = ChangesQuery::new(10, 1).unwrap();
        let full_page = ChangesPage::new(query, vec![sample_change(11)], true).unwrap();
        assert_eq!(full_page.to_seq.value(), 11);
        assert!(full_page.has_more);

        assert_eq!(
            ChangesPage::new(query, vec![sample_change(11), sample_change(12)], false),
            Err(OperationLogError::PageTooLarge { max: 1 })
        );
        assert_eq!(
            ChangesPage::new(query, Vec::new(), true),
            Err(OperationLogError::NonAdvancingPage)
        );

        let non_advancing = json!({
            "from_seq": 10,
            "to_seq": 10,
            "has_more": true,
            "changes": []
        });
        assert!(serde_json::from_value::<ChangesPage>(non_advancing).is_err());

        let oversized_changes: Vec<_> = (1..=i64::from(MAX_CHANGES_LIMIT) + 1)
            .map(sample_change)
            .collect();
        let oversized = json!({
            "from_seq": 0,
            "to_seq": i64::from(MAX_CHANGES_LIMIT) + 1,
            "has_more": false,
            "changes": oversized_changes
        });
        assert!(serde_json::from_value::<ChangesPage>(oversized).is_err());
    }

    #[test]
    fn cursor_update_classification_is_monotonic_at_boundaries() {
        assert_eq!(
            classify_cursor_update(OperationSequence::ZERO, OperationSequence::new(1).unwrap()),
            CursorUpdateOutcome::Advanced
        );

        let cursor = adapter_cursor(5);
        assert_eq!(
            cursor.classify_core_sequence_update(OperationSequence::new(8).unwrap()),
            CursorUpdateOutcome::Advanced
        );
        assert_eq!(
            cursor.classify_core_sequence_update(OperationSequence::new(5).unwrap()),
            CursorUpdateOutcome::Unchanged
        );
        assert_eq!(
            cursor.classify_core_sequence_update(OperationSequence::new(4).unwrap()),
            CursorUpdateOutcome::RejectedRegression
        );

        let max = OperationSequence::new(i64::MAX).unwrap();
        assert_eq!(
            classify_cursor_update(max, max),
            CursorUpdateOutcome::Unchanged
        );
        assert_eq!(
            classify_cursor_update(max, OperationSequence::new(i64::MAX - 1).unwrap()),
            CursorUpdateOutcome::RejectedRegression
        );
    }

    #[test]
    fn cursor_update_outcome_serde_names_are_stable() {
        for (outcome, expected) in [
            (CursorUpdateOutcome::Advanced, "advanced"),
            (CursorUpdateOutcome::Unchanged, "unchanged"),
            (
                CursorUpdateOutcome::RejectedRegression,
                "rejected_regression",
            ),
        ] {
            let serialized = serde_json::to_string(&outcome).unwrap();
            assert_eq!(serialized, format!("\"{expected}\""));
            assert_eq!(
                serde_json::from_str::<CursorUpdateOutcome>(&serialized).unwrap(),
                outcome
            );
        }
    }
}
