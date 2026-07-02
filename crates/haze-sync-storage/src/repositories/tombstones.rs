//! Passive SQLx repository helpers for `tombstones` rows.
//!
//! This module creates and reads tombstone metadata only. It never hard-deletes
//! rows, removes blobs, clears retention, runs cleanup jobs, calls providers, or
//! decides whether a delete is safe.

use super::{map_sqlx_error, validate_limit, RepositoryError, RepositoryResult};
use crate::models::TombstoneRow;
use chrono::{DateTime, Utc};
use haze_sync_common::{AdapterId, RevisionId, VaultPath};
use sqlx::{postgres::PgRow, Executor, Postgres, Row};

const INSERT_TOMBSTONE_SQL: &str = "insert into tombstones \
     (tombstone_id, path, deleted_revision_id, deleted_by, retention_until) \
     values ($1, $2, $3, $4, $5) \
     returning tombstone_id, path, deleted_revision_id, deleted_by, deleted_at, \
     retention_until, restored_at";
const GET_TOMBSTONE_BY_ID_SQL: &str = "select tombstone_id, path, \
     deleted_revision_id, deleted_by, deleted_at, retention_until, restored_at \
     from tombstones where tombstone_id = $1";
const LIST_TOMBSTONES_BY_PATH_SQL: &str = "select tombstone_id, path, \
     deleted_revision_id, deleted_by, deleted_at, retention_until, restored_at \
     from tombstones where path = $1 order by deleted_at desc, tombstone_id desc limit $2";
const LIST_ACTIVE_TOMBSTONES_SQL: &str = "select tombstone_id, path, \
     deleted_revision_id, deleted_by, deleted_at, retention_until, restored_at \
     from tombstones where restored_at is null order by deleted_at desc, \
     tombstone_id desc limit $1";

/// Input for inserting a safe tombstone row.
#[derive(Clone, Debug)]
pub struct NewTombstone<'a> {
    /// Caller-assigned tombstone id, typically with `tmb_` prefix.
    pub tombstone_id: &'a str,
    /// Normalized vault path that was deleted.
    pub path: &'a VaultPath,
    /// Current/deleted revision being tombstoned when known.
    pub deleted_revision_id: Option<&'a RevisionId>,
    /// Adapter or actor that requested the delete.
    pub deleted_by: &'a AdapterId,
    /// Retention boundary. Physical cleanup is outside this repository.
    pub retention_until: DateTime<Utc>,
}

/// Inserts a tombstone row and returns database-generated timestamp metadata.
///
/// This helper does not mark `sync_objects.deleted_at`, append operation-log
/// entries, enforce delete guards, move files to trash, or physically delete
/// anything.
pub async fn insert_tombstone<'executor, ExecutorType>(
    executor: ExecutorType,
    input: NewTombstone<'_>,
) -> RepositoryResult<TombstoneRow>
where
    ExecutorType: Executor<'executor, Database = Postgres>,
{
    let deleted_revision_id = input.deleted_revision_id.map(RevisionId::as_str);
    let row = sqlx::query(INSERT_TOMBSTONE_SQL)
        .bind(input.tombstone_id)
        .bind(input.path.as_str())
        .bind(deleted_revision_id)
        .bind(input.deleted_by.as_str())
        .bind(input.retention_until)
        .fetch_one(executor)
        .await
        .map_err(map_sqlx_error)?;

    tombstone_from_row(&row)
}

/// Reads a tombstone by id.
pub async fn get_tombstone_by_id<'executor, ExecutorType>(
    executor: ExecutorType,
    tombstone_id: &str,
) -> RepositoryResult<Option<TombstoneRow>>
where
    ExecutorType: Executor<'executor, Database = Postgres>,
{
    let row = sqlx::query(GET_TOMBSTONE_BY_ID_SQL)
        .bind(tombstone_id)
        .fetch_optional(executor)
        .await
        .map_err(map_sqlx_error)?;

    row.as_ref().map(tombstone_from_row).transpose()
}

/// Lists tombstones for a normalized path, newest first.
pub async fn list_tombstones_by_path<'executor, ExecutorType>(
    executor: ExecutorType,
    path: &VaultPath,
    limit: u32,
) -> RepositoryResult<Vec<TombstoneRow>>
where
    ExecutorType: Executor<'executor, Database = Postgres>,
{
    validate_limit(limit)?;
    let rows = sqlx::query(LIST_TOMBSTONES_BY_PATH_SQL)
        .bind(path.as_str())
        .bind(i64::from(limit))
        .fetch_all(executor)
        .await
        .map_err(map_sqlx_error)?;

    rows.iter().map(tombstone_from_row).collect()
}

/// Lists active tombstones that have not been restored, newest first.
///
/// This is a read-only helper. It is not a retention cleanup worker.
pub async fn list_active_tombstones<'executor, ExecutorType>(
    executor: ExecutorType,
    limit: u32,
) -> RepositoryResult<Vec<TombstoneRow>>
where
    ExecutorType: Executor<'executor, Database = Postgres>,
{
    validate_limit(limit)?;
    let rows = sqlx::query(LIST_ACTIVE_TOMBSTONES_SQL)
        .bind(i64::from(limit))
        .fetch_all(executor)
        .await
        .map_err(map_sqlx_error)?;

    rows.iter().map(tombstone_from_row).collect()
}

/// Repository facade for callers that prefer value-style helpers.
#[derive(Clone, Copy, Debug, Default)]
pub struct TombstoneRepository;

impl TombstoneRepository {
    /// Construct a tombstone repository helper.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Insert a tombstone row.
    pub async fn insert<'executor, ExecutorType>(
        &self,
        executor: ExecutorType,
        input: NewTombstone<'_>,
    ) -> Result<TombstoneRow, RepositoryError>
    where
        ExecutorType: Executor<'executor, Database = Postgres>,
    {
        insert_tombstone(executor, input).await
    }

    /// Read a tombstone by id.
    pub async fn get_by_id<'executor, ExecutorType>(
        &self,
        executor: ExecutorType,
        tombstone_id: &str,
    ) -> Result<Option<TombstoneRow>, RepositoryError>
    where
        ExecutorType: Executor<'executor, Database = Postgres>,
    {
        get_tombstone_by_id(executor, tombstone_id).await
    }

    /// List tombstones for a normalized path.
    pub async fn list_by_path<'executor, ExecutorType>(
        &self,
        executor: ExecutorType,
        path: &VaultPath,
        limit: u32,
    ) -> Result<Vec<TombstoneRow>, RepositoryError>
    where
        ExecutorType: Executor<'executor, Database = Postgres>,
    {
        list_tombstones_by_path(executor, path, limit).await
    }

    /// List active tombstones without performing cleanup.
    pub async fn list_active<'executor, ExecutorType>(
        &self,
        executor: ExecutorType,
        limit: u32,
    ) -> Result<Vec<TombstoneRow>, RepositoryError>
    where
        ExecutorType: Executor<'executor, Database = Postgres>,
    {
        list_active_tombstones(executor, limit).await
    }
}

fn tombstone_from_row(row: &PgRow) -> RepositoryResult<TombstoneRow> {
    Ok(TombstoneRow {
        tombstone_id: row.try_get("tombstone_id").map_err(map_sqlx_error)?,
        path: row.try_get("path").map_err(map_sqlx_error)?,
        deleted_revision_id: row.try_get("deleted_revision_id").map_err(map_sqlx_error)?,
        deleted_by: row.try_get("deleted_by").map_err(map_sqlx_error)?,
        deleted_at: row.try_get("deleted_at").map_err(map_sqlx_error)?,
        retention_until: row.try_get("retention_until").map_err(map_sqlx_error)?,
        restored_at: row.try_get("restored_at").map_err(map_sqlx_error)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sql_fragments() -> [&'static str; 4] {
        [
            INSERT_TOMBSTONE_SQL,
            GET_TOMBSTONE_BY_ID_SQL,
            LIST_TOMBSTONES_BY_PATH_SQL,
            LIST_ACTIVE_TOMBSTONES_SQL,
        ]
    }

    #[test]
    fn repository_sql_has_no_hard_delete_statement() {
        for sql in sql_fragments() {
            assert!(!sql.to_ascii_lowercase().contains("delete from"));
            assert!(!sql.to_ascii_lowercase().contains("truncate"));
        }
    }

    #[test]
    fn validates_list_limit_before_querying() {
        assert_eq!(
            validate_limit(0),
            Err(RepositoryError::InvalidLimit { max: 1_000 })
        );
        assert_eq!(validate_limit(1), Ok(()));
    }

    #[test]
    fn new_tombstone_input_uses_validated_value_types() {
        let path = VaultPath::parse("Notes/old.md").unwrap();
        let revision_id = RevisionId::parse("rev_01JDELETE").unwrap();
        let deleted_by = AdapterId::parse("worktree-adapter").unwrap();
        let retention_until = DateTime::parse_from_rfc3339("2026-08-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let input = NewTombstone {
            tombstone_id: "tmb_01JDELETE",
            path: &path,
            deleted_revision_id: Some(&revision_id),
            deleted_by: &deleted_by,
            retention_until,
        };

        assert_eq!(input.tombstone_id, "tmb_01JDELETE");
        assert_eq!(input.path.as_str(), "Notes/old.md");
        assert_eq!(
            input.deleted_revision_id.map(RevisionId::as_str),
            Some("rev_01JDELETE")
        );
        assert_eq!(input.deleted_by.as_str(), "worktree-adapter");
    }
}
