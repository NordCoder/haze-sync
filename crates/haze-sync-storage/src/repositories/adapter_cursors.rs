//! Adapter cursor storage primitives.
//!
//! Cursors track adapter progress through the Core operation log. This module
//! provides storage helpers only; it does not fetch batches, run adapters, apply
//! external changes, or decide when Server should checkpoint. Raw external cursor
//! JSON remains internal; status surfaces should use [`AdapterCursorSummary`].

use crate::models::AdapterCursorRow;
use crate::repositories::{map_sqlx_error, validate_sequence, RepositoryError, RepositoryResult};
use chrono::{DateTime, Utc};
use haze_sync_common::AdapterId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{postgres::PgRow, Executor, Postgres, Row, Transaction};

const CURSOR_COLUMNS: &str =
    "adapter_id, last_core_seq, external_cursor_json, last_success_at, updated_at";

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

const LOCK_CURSOR_SQL: &str = "select adapter_id, last_core_seq, external_cursor_json, \
     last_success_at, updated_at from adapter_cursors \
     where adapter_id = $1 for update";

const ADVANCE_EXACT_CURSOR_SQL: &str = "update adapter_cursors \
     set last_core_seq = $3, last_success_at = now(), updated_at = now() \
     where adapter_id = $1 and last_core_seq = $2 \
     returning adapter_id, last_core_seq, external_cursor_json, last_success_at, updated_at";

/// Requested broad monotonic adapter cursor update retained for existing adapters.
#[derive(Clone, Debug, PartialEq)]
pub struct AdapterCursorUpdate {
    pub adapter_id: AdapterId,
    pub last_core_seq: i64,
    pub external_cursor_json: Option<Value>,
    pub mark_success: bool,
}

/// Result of the compatibility monotonic cursor update.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum AdapterCursorUpdateOutcome {
    Updated(AdapterCursorRow),
    RejectedRegression {
        current: AdapterCursorRow,
        requested_seq: i64,
    },
}

impl AdapterCursorUpdateOutcome {
    #[must_use]
    pub const fn is_updated(&self) -> bool {
        matches!(self, Self::Updated(_))
    }
}

/// Cursor metadata safe for status/admin mapping because it excludes raw external
/// cursor JSON.
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

#[derive(Clone, Copy, Debug, Default)]
pub struct AdapterCursorRepository;

impl AdapterCursorRepository {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Read the current internal cursor row without taking a lock.
    pub async fn get_by_adapter_id<'executor, E>(
        &self,
        executor: E,
        adapter_id: &AdapterId,
    ) -> RepositoryResult<Option<AdapterCursorRow>>
    where
        E: Executor<'executor, Database = Postgres>,
    {
        let sql = format!(
            "select {CURSOR_COLUMNS} from adapter_cursors where adapter_id = $1"
        );
        let row = sqlx::query(&sql)
            .bind(adapter_id.as_str())
            .fetch_optional(executor)
            .await
            .map_err(map_sqlx_error)?;

        row.as_ref().map(adapter_cursor_row_from_pg).transpose()
    }

    /// Initialize a cursor at sequence zero if missing and return its internal row.
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

    /// Compatibility update for adapters that persist arbitrary monotonic cursor
    /// progress. Worktree export must use [`Self::advance_exact_contiguous`].
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

    /// Lock and read the current cursor in the caller-owned transaction.
    ///
    /// The returned summary never exposes raw external cursor payloads. The row
    /// lock remains held until the caller commits or rolls back the transaction.
    pub async fn lock_current(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
        adapter_id: &AdapterId,
    ) -> RepositoryResult<Option<AdapterCursorSummary>> {
        let row = sqlx::query(LOCK_CURSOR_SQL)
            .bind(adapter_id.as_str())
            .fetch_optional(&mut **transaction)
            .await
            .map_err(map_sqlx_error)?;

        row.as_ref()
            .map(adapter_cursor_row_from_pg)
            .transpose()
            .map(|cursor| cursor.as_ref().map(AdapterCursorSummary::from))
    }

    /// Initialize a missing cursor at zero and lock it in the caller transaction.
    pub async fn initialize_and_lock(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
        adapter_id: &AdapterId,
    ) -> RepositoryResult<AdapterCursorSummary> {
        let row = sqlx::query(INITIALIZE_CURSOR_SQL)
            .bind(adapter_id.as_str())
            .fetch_one(&mut **transaction)
            .await
            .map_err(map_sqlx_error)?;
        let cursor = adapter_cursor_row_from_pg(&row)?;

        Ok(AdapterCursorSummary::from(&cursor))
    }

    /// Advance a locked cursor only from the exact expected value to its exact
    /// contiguous successor.
    ///
    /// Validation rejects regression/equality, gaps, stale expected values and
    /// overflow. The caller must materialize one authoritative operation and
    /// update Worktree path state in the same transaction before invoking this
    /// method, then commit both facts together.
    pub async fn advance_exact_contiguous(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
        adapter_id: &AdapterId,
        expected_current: i64,
        next_sequence: i64,
    ) -> RepositoryResult<AdapterCursorSummary> {
        validate_exact_transition(expected_current, next_sequence)?;

        let current = self
            .lock_current(transaction, adapter_id)
            .await?
            .ok_or(RepositoryError::CursorMissing)?;
        if current.last_core_seq != expected_current {
            return Err(RepositoryError::CursorStaleExpected);
        }

        let row = sqlx::query(ADVANCE_EXACT_CURSOR_SQL)
            .bind(adapter_id.as_str())
            .bind(expected_current)
            .bind(next_sequence)
            .fetch_optional(&mut **transaction)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(RepositoryError::CursorStaleExpected)?;
        let cursor = adapter_cursor_row_from_pg(&row)?;

        Ok(AdapterCursorSummary::from(&cursor))
    }
}

fn validate_exact_transition(expected_current: i64, next_sequence: i64) -> RepositoryResult<()> {
    validate_sequence(expected_current)?;
    validate_sequence(next_sequence)?;

    if next_sequence <= expected_current {
        return Err(RepositoryError::CursorRegression);
    }
    let contiguous = expected_current
        .checked_add(1)
        .ok_or(RepositoryError::CursorOverflow)?;
    if next_sequence != contiguous {
        return Err(RepositoryError::CursorGap);
    }

    Ok(())
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
