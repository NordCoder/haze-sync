//! Worktree-specific authoritative reads through the reusable application boundary.

use super::{files, ApplicationError, AuthoritativeRevisionContent, RevisionContentQuery};
use haze_sync_common::{RevisionId, VaultPath};
use haze_sync_storage::{repositories::revisions::get_file_revision_by_id, LocalObjectStore};
use sqlx::PgPool;

/// Resolve a revision's authoritative path before loading and verifying its bytes.
///
/// Conflict-created and backup operations can carry an operation path that differs
/// from the stored revision path. The executor therefore resolves the revision row
/// inside the application-service boundary instead of guessing a filesystem path.
pub(super) async fn revision_content_by_id(
    pool: &PgPool,
    object_store: &LocalObjectStore,
    revision_id: RevisionId,
) -> Result<AuthoritativeRevisionContent, ApplicationError> {
    let row = get_file_revision_by_id(pool, &revision_id)
        .await
        .map_err(|_| ApplicationError::Internal)?
        .ok_or(ApplicationError::NotFound)?;
    let path = VaultPath::parse(&row.path).map_err(|_| ApplicationError::Internal)?;

    files::revision_content(
        pool,
        object_store,
        RevisionContentQuery {
            path,
            revision_id: Some(revision_id),
        },
    )
    .await
}
