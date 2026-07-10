//! Operation-log storage primitives for future changes-feed behavior.
//!
//! This module appends and reads operation metadata only. It deliberately avoids
//! route handlers, Core revision apply behavior, conflict/delete policy,
//! idempotency integration, and adapter runtime behavior.

use crate::models::OperationLogRow;
use crate::repositories::{
    map_sqlx_error, validate_limit, validate_sequence, RepositoryError, RepositoryResult,
};
use chrono::{DateTime, Utc};
use haze_sync_common::{AdapterId, ConflictId, OperationId, RevisionId, VaultPath};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgRow, Executor, Postgres, Row};
use std::{fmt, str::FromStr};

/// Storage vocabulary for V1 operation-log kinds.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationKindName {
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
    /// Backup copy was created by conflict policy.
    BackupCreated,
}

impl OperationKindName {
    /// Return the canonical contract string stored in Postgres.
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

impl fmt::Display for OperationKindName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for OperationKindName {
    type Err = RepositoryError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "upsert_file" => Ok(Self::UpsertFile),
            "delete_file" => Ok(Self::DeleteFile),
            "restore_file" => Ok(Self::RestoreFile),
            "conflict_created" => Ok(Self::ConflictCreated),
            "conflict_resolved" => Ok(Self::ConflictResolved),
            "backup_created" => Ok(Self::BackupCreated),
            _ => Err(RepositoryError::InvalidOperationKind),
        }
    }
}

/// Operation metadata to append after a Core storage mutation has already
/// decided the revision/conflict/delete outcome.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppendOperationLogEntry {
    pub op_id: OperationId,
    pub adapter_id: AdapterId,
    pub kind: OperationKindName,
    pub path: VaultPath,
    pub revision_id: Option<RevisionId>,
    pub tombstone_id: Option<String>,
    pub conflict_id: Option<ConflictId>,
}

/// A changes-feed row enriched with file revision metadata when the operation
/// references a file revision.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ChangeFeedRow {
    pub seq: i64,
    pub op_id: String,
    pub adapter_id: String,
    pub kind: String,
    pub path: String,
    pub revision_id: Option<String>,
    pub content_sha256: Option<String>,
    pub size_bytes: Option<i64>,
    pub tombstone_id: Option<String>,
    pub conflict_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Bounded operation-log page returned by `changes_since`.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ChangeFeedPage {
    pub from_seq: i64,
    pub to_seq: i64,
    pub has_more: bool,
    pub changes: Vec<ChangeFeedRow>,
}

/// Repository for append-only operation-log storage.
#[derive(Clone, Copy, Debug, Default)]
pub struct OperationLogRepository;

impl OperationLogRepository {
    /// Construct a repository helper.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Append operation metadata and return the DB-generated global sequence.
    pub async fn append<'executor, E>(
        &self,
        executor: E,
        entry: &AppendOperationLogEntry,
    ) -> RepositoryResult<OperationLogRow>
    where
        E: Executor<'executor, Database = Postgres>,
    {
        let row = sqlx::query(
            "insert into operation_log \
             (op_id, adapter_id, kind, path, revision_id, tombstone_id, conflict_id) \
             values ($1, $2, $3, $4, $5, $6, $7) \
             returning seq, op_id, adapter_id, kind, path, revision_id, tombstone_id, conflict_id, created_at",
        )
        .bind(entry.op_id.as_str())
        .bind(entry.adapter_id.as_str())
        .bind(entry.kind.as_str())
        .bind(entry.path.as_str())
        .bind(entry.revision_id.as_ref().map(RevisionId::as_str))
        .bind(entry.tombstone_id.as_deref())
        .bind(entry.conflict_id.as_ref().map(ConflictId::as_str))
        .fetch_one(executor)
        .await
        .map_err(map_sqlx_error)?;

        operation_log_row_from_pg(&row)
    }

    /// Read an operation by global sequence.
    pub async fn get_by_sequence<'executor, E>(
        &self,
        executor: E,
        sequence: i64,
    ) -> RepositoryResult<Option<OperationLogRow>>
    where
        E: Executor<'executor, Database = Postgres>,
    {
        validate_sequence(sequence)?;

        let row = sqlx::query(
            "select seq, op_id, adapter_id, kind, path, revision_id, tombstone_id, conflict_id, created_at \
             from operation_log \
             where seq = $1",
        )
        .bind(sequence)
        .fetch_optional(executor)
        .await
        .map_err(map_sqlx_error)?;

        row.as_ref().map(operation_log_row_from_pg).transpose()
    }

    /// Read an operation by unique operation id.
    pub async fn get_by_operation_id<'executor, E>(
        &self,
        executor: E,
        op_id: &OperationId,
    ) -> RepositoryResult<Option<OperationLogRow>>
    where
        E: Executor<'executor, Database = Postgres>,
    {
        let row = sqlx::query(
            "select seq, op_id, adapter_id, kind, path, revision_id, tombstone_id, conflict_id, created_at \
             from operation_log \
             where op_id = $1",
        )
        .bind(op_id.as_str())
        .fetch_optional(executor)
        .await
        .map_err(map_sqlx_error)?;

        row.as_ref().map(operation_log_row_from_pg).transpose()
    }

    /// Query operation metadata strictly after `since`, ordered by sequence.
    pub async fn list_since<'executor, E>(
        &self,
        executor: E,
        since: i64,
        limit: u32,
    ) -> RepositoryResult<Vec<OperationLogRow>>
    where
        E: Executor<'executor, Database = Postgres>,
    {
        validate_sequence(since)?;
        validate_limit(limit)?;

        let rows = sqlx::query(
            "select seq, op_id, adapter_id, kind, path, revision_id, tombstone_id, conflict_id, created_at \
             from operation_log \
             where seq > $1 \
             order by seq asc \
             limit $2",
        )
        .bind(since)
        .bind(i64::from(limit))
        .fetch_all(executor)
        .await
        .map_err(map_sqlx_error)?;

        rows.iter().map(operation_log_row_from_pg).collect()
    }

    /// Query a safe changes-feed page strictly after `since`, using one sentinel
    /// row to compute `has_more` without skipping or reordering operations.
    pub async fn changes_since<'executor, E>(
        &self,
        executor: E,
        since: i64,
        limit: u32,
    ) -> RepositoryResult<ChangeFeedPage>
    where
        E: Executor<'executor, Database = Postgres>,
    {
        validate_sequence(since)?;
        validate_limit(limit)?;

        let rows = sqlx::query(
            "select \
                 operation_log.seq, \
                 operation_log.op_id, \
                 operation_log.adapter_id, \
                 operation_log.kind, \
                 operation_log.path, \
                 operation_log.revision_id, \
                 file_revisions.content_sha256, \
                 file_revisions.size_bytes, \
                 operation_log.tombstone_id, \
                 operation_log.conflict_id, \
                 operation_log.created_at \
             from operation_log \
             left join file_revisions on file_revisions.revision_id = operation_log.revision_id \
             where operation_log.seq > $1 \
             order by operation_log.seq asc \
             limit $2",
        )
        .bind(since)
        .bind(i64::from(limit) + 1)
        .fetch_all(executor)
        .await
        .map_err(map_sqlx_error)?;
        let rows = rows
            .iter()
            .map(change_feed_row_from_pg)
            .collect::<RepositoryResult<Vec<_>>>()?;

        Ok(change_feed_page_from_rows(since, limit, rows))
    }
}

fn change_feed_page_from_rows(
    since: i64,
    limit: u32,
    mut rows: Vec<ChangeFeedRow>,
) -> ChangeFeedPage {
    let has_more = rows.len() > limit as usize;
    if has_more {
        rows.truncate(limit as usize);
    }

    let to_seq = rows.last().map_or(since, |row| row.seq);

    ChangeFeedPage {
        from_seq: since,
        to_seq,
        has_more,
        changes: rows,
    }
}

fn operation_log_row_from_pg(row: &PgRow) -> RepositoryResult<OperationLogRow> {
    let kind = validated_persisted_operation_kind(
        row.try_get("kind").map_err(map_sqlx_error)?,
    )?;

    Ok(OperationLogRow {
        seq: row.try_get("seq").map_err(map_sqlx_error)?,
        op_id: row.try_get("op_id").map_err(map_sqlx_error)?,
        adapter_id: row.try_get("adapter_id").map_err(map_sqlx_error)?,
        kind,
        path: row.try_get("path").map_err(map_sqlx_error)?,
        revision_id: row.try_get("revision_id").map_err(map_sqlx_error)?,
        tombstone_id: row.try_get("tombstone_id").map_err(map_sqlx_error)?,
        conflict_id: row.try_get("conflict_id").map_err(map_sqlx_error)?,
        created_at: row.try_get("created_at").map_err(map_sqlx_error)?,
    })
}

fn change_feed_row_from_pg(row: &PgRow) -> RepositoryResult<ChangeFeedRow> {
    let kind = validated_persisted_operation_kind(
        row.try_get("kind").map_err(map_sqlx_error)?,
    )?;
    let size_bytes =
        validated_persisted_size_bytes(row.try_get("size_bytes").map_err(map_sqlx_error)?)?;

    Ok(ChangeFeedRow {
        seq: row.try_get("seq").map_err(map_sqlx_error)?,
        op_id: row.try_get("op_id").map_err(map_sqlx_error)?,
        adapter_id: row.try_get("adapter_id").map_err(map_sqlx_error)?,
        kind,
        path: row.try_get("path").map_err(map_sqlx_error)?,
        revision_id: row.try_get("revision_id").map_err(map_sqlx_error)?,
        content_sha256: row.try_get("content_sha256").map_err(map_sqlx_error)?,
        size_bytes,
        tombstone_id: row.try_get("tombstone_id").map_err(map_sqlx_error)?,
        conflict_id: row.try_get("conflict_id").map_err(map_sqlx_error)?,
        created_at: row.try_get("created_at").map_err(map_sqlx_error)?,
    })
}

fn validated_persisted_operation_kind(kind: String) -> RepositoryResult<String> {
    OperationKindName::from_str(&kind)?;
    Ok(kind)
}

fn validated_persisted_size_bytes(size_bytes: Option<i64>) -> RepositoryResult<Option<i64>> {
    if size_bytes.is_some_and(|value| value < 0) {
        return Err(RepositoryError::InvalidSizeBytes);
    }

    Ok(size_bytes)
}

#[cfg(test)]
mod tests;

#[cfg(all(test, feature = "test-support"))]
mod postgres_tests;
