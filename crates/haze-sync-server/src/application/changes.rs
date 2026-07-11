use super::ApplicationError;
use chrono::{DateTime, Utc};
use haze_sync_common::{AdapterId, ConflictId, ContentHash, RevisionId, VaultPath};
use haze_sync_storage::repositories::operation_log::{
    ChangeFeedRow, OperationKindName, OperationLogRepository,
};
use sqlx::PgPool;
use std::str::FromStr;

const MAX_AUTHORITATIVE_CHANGES_LIMIT: u32 = 1_000;

/// Validated bounded internal changes query.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct AuthoritativeChangesQuery {
    since: i64,
    limit: u32,
}

impl AuthoritativeChangesQuery {
    pub(crate) fn new(since: i64, limit: u32) -> Result<Self, ApplicationError> {
        if since < 0 || limit == 0 || limit > MAX_AUTHORITATIVE_CHANGES_LIMIT {
            return Err(ApplicationError::InvalidInput);
        }
        Ok(Self { since, limit })
    }

    #[must_use]
    pub(crate) const fn since(self) -> i64 {
        self.since
    }

    #[must_use]
    pub(crate) const fn limit(self) -> u32 {
        self.limit
    }
}

/// One ordered authoritative operation-log fact.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AuthoritativeChange {
    pub(crate) seq: i64,
    pub(crate) kind: OperationKindName,
    pub(crate) path: VaultPath,
    pub(crate) revision_id: Option<RevisionId>,
    pub(crate) content_hash: Option<ContentHash>,
    pub(crate) size_bytes: Option<u64>,
    pub(crate) tombstone_id: Option<String>,
    pub(crate) conflict_id: Option<ConflictId>,
    pub(crate) actor_id: AdapterId,
    pub(crate) occurred_at: DateTime<Utc>,
}

/// Bounded ordered authoritative change batch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AuthoritativeChangeBatch {
    pub(crate) from: i64,
    pub(crate) to: i64,
    pub(crate) has_more: bool,
    pub(crate) changes: Vec<AuthoritativeChange>,
}

pub(super) async fn authoritative_changes(
    pool: &PgPool,
    query: AuthoritativeChangesQuery,
) -> Result<AuthoritativeChangeBatch, ApplicationError> {
    let page = OperationLogRepository::new()
        .changes_since(pool, query.since(), query.limit())
        .await
        .map_err(|_| ApplicationError::Internal)?;
    let changes = page
        .changes
        .into_iter()
        .map(authoritative_change_from_row)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(AuthoritativeChangeBatch {
        from: page.from_seq,
        to: page.to_seq,
        has_more: page.has_more,
        changes,
    })
}

fn authoritative_change_from_row(
    row: ChangeFeedRow,
) -> Result<AuthoritativeChange, ApplicationError> {
    Ok(AuthoritativeChange {
        seq: row.seq,
        kind: OperationKindName::from_str(row.kind.as_str())
            .map_err(|_| ApplicationError::Internal)?,
        path: VaultPath::parse(row.path.as_str()).map_err(|_| ApplicationError::Internal)?,
        revision_id: row
            .revision_id
            .as_deref()
            .map(RevisionId::parse)
            .transpose()
            .map_err(|_| ApplicationError::Internal)?,
        content_hash: row
            .content_sha256
            .as_deref()
            .map(ContentHash::parse)
            .transpose()
            .map_err(|_| ApplicationError::Internal)?,
        size_bytes: row
            .size_bytes
            .map(u64::try_from)
            .transpose()
            .map_err(|_| ApplicationError::Internal)?,
        tombstone_id: row.tombstone_id,
        conflict_id: row
            .conflict_id
            .as_deref()
            .map(ConflictId::parse)
            .transpose()
            .map_err(|_| ApplicationError::Internal)?,
        actor_id: AdapterId::parse(row.adapter_id.as_str())
            .map_err(|_| ApplicationError::Internal)?,
        occurred_at: row.created_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_rejects_zero_unbounded_and_negative_values() {
        assert_eq!(
            AuthoritativeChangesQuery::new(-1, 1),
            Err(ApplicationError::InvalidInput)
        );
        assert_eq!(
            AuthoritativeChangesQuery::new(0, 0),
            Err(ApplicationError::InvalidInput)
        );
        assert_eq!(
            AuthoritativeChangesQuery::new(0, 1_001),
            Err(ApplicationError::InvalidInput)
        );
        assert_eq!(AuthoritativeChangesQuery::new(0, 1).unwrap().limit(), 1);
    }
}
