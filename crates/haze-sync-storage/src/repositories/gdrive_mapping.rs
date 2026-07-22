//! Passive repository helpers for durable Google Drive mapping facts.
//!
//! Storage persists caller-supplied provider metadata and Core references. It does
//! not interpret Drive identities, call provider APIs, decide import/export
//! direction, apply echo guards, or classify delete candidates.

use super::{map_sqlx_error, validate_sequence, RepositoryError, RepositoryResult};
use crate::models::GDriveMappingRow;
use chrono::{DateTime, Utc};
use haze_sync_common::{RevisionId, VaultPath};
use sqlx::{postgres::PgRow, Executor, Postgres, Row};

const MAX_PROVIDER_IDENTIFIER_LEN: usize = 1_024;
const MAX_PROVIDER_TEXT_LEN: usize = 4_096;
const MD5_HEX_LEN: usize = 32;

const SELECT_BY_PATH_SQL: &str =
    "select path, drive_file_id, drive_parent_id, drive_name, mime_type, \
     md5_checksum, head_revision_id, drive_version, drive_modified_time, \
     core_revision_id, core_seq, last_imported_at, last_exported_at, \
     last_seen_at, delete_candidate_at \
     from gdrive_mapping where path = $1";

const SELECT_BY_DRIVE_FILE_ID_SQL: &str =
    "select path, drive_file_id, drive_parent_id, drive_name, mime_type, \
     md5_checksum, head_revision_id, drive_version, drive_modified_time, \
     core_revision_id, core_seq, last_imported_at, last_exported_at, \
     last_seen_at, delete_candidate_at \
     from gdrive_mapping where drive_file_id = $1";

const UPSERT_SQL: &str = "insert into gdrive_mapping ( \
         path, drive_file_id, drive_parent_id, drive_name, mime_type, \
         md5_checksum, head_revision_id, drive_version, drive_modified_time, \
         core_revision_id, core_seq, last_imported_at, last_exported_at, \
         last_seen_at, delete_candidate_at \
     ) values ( \
         $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15 \
     ) \
     on conflict (path) do update set \
         drive_file_id = excluded.drive_file_id, \
         drive_parent_id = excluded.drive_parent_id, \
         drive_name = excluded.drive_name, \
         mime_type = excluded.mime_type, \
         md5_checksum = excluded.md5_checksum, \
         head_revision_id = excluded.head_revision_id, \
         drive_version = excluded.drive_version, \
         drive_modified_time = excluded.drive_modified_time, \
         core_revision_id = excluded.core_revision_id, \
         core_seq = excluded.core_seq, \
         last_imported_at = excluded.last_imported_at, \
         last_exported_at = excluded.last_exported_at, \
         last_seen_at = excluded.last_seen_at, \
         delete_candidate_at = excluded.delete_candidate_at \
     returning path, drive_file_id, drive_parent_id, drive_name, mime_type, \
         md5_checksum, head_revision_id, drive_version, drive_modified_time, \
         core_revision_id, core_seq, last_imported_at, last_exported_at, \
         last_seen_at, delete_candidate_at";

/// Complete caller-decided fact set for one `gdrive_mapping` row.
///
/// Supplying or clearing fields is an adapter/Core decision made before this
/// repository helper is called. The helper only validates storage-level shapes
/// and persists the exact fact set atomically by vault path.
#[derive(Clone, Debug)]
pub struct GDriveMappingUpsert<'a> {
    pub path: &'a VaultPath,
    pub drive_file_id: Option<&'a str>,
    pub drive_parent_id: Option<&'a str>,
    pub drive_name: Option<&'a str>,
    pub mime_type: Option<&'a str>,
    pub md5_checksum: Option<&'a str>,
    pub head_revision_id: Option<&'a str>,
    pub drive_version: Option<&'a str>,
    pub drive_modified_time: Option<DateTime<Utc>>,
    pub core_revision_id: Option<&'a RevisionId>,
    pub core_seq: Option<i64>,
    pub last_imported_at: Option<DateTime<Utc>>,
    pub last_exported_at: Option<DateTime<Utc>>,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub delete_candidate_at: Option<DateTime<Utc>>,
}

/// Inserts or replaces the complete persisted mapping fact set for a path.
///
/// The caller owns transaction boundaries and any provider/Core policy that
/// produced the values. A unique `drive_file_id` collision is returned through
/// the safe database error boundary without exposing provider identifiers.
pub async fn upsert_gdrive_mapping<'executor, ExecutorType>(
    executor: ExecutorType,
    input: GDriveMappingUpsert<'_>,
) -> RepositoryResult<GDriveMappingRow>
where
    ExecutorType: Executor<'executor, Database = Postgres>,
{
    validate_gdrive_mapping_input(&input)?;
    let core_revision_id = input.core_revision_id.map(RevisionId::as_str);

    let row = sqlx::query(UPSERT_SQL)
        .bind(input.path.as_str())
        .bind(input.drive_file_id)
        .bind(input.drive_parent_id)
        .bind(input.drive_name)
        .bind(input.mime_type)
        .bind(input.md5_checksum)
        .bind(input.head_revision_id)
        .bind(input.drive_version)
        .bind(input.drive_modified_time)
        .bind(core_revision_id)
        .bind(input.core_seq)
        .bind(input.last_imported_at)
        .bind(input.last_exported_at)
        .bind(input.last_seen_at)
        .bind(input.delete_candidate_at)
        .fetch_one(executor)
        .await
        .map_err(map_sqlx_error)?;

    gdrive_mapping_from_pg(&row)
}

/// Reads one mapping by normalized vault path.
pub async fn get_gdrive_mapping_by_path<'executor, ExecutorType>(
    executor: ExecutorType,
    path: &VaultPath,
) -> RepositoryResult<Option<GDriveMappingRow>>
where
    ExecutorType: Executor<'executor, Database = Postgres>,
{
    let row = sqlx::query(SELECT_BY_PATH_SQL)
        .bind(path.as_str())
        .fetch_optional(executor)
        .await
        .map_err(map_sqlx_error)?;

    row.as_ref().map(gdrive_mapping_from_pg).transpose()
}

/// Reads one mapping by opaque Drive file identifier.
///
/// The identifier is treated only as an opaque storage key. Storage does not
/// interpret its provider meaning or expose it through public status output.
pub async fn get_gdrive_mapping_by_drive_file_id<'executor, ExecutorType>(
    executor: ExecutorType,
    drive_file_id: &str,
) -> RepositoryResult<Option<GDriveMappingRow>>
where
    ExecutorType: Executor<'executor, Database = Postgres>,
{
    validate_provider_identifier(drive_file_id)?;

    let row = sqlx::query(SELECT_BY_DRIVE_FILE_ID_SQL)
        .bind(drive_file_id)
        .fetch_optional(executor)
        .await
        .map_err(map_sqlx_error)?;

    row.as_ref().map(gdrive_mapping_from_pg).transpose()
}

fn validate_gdrive_mapping_input(input: &GDriveMappingUpsert<'_>) -> RepositoryResult<()> {
    validate_optional_provider_identifier(input.drive_file_id)?;
    validate_optional_provider_identifier(input.drive_parent_id)?;
    validate_optional_provider_text(input.drive_name)?;
    validate_optional_provider_text(input.mime_type)?;
    validate_optional_md5(input.md5_checksum)?;
    validate_optional_provider_identifier(input.head_revision_id)?;
    validate_optional_provider_identifier(input.drive_version)?;

    if let Some(core_seq) = input.core_seq {
        validate_sequence(core_seq)?;
    }

    Ok(())
}

fn gdrive_mapping_from_pg(row: &PgRow) -> RepositoryResult<GDriveMappingRow> {
    let mapped = GDriveMappingRow {
        path: row.try_get("path").map_err(map_sqlx_error)?,
        drive_file_id: row.try_get("drive_file_id").map_err(map_sqlx_error)?,
        drive_parent_id: row.try_get("drive_parent_id").map_err(map_sqlx_error)?,
        drive_name: row.try_get("drive_name").map_err(map_sqlx_error)?,
        mime_type: row.try_get("mime_type").map_err(map_sqlx_error)?,
        md5_checksum: row.try_get("md5_checksum").map_err(map_sqlx_error)?,
        head_revision_id: row.try_get("head_revision_id").map_err(map_sqlx_error)?,
        drive_version: row.try_get("drive_version").map_err(map_sqlx_error)?,
        drive_modified_time: row.try_get("drive_modified_time").map_err(map_sqlx_error)?,
        core_revision_id: row.try_get("core_revision_id").map_err(map_sqlx_error)?,
        core_seq: row.try_get("core_seq").map_err(map_sqlx_error)?,
        last_imported_at: row.try_get("last_imported_at").map_err(map_sqlx_error)?,
        last_exported_at: row.try_get("last_exported_at").map_err(map_sqlx_error)?,
        last_seen_at: row.try_get("last_seen_at").map_err(map_sqlx_error)?,
        delete_candidate_at: row.try_get("delete_candidate_at").map_err(map_sqlx_error)?,
    };

    validate_gdrive_mapping_row(mapped)
}

fn validate_gdrive_mapping_row(row: GDriveMappingRow) -> RepositoryResult<GDriveMappingRow> {
    let normalized_path = VaultPath::parse(&row.path).map_err(|_| RepositoryError::InvalidPath)?;
    if normalized_path.as_str() != row.path {
        return Err(RepositoryError::InvalidPath);
    }

    validate_optional_provider_identifier(row.drive_file_id.as_deref())?;
    validate_optional_provider_identifier(row.drive_parent_id.as_deref())?;
    validate_optional_provider_text(row.drive_name.as_deref())?;
    validate_optional_provider_text(row.mime_type.as_deref())?;
    validate_optional_md5(row.md5_checksum.as_deref())?;
    validate_optional_provider_identifier(row.head_revision_id.as_deref())?;
    validate_optional_provider_identifier(row.drive_version.as_deref())?;

    if let Some(core_revision_id) = row.core_revision_id.as_deref() {
        RevisionId::parse(core_revision_id).map_err(|_| RepositoryError::InvalidIdentifier)?;
    }
    if let Some(core_seq) = row.core_seq {
        validate_sequence(core_seq)?;
    }

    Ok(row)
}

fn validate_optional_provider_identifier(value: Option<&str>) -> RepositoryResult<()> {
    value.map_or(Ok(()), validate_provider_identifier)
}

fn validate_provider_identifier(value: &str) -> RepositoryResult<()> {
    if value.is_empty()
        || value.len() > MAX_PROVIDER_IDENTIFIER_LEN
        || value
            .chars()
            .any(|character| character.is_control() || character.is_whitespace())
    {
        return Err(RepositoryError::InvalidIdentifier);
    }

    Ok(())
}

fn validate_optional_provider_text(value: Option<&str>) -> RepositoryResult<()> {
    let Some(value) = value else {
        return Ok(());
    };

    if value.is_empty()
        || value.len() > MAX_PROVIDER_TEXT_LEN
        || value.chars().any(char::is_control)
    {
        return Err(RepositoryError::InvalidProviderMetadata);
    }

    Ok(())
}

fn validate_optional_md5(value: Option<&str>) -> RepositoryResult<()> {
    let Some(value) = value else {
        return Ok(());
    };

    if value.len() != MD5_HEX_LEN || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(RepositoryError::InvalidHash);
    }

    Ok(())
}

#[cfg(test)]
mod tests;

#[cfg(all(test, feature = "test-support"))]
mod postgres_tests;
