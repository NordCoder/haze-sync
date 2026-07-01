//! Operation-log storage primitives for future changes-feed behavior.
//!
//! This module appends and reads operation metadata only. It deliberately avoids
//! route handlers, Core revision apply behavior, conflict/delete policy,
//! idempotency integration, and adapter runtime behavior.

use crate::models::OperationLogRow;
use crate::repositories::{map_sqlx_error, validate_limit, validate_sequence, RepositoryError};
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
            _ => Err(RepositoryError::DatabaseOperationFailed),
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
    ) -> Result<OperationLogRow, RepositoryError>
    where
        E: Executor<'executor, Database = Postgres>,
    {
        sqlx::query(
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
        .try_map(operation_log_row_from_pg)
        .fetch_one(executor)
        .await
        .map_err(map_sqlx_error)
    }

    /// Read an operation by global sequence.
    pub async fn get_by_sequence<'executor, E>(
        &self,
        executor: E,
        sequence: i64,
    ) -> Result<Option<OperationLogRow>, RepositoryError>
    where
        E: Executor<'executor, Database = Postgres>,
    {
        validate_sequence(sequence)?;

        sqlx::query(
            "select seq, op_id, adapter_id, kind, path, revision_id, tombstone_id, conflict_id, created_at \
             from operation_log \
             where seq = $1",
        )
        .bind(sequence)
        .try_map(operation_log_row_from_pg)
        .fetch_optional(executor)
        .await
        .map_err(map_sqlx_error)
    }

    /// Read an operation by unique operation id.
    pub async fn get_by_operation_id<'executor, E>(
        &self,
        executor: E,
        op_id: &OperationId,
    ) -> Result<Option<OperationLogRow>, RepositoryError>
    where
        E: Executor<'executor, Database = Postgres>,
    {
        sqlx::query(
            "select seq, op_id, adapter_id, kind, path, revision_id, tombstone_id, conflict_id, created_at \
             from operation_log \
             where op_id = $1",
        )
        .bind(op_id.as_str())
        .try_map(operation_log_row_from_pg)
        .fetch_optional(executor)
        .await
        .map_err(map_sqlx_error)
    }

    /// Query operation metadata strictly after `since`, ordered by sequence.
    pub async fn list_since<'executor, E>(
        &self,
        executor: E,
        since: i64,
        limit: u32,
    ) -> Result<Vec<OperationLogRow>, RepositoryError>
    where
        E: Executor<'executor, Database = Postgres>,
    {
        validate_sequence(since)?;
        validate_limit(limit)?;

        sqlx::query(
            "select seq, op_id, adapter_id, kind, path, revision_id, tombstone_id, conflict_id, created_at \
             from operation_log \
             where seq > $1 \
             order by seq asc \
             limit $2",
        )
        .bind(since)
        .bind(i64::from(limit))
        .try_map(operation_log_row_from_pg)
        .fetch_all(executor)
        .await
        .map_err(map_sqlx_error)
    }

    /// Query a safe changes-feed page strictly after `since`, using one sentinel
    /// row to compute `has_more` without skipping or reordering operations.
    pub async fn changes_since<'executor, E>(
        &self,
        executor: E,
        since: i64,
        limit: u32,
    ) -> Result<ChangeFeedPage, RepositoryError>
    where
        E: Executor<'executor, Database = Postgres>,
    {
        validate_sequence(since)?;
        validate_limit(limit)?;

        let mut rows = sqlx::query(
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
        .try_map(change_feed_row_from_pg)
        .fetch_all(executor)
        .await
        .map_err(map_sqlx_error)?;

        let has_more = rows.len() > limit as usize;
        if has_more {
            rows.truncate(limit as usize);
        }

        let to_seq = rows.last().map_or(since, |row| row.seq);

        Ok(ChangeFeedPage {
            from_seq: since,
            to_seq,
            has_more,
            changes: rows,
        })
    }
}

fn operation_log_row_from_pg(row: PgRow) -> Result<OperationLogRow, sqlx::Error> {
    Ok(OperationLogRow {
        seq: row.try_get("seq")?,
        op_id: row.try_get("op_id")?,
        adapter_id: row.try_get("adapter_id")?,
        kind: row.try_get("kind")?,
        path: row.try_get("path")?,
        revision_id: row.try_get("revision_id")?,
        tombstone_id: row.try_get("tombstone_id")?,
        conflict_id: row.try_get("conflict_id")?,
        created_at: row.try_get("created_at")?,
    })
}

fn change_feed_row_from_pg(row: PgRow) -> Result<ChangeFeedRow, sqlx::Error> {
    Ok(ChangeFeedRow {
        seq: row.try_get("seq")?,
        op_id: row.try_get("op_id")?,
        adapter_id: row.try_get("adapter_id")?,
        kind: row.try_get("kind")?,
        path: row.try_get("path")?,
        revision_id: row.try_get("revision_id")?,
        content_sha256: row.try_get("content_sha256")?,
        size_bytes: row.try_get("size_bytes")?,
        tombstone_id: row.try_get("tombstone_id")?,
        conflict_id: row.try_get("conflict_id")?,
        created_at: row.try_get("created_at")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_kind_names_match_contract_strings() {
        assert_eq!(OperationKindName::UpsertFile.as_str(), "upsert_file");
        assert_eq!(OperationKindName::DeleteFile.as_str(), "delete_file");
        assert_eq!(OperationKindName::RestoreFile.as_str(), "restore_file");
        assert_eq!(OperationKindName::ConflictCreated.as_str(), "conflict_created");
        assert_eq!(OperationKindName::ConflictResolved.as_str(), "conflict_resolved");
        assert_eq!(OperationKindName::BackupCreated.as_str(), "backup_created");
    }

    #[test]
    fn operation_kind_roundtrips_json() {
        let json = serde_json::to_string(&OperationKindName::ConflictCreated).unwrap();
        assert_eq!(json, "\"conflict_created\"");
        assert_eq!(
            serde_json::from_str::<OperationKindName>(&json).unwrap(),
            OperationKindName::ConflictCreated
        );
    }
}
