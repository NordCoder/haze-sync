//! Repository helpers for immutable `file_revisions` rows.

use super::{map_sqlx_error, size_bytes_to_i64, RepositoryResult};
use crate::models::FileRevisionRow;
use haze_sync_common::{AdapterId, ContentHash, RevisionId, VaultPath};
use sqlx::{postgres::PgRow, Executor, Postgres, Row};

/// Input for inserting an immutable file revision row.
#[derive(Clone, Copy, Debug)]
pub struct NewFileRevision<'a> {
    /// Caller-assigned immutable revision id.
    pub revision_id: &'a RevisionId,
    /// Stable object id from `sync_objects`.
    pub object_id: &'a str,
    /// Normalized path snapshot for this revision.
    pub path: &'a VaultPath,
    /// Optional parent/base revision metadata recorded by Core.
    pub parent_revision_id: Option<&'a RevisionId>,
    /// Content-addressed blob hash already verified by the caller.
    pub content_hash: ContentHash,
    /// Non-negative content size in bytes.
    pub size_bytes: u64,
    /// Adapter or actor that created the revision metadata.
    pub created_by: &'a AdapterId,
}

/// Inserts an immutable file revision row.
///
/// This helper does not store file bytes, verify hashes, check idempotency,
/// update `sync_objects.current_revision_id`, append operation-log rows, or
/// resolve conflicts.
pub async fn insert_file_revision<'executor, ExecutorType>(
    executor: ExecutorType,
    input: NewFileRevision<'_>,
) -> RepositoryResult<FileRevisionRow>
where
    ExecutorType: Executor<'executor, Database = Postgres>,
{
    let parent_revision_id = input.parent_revision_id.map(RevisionId::as_str);
    let content_hash = input.content_hash.to_prefixed_string();
    let size_bytes = size_bytes_to_i64(input.size_bytes)?;

    let row = sqlx::query(
        "insert into file_revisions (\n             revision_id, object_id, path, parent_revision_id, content_sha256, size_bytes, created_by\n         )\n         values ($1, $2, $3, $4, $5, $6, $7)\n         returning revision_id, object_id, path, parent_revision_id, content_sha256, size_bytes, created_by, created_at",
    )
    .bind(input.revision_id.as_str())
    .bind(input.object_id)
    .bind(input.path.as_str())
    .bind(parent_revision_id)
    .bind(content_hash)
    .bind(size_bytes)
    .bind(input.created_by.as_str())
    .fetch_one(executor)
    .await
    .map_err(map_sqlx_error)?;

    file_revision_from_row(&row)
}

/// Reads an immutable file revision by id.
pub async fn get_file_revision_by_id<'executor, ExecutorType>(
    executor: ExecutorType,
    revision_id: &RevisionId,
) -> RepositoryResult<Option<FileRevisionRow>>
where
    ExecutorType: Executor<'executor, Database = Postgres>,
{
    let row = sqlx::query(
        "select revision_id, object_id, path, parent_revision_id, content_sha256, size_bytes, created_by, created_at\n         from file_revisions\n         where revision_id = $1",
    )
    .bind(revision_id.as_str())
    .fetch_optional(executor)
    .await
    .map_err(map_sqlx_error)?;

    row.as_ref().map(file_revision_from_row).transpose()
}

/// Reads the current file revision for a normalized vault path.
pub async fn get_current_revision_by_path<'executor, ExecutorType>(
    executor: ExecutorType,
    path: &VaultPath,
) -> RepositoryResult<Option<FileRevisionRow>>
where
    ExecutorType: Executor<'executor, Database = Postgres>,
{
    let row = sqlx::query(
        "select fr.revision_id, fr.object_id, fr.path, fr.parent_revision_id,\n                fr.content_sha256, fr.size_bytes, fr.created_by, fr.created_at\n         from sync_objects so\n         join file_revisions fr on fr.revision_id = so.current_revision_id\n         where so.path = $1",
    )
    .bind(path.as_str())
    .fetch_optional(executor)
    .await
    .map_err(map_sqlx_error)?;

    row.as_ref().map(file_revision_from_row).transpose()
}

/// Reads the current file revision for an object id.
pub async fn get_current_revision_by_object_id<'executor, ExecutorType>(
    executor: ExecutorType,
    object_id: &str,
) -> RepositoryResult<Option<FileRevisionRow>>
where
    ExecutorType: Executor<'executor, Database = Postgres>,
{
    let row = sqlx::query(
        "select fr.revision_id, fr.object_id, fr.path, fr.parent_revision_id,\n                fr.content_sha256, fr.size_bytes, fr.created_by, fr.created_at\n         from sync_objects so\n         join file_revisions fr on fr.revision_id = so.current_revision_id\n         where so.object_id = $1",
    )
    .bind(object_id)
    .fetch_optional(executor)
    .await
    .map_err(map_sqlx_error)?;

    row.as_ref().map(file_revision_from_row).transpose()
}

/// Lists revisions for an object, newest first.
pub async fn list_file_revisions_by_object_id<'executor, ExecutorType>(
    executor: ExecutorType,
    object_id: &str,
    limit: u32,
) -> RepositoryResult<Vec<FileRevisionRow>>
where
    ExecutorType: Executor<'executor, Database = Postgres>,
{
    let limit = i64::from(limit);
    let rows = sqlx::query(
        "select revision_id, object_id, path, parent_revision_id, content_sha256, size_bytes, created_by, created_at\n         from file_revisions\n         where object_id = $1\n         order by created_at desc, revision_id desc\n         limit $2",
    )
    .bind(object_id)
    .bind(limit)
    .fetch_all(executor)
    .await
    .map_err(map_sqlx_error)?;

    rows.iter().map(file_revision_from_row).collect()
}

/// Lists revisions for a normalized path snapshot, newest first.
pub async fn list_file_revisions_by_path<'executor, ExecutorType>(
    executor: ExecutorType,
    path: &VaultPath,
    limit: u32,
) -> RepositoryResult<Vec<FileRevisionRow>>
where
    ExecutorType: Executor<'executor, Database = Postgres>,
{
    let limit = i64::from(limit);
    let rows = sqlx::query(
        "select revision_id, object_id, path, parent_revision_id, content_sha256, size_bytes, created_by, created_at\n         from file_revisions\n         where path = $1\n         order by created_at desc, revision_id desc\n         limit $2",
    )
    .bind(path.as_str())
    .bind(limit)
    .fetch_all(executor)
    .await
    .map_err(map_sqlx_error)?;

    rows.iter().map(file_revision_from_row).collect()
}

/// Finds the newest revision for a path and content hash.
///
/// Core can use this as a passive read primitive when checking whether incoming
/// bytes are already represented by existing metadata. It does not decide the
/// duplicate-write outcome.
pub async fn find_file_revision_by_path_and_content<'executor, ExecutorType>(
    executor: ExecutorType,
    path: &VaultPath,
    content_hash: ContentHash,
) -> RepositoryResult<Option<FileRevisionRow>>
where
    ExecutorType: Executor<'executor, Database = Postgres>,
{
    let content_hash = content_hash.to_prefixed_string();
    let row = sqlx::query(
        "select revision_id, object_id, path, parent_revision_id, content_sha256, size_bytes, created_by, created_at\n         from file_revisions\n         where path = $1 and content_sha256 = $2\n         order by created_at desc, revision_id desc\n         limit 1",
    )
    .bind(path.as_str())
    .bind(content_hash)
    .fetch_optional(executor)
    .await
    .map_err(map_sqlx_error)?;

    row.as_ref().map(file_revision_from_row).transpose()
}

fn file_revision_from_row(row: &PgRow) -> RepositoryResult<FileRevisionRow> {
    Ok(FileRevisionRow {
        revision_id: row.try_get("revision_id").map_err(map_sqlx_error)?,
        object_id: row.try_get("object_id").map_err(map_sqlx_error)?,
        path: row.try_get("path").map_err(map_sqlx_error)?,
        parent_revision_id: row.try_get("parent_revision_id").map_err(map_sqlx_error)?,
        content_sha256: row.try_get("content_sha256").map_err(map_sqlx_error)?,
        size_bytes: row.try_get("size_bytes").map_err(map_sqlx_error)?,
        created_by: row.try_get("created_by").map_err(map_sqlx_error)?,
        created_at: row.try_get("created_at").map_err(map_sqlx_error)?,
    })
}
