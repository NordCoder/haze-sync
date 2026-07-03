//! Conflict storage repository helpers for W3 conflict API fan-in.
//!
//! This module performs narrow metadata queries and open-to-resolved status
//! updates for the existing `conflicts` table. It does not apply conflict
//! policy, move or delete content, call adapters/providers, or perform semantic
//! merges.

use crate::{
    models::ConflictRow,
    repositories::{map_sqlx_error, RepositoryError},
};
use haze_sync_common::{AdapterId, ConflictId};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgRow, Executor, Postgres, Row};
use std::{fmt, str::FromStr};

/// Storage vocabulary for conflict lifecycle statuses.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictStatusName {
    /// Conflict is unresolved and visible to clients.
    Open,
    /// Conflict has been resolved by a supported metadata-only action.
    Resolved,
    /// Conflict was intentionally ignored.
    Ignored,
}

impl ConflictStatusName {
    /// Return the canonical status string stored in Postgres.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Resolved => "resolved",
            Self::Ignored => "ignored",
        }
    }
}

impl fmt::Display for ConflictStatusName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for ConflictStatusName {
    type Err = RepositoryError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "open" => Ok(Self::Open),
            "resolved" => Ok(Self::Resolved),
            "ignored" => Ok(Self::Ignored),
            _ => Err(RepositoryError::InvalidConflictStatus),
        }
    }
}

/// Repository for conflict metadata reads and safe status updates.
#[derive(Clone, Copy, Debug, Default)]
pub struct ConflictRepository;

impl ConflictRepository {
    /// Construct a repository helper.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// List conflicts with a specific public lifecycle status.
    pub async fn list_by_status<'executor, E>(
        &self,
        executor: E,
        status: ConflictStatusName,
    ) -> Result<Vec<ConflictRow>, RepositoryError>
    where
        E: Executor<'executor, Database = Postgres>,
    {
        sqlx::query(conflict_select_sql(
            "where status = $1 order by created_at asc, conflict_id asc",
        ))
        .bind(status.as_str())
        .try_map(conflict_row_from_pg)
        .fetch_all(executor)
        .await
        .map_err(map_sqlx_error)
    }

    /// Read one conflict by id.
    pub async fn get_by_id<'executor, E>(
        &self,
        executor: E,
        conflict_id: &ConflictId,
    ) -> Result<Option<ConflictRow>, RepositoryError>
    where
        E: Executor<'executor, Database = Postgres>,
    {
        sqlx::query(conflict_select_sql("where conflict_id = $1"))
            .bind(conflict_id.as_str())
            .try_map(conflict_row_from_pg)
            .fetch_optional(executor)
            .await
            .map_err(map_sqlx_error)
    }

    /// Mark an open conflict resolved without changing file content.
    ///
    /// The status guard keeps this update safe under concurrent resolution attempts.
    pub async fn mark_open_resolved<'executor, E>(
        &self,
        executor: E,
        conflict_id: &ConflictId,
        resolved_by: &AdapterId,
    ) -> Result<Option<ConflictRow>, RepositoryError>
    where
        E: Executor<'executor, Database = Postgres>,
    {
        sqlx::query(
            "update conflicts \
             set status = 'resolved', resolved_at = now(), resolved_by = $2 \
             where conflict_id = $1 and status = 'open' \
             returning conflict_id, original_path, base_revision_id, current_revision_id, \
                       incoming_revision_id, incoming_adapter_id, policy_applied, \
                       materialized_path, status, created_at, resolved_at, resolved_by",
        )
        .bind(conflict_id.as_str())
        .bind(resolved_by.as_str())
        .try_map(conflict_row_from_pg)
        .fetch_optional(executor)
        .await
        .map_err(map_sqlx_error)
    }
}

fn conflict_select_sql(where_clause: &str) -> String {
    format!(
        "select conflict_id, original_path, base_revision_id, current_revision_id, \
              incoming_revision_id, incoming_adapter_id, policy_applied, materialized_path, \
              status, created_at, resolved_at, resolved_by \
         from conflicts {where_clause}"
    )
}

fn conflict_row_from_pg(row: PgRow) -> Result<ConflictRow, sqlx::Error> {
    Ok(ConflictRow {
        conflict_id: row.try_get("conflict_id")?,
        original_path: row.try_get("original_path")?,
        base_revision_id: row.try_get("base_revision_id")?,
        current_revision_id: row.try_get("current_revision_id")?,
        incoming_revision_id: row.try_get("incoming_revision_id")?,
        incoming_adapter_id: row.try_get("incoming_adapter_id")?,
        policy_applied: row.try_get("policy_applied")?,
        materialized_path: row.try_get("materialized_path")?,
        status: row.try_get("status")?,
        created_at: row.try_get("created_at")?,
        resolved_at: row.try_get("resolved_at")?,
        resolved_by: row.try_get("resolved_by")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conflict_status_names_match_contract_strings() {
        assert_eq!(ConflictStatusName::Open.as_str(), "open");
        assert_eq!(ConflictStatusName::Resolved.as_str(), "resolved");
        assert_eq!(ConflictStatusName::Ignored.as_str(), "ignored");
    }

    #[test]
    fn conflict_status_names_roundtrip_json_and_reject_unknown() {
        let json = serde_json::to_string(&ConflictStatusName::Open).unwrap();
        assert_eq!(json, "\"open\"");
        assert_eq!(
            serde_json::from_str::<ConflictStatusName>(&json).unwrap(),
            ConflictStatusName::Open
        );
        assert_eq!(
            ConflictStatusName::from_str("deleted"),
            Err(RepositoryError::InvalidConflictStatus)
        );
    }
}
