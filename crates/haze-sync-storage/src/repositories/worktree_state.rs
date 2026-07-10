//! Passive repository helpers for durable worktree state facts.
//!
//! Storage records caller-supplied materialization metadata. It does not scan the
//! filesystem, decide dirty-state meaning, suppress echo writes, materialize
//! files, or repair worktree contents.

use super::{map_sqlx_error, RepositoryError, RepositoryResult};
use crate::models::WorktreeStateRow;
use chrono::{DateTime, Utc};
use haze_sync_common::{RevisionId, Sha256, VaultPath};
use sqlx::{postgres::PgRow, Executor, Postgres, Row};

const SELECT_BY_PATH_SQL: &str =
    "select path, last_applied_revision_id, last_seen_sha256, last_seen_mtime, \
     dirty, last_scanned_at, last_written_by_adapter \
     from worktree_state where path = $1";

const UPSERT_SQL: &str =
    "insert into worktree_state ( \
         path, last_applied_revision_id, last_seen_sha256, last_seen_mtime, \
         dirty, last_scanned_at, last_written_by_adapter \
     ) values ($1, $2, $3, $4, $5, $6, $7) \
     on conflict (path) do update set \
         last_applied_revision_id = excluded.last_applied_revision_id, \
         last_seen_sha256 = excluded.last_seen_sha256, \
         last_seen_mtime = excluded.last_seen_mtime, \
         dirty = excluded.dirty, \
         last_scanned_at = excluded.last_scanned_at, \
         last_written_by_adapter = excluded.last_written_by_adapter \
     returning path, last_applied_revision_id, last_seen_sha256, \
         last_seen_mtime, dirty, last_scanned_at, last_written_by_adapter";

/// Complete caller-decided fact set for one `worktree_state` row.
///
/// Boolean flags are persisted facts only. Their scanner, watcher, echo, repair,
/// and materialization semantics remain owned by the Worktree adapter.
#[derive(Clone, Debug)]
pub struct WorktreeStateUpsert<'a> {
    pub path: &'a VaultPath,
    pub last_applied_revision_id: Option<&'a RevisionId>,
    pub last_seen_sha256: Option<&'a Sha256>,
    pub last_seen_mtime: Option<DateTime<Utc>>,
    pub dirty: bool,
    pub last_scanned_at: Option<DateTime<Utc>>,
    pub last_written_by_adapter: bool,
}

/// Inserts or replaces the complete persisted worktree fact set for a path.
///
/// The helper executes inside the caller-owned executor or transaction and does
/// not infer whether the row is dirty or whether a write should be suppressed.
pub async fn upsert_worktree_state<'executor, ExecutorType>(
    executor: ExecutorType,
    input: WorktreeStateUpsert<'_>,
) -> RepositoryResult<WorktreeStateRow>
where
    ExecutorType: Executor<'executor, Database = Postgres>,
{
    let revision_id = input
        .last_applied_revision_id
        .map(RevisionId::as_str);
    let last_seen_sha256 = input.last_seen_sha256.map(Sha256::to_string);

    let row = sqlx::query(UPSERT_SQL)
        .bind(input.path.as_str())
        .bind(revision_id)
        .bind(last_seen_sha256)
        .bind(input.last_seen_mtime)
        .bind(input.dirty)
        .bind(input.last_scanned_at)
        .bind(input.last_written_by_adapter)
        .fetch_one(executor)
        .await
        .map_err(map_sqlx_error)?;

    worktree_state_from_pg(&row)
}

/// Reads the persisted worktree fact set for a normalized vault path.
pub async fn get_worktree_state_by_path<'executor, ExecutorType>(
    executor: ExecutorType,
    path: &VaultPath,
) -> RepositoryResult<Option<WorktreeStateRow>>
where
    ExecutorType: Executor<'executor, Database = Postgres>,
{
    let row = sqlx::query(SELECT_BY_PATH_SQL)
        .bind(path.as_str())
        .fetch_optional(executor)
        .await
        .map_err(map_sqlx_error)?;

    row.as_ref().map(worktree_state_from_pg).transpose()
}

fn worktree_state_from_pg(row: &PgRow) -> RepositoryResult<WorktreeStateRow> {
    let mapped = WorktreeStateRow {
        path: row.try_get("path").map_err(map_sqlx_error)?,
        last_applied_revision_id: row
            .try_get("last_applied_revision_id")
            .map_err(map_sqlx_error)?,
        last_seen_sha256: row.try_get("last_seen_sha256").map_err(map_sqlx_error)?,
        last_seen_mtime: row.try_get("last_seen_mtime").map_err(map_sqlx_error)?,
        dirty: row.try_get("dirty").map_err(map_sqlx_error)?,
        last_scanned_at: row.try_get("last_scanned_at").map_err(map_sqlx_error)?,
        last_written_by_adapter: row
            .try_get("last_written_by_adapter")
            .map_err(map_sqlx_error)?,
    };

    validate_worktree_state_row(mapped)
}

fn validate_worktree_state_row(row: WorktreeStateRow) -> RepositoryResult<WorktreeStateRow> {
    let normalized_path =
        VaultPath::parse(&row.path).map_err(|_| RepositoryError::InvalidPath)?;
    if normalized_path.as_str() != row.path {
        return Err(RepositoryError::InvalidPath);
    }

    if let Some(revision_id) = row.last_applied_revision_id.as_deref() {
        RevisionId::parse(revision_id).map_err(|_| RepositoryError::InvalidIdentifier)?;
    }
    if let Some(last_seen_sha256) = row.last_seen_sha256.as_deref() {
        Sha256::parse(last_seen_sha256).map_err(|_| RepositoryError::InvalidHash)?;
    }

    Ok(row)
}

#[cfg(test)]
mod tests;

#[cfg(all(test, feature = "test-support"))]
mod postgres_tests;
