//! Passive, adapter-scoped durable Google Drive state repositories.
//!
//! Storage persists caller-confirmed provider/Core facts. It never calls Google,
//! chooses import/export/delete policy, owns scheduling, or starts hidden
//! transactions. Compare-and-commit mutations require a caller-owned PostgreSQL
//! transaction and one exact expected aggregate version.

use super::{map_sqlx_error, validate_limit, validate_sequence, RepositoryError, RepositoryResult};
use chrono::{DateTime, Utc};
use haze_sync_common::{AdapterId, OperationId, RevisionId, VaultPath};
use sqlx::{postgres::PgRow, Postgres, Row, Transaction};
use std::fmt;

pub const GDRIVE_STATE_FORMAT_VERSION: i32 = 1;
const MAX_CURSOR_LEN: usize = 8_192;
const MAX_IDENTIFIER_LEN: usize = 1_024;
const MAX_PROVIDER_TEXT_LEN: usize = 4_096;
const SHA256_HEX_LEN: usize = 64;
const MD5_HEX_LEN: usize = 32;

include!("gdrive_state/types.rs");
include!("gdrive_state/repository.rs");
include!("gdrive_state/validation.rs");

#[cfg(test)]
mod tests;

#[cfg(all(test, feature = "test-support"))]
mod postgres_tests;
