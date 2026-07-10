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

fn change_feed_row_from_pg(row: &PgRow) -> RepositoryResult<ChangeFeedRow> {
    let size_bytes = validated_persisted_size_bytes(
        row.try_get("size_bytes").map_err(map_sqlx_error)?,
    )?;

    Ok(ChangeFeedRow {
        seq: row.try_get("seq").map_err(map_sqlx_error)?,
        op_id: row.try_get("op_id").map_err(map_sqlx_error)?,
        adapter_id: row.try_get("adapter_id").map_err(map_sqlx_error)?,
        kind: row.try_get("kind").map_err(map_sqlx_error)?,
        path: row.try_get("path").map_err(map_sqlx_error)?,
        revision_id: row.try_get("revision_id").map_err(map_sqlx_error)?,
        content_sha256: row.try_get("content_sha256").map_err(map_sqlx_error)?,
        size_bytes,
        tombstone_id: row.try_get("tombstone_id").map_err(map_sqlx_error)?,
        conflict_id: row.try_get("conflict_id").map_err(map_sqlx_error)?,
        created_at: row.try_get("created_at").map_err(map_sqlx_error)?,
    })
}

fn validated_persisted_size_bytes(size_bytes: Option<i64>) -> RepositoryResult<Option<i64>> {
    if size_bytes.is_some_and(|value| value < 0) {
        return Err(RepositoryError::InvalidSizeBytes);
    }

    Ok(size_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_kind_names_match_contract_strings() {
        assert_eq!(OperationKindName::UpsertFile.as_str(), "upsert_file");
        assert_eq!(OperationKindName::DeleteFile.as_str(), "delete_file");
        assert_eq!(OperationKindName::RestoreFile.as_str(), "restore_file");
        assert_eq!(
            OperationKindName::ConflictCreated.as_str(),
            "conflict_created"
        );
        assert_eq!(
            OperationKindName::ConflictResolved.as_str(),
            "conflict_resolved"
        );
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
        assert_eq!(
            OperationKindName::from_str("overwrite_file"),
            Err(RepositoryError::InvalidOperationKind)
        );
    }

    #[test]
    fn append_entry_carries_revision_metadata_without_policy_outcome() {
        let path = VaultPath::parse("Notes/today.md").unwrap();
        let revision_id = RevisionId::parse("rev_01JSTORP5").unwrap();
        let entry = AppendOperationLogEntry {
            op_id: OperationId::parse("op_01JSTORP5").unwrap(),
            adapter_id: AdapterId::parse("worktree-adapter").unwrap(),
            kind: OperationKindName::UpsertFile,
            path: path.clone(),
            revision_id: Some(revision_id.clone()),
            tombstone_id: None,
            conflict_id: None,
        };

        assert_eq!(entry.kind.as_str(), "upsert_file");
        assert_eq!(entry.path, path);
        assert_eq!(entry.revision_id.as_ref(), Some(&revision_id));
        assert!(entry.tombstone_id.is_none());
        assert!(entry.conflict_id.is_none());
    }

    #[test]
    fn conflict_delete_and_restore_entries_preserve_reference_metadata() {
        let path = VaultPath::parse("Notes/today.md").unwrap();
        let adapter_id = AdapterId::parse("worktree-adapter").unwrap();
        let revision_id = RevisionId::parse("rev_01JSTORP6").unwrap();
        let conflict_id = ConflictId::parse("conf_01JSTORP6").unwrap();

        let delete = AppendOperationLogEntry {
            op_id: OperationId::parse("op_01JDELETE").unwrap(),
            adapter_id: adapter_id.clone(),
            kind: OperationKindName::DeleteFile,
            path: path.clone(),
            revision_id: Some(revision_id.clone()),
            tombstone_id: Some("tmb_01JSTORP6".to_owned()),
            conflict_id: None,
        };
        let restore = AppendOperationLogEntry {
            op_id: OperationId::parse("op_01JRESTORE").unwrap(),
            adapter_id: adapter_id.clone(),
            kind: OperationKindName::RestoreFile,
            path: path.clone(),
            revision_id: Some(revision_id.clone()),
            tombstone_id: Some("tmb_01JSTORP6".to_owned()),
            conflict_id: None,
        };
        let conflict = AppendOperationLogEntry {
            op_id: OperationId::parse("op_01JCONFLICT").unwrap(),
            adapter_id,
            kind: OperationKindName::ConflictCreated,
            path,
            revision_id: Some(revision_id),
            tombstone_id: None,
            conflict_id: Some(conflict_id.clone()),
        };

        assert_eq!(delete.tombstone_id.as_deref(), Some("tmb_01JSTORP6"));
        assert_eq!(restore.tombstone_id, delete.tombstone_id);
        assert_eq!(conflict.conflict_id.as_ref(), Some(&conflict_id));
        assert!(conflict.tombstone_id.is_none());
    }

    #[test]
    fn persisted_sizes_reject_negative_values_safely() {
        assert_eq!(validated_persisted_size_bytes(None), Ok(None));
        assert_eq!(validated_persisted_size_bytes(Some(0)), Ok(Some(0)));
        assert_eq!(validated_persisted_size_bytes(Some(42)), Ok(Some(42)));
        assert_eq!(
            validated_persisted_size_bytes(Some(-1)),
            Err(RepositoryError::InvalidSizeBytes)
        );
    }

    #[test]
    fn change_feed_page_uses_one_sentinel_row_for_has_more() {
        let page = change_feed_page_from_rows(10, 2, vec![change(11), change(12), change(13)]);

        assert_eq!(page.from_seq, 10);
        assert_eq!(page.to_seq, 12);
        assert!(page.has_more);
        assert_eq!(page.changes.len(), 2);
        assert_eq!(page.changes[0].seq, 11);
        assert_eq!(page.changes[1].seq, 12);
    }

    #[test]
    fn empty_change_feed_page_keeps_cursor_at_requested_sequence() {
        let page = change_feed_page_from_rows(10, 5, Vec::new());

        assert_eq!(page.from_seq, 10);
        assert_eq!(page.to_seq, 10);
        assert!(!page.has_more);
        assert!(page.changes.is_empty());
    }

    fn change(seq: i64) -> ChangeFeedRow {
        ChangeFeedRow {
            seq,
            op_id: format!("op_01JSTORP5{seq}"),
            adapter_id: "worktree-adapter".to_owned(),
            kind: OperationKindName::UpsertFile.as_str().to_owned(),
            path: "Notes/today.md".to_owned(),
            revision_id: Some(format!("rev_01JSTORP5{seq}")),
            content_sha256: Some(
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                    .to_owned(),
            ),
            size_bytes: Some(42),
            tombstone_id: None,
            conflict_id: None,
            created_at: DateTime::<Utc>::from(std::time::UNIX_EPOCH),
        }
    }
}

#[cfg(all(test, feature = "test-support"))]
mod postgres_tests {
    use super::*;
    use crate::locks::{lock_vault_path, path_lock_key};
    use crate::repositories::conflicts::{ConflictRepository, ConflictStatusName, NewConflict};
    use crate::repositories::content_blobs::{
        create_or_get_content_blob, get_content_blob_by_hash, NewContentBlob,
    };
    use crate::repositories::objects::{
        create_or_find_sync_object_by_path, get_sync_object_by_path, set_current_revision_by_path,
        NewSyncObject, SyncObjectKind,
    };
    use crate::repositories::revisions::{
        get_current_revision_by_path, get_file_revision_by_id, insert_file_revision,
        NewFileRevision,
    };
    use crate::repositories::tombstones::{NewTombstone, TombstoneRepository};
    use crate::test_support::connect_test_database_from_env;
    use chrono::Duration;
    use haze_sync_common::ContentHash;

    #[tokio::test]
    async fn file_conflict_and_delete_flows_roundtrip_under_caller_owned_transaction() {
        let Some(context) = connect_test_database_from_env().await.unwrap() else {
            return;
        };
        context.apply_migrations().await.unwrap();
        context.clean_storage_tables().await.unwrap();

        let adapter_id = AdapterId::parse("worktree-adapter").unwrap();
        let path = VaultPath::parse("Notes/today.md").unwrap();
        let content_hash = ContentHash::parse(&"a".repeat(64)).unwrap();
        let revision_id = RevisionId::parse("rev_01JSTORP5").unwrap();
        let incoming_revision_id = RevisionId::parse("rev_01JSTORP6").unwrap();
        let op_id = OperationId::parse("op_01JSTORP5").unwrap();
        let conflict_id = ConflictId::parse("conf_01JSTORP6").unwrap();
        let materialized_path = VaultPath::parse("Notes/today.conflict.md").unwrap();
        let tombstone_id = "tmb_01JSTORP6";
        let operation_log = OperationLogRepository::new();
        let conflict_repository = ConflictRepository::new();
        let tombstone_repository = TombstoneRepository::new();

        let mut tx = context.pool().begin().await.unwrap();
        sqlx::query(
            "insert into sync_adapters (adapter_id, display_name, role, token_hash) \
             values ($1, $2, $3, $4)",
        )
        .bind(adapter_id.as_str())
        .bind("Worktree Adapter")
        .bind("worktree")
        .bind("sha256:test-token-hash")
        .execute(&mut *tx)
        .await
        .unwrap();

        let lock_key = lock_vault_path(&mut *tx, &path).await.unwrap();
        assert_eq!(lock_key, path_lock_key(&path));

        let blob = create_or_get_content_blob(
            &mut *tx,
            &NewContentBlob {
                sha256: content_hash,
                size_bytes: 42,
                object_store_path:
                    "sha256/aa/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            },
        )
        .await
        .unwrap();
        let duplicate_blob = create_or_get_content_blob(
            &mut *tx,
            &NewContentBlob {
                sha256: content_hash,
                size_bytes: 999,
                object_store_path: "sha256/aa/ignored-on-duplicate",
            },
        )
        .await
        .unwrap();
        assert_eq!(duplicate_blob, blob);

        let object = create_or_find_sync_object_by_path(
            &mut *tx,
            NewSyncObject {
                object_id: "obj_01JSTORP5",
                path: &path,
                kind: SyncObjectKind::File,
                updated_by: &adapter_id,
            },
        )
        .await
        .unwrap();
        assert_eq!(object.path, path.as_str());
        assert!(object.current_revision_id.is_none());

        let revision = insert_file_revision(
            &mut *tx,
            NewFileRevision {
                revision_id: &revision_id,
                object_id: "obj_01JSTORP5",
                path: &path,
                parent_revision_id: None,
                content_hash,
                size_bytes: 42,
                created_by: &adapter_id,
            },
        )
        .await
        .unwrap();
        let incoming_revision = insert_file_revision(
            &mut *tx,
            NewFileRevision {
                revision_id: &incoming_revision_id,
                object_id: "obj_01JSTORP5",
                path: &path,
                parent_revision_id: Some(&revision_id),
                content_hash,
                size_bytes: 42,
                created_by: &adapter_id,
            },
        )
        .await
        .unwrap();
        assert_eq!(revision.revision_id, revision_id.as_str());
        assert_eq!(incoming_revision.revision_id, incoming_revision_id.as_str());
        assert_eq!(revision.content_sha256, content_hash.to_prefixed_string());

        let updated_object =
            set_current_revision_by_path(&mut *tx, &path, Some(&revision_id), &adapter_id)
                .await
                .unwrap()
                .unwrap();
        assert_eq!(
            updated_object.current_revision_id.as_deref(),
            Some(revision_id.as_str())
        );

        let operation = operation_log
            .append(
                &mut *tx,
                &AppendOperationLogEntry {
                    op_id: op_id.clone(),
                    adapter_id: adapter_id.clone(),
                    kind: OperationKindName::UpsertFile,
                    path: path.clone(),
                    revision_id: Some(revision_id.clone()),
                    tombstone_id: None,
                    conflict_id: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(operation.seq, 1);
        assert_eq!(operation.kind, OperationKindName::UpsertFile.as_str());

        let conflict = conflict_repository
            .insert(
                &mut *tx,
                &NewConflict {
                    conflict_id: &conflict_id,
                    original_path: &path,
                    base_revision_id: Some(&revision_id),
                    current_revision_id: &revision_id,
                    incoming_revision_id: &incoming_revision_id,
                    incoming_adapter_id: &adapter_id,
                    policy_applied: "preserve_both",
                    materialized_path: &materialized_path,
                    status: ConflictStatusName::Open,
                },
            )
            .await
            .unwrap();
        assert_eq!(conflict.status, ConflictStatusName::Open.as_str());
        assert_eq!(
            conflict_repository
                .list_by_status(&mut *tx, ConflictStatusName::Open, 10)
                .await
                .unwrap(),
            vec![conflict.clone()]
        );
        assert_eq!(
            conflict_repository
                .get_by_id(&mut *tx, &conflict_id)
                .await
                .unwrap(),
            Some(conflict.clone())
        );

        let tombstone = tombstone_repository
            .insert(
                &mut *tx,
                NewTombstone {
                    tombstone_id,
                    path: &path,
                    deleted_revision_id: Some(&revision_id),
                    deleted_by: &adapter_id,
                    retention_until: Utc::now() + Duration::days(30),
                },
            )
            .await
            .unwrap();
        assert!(tombstone.restored_at.is_none());
        assert_eq!(
            tombstone_repository
                .get_by_id(&mut *tx, tombstone_id)
                .await
                .unwrap(),
            Some(tombstone.clone())
        );
        assert_eq!(
            tombstone_repository
                .list_active(&mut *tx, 10)
                .await
                .unwrap(),
            vec![tombstone.clone()]
        );

        let conflict_created = operation_log
            .append(
                &mut *tx,
                &AppendOperationLogEntry {
                    op_id: OperationId::parse("op_01JCONFLICTCREATE").unwrap(),
                    adapter_id: adapter_id.clone(),
                    kind: OperationKindName::ConflictCreated,
                    path: path.clone(),
                    revision_id: Some(incoming_revision_id.clone()),
                    tombstone_id: None,
                    conflict_id: Some(conflict_id.clone()),
                },
            )
            .await
            .unwrap();
        let deleted = operation_log
            .append(
                &mut *tx,
                &AppendOperationLogEntry {
                    op_id: OperationId::parse("op_01JDELETE").unwrap(),
                    adapter_id: adapter_id.clone(),
                    kind: OperationKindName::DeleteFile,
                    path: path.clone(),
                    revision_id: Some(revision_id.clone()),
                    tombstone_id: Some(tombstone_id.to_owned()),
                    conflict_id: None,
                },
            )
            .await
            .unwrap();

        let resolved = conflict_repository
            .mark_open_resolved(&mut *tx, &conflict_id, &adapter_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(resolved.status, ConflictStatusName::Resolved.as_str());
        assert!(resolved.resolved_at.is_some());
        assert_eq!(resolved.resolved_by.as_deref(), Some(adapter_id.as_str()));
        assert!(
            conflict_repository
                .mark_open_ignored(&mut *tx, &conflict_id, &adapter_id)
                .await
                .unwrap()
                .is_none()
        );

        let restored = tombstone_repository
            .mark_restored(&mut *tx, tombstone_id)
            .await
            .unwrap()
            .unwrap();
        assert!(restored.restored_at.is_some());
        assert!(
            tombstone_repository
                .mark_restored(&mut *tx, tombstone_id)
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            tombstone_repository
                .list_active(&mut *tx, 10)
                .await
                .unwrap()
                .is_empty()
        );

        let conflict_resolved = operation_log
            .append(
                &mut *tx,
                &AppendOperationLogEntry {
                    op_id: OperationId::parse("op_01JCONFLICTRESOLVE").unwrap(),
                    adapter_id: adapter_id.clone(),
                    kind: OperationKindName::ConflictResolved,
                    path: path.clone(),
                    revision_id: Some(revision_id.clone()),
                    tombstone_id: None,
                    conflict_id: Some(conflict_id.clone()),
                },
            )
            .await
            .unwrap();
        let restored_operation = operation_log
            .append(
                &mut *tx,
                &AppendOperationLogEntry {
                    op_id: OperationId::parse("op_01JRESTORE").unwrap(),
                    adapter_id: adapter_id.clone(),
                    kind: OperationKindName::RestoreFile,
                    path: path.clone(),
                    revision_id: Some(revision_id.clone()),
                    tombstone_id: Some(tombstone_id.to_owned()),
                    conflict_id: None,
                },
            )
            .await
            .unwrap();

        let loaded_blob = get_content_blob_by_hash(&mut *tx, content_hash)
            .await
            .unwrap()
            .unwrap();
        let loaded_object = get_sync_object_by_path(&mut *tx, &path)
            .await
            .unwrap()
            .unwrap();
        let loaded_revision = get_file_revision_by_id(&mut *tx, &revision_id)
            .await
            .unwrap()
            .unwrap();
        let current_revision = get_current_revision_by_path(&mut *tx, &path)
            .await
            .unwrap()
            .unwrap();
        let loaded_operation = operation_log
            .get_by_operation_id(&mut *tx, &op_id)
            .await
            .unwrap()
            .unwrap();
        let changes = operation_log
            .changes_since(&mut *tx, operation.seq, 10)
            .await
            .unwrap();
        let expected_hash = content_hash.to_prefixed_string();

        assert_eq!(loaded_blob, blob);
        assert_eq!(
            loaded_object.current_revision_id,
            updated_object.current_revision_id
        );
        assert_eq!(loaded_revision, revision);
        assert_eq!(current_revision, revision);
        assert_eq!(loaded_operation.seq, operation.seq);
        assert_eq!(changes.from_seq, operation.seq);
        assert_eq!(changes.to_seq, restored_operation.seq);
        assert!(!changes.has_more);
        assert_eq!(changes.changes.len(), 4);
        assert_eq!(changes.changes[0].seq, conflict_created.seq);
        assert_eq!(changes.changes[0].conflict_id.as_deref(), Some(conflict_id.as_str()));
        assert_eq!(changes.changes[1].seq, deleted.seq);
        assert_eq!(changes.changes[1].tombstone_id.as_deref(), Some(tombstone_id));
        assert_eq!(changes.changes[2].seq, conflict_resolved.seq);
        assert_eq!(changes.changes[3].seq, restored_operation.seq);
        assert_eq!(
            changes.changes[3].content_sha256.as_deref(),
            Some(expected_hash.as_str())
        );
        assert_eq!(changes.changes[3].size_bytes, Some(42));

        tx.commit().await.unwrap();
        context.clean_storage_tables().await.unwrap();
    }
}
