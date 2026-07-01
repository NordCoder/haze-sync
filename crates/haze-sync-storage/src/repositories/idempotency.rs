//! PostgreSQL repository helpers for idempotency records.
//!
//! The repository stores only adapter-scoped idempotency keys, SHA-256 request
//! fingerprints, and safe JSON response snapshots. It does not store raw request
//! bodies, file bytes, secrets, provider payloads, database URLs, or runtime
//! details.

use crate::{models::IdempotencyRecordRow, schema::table_names};
use chrono::{DateTime, Utc};
use haze_sync_common::{AdapterId, Sha256};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{postgres::PgRow, PgConnection, Row};
use std::{error::Error, fmt};

const MAX_IDEMPOTENCY_KEY_LEN: usize = 512;

/// New idempotency record ready to insert into `idempotency_records`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IdempotencyRecordInput {
    adapter_id: AdapterId,
    idempotency_key: String,
    request_hash: Sha256,
    response_json: Value,
}

impl IdempotencyRecordInput {
    /// Creates a new record input from safe idempotency metadata.
    pub fn new(
        adapter_id: AdapterId,
        idempotency_key: impl Into<String>,
        request_hash: Sha256,
        response_json: Value,
    ) -> Result<Self, IdempotencyRepositoryError> {
        let idempotency_key = idempotency_key.into();
        validate_idempotency_key(&idempotency_key)?;

        Ok(Self {
            adapter_id,
            idempotency_key,
            request_hash,
            response_json,
        })
    }

    #[must_use]
    pub const fn adapter_id(&self) -> &AdapterId {
        &self.adapter_id
    }

    #[must_use]
    pub fn idempotency_key(&self) -> &str {
        &self.idempotency_key
    }

    #[must_use]
    pub const fn request_hash(&self) -> &Sha256 {
        &self.request_hash
    }

    #[must_use]
    pub const fn response_json(&self) -> &Value {
        &self.response_json
    }
}

/// Fingerprint comparison result for an existing idempotency record.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IdempotencyRequestComparison {
    SameRequest,
    DifferentRequest,
}

/// Outcome of inserting an idempotency record.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum IdempotencyStoreOutcome {
    Stored { record: IdempotencyRecordRow },
    AlreadyExists { record: IdempotencyRecordRow },
}

/// Outcome of checking or storing an idempotency record.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum IdempotencyRepositoryOutcome {
    NewRequest { record: IdempotencyRecordRow },
    ReplaySameRequest { record: IdempotencyRecordRow },
    ConflictDifferentRequest { record: IdempotencyRecordRow },
}

/// Safe storage errors for idempotency repository operations.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IdempotencyRepositoryError {
    InvalidKey,
    InvalidStoredRequestHash,
    DatabaseOperationFailed,
    DuplicateInsertRace,
}

impl IdempotencyRepositoryError {
    /// Stable machine-readable error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidKey => "invalid_idempotency_key",
            Self::InvalidStoredRequestHash => "invalid_stored_request_hash",
            Self::DatabaseOperationFailed => "idempotency_database_operation_failed",
            Self::DuplicateInsertRace => "idempotency_duplicate_insert_race",
        }
    }

    /// Stable human-readable message with no secrets or database details.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::InvalidKey => "idempotency key is invalid",
            Self::InvalidStoredRequestHash => "stored request fingerprint is invalid",
            Self::DatabaseOperationFailed => "idempotency database operation failed",
            Self::DuplicateInsertRace => "idempotency record insert raced with another writer",
        }
    }
}

impl fmt::Display for IdempotencyRepositoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message())
    }
}

impl Error for IdempotencyRepositoryError {}

/// Reads one idempotency record by adapter scope and idempotency key.
pub async fn read_idempotency_record(
    connection: &mut PgConnection,
    adapter_id: &AdapterId,
    idempotency_key: &str,
) -> Result<Option<IdempotencyRecordRow>, IdempotencyRepositoryError> {
    validate_idempotency_key(idempotency_key)?;

    let sql = format!(
        "select adapter_id, idempotency_key, request_hash, response_json, created_at from {table} where adapter_id = $1 and idempotency_key = $2",
        table = table_names::IDEMPOTENCY_RECORDS
    );

    let row = sqlx::query(&sql)
        .bind(adapter_id.as_str())
        .bind(idempotency_key)
        .fetch_optional(&mut *connection)
        .await
        .map_err(|_| IdempotencyRepositoryError::DatabaseOperationFailed)?;

    row.map(record_from_row).transpose()
}

/// Inserts an idempotency record and returns the inserted or pre-existing row.
pub async fn insert_idempotency_record(
    connection: &mut PgConnection,
    input: &IdempotencyRecordInput,
) -> Result<IdempotencyStoreOutcome, IdempotencyRepositoryError> {
    let sql = format!(
        "insert into {table} (adapter_id, idempotency_key, request_hash, response_json) values ($1, $2, $3, $4) on conflict (adapter_id, idempotency_key) do nothing returning adapter_id, idempotency_key, request_hash, response_json, created_at",
        table = table_names::IDEMPOTENCY_RECORDS
    );

    let inserted = sqlx::query(&sql)
        .bind(input.adapter_id().as_str())
        .bind(input.idempotency_key())
        .bind(input.request_hash().to_string())
        .bind(input.response_json().clone())
        .fetch_optional(&mut *connection)
        .await
        .map_err(|_| IdempotencyRepositoryError::DatabaseOperationFailed)?;

    if let Some(row) = inserted {
        return Ok(IdempotencyStoreOutcome::Stored {
            record: record_from_row(row)?,
        });
    }

    let record = read_idempotency_record(connection, input.adapter_id(), input.idempotency_key())
        .await?
        .ok_or(IdempotencyRepositoryError::DuplicateInsertRace)?;

    Ok(IdempotencyStoreOutcome::AlreadyExists { record })
}

/// Checks an incoming idempotent request, storing it when no prior record exists.
pub async fn check_or_store_idempotency_record(
    connection: &mut PgConnection,
    input: &IdempotencyRecordInput,
) -> Result<IdempotencyRepositoryOutcome, IdempotencyRepositoryError> {
    if let Some(record) = read_idempotency_record(
        connection,
        input.adapter_id(),
        input.idempotency_key(),
    )
    .await?
    {
        return outcome_from_existing_record(record, input.request_hash());
    }

    match insert_idempotency_record(connection, input).await? {
        IdempotencyStoreOutcome::Stored { record } => {
            Ok(IdempotencyRepositoryOutcome::NewRequest { record })
        }
        IdempotencyStoreOutcome::AlreadyExists { record } => {
            outcome_from_existing_record(record, input.request_hash())
        }
    }
}

/// Compares a stored row request hash with an incoming request hash.
pub fn compare_request_fingerprint(
    record: &IdempotencyRecordRow,
    incoming_request_hash: &Sha256,
) -> Result<IdempotencyRequestComparison, IdempotencyRepositoryError> {
    let stored = Sha256::parse(&record.request_hash)
        .map_err(|_| IdempotencyRepositoryError::InvalidStoredRequestHash)?;

    if &stored == incoming_request_hash {
        Ok(IdempotencyRequestComparison::SameRequest)
    } else {
        Ok(IdempotencyRequestComparison::DifferentRequest)
    }
}

fn outcome_from_existing_record(
    record: IdempotencyRecordRow,
    incoming_request_hash: &Sha256,
) -> Result<IdempotencyRepositoryOutcome, IdempotencyRepositoryError> {
    match compare_request_fingerprint(&record, incoming_request_hash)? {
        IdempotencyRequestComparison::SameRequest => {
            Ok(IdempotencyRepositoryOutcome::ReplaySameRequest { record })
        }
        IdempotencyRequestComparison::DifferentRequest => {
            Ok(IdempotencyRepositoryOutcome::ConflictDifferentRequest { record })
        }
    }
}

fn record_from_row(row: PgRow) -> Result<IdempotencyRecordRow, IdempotencyRepositoryError> {
    let request_hash: String = row
        .try_get("request_hash")
        .map_err(|_| IdempotencyRepositoryError::DatabaseOperationFailed)?;
    Sha256::parse(&request_hash)
        .map_err(|_| IdempotencyRepositoryError::InvalidStoredRequestHash)?;

    Ok(IdempotencyRecordRow {
        adapter_id: row
            .try_get("adapter_id")
            .map_err(|_| IdempotencyRepositoryError::DatabaseOperationFailed)?,
        idempotency_key: row
            .try_get("idempotency_key")
            .map_err(|_| IdempotencyRepositoryError::DatabaseOperationFailed)?,
        request_hash,
        response_json: row
            .try_get("response_json")
            .map_err(|_| IdempotencyRepositoryError::DatabaseOperationFailed)?,
        created_at: row
            .try_get::<DateTime<Utc>, _>("created_at")
            .map_err(|_| IdempotencyRepositoryError::DatabaseOperationFailed)?,
    })
}

fn validate_idempotency_key(key: &str) -> Result<(), IdempotencyRepositoryError> {
    if key.is_empty()
        || key.len() > MAX_IDEMPOTENCY_KEY_LEN
        || key.as_bytes().iter().any(|byte| !matches!(*byte, b'!'..=b'~'))
    {
        return Err(IdempotencyRepositoryError::InvalidKey);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use serde_json::json;

    fn row(request_hash: &Sha256) -> IdempotencyRecordRow {
        IdempotencyRecordRow {
            adapter_id: "iphone-anna".to_owned(),
            idempotency_key: "iphone:op-1".to_owned(),
            request_hash: request_hash.to_string(),
            response_json: json!({ "status": "accepted" }),
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
        }
    }

    #[test]
    fn compare_request_fingerprint_detects_same_request() {
        let hash = Sha256::parse(&"a".repeat(64)).unwrap();
        assert_eq!(
            compare_request_fingerprint(&row(&hash), &hash).unwrap(),
            IdempotencyRequestComparison::SameRequest
        );
    }

    #[test]
    fn compare_request_fingerprint_detects_different_request() {
        let stored = Sha256::parse(&"a".repeat(64)).unwrap();
        let incoming = Sha256::parse(&"b".repeat(64)).unwrap();
        assert_eq!(
            compare_request_fingerprint(&row(&stored), &incoming).unwrap(),
            IdempotencyRequestComparison::DifferentRequest
        );
    }

    #[test]
    fn invalid_key_is_rejected() {
        assert_eq!(
            IdempotencyRecordInput::new(
                AdapterId::parse("iphone-anna").unwrap(),
                "contains space",
                Sha256::parse(&"a".repeat(64)).unwrap(),
                json!({}),
            )
            .unwrap_err(),
            IdempotencyRepositoryError::InvalidKey
        );
    }
}
