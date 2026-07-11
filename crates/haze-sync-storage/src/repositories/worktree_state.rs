//! Passive repositories for versioned durable Worktree instance and path state.
//!
//! Storage records already-decided facts. It does not normalize roots, scan or
//! materialize files, classify drift, choose import/export behavior, or own
//! transaction timing.

use super::{
    map_sqlx_error, size_bytes_to_i64, validate_limit, RepositoryError, RepositoryResult,
};
use crate::models::{WorktreeInstanceRow, WorktreeStateRow};
use chrono::{DateTime, Utc};
use haze_sync_common::{AdapterId, ContentHash, RevisionId, Sha256, VaultPath};
use sqlx::{postgres::PgRow, Executor, Postgres, Row};
use std::fmt;

pub const WORKTREE_STATE_FORMAT_VERSION: i32 = 1;
pub const WORKTREE_RECONCILIATION_FORMAT_VERSION: i32 = 1;

const INSTANCE_COLUMNS: &str =
    "adapter_id, root_fingerprint, state_format_version, created_at, updated_at";
const STATE_COLUMNS: &str = "adapter_id, path, state_kind, state_format_version, \
     last_applied_revision_id, content_sha256, observation_schema_version, \
     observed_size_bytes, observed_mtime, created_at, updated_at";

const BIND_INSTANCE_SQL: &str = "insert into worktree_instances ( \
         adapter_id, root_fingerprint, state_format_version \
     ) values ($1, $2, $3) \
     on conflict (adapter_id) do update \
     set adapter_id = worktree_instances.adapter_id \
     returning adapter_id, root_fingerprint, state_format_version, created_at, updated_at";

const UPSERT_STATE_SQL: &str = "insert into worktree_state ( \
         adapter_id, path, state_kind, state_format_version, \
         last_applied_revision_id, content_sha256, observation_schema_version, \
         observed_size_bytes, observed_mtime \
     ) values ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
     on conflict (adapter_id, path) do update set \
         state_kind = excluded.state_kind, \
         state_format_version = excluded.state_format_version, \
         last_applied_revision_id = excluded.last_applied_revision_id, \
         content_sha256 = excluded.content_sha256, \
         observation_schema_version = excluded.observation_schema_version, \
         observed_size_bytes = excluded.observed_size_bytes, \
         observed_mtime = excluded.observed_mtime, \
         updated_at = now() \
     returning adapter_id, path, state_kind, state_format_version, \
         last_applied_revision_id, content_sha256, observation_schema_version, \
         observed_size_bytes, observed_mtime, created_at, updated_at";

const UPDATE_OBSERVATION_SQL: &str = "update worktree_state \
     set observation_schema_version = $5, observed_size_bytes = $6, \
         observed_mtime = $7, updated_at = now() \
     where adapter_id = $1 and path = $2 and state_kind = 'present' \
       and last_applied_revision_id = $3 and content_sha256 = $4 \
     returning adapter_id, path, state_kind, state_format_version, \
         last_applied_revision_id, content_sha256, observation_schema_version, \
         observed_size_bytes, observed_mtime, created_at, updated_at";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorktreeStateKind {
    Present,
    Tombstoned,
}

impl WorktreeStateKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Present => "present",
            Self::Tombstoned => "tombstoned",
        }
    }

    fn parse(value: &str) -> RepositoryResult<Self> {
        match value {
            "present" => Ok(Self::Present),
            "tombstoned" => Ok(Self::Tombstoned),
            _ => Err(RepositoryError::InvalidWorktreeStateKind),
        }
    }
}

/// Typed adapter/root binding input. Debug intentionally redacts the fingerprint.
#[derive(Clone, PartialEq)]
pub struct WorktreeInstanceBinding {
    adapter_id: AdapterId,
    root_fingerprint: Sha256,
    state_format_version: i32,
}

impl WorktreeInstanceBinding {
    pub fn new(adapter_id: AdapterId, root_fingerprint: Sha256) -> Self {
        Self {
            adapter_id,
            root_fingerprint,
            state_format_version: WORKTREE_STATE_FORMAT_VERSION,
        }
    }

    pub fn try_with_version(
        adapter_id: AdapterId,
        root_fingerprint: Sha256,
        state_format_version: i32,
    ) -> RepositoryResult<Self> {
        validate_state_version(state_format_version)?;
        Ok(Self {
            adapter_id,
            root_fingerprint,
            state_format_version,
        })
    }

    #[must_use]
    pub const fn adapter_id(&self) -> &AdapterId {
        &self.adapter_id
    }

    #[must_use]
    pub const fn state_format_version(&self) -> i32 {
        self.state_format_version
    }
}

impl fmt::Debug for WorktreeInstanceBinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WorktreeInstanceBinding")
            .field("adapter_id", &self.adapter_id)
            .field("root_fingerprint", &"[REDACTED]")
            .field("state_format_version", &self.state_format_version)
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WorktreeReconciliationObservation {
    schema_version: i32,
    size_bytes: u64,
    modified_at: Option<DateTime<Utc>>,
}

impl WorktreeReconciliationObservation {
    #[must_use]
    pub fn new(size_bytes: u64, modified_at: Option<DateTime<Utc>>) -> Self {
        Self {
            schema_version: WORKTREE_RECONCILIATION_FORMAT_VERSION,
            size_bytes,
            modified_at,
        }
    }

    pub fn try_with_version(
        schema_version: i32,
        size_bytes: u64,
        modified_at: Option<DateTime<Utc>>,
    ) -> RepositoryResult<Self> {
        validate_reconciliation_version(schema_version)?;
        size_bytes_to_i64(size_bytes)?;
        Ok(Self {
            schema_version,
            size_bytes,
            modified_at,
        })
    }
}

#[derive(Clone, Debug)]
pub struct WorktreePresentStateUpsert<'a> {
    pub adapter_id: &'a AdapterId,
    pub path: &'a VaultPath,
    pub last_applied_revision_id: &'a RevisionId,
    pub content_hash: ContentHash,
    pub observation: Option<WorktreeReconciliationObservation>,
}

#[derive(Clone, Debug)]
pub struct WorktreeTombstonedStateUpsert<'a> {
    pub adapter_id: &'a AdapterId,
    pub path: &'a VaultPath,
    pub last_applied_revision_id: &'a RevisionId,
}

#[derive(Clone, Debug)]
pub struct WorktreeObservationUpdate<'a> {
    pub adapter_id: &'a AdapterId,
    pub path: &'a VaultPath,
    pub expected_revision_id: &'a RevisionId,
    pub expected_content_hash: ContentHash,
    pub observation: WorktreeReconciliationObservation,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WorktreeStateSnapshotPage {
    pub states: Vec<WorktreeStateRow>,
    pub next_after_path: Option<VaultPath>,
}

pub async fn bind_or_verify_worktree_instance<'executor, E>(
    executor: E,
    binding: &WorktreeInstanceBinding,
) -> RepositoryResult<WorktreeInstanceRow>
where
    E: Executor<'executor, Database = Postgres>,
{
    validate_state_version(binding.state_format_version)?;
    let fingerprint = binding.root_fingerprint.to_string();
    let row = sqlx::query(BIND_INSTANCE_SQL)
        .bind(binding.adapter_id.as_str())
        .bind(&fingerprint)
        .bind(binding.state_format_version)
        .fetch_one(executor)
        .await
        .map_err(map_sqlx_error)?;
    let persisted = worktree_instance_from_pg(&row)?;

    if persisted.root_fingerprint != fingerprint
        || persisted.state_format_version != binding.state_format_version
    {
        return Err(RepositoryError::WorktreeInstanceBindingMismatch);
    }

    Ok(persisted)
}

pub async fn load_worktree_instance<'executor, E>(
    executor: E,
    adapter_id: &AdapterId,
) -> RepositoryResult<Option<WorktreeInstanceRow>>
where
    E: Executor<'executor, Database = Postgres>,
{
    let sql = format!(
        "select {INSTANCE_COLUMNS} from worktree_instances where adapter_id = $1"
    );
    let row = sqlx::query(&sql)
        .bind(adapter_id.as_str())
        .fetch_optional(executor)
        .await
        .map_err(map_sqlx_error)?;

    row.as_ref().map(worktree_instance_from_pg).transpose()
}

pub async fn load_path_state<'executor, E>(
    executor: E,
    adapter_id: &AdapterId,
    path: &VaultPath,
) -> RepositoryResult<Option<WorktreeStateRow>>
where
    E: Executor<'executor, Database = Postgres>,
{
    let sql = format!(
        "select {STATE_COLUMNS} from worktree_state where adapter_id = $1 and path = $2"
    );
    let row = sqlx::query(&sql)
        .bind(adapter_id.as_str())
        .bind(path.as_str())
        .fetch_optional(executor)
        .await
        .map_err(map_sqlx_error)?;

    row.as_ref().map(worktree_state_from_pg).transpose()
}

pub async fn load_snapshot<'executor, E>(
    executor: E,
    adapter_id: &AdapterId,
    after_path: Option<&VaultPath>,
    limit: u32,
) -> RepositoryResult<WorktreeStateSnapshotPage>
where
    E: Executor<'executor, Database = Postgres>,
{
    validate_limit(limit)?;
    let sql = format!(
        "select {STATE_COLUMNS} from worktree_state \
         where adapter_id = $1 and ($2::text is null or path > $2) \
         order by path asc limit $3"
    );
    let rows = sqlx::query(&sql)
        .bind(adapter_id.as_str())
        .bind(after_path.map(VaultPath::as_str))
        .bind(i64::from(limit) + 1)
        .fetch_all(executor)
        .await
        .map_err(map_sqlx_error)?;
    let mut states = rows
        .iter()
        .map(worktree_state_from_pg)
        .collect::<RepositoryResult<Vec<_>>>()?;
    let has_more = states.len() > limit as usize;
    if has_more {
        states.truncate(limit as usize);
    }
    let next_after_path = if has_more {
        states
            .last()
            .map(|state| VaultPath::parse(&state.path).map_err(|_| RepositoryError::InvalidPath))
            .transpose()?
    } else {
        None
    };

    Ok(WorktreeStateSnapshotPage {
        states,
        next_after_path,
    })
}

pub async fn upsert_present_state<'executor, E>(
    executor: E,
    input: &WorktreePresentStateUpsert<'_>,
) -> RepositoryResult<WorktreeStateRow>
where
    E: Executor<'executor, Database = Postgres>,
{
    let observation = observation_columns(input.observation.as_ref())?;
    upsert_state(
        executor,
        input.adapter_id,
        input.path,
        WorktreeStateKind::Present,
        input.last_applied_revision_id,
        Some(input.content_hash),
        observation,
    )
    .await
}

pub async fn upsert_tombstoned_state<'executor, E>(
    executor: E,
    input: &WorktreeTombstonedStateUpsert<'_>,
) -> RepositoryResult<WorktreeStateRow>
where
    E: Executor<'executor, Database = Postgres>,
{
    upsert_state(
        executor,
        input.adapter_id,
        input.path,
        WorktreeStateKind::Tombstoned,
        input.last_applied_revision_id,
        None,
        ObservationColumns::none(),
    )
    .await
}

pub async fn update_reconciliation_observation<'executor, E>(
    executor: E,
    input: &WorktreeObservationUpdate<'_>,
) -> RepositoryResult<Option<WorktreeStateRow>>
where
    E: Executor<'executor, Database = Postgres>,
{
    let observation = observation_columns(Some(&input.observation))?;
    let row = sqlx::query(UPDATE_OBSERVATION_SQL)
        .bind(input.adapter_id.as_str())
        .bind(input.path.as_str())
        .bind(input.expected_revision_id.as_str())
        .bind(input.expected_content_hash.to_string())
        .bind(observation.schema_version)
        .bind(observation.size_bytes)
        .bind(observation.modified_at)
        .fetch_optional(executor)
        .await
        .map_err(map_sqlx_error)?;

    row.as_ref().map(worktree_state_from_pg).transpose()
}

async fn upsert_state<'executor, E>(
    executor: E,
    adapter_id: &AdapterId,
    path: &VaultPath,
    state_kind: WorktreeStateKind,
    revision_id: &RevisionId,
    content_hash: Option<ContentHash>,
    observation: ObservationColumns,
) -> RepositoryResult<WorktreeStateRow>
where
    E: Executor<'executor, Database = Postgres>,
{
    if state_kind == WorktreeStateKind::Present && content_hash.is_none() {
        return Err(RepositoryError::InvalidWorktreeStateKind);
    }
    if state_kind == WorktreeStateKind::Tombstoned
        && (content_hash.is_some() || observation.schema_version.is_some())
    {
        return Err(RepositoryError::InvalidWorktreeStateKind);
    }

    let row = sqlx::query(UPSERT_STATE_SQL)
        .bind(adapter_id.as_str())
        .bind(path.as_str())
        .bind(state_kind.as_str())
        .bind(WORKTREE_STATE_FORMAT_VERSION)
        .bind(revision_id.as_str())
        .bind(content_hash.map(|hash| hash.to_string()))
        .bind(observation.schema_version)
        .bind(observation.size_bytes)
        .bind(observation.modified_at)
        .fetch_one(executor)
        .await
        .map_err(map_sqlx_error)?;

    worktree_state_from_pg(&row)
}

#[derive(Clone)]
struct ObservationColumns {
    schema_version: Option<i32>,
    size_bytes: Option<i64>,
    modified_at: Option<DateTime<Utc>>,
}

impl ObservationColumns {
    const fn none() -> Self {
        Self {
            schema_version: None,
            size_bytes: None,
            modified_at: None,
        }
    }
}

fn observation_columns(
    observation: Option<&WorktreeReconciliationObservation>,
) -> RepositoryResult<ObservationColumns> {
    let Some(observation) = observation else {
        return Ok(ObservationColumns::none());
    };
    validate_reconciliation_version(observation.schema_version)?;

    Ok(ObservationColumns {
        schema_version: Some(observation.schema_version),
        size_bytes: Some(size_bytes_to_i64(observation.size_bytes)?),
        modified_at: observation.modified_at,
    })
}

fn worktree_instance_from_pg(row: &PgRow) -> RepositoryResult<WorktreeInstanceRow> {
    let adapter_id: String = row.try_get("adapter_id").map_err(map_sqlx_error)?;
    AdapterId::parse(&adapter_id).map_err(|_| RepositoryError::InvalidIdentifier)?;
    let root_fingerprint: String = row
        .try_get("root_fingerprint")
        .map_err(map_sqlx_error)?;
    validate_canonical_sha256(&root_fingerprint)?;
    let state_format_version = row
        .try_get("state_format_version")
        .map_err(map_sqlx_error)?;
    validate_state_version(state_format_version)?;

    Ok(WorktreeInstanceRow {
        adapter_id,
        root_fingerprint,
        state_format_version,
        created_at: row.try_get("created_at").map_err(map_sqlx_error)?,
        updated_at: row.try_get("updated_at").map_err(map_sqlx_error)?,
    })
}

fn worktree_state_from_pg(row: &PgRow) -> RepositoryResult<WorktreeStateRow> {
    let mapped = WorktreeStateRow {
        adapter_id: row.try_get("adapter_id").map_err(map_sqlx_error)?,
        path: row.try_get("path").map_err(map_sqlx_error)?,
        state_kind: row.try_get("state_kind").map_err(map_sqlx_error)?,
        state_format_version: row
            .try_get("state_format_version")
            .map_err(map_sqlx_error)?,
        last_applied_revision_id: row
            .try_get("last_applied_revision_id")
            .map_err(map_sqlx_error)?,
        content_sha256: row.try_get("content_sha256").map_err(map_sqlx_error)?,
        observation_schema_version: row
            .try_get("observation_schema_version")
            .map_err(map_sqlx_error)?,
        observed_size_bytes: row
            .try_get("observed_size_bytes")
            .map_err(map_sqlx_error)?,
        observed_mtime: row.try_get("observed_mtime").map_err(map_sqlx_error)?,
        created_at: row.try_get("created_at").map_err(map_sqlx_error)?,
        updated_at: row.try_get("updated_at").map_err(map_sqlx_error)?,
    };

    validate_worktree_state_row(mapped)
}

fn validate_worktree_state_row(row: WorktreeStateRow) -> RepositoryResult<WorktreeStateRow> {
    AdapterId::parse(&row.adapter_id).map_err(|_| RepositoryError::InvalidIdentifier)?;
    let path = VaultPath::parse(&row.path).map_err(|_| RepositoryError::InvalidPath)?;
    if path.as_str() != row.path {
        return Err(RepositoryError::InvalidPath);
    }
    validate_state_version(row.state_format_version)?;
    RevisionId::parse(&row.last_applied_revision_id)
        .map_err(|_| RepositoryError::InvalidIdentifier)?;
    let kind = WorktreeStateKind::parse(&row.state_kind)?;

    match (kind, row.content_sha256.as_deref()) {
        (WorktreeStateKind::Present, Some(hash)) => validate_canonical_sha256(hash)?,
        (WorktreeStateKind::Tombstoned, None) => {}
        _ => return Err(RepositoryError::InvalidWorktreeStateKind),
    }

    match (
        row.observation_schema_version,
        row.observed_size_bytes,
        row.observed_mtime,
    ) {
        (None, None, None) => {}
        (Some(version), Some(size), _) if kind == WorktreeStateKind::Present && size >= 0 => {
            validate_reconciliation_version(version)?;
        }
        _ => return Err(RepositoryError::InvalidWorktreeObservation),
    }

    Ok(row)
}

fn validate_state_version(version: i32) -> RepositoryResult<()> {
    if version != WORKTREE_STATE_FORMAT_VERSION {
        return Err(RepositoryError::UnsupportedWorktreeStateVersion);
    }
    Ok(())
}

fn validate_reconciliation_version(version: i32) -> RepositoryResult<()> {
    if version != WORKTREE_RECONCILIATION_FORMAT_VERSION {
        return Err(RepositoryError::UnsupportedWorktreeStateVersion);
    }
    Ok(())
}

fn validate_canonical_sha256(value: &str) -> RepositoryResult<()> {
    let parsed = Sha256::parse(value).map_err(|_| RepositoryError::InvalidHash)?;
    if parsed.to_string() != value {
        return Err(RepositoryError::InvalidHash);
    }
    Ok(())
}

#[cfg(test)]
mod tests;

#[cfg(all(test, feature = "test-support"))]
mod postgres_tests;
