//! Conflict storage repository helpers for W3 conflict API fan-in.
//!
//! This module performs narrow metadata inserts, reads, and guarded lifecycle
//! updates for the existing `conflicts` table. It does not apply conflict policy,
//! move or delete content, call adapters/providers, or perform semantic merges.

use crate::{
    models::ConflictRow,
    repositories::{map_sqlx_error, validate_limit, RepositoryError, RepositoryResult},
};
use haze_sync_common::{AdapterId, ConflictId, RevisionId, VaultPath};
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

/// Caller-decided conflict metadata to persist.
#[derive(Clone, Debug)]
pub struct NewConflict<'a> {
    pub conflict_id: &'a ConflictId,
    pub original_path: &'a VaultPath,
    pub base_revision_id: Option<&'a RevisionId>,
    pub current_revision_id: &'a RevisionId,
    pub incoming_revision_id: &'a RevisionId,
    pub incoming_adapter_id: &'a AdapterId,
    pub policy_applied: &'a str,
    pub materialized_path: &'a VaultPath,
    pub status: ConflictStatusName,
}

/// Repository for conflict metadata inserts, reads, and safe status updates.
#[derive(Clone, Copy, Debug, Default)]
pub struct ConflictRepository;

impl ConflictRepository {
    /// Construct a repository helper.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Insert caller-decided conflict metadata.
    ///
    /// This helper does not choose policy, materialize content, append operation
    /// log rows, or update the current object revision.
    pub async fn insert<'executor, E>(
        &self,
        executor: E,
        input: &NewConflict<'_>,
    ) -> RepositoryResult<ConflictRow>
    where
        E: Executor<'executor, Database = Postgres>,
    {
        let row = sqlx::query(
            "insert into conflicts ( \
                 conflict_id, original_path, base_revision_id, current_revision_id, \
                 incoming_revision_id, incoming_adapter_id, policy_applied, \
                 materialized_path, status \
             ) values ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
             returning conflict_id, original_path, base_revision_id, current_revision_id, \
                       incoming_revision_id, incoming_adapter_id, policy_applied, \
                       materialized_path, status, created_at, resolved_at, resolved_by",
        )
        .bind(input.conflict_id.as_str())
        .bind(input.original_path.as_str())
        .bind(input.base_revision_id.map(RevisionId::as_str))
        .bind(input.current_revision_id.as_str())
        .bind(input.incoming_revision_id.as_str())
        .bind(input.incoming_adapter_id.as_str())
        .bind(input.policy_applied)
        .bind(input.materialized_path.as_str())
        .bind(input.status.as_str())
        .fetch_one(executor)
        .await
        .map_err(map_sqlx_error)?;

        conflict_row_from_pg(&row)
    }

    /// List conflicts with a specific lifecycle status.
    pub async fn list_by_status<'executor, E>(
        &self,
        executor: E,
        status: ConflictStatusName,
        limit: u32,
    ) -> RepositoryResult<Vec<ConflictRow>>
    where
        E: Executor<'executor, Database = Postgres>,
    {
        validate_limit(limit)?;
        let sql = conflict_select_sql(
            "where status = $1 order by created_at asc, conflict_id asc limit $2",
        );
        let rows = sqlx::query(&sql)
            .bind(status.as_str())
            .bind(i64::from(limit))
            .fetch_all(executor)
            .await
            .map_err(map_sqlx_error)?;

        rows.iter().map(conflict_row_from_pg).collect()
    }

    /// Read one conflict by id.
    pub async fn get_by_id<'executor, E>(
        &self,
        executor: E,
        conflict_id: &ConflictId,
    ) -> RepositoryResult<Option<ConflictRow>>
    where
        E: Executor<'executor, Database = Postgres>,
    {
        let sql = conflict_select_sql("where conflict_id = $1");
        let row = sqlx::query(&sql)
            .bind(conflict_id.as_str())
            .fetch_optional(executor)
            .await
            .map_err(map_sqlx_error)?;

        row.as_ref().map(conflict_row_from_pg).transpose()
    }

    /// Mark an open conflict resolved without changing file content.
    pub async fn mark_open_resolved<'executor, E>(
        &self,
        executor: E,
        conflict_id: &ConflictId,
        resolved_by: &AdapterId,
    ) -> RepositoryResult<Option<ConflictRow>>
    where
        E: Executor<'executor, Database = Postgres>,
    {
        self.mark_open_status(
            executor,
            conflict_id,
            resolved_by,
            ConflictStatusName::Resolved,
        )
        .await
    }

    /// Mark an open conflict ignored without changing file content.
    pub async fn mark_open_ignored<'executor, E>(
        &self,
        executor: E,
        conflict_id: &ConflictId,
        resolved_by: &AdapterId,
    ) -> RepositoryResult<Option<ConflictRow>>
    where
        E: Executor<'executor, Database = Postgres>,
    {
        self.mark_open_status(
            executor,
            conflict_id,
            resolved_by,
            ConflictStatusName::Ignored,
        )
        .await
    }

    async fn mark_open_status<'executor, E>(
        &self,
        executor: E,
        conflict_id: &ConflictId,
        resolved_by: &AdapterId,
        status: ConflictStatusName,
    ) -> RepositoryResult<Option<ConflictRow>>
    where
        E: Executor<'executor, Database = Postgres>,
    {
        let row = sqlx::query(
            "update conflicts \
             set status = $2, resolved_at = now(), resolved_by = $3 \
             where conflict_id = $1 and status = 'open' \
             returning conflict_id, original_path, base_revision_id, current_revision_id, \
                       incoming_revision_id, incoming_adapter_id, policy_applied, \
                       materialized_path, status, created_at, resolved_at, resolved_by",
        )
        .bind(conflict_id.as_str())
        .bind(status.as_str())
        .bind(resolved_by.as_str())
        .fetch_optional(executor)
        .await
        .map_err(map_sqlx_error)?;

        row.as_ref().map(conflict_row_from_pg).transpose()
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

fn conflict_row_from_pg(row: &PgRow) -> RepositoryResult<ConflictRow> {
    let status: String = row.try_get("status").map_err(map_sqlx_error)?;
    ConflictStatusName::from_str(&status)?;

    Ok(ConflictRow {
        conflict_id: row.try_get("conflict_id").map_err(map_sqlx_error)?,
        original_path: row.try_get("original_path").map_err(map_sqlx_error)?,
        base_revision_id: row.try_get("base_revision_id").map_err(map_sqlx_error)?,
        current_revision_id: row
            .try_get("current_revision_id")
            .map_err(map_sqlx_error)?,
        incoming_revision_id: row
            .try_get("incoming_revision_id")
            .map_err(map_sqlx_error)?,
        incoming_adapter_id: row
            .try_get("incoming_adapter_id")
            .map_err(map_sqlx_error)?,
        policy_applied: row.try_get("policy_applied").map_err(map_sqlx_error)?,
        materialized_path: row.try_get("materialized_path").map_err(map_sqlx_error)?,
        status,
        created_at: row.try_get("created_at").map_err(map_sqlx_error)?,
        resolved_at: row.try_get("resolved_at").map_err(map_sqlx_error)?,
        resolved_by: row.try_get("resolved_by").map_err(map_sqlx_error)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::MAX_CHANGES_LIMIT;

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

    #[test]
    fn conflict_listing_uses_shared_page_bounds() {
        assert_eq!(validate_limit(1), Ok(()));
        assert_eq!(validate_limit(MAX_CHANGES_LIMIT), Ok(()));
        assert_eq!(
            validate_limit(0),
            Err(RepositoryError::InvalidLimit {
                max: MAX_CHANGES_LIMIT
            })
        );
    }

    #[test]
    fn new_conflict_input_uses_validated_identifiers_and_paths() {
        let conflict_id = ConflictId::parse("conf_01JSTORP6").unwrap();
        let original_path = VaultPath::parse("Notes/today.md").unwrap();
        let materialized_path = VaultPath::parse("Notes/today.conflict.md").unwrap();
        let base_revision_id = RevisionId::parse("rev_01JBASE").unwrap();
        let current_revision_id = RevisionId::parse("rev_01JCURRENT").unwrap();
        let incoming_revision_id = RevisionId::parse("rev_01JINCOMING").unwrap();
        let incoming_adapter_id = AdapterId::parse("worktree-adapter").unwrap();
        let input = NewConflict {
            conflict_id: &conflict_id,
            original_path: &original_path,
            base_revision_id: Some(&base_revision_id),
            current_revision_id: &current_revision_id,
            incoming_revision_id: &incoming_revision_id,
            incoming_adapter_id: &incoming_adapter_id,
            policy_applied: "preserve_both",
            materialized_path: &materialized_path,
            status: ConflictStatusName::Open,
        };

        assert_eq!(input.conflict_id.as_str(), "conf_01JSTORP6");
        assert_eq!(input.original_path.as_str(), "Notes/today.md");
        assert_eq!(
            input.base_revision_id.map(RevisionId::as_str),
            Some("rev_01JBASE")
        );
        assert_eq!(input.status, ConflictStatusName::Open);
    }
}
