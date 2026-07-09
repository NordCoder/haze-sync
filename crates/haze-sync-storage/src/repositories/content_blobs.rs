//! Repository helpers for immutable `content_blobs` rows.
//!
//! These helpers persist only blob metadata. They do not write object-store
//! bytes, verify content against the object store, perform retention cleanup, or
//! decide revision/conflict/delete policy.

use super::{map_sqlx_error, size_bytes_to_i64, RepositoryResult};
use crate::models::ContentBlobRow;
use haze_sync_common::ContentHash;
use sqlx::{postgres::PgRow, Executor, Postgres, Row};

/// Input for inserting immutable content-blob metadata.
#[derive(Clone, Debug)]
pub struct NewContentBlob<'a> {
    /// Content-addressed blob hash.
    pub sha256: ContentHash,
    /// Blob size in bytes.
    pub size_bytes: u64,
    /// Relative object-store location chosen by the caller.
    pub object_store_path: &'a str,
}

/// Inserts blob metadata when missing, otherwise returns the existing row.
///
/// The helper is immutable-by-hash and therefore uses the existing row on hash
/// collisions instead of overwriting metadata.
pub async fn create_or_get_content_blob<'executor, E>(
    executor: E,
    input: &NewContentBlob<'_>,
) -> RepositoryResult<ContentBlobRow>
where
    E: Executor<'executor, Database = Postgres>,
{
    let size_bytes = size_bytes_to_i64(input.size_bytes)?;
    let row = sqlx::query(
        "with inserted as ( \
             insert into content_blobs (sha256, size_bytes, object_store_path) \
             values ($1, $2, $3) \
             on conflict (sha256) do nothing \
             returning sha256, size_bytes, object_store_path, created_at \
         ) \
         select sha256, size_bytes, object_store_path, created_at \
         from inserted \
         union all \
         select sha256, size_bytes, object_store_path, created_at \
         from content_blobs \
         where sha256 = $1 \
         limit 1",
    )
    .bind(input.sha256.to_prefixed_string())
    .bind(size_bytes)
    .bind(input.object_store_path)
    .fetch_one(executor)
    .await
    .map_err(map_sqlx_error)?;

    content_blob_from_row(&row)
}

/// Reads blob metadata by hash.
pub async fn get_content_blob_by_hash<'executor, E>(
    executor: E,
    sha256: ContentHash,
) -> RepositoryResult<Option<ContentBlobRow>>
where
    E: Executor<'executor, Database = Postgres>,
{
    let row = sqlx::query(
        "select sha256, size_bytes, object_store_path, created_at \
         from content_blobs \
         where sha256 = $1",
    )
    .bind(sha256.to_prefixed_string())
    .fetch_optional(executor)
    .await
    .map_err(map_sqlx_error)?;

    row.as_ref().map(content_blob_from_row).transpose()
}

fn content_blob_from_row(row: &PgRow) -> RepositoryResult<ContentBlobRow> {
    Ok(ContentBlobRow {
        sha256: row.try_get("sha256").map_err(map_sqlx_error)?,
        size_bytes: row.try_get("size_bytes").map_err(map_sqlx_error)?,
        object_store_path: row.try_get("object_store_path").map_err(map_sqlx_error)?,
        created_at: row.try_get("created_at").map_err(map_sqlx_error)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::RepositoryError;

    #[test]
    fn new_content_blob_metadata_uses_content_hash_not_vault_path() {
        let hash = ContentHash::parse(&"a".repeat(64)).unwrap();
        let input = NewContentBlob {
            sha256: hash,
            size_bytes: 42,
            object_store_path:
                "sha256/aa/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        };

        assert_eq!(
            input.sha256.to_prefixed_string(),
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        );
        assert_eq!(input.size_bytes, 42);
        assert!(!input.object_store_path.contains("Notes/"));
        assert!(!input.object_store_path.starts_with('/'));
    }

    #[test]
    fn content_blob_size_uses_repository_range_validation() {
        let hash = ContentHash::parse(&"b".repeat(64)).unwrap();
        let valid = NewContentBlob {
            sha256: hash,
            size_bytes: i64::MAX as u64,
            object_store_path: "sha256/bb/blob",
        };
        let invalid = NewContentBlob {
            sha256: hash,
            size_bytes: i64::MAX as u64 + 1,
            object_store_path: "sha256/bb/blob",
        };

        assert_eq!(size_bytes_to_i64(valid.size_bytes), Ok(i64::MAX));
        assert_eq!(
            size_bytes_to_i64(invalid.size_bytes),
            Err(RepositoryError::InvalidSizeBytes)
        );
    }
}
