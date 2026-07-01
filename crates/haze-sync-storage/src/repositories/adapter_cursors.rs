//! Adapter cursor storage primitives.
//!
//! Cursors track adapter progress through the Core operation log. This module
//! provides monotonic storage helpers only; it does not run adapters, apply
//! external changes, decide conflict/delete policy, or expose route handlers.

use crate::models::AdapterCursorRow;
use crate::repositories::{map_sqlx_error, validate_sequence, RepositoryError};
use haze_sync_common::AdapterId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{postgres::PgRow, Executor, Postgres, Row};

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

#[derive(Clone, Debug, PartialEq)]
struct CursorUpdateRow {
    cursor: AdapterCursorRow,
    update_accepted: bool,
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
    ) -> Result<Option<AdapterCursorRow>, RepositoryError>
    where
        E: Executor<'executor, Database = Postgres>,
    {
        sqlx::query(
            "select adapter_id, last_core_seq, external_cursor_json, last_success_at, updated_at \
             from adapter_cursors \
             where adapter_id = $1",
        )
        .bind(adapter_id.as_str())
        .try_map(adapter_cursor_row_from_pg)
        .fetch_optional(executor)
        .await
        .map_err(map_sqlx_error)
    }

    /// Initialize a cursor at sequence zero if it is missing, otherwise return
    /// the existing row unchanged.
    pub async fn initialize_if_missing<'executor, E>(
        &self,
        executor: E,
        adapter_id: &AdapterId,
    ) -> Result<AdapterCursorRow, RepositoryError>
    where
        E: Executor<'executor, Database = Postgres>,
    {
        sqlx::query(
            "with inserted as ( \
                 insert into adapter_cursors (adapter_id) \
                 values ($1) \
                 on conflict (adapter_id) do nothing \
                 returning adapter_id, last_core_seq, external_cursor_json, last_success_at, updated_at \
             ) \
             select adapter_id, last_core_seq, external_cursor_json, last_success_at, updated_at \
             from inserted \
             union all \
             select adapter_id, last_core_seq, external_cursor_json, last_success_at, updated_at \
             from adapter_cursors \
             where adapter_id = $1 \
             limit 1",
        )
        .bind(adapter_id.as_str())
        .try_map(adapter_cursor_row_from_pg)
        .fetch_one(executor)
        .await
        .map_err(map_sqlx_error)
    }

    /// Initialize when missing and update the Core sequence only when the update
    /// does not move the cursor backwards.
    pub async fn update_monotonic<'executor, E>(
        &self,
        executor: E,
        update: &AdapterCursorUpdate,
    ) -> Result<AdapterCursorUpdateOutcome, RepositoryError>
    where
        E: Executor<'executor, Database = Postgres>,
    {
        validate_sequence(update.last_core_seq)?;

        let row = sqlx::query(
            "with ensured as ( \
                 insert into adapter_cursors (adapter_id) \
                 values ($1) \
                 on conflict (adapter_id) do nothing \
             ), \
             updated as ( \
                 update adapter_cursors \
                 set last_core_seq = $2, \
                     external_cursor_json = coalesce($3::jsonb, external_cursor_json), \
                     last_success_at = case when $4 then now() else last_success_at end, \
                     updated_at = now() \
                 where adapter_id = $1 \
                   and last_core_seq <= $2 \
                 returning adapter_id, last_core_seq, external_cursor_json, last_success_at, updated_at, true as update_accepted \
             ), \
             current_row as ( \
                 select adapter_id, last_core_seq, external_cursor_json, last_success_at, updated_at, false as update_accepted \
                 from adapter_cursors \
                 where adapter_id = $1 \
                   and not exists (select 1 from updated) \
             ) \
             select adapter_id, last_core_seq, external_cursor_json, last_success_at, updated_at, update_accepted \
             from updated \
             union all \
             select adapter_id, last_core_seq, external_cursor_json, last_success_at, updated_at, update_accepted \
             from current_row \
             limit 1",
        )
        .bind(update.adapter_id.as_str())
        .bind(update.last_core_seq)
        .bind(update.external_cursor_json.clone())
        .bind(update.mark_success)
        .try_map(cursor_update_row_from_pg)
        .fetch_one(executor)
        .await
        .map_err(map_sqlx_error)?;

        if row.update_accepted {
            Ok(AdapterCursorUpdateOutcome::Updated(row.cursor))
        } else {
            Ok(AdapterCursorUpdateOutcome::RejectedRegression {
                current: row.cursor,
                requested_seq: update.last_core_seq,
            })
        }
    }
}

fn adapter_cursor_row_from_pg(row: PgRow) -> Result<AdapterCursorRow, sqlx::Error> {
    Ok(AdapterCursorRow {
        adapter_id: row.try_get("adapter_id")?,
        last_core_seq: row.try_get("last_core_seq")?,
        external_cursor_json: row.try_get("external_cursor_json")?,
        last_success_at: row.try_get("last_success_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn cursor_update_row_from_pg(row: PgRow) -> Result<CursorUpdateRow, sqlx::Error> {
    Ok(CursorUpdateRow {
        cursor: AdapterCursorRow {
            adapter_id: row.try_get("adapter_id")?,
            last_core_seq: row.try_get("last_core_seq")?,
            external_cursor_json: row.try_get("external_cursor_json")?,
            last_success_at: row.try_get("last_success_at")?,
            updated_at: row.try_get("updated_at")?,
        },
        update_accepted: row.try_get("update_accepted")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_outcome_reports_updated_status() {
        let row = AdapterCursorRow {
            adapter_id: "worktree-adapter".to_owned(),
            last_core_seq: 7,
            external_cursor_json: serde_json::json!({}),
            last_success_at: None,
            updated_at: chrono::DateTime::<chrono::Utc>::from(std::time::UNIX_EPOCH),
        };

        assert!(AdapterCursorUpdateOutcome::Updated(row.clone()).is_updated());
        assert!(!AdapterCursorUpdateOutcome::RejectedRegression {
            current: row,
            requested_seq: 3,
        }
        .is_updated());
    }

    #[test]
    fn update_request_accepts_json_cursor_metadata_without_secrets() {
        let update = AdapterCursorUpdate {
            adapter_id: AdapterId::parse("gdrive-adapter").unwrap(),
            last_core_seq: 42,
            external_cursor_json: Some(serde_json::json!({ "page_token": "opaque-test-token" })),
            mark_success: true,
        };

        assert_eq!(update.last_core_seq, 42);
        assert!(update.external_cursor_json.is_some());
    }
}
