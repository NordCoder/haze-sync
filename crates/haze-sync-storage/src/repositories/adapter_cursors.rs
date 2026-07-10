//! Adapter cursor storage primitives.
//!
//! Cursors track adapter progress through the Core operation log. This module
//! provides monotonic storage helpers only; it does not run adapters, apply
//! external changes, decide conflict/delete policy, or expose route handlers.
//! Raw external cursor JSON remains an internal persisted value; status surfaces
//! should use [`AdapterCursorSummary`] instead of serializing row models.

use crate::models::AdapterCursorRow;
use crate::repositories::{map_sqlx_error, validate_sequence, RepositoryError, RepositoryResult};
use chrono::{DateTime, Utc};
use haze_sync_common::AdapterId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{postgres::PgRow, Executor, Postgres, Row};

const INITIALIZE_CURSOR_SQL: &str = "insert into adapter_cursors (adapter_id) \
     values ($1) \
     on conflict (adapter_id) do update \
     set adapter_id = adapter_cursors.adapter_id \
     returning adapter_id, last_core_seq, external_cursor_json, last_success_at, updated_at";

const UPDATE_CURSOR_SQL: &str = "insert into adapter_cursors ( \
         adapter_id, last_core_seq, external_cursor_json, last_success_at \
     ) values ( \
         $1, $2, coalesce($3::jsonb, '{}'::jsonb), \
         case when $4 then now() else null end \
     ) \
     on conflict (adapter_id) do update \
     set last_core_seq = case \
             when adapter_cursors.last_core_seq <= excluded.last_core_seq \
             then excluded.last_core_seq \
             else adapter_cursors.last_core_seq \
         end, \
         external_cursor_json = case \
             when adapter_cursors.last_core_seq <= excluded.last_core_seq \
             then coalesce($3::jsonb, adapter_cursors.external_cursor_json) \
             else adapter_cursors.external_cursor_json \
         end, \
         last_success_at = case \
             when adapter_cursors.last_core_seq <= excluded.last_core_seq and $4 \
             then now() \
             else adapter_cursors.last_success_at \
         end, \
         updated_at = case \
             when adapter_cursors.last_core_seq <= excluded.last_core_seq \
             then now() \
             else adapter_cursors.updated_at \
         end \
     returning adapter_id, last_core_seq, external_cursor_json, last_success_at, updated_at";

/// Requested adapter cursor update.
#[derive(Clone, Debug, PartialEq)]
pub struct AdapterCursorUpdate {
    pub adapter_id: AdapterId,
    pub last_core_seq: i64,
    pub external_cursor_json: Option<Value>,
    pub mark_success: bool,
}

/// Result of a monotonic cursor update.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum AdapterCursorUpdateOutcome {
    /// The row was initialized or updated without moving backwards.
    Updated(AdapterCursorRow),
    /// The requested sequence was lower than the persisted cursor.
    RejectedRegression {
        current: AdapterCursorRow,
        requested_seq: i64,
    },
}

impl AdapterCursorUpdateOutcome {
    /// Returns true when the requested cursor value was persisted.
    #[must_use]
    pub const fn is_updated(&self) -> bool {
        matches!(self, Self::Updated(_))
    }
}

/// Cursor metadata safe for status/admin mapping because it excludes the raw
/// external cursor JSON value.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AdapterCursorSummary {
    pub adapter_id: String,
    pub last_core_seq: i64,
    pub has_external_cursor: bool,
    pub last_success_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

impl From<&AdapterCursorRow> for AdapterCursorSummary {
    fn from(row: &AdapterCursorRow) -> Self {
        Self {
            adapter_id: row.adapter_id.clone(),
            last_core_seq: row.last_core_seq,
            has_external_cursor: has_external_cursor(&row.external_cursor_json),
            last_success_at: row.last_success_at,
            updated_at: row.updated_at,
        }
    }
}

/// Repository for per-adapter cursor rows.
#[derive(Clone, Copy, Debug, Default)]
pub struct AdapterCursorRepository;

impl AdapterCursorRepository {
    /// Construct a repository helper.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Read the current cursor for an adapter.
    pub async fn get_by_adapter_id<'executor, E>(
        &self,
        executor: E,
        adapter_id: &AdapterId,
    ) -> RepositoryResult<Option<AdapterCursorRow>>
    where
        E: Executor<'executor, Database = Postgres>,
    {
        let row = sqlx::query(
            "select adapter_id, last_core_seq, external_cursor_json, last_success_at, updated_at \
             from adapter_cursors \
             where adapter_id = $1",
        )
        .bind(adapter_id.as_str())
        .fetch_optional(executor)
        .await
        .map_err(map_sqlx_error)?;

        row.as_ref().map(adapter_cursor_row_from_pg).transpose()
    }

    /// Initialize a cursor at sequence zero if it is missing, otherwise return
    /// the existing row unchanged.
    ///
    /// The no-op upsert always returns the canonical row, including after a
    /// concurrent initializer wins the unique-key race.
    pub async fn initialize_if_missing<'executor, E>(
        &self,
        executor: E,
        adapter_id: &AdapterId,
    ) -> RepositoryResult<AdapterCursorRow>
    where
        E: Executor<'executor, Database = Postgres>,
    {
        let row = sqlx::query(INITIALIZE_CURSOR_SQL)
            .bind(adapter_id.as_str())
            .fetch_one(executor)
            .await
            .map_err(map_sqlx_error)?;

        adapter_cursor_row_from_pg(&row)
    }

    /// Initialize when missing and update the Core sequence only when the update
    /// does not move the cursor backwards.
    ///
    /// A single upsert handles missing rows and concurrent initializers. When the
    /// requested sequence regresses, the persisted sequence, external cursor,
    /// success timestamp, and updated timestamp remain unchanged.
    pub async fn update_monotonic<'executor, E>(
        &self,
        executor: E,
        update: &AdapterCursorUpdate,
    ) -> RepositoryResult<AdapterCursorUpdateOutcome>
    where
        E: Executor<'executor, Database = Postgres>,
    {
        validate_sequence(update.last_core_seq)?;

        let row = sqlx::query(UPDATE_CURSOR_SQL)
            .bind(update.adapter_id.as_str())
            .bind(update.last_core_seq)
            .bind(update.external_cursor_json.clone())
            .bind(update.mark_success)
            .fetch_one(executor)
            .await
            .map_err(map_sqlx_error)?;
        let cursor = adapter_cursor_row_from_pg(&row)?;

        if cursor.last_core_seq == update.last_core_seq {
            Ok(AdapterCursorUpdateOutcome::Updated(cursor))
        } else {
            Ok(AdapterCursorUpdateOutcome::RejectedRegression {
                current: cursor,
                requested_seq: update.last_core_seq,
            })
        }
    }
}

fn adapter_cursor_row_from_pg(row: &PgRow) -> RepositoryResult<AdapterCursorRow> {
    let adapter_id: String = row.try_get("adapter_id").map_err(map_sqlx_error)?;
    AdapterId::parse(&adapter_id).map_err(|_| RepositoryError::DatabaseOperationFailed)?;

    let last_core_seq = row.try_get("last_core_seq").map_err(map_sqlx_error)?;
    validate_sequence(last_core_seq)?;

    Ok(AdapterCursorRow {
        adapter_id,
        last_core_seq,
        external_cursor_json: row
            .try_get("external_cursor_json")
            .map_err(map_sqlx_error)?,
        last_success_at: row.try_get("last_success_at").map_err(map_sqlx_error)?,
        updated_at: row.try_get("updated_at").map_err(map_sqlx_error)?,
    })
}

fn has_external_cursor(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Object(fields) => !fields.is_empty(),
        _ => true,
    }
}

#[cfg(test)]
mod tests;

#[cfg(all(test, feature = "test-support"))]
mod postgres_tests;
