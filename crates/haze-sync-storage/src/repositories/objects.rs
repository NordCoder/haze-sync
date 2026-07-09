//! Repository helpers for `sync_objects` rows.

use super::{map_sqlx_error, RepositoryResult};
use crate::models::SyncObjectRow;
use chrono::{DateTime, Utc};
use haze_sync_common::{AdapterId, RevisionId, VaultPath};
use sqlx::{postgres::PgRow, Executor, Postgres, Row};

/// Storage value for `sync_objects.kind`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SyncObjectKind {
    /// A regular synchronized file.
    File,
    /// A directory placeholder reserved for future UI or adapter needs.
    DirectoryPlaceholder,
}

impl SyncObjectKind {
    /// Database representation used by the storage schema.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::DirectoryPlaceholder => "directory_placeholder",
        }
    }
}

/// Input for creating a passive `sync_objects` row.
#[derive(Clone, Copy, Debug)]
pub struct NewSyncObject<'a> {
    /// Stable object identifier chosen by the caller.
    pub object_id: &'a str,
    /// Normalized vault path.
    pub path: &'a VaultPath,
    /// Stored object kind.
    pub kind: SyncObjectKind,
    /// Adapter or actor responsible for the metadata update.
    pub updated_by: &'a AdapterId,
}

/// Inserts a new sync object row and returns the inserted row.
pub async fn insert_sync_object<'executor, ExecutorType>(
    executor: ExecutorType,
    input: NewSyncObject<'_>,
) -> RepositoryResult<SyncObjectRow>
where
    ExecutorType: Executor<'executor, Database = Postgres>,
{
    let row = sqlx::query(
        "insert into sync_objects (object_id, path, kind, updated_by)\n         values ($1, $2, $3, $4)\n         returning object_id, path, kind, current_revision_id, deleted_at, updated_at, updated_by",
    )
    .bind(input.object_id)
    .bind(input.path.as_str())
    .bind(input.kind.as_str())
    .bind(input.updated_by.as_str())
    .fetch_one(executor)
    .await
    .map_err(map_sqlx_error)?;

    sync_object_from_row(&row)
}

/// Creates an object for `path` when absent, otherwise returns the existing row.
///
/// The helper is concurrency-safe at the unique-path constraint level but does
/// not acquire locks or decide overwrite/conflict policy. Core callers should
/// acquire the per-path transaction lock before using this in an apply path.
pub async fn create_or_find_sync_object_by_path<'executor, ExecutorType>(
    executor: ExecutorType,
    input: NewSyncObject<'_>,
) -> RepositoryResult<SyncObjectRow>
where
    ExecutorType: Executor<'executor, Database = Postgres>,
{
    let row = sqlx::query(
        "with inserted as (\n             insert into sync_objects (object_id, path, kind, updated_by)\n             values ($1, $2, $3, $4)\n             on conflict (path) do nothing\n             returning object_id, path, kind, current_revision_id, deleted_at, updated_at, updated_by\n         )\n         select object_id, path, kind, current_revision_id, deleted_at, updated_at, updated_by\n         from inserted\n         union all\n         select object_id, path, kind, current_revision_id, deleted_at, updated_at, updated_by\n         from sync_objects\n         where path = $5\n         limit 1",
    )
    .bind(input.object_id)
    .bind(input.path.as_str())
    .bind(input.kind.as_str())
    .bind(input.updated_by.as_str())
    .bind(input.path.as_str())
    .fetch_one(executor)
    .await
    .map_err(map_sqlx_error)?;

    sync_object_from_row(&row)
}

/// Reads a sync object by stable object id.
pub async fn get_sync_object_by_id<'executor, ExecutorType>(
    executor: ExecutorType,
    object_id: &str,
) -> RepositoryResult<Option<SyncObjectRow>>
where
    ExecutorType: Executor<'executor, Database = Postgres>,
{
    let row = sqlx::query(
        "select object_id, path, kind, current_revision_id, deleted_at, updated_at, updated_by\n         from sync_objects\n         where object_id = $1",
    )
    .bind(object_id)
    .fetch_optional(executor)
    .await
    .map_err(map_sqlx_error)?;

    row.as_ref().map(sync_object_from_row).transpose()
}

/// Reads the current sync object row for a normalized vault path.
pub async fn get_sync_object_by_path<'executor, ExecutorType>(
    executor: ExecutorType,
    path: &VaultPath,
) -> RepositoryResult<Option<SyncObjectRow>>
where
    ExecutorType: Executor<'executor, Database = Postgres>,
{
    let row = sqlx::query(
        "select object_id, path, kind, current_revision_id, deleted_at, updated_at, updated_by\n         from sync_objects\n         where path = $1",
    )
    .bind(path.as_str())
    .fetch_optional(executor)
    .await
    .map_err(map_sqlx_error)?;

    row.as_ref().map(sync_object_from_row).transpose()
}

/// Updates only `current_revision_id` and safe update metadata for an object id.
///
/// This is a passive metadata helper. It does not validate base revisions,
/// append operations, create idempotency records, or apply conflict/delete
/// policy.
pub async fn set_current_revision_by_object_id<'executor, ExecutorType>(
    executor: ExecutorType,
    object_id: &str,
    current_revision_id: Option<&RevisionId>,
    updated_by: &AdapterId,
) -> RepositoryResult<Option<SyncObjectRow>>
where
    ExecutorType: Executor<'executor, Database = Postgres>,
{
    let revision_id = current_revision_id.map(RevisionId::as_str);
    let row = sqlx::query(
        "update sync_objects\n         set current_revision_id = $2, updated_by = $3, updated_at = now()\n         where object_id = $1\n         returning object_id, path, kind, current_revision_id, deleted_at, updated_at, updated_by",
    )
    .bind(object_id)
    .bind(revision_id)
    .bind(updated_by.as_str())
    .fetch_optional(executor)
    .await
    .map_err(map_sqlx_error)?;

    row.as_ref().map(sync_object_from_row).transpose()
}

/// Updates only `current_revision_id` and safe update metadata for a path.
///
/// This helper is intended for caller-owned transactions after the caller has
/// acquired the path lock and made Core policy decisions.
pub async fn set_current_revision_by_path<'executor, ExecutorType>(
    executor: ExecutorType,
    path: &VaultPath,
    current_revision_id: Option<&RevisionId>,
    updated_by: &AdapterId,
) -> RepositoryResult<Option<SyncObjectRow>>
where
    ExecutorType: Executor<'executor, Database = Postgres>,
{
    let revision_id = current_revision_id.map(RevisionId::as_str);
    let row = sqlx::query(
        "update sync_objects\n         set current_revision_id = $2, updated_by = $3, updated_at = now()\n         where path = $1\n         returning object_id, path, kind, current_revision_id, deleted_at, updated_at, updated_by",
    )
    .bind(path.as_str())
    .bind(revision_id)
    .bind(updated_by.as_str())
    .fetch_optional(executor)
    .await
    .map_err(map_sqlx_error)?;

    row.as_ref().map(sync_object_from_row).transpose()
}

/// Updates only the `deleted_at` metadata for an object id.
///
/// This passive helper records or clears the object delete marker chosen by a
/// future delete service. It does not create tombstones, move bytes to trash,
/// enforce retention, apply delete guards, append operation-log entries, or
/// physically delete database rows.
pub async fn set_sync_object_deleted_at_by_object_id<'executor, ExecutorType>(
    executor: ExecutorType,
    object_id: &str,
    deleted_at: Option<DateTime<Utc>>,
    updated_by: &AdapterId,
) -> RepositoryResult<Option<SyncObjectRow>>
where
    ExecutorType: Executor<'executor, Database = Postgres>,
{
    let row = sqlx::query(
        "update sync_objects\n         set deleted_at = $2, updated_by = $3, updated_at = now()\n         where object_id = $1\n         returning object_id, path, kind, current_revision_id, deleted_at, updated_at, updated_by",
    )
    .bind(object_id)
    .bind(deleted_at)
    .bind(updated_by.as_str())
    .fetch_optional(executor)
    .await
    .map_err(map_sqlx_error)?;

    row.as_ref().map(sync_object_from_row).transpose()
}

/// Updates only the `deleted_at` metadata for a normalized path.
///
/// This helper is storage-only. It does not decide whether a delete is safe and
/// never performs a hard delete.
pub async fn set_sync_object_deleted_at_by_path<'executor, ExecutorType>(
    executor: ExecutorType,
    path: &VaultPath,
    deleted_at: Option<DateTime<Utc>>,
    updated_by: &AdapterId,
) -> RepositoryResult<Option<SyncObjectRow>>
where
    ExecutorType: Executor<'executor, Database = Postgres>,
{
    let row = sqlx::query(
        "update sync_objects\n         set deleted_at = $2, updated_by = $3, updated_at = now()\n         where path = $1\n         returning object_id, path, kind, current_revision_id, deleted_at, updated_at, updated_by",
    )
    .bind(path.as_str())
    .bind(deleted_at)
    .bind(updated_by.as_str())
    .fetch_optional(executor)
    .await
    .map_err(map_sqlx_error)?;

    row.as_ref().map(sync_object_from_row).transpose()
}

fn sync_object_from_row(row: &PgRow) -> RepositoryResult<SyncObjectRow> {
    Ok(SyncObjectRow {
        object_id: row.try_get("object_id").map_err(map_sqlx_error)?,
        path: row.try_get("path").map_err(map_sqlx_error)?,
        kind: row.try_get("kind").map_err(map_sqlx_error)?,
        current_revision_id: row.try_get("current_revision_id").map_err(map_sqlx_error)?,
        deleted_at: row.try_get("deleted_at").map_err(map_sqlx_error)?,
        updated_at: row.try_get("updated_at").map_err(map_sqlx_error)?,
        updated_by: row.try_get("updated_by").map_err(map_sqlx_error)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_object_kind_names_match_storage_contract() {
        assert_eq!(SyncObjectKind::File.as_str(), "file");
        assert_eq!(
            SyncObjectKind::DirectoryPlaceholder.as_str(),
            "directory_placeholder"
        );
    }

    #[test]
    fn new_sync_object_input_uses_validated_path_and_adapter_values() {
        let path = VaultPath::parse("Notes/today.md").unwrap();
        let adapter_id = AdapterId::parse("worktree-adapter").unwrap();
        let input = NewSyncObject {
            object_id: "obj_01JSTORP5",
            path: &path,
            kind: SyncObjectKind::File,
            updated_by: &adapter_id,
        };

        assert_eq!(input.object_id, "obj_01JSTORP5");
        assert_eq!(input.path.as_str(), "Notes/today.md");
        assert_eq!(input.kind.as_str(), "file");
        assert_eq!(input.updated_by.as_str(), "worktree-adapter");
    }

    #[test]
    fn current_revision_update_input_remains_passive_metadata() {
        let revision_id = RevisionId::parse("rev_01JSTORP5").unwrap();
        let revision_id = Some(&revision_id).map(RevisionId::as_str);

        assert_eq!(revision_id, Some("rev_01JSTORP5"));
    }
}
