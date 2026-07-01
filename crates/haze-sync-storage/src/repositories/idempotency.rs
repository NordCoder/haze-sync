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

    /// Adapter scope for this record.
    #[must_use]
    pub const fn adapter_id(&self) -> &AdapterId {
        &self.adapter_id
    }

    /// Validated idempotency key.
    #[must_use]
    pub fn idempotency_key(&self) -> &str {
        &self.idempotency_key
    }

    /// Canonical request fingerprint.
    #[must_use]
    pub const fn request_hash(&self) -> &Sha256 {
        &self.request_hash
    }

    /// Safe public response JSON snapshot.
    #[must_use]
    pub const fn response_json(&self) -> &Value {
        &self.response_json
    }
}

/// Fingerprint comparison result for an existing idempotency record.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IdempotencyRequestComparison {
    /// Stored request hash matches the incoming fingerprint.
    SameRequest,
    /// The idempotency key was reused for a different request fingerprint.
    DifferentRequest,
}

/// Outcome of inserting an idempotency record.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum IdempotencyStoreOutcome {
    /// Record was inserted by this call.
    Stored { record: IdempotencyRecordRow },
    /// A record for the same adapter and key already exists.
    AlreadyExists { record: IdempotencyRecordRow },
}

/// Outcome of checking or storing an idempotency record.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum IdempotencyRepositoryOutcome {
    /// No prior record existed, and the input was stored.
    NewRequest { record: IdempotencyRecordRow },
    /// Existing key and request fingerprint matched; replay the stored response.
    ReplaySameRequest { record: IdempotencyRecordRow },
    /// Existing key was reused for a different request fingerprint.
    ConflictDifferentRequest { record: IdempotencyRecordRow },
}

/// Safe storage errors for idempotency repository operations.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IdempotencyRepositoryError {
    /// The idempotency key is empty, too long, or unsafe as request metadata.
    InvalidKey,
    /// A stored request hash could not be parsed as SHA-256.
    InvalidStoredRequestHash,
    /// Database operation failed; details are intentionally not exposed.
    DatabaseOperationFailed,
    /// A duplicate insert race was detected but the winning row was not visible.
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

    let record = read_idempotency_record(
        connection,
        input.adapter_id(),
        input.idempotency_key(),
    )
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

/// Compares a stored request hash with an incoming request fingerprint.
pub fn compare_request_fingerprint(
    record: &IdempotencyRecordRow,
    incoming_request_hash: &Sha256,
) -> Result<IdempotencyRequestComparison, IdempotencyRepositoryError> {
    let stored_request_hash = Sha256::parse(&record.request_hash)
        .map_err(|_| IdempotencyRepositoryError::InvalidStoredRequestHash)?;

    if stored_request_hash == *incoming_request_hash {
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
    let created_at: DateTime<Utc> = row
        .try_get("created_at")
        .map_err(|_| IdempotencyRepositoryError::DatabaseOperationFailed)?;

    Ok(IdempotencyRecordRow {
        adapter_id: row
            .try_get("adapter_id")
            .map_err(|_| IdempotencyRepositoryError::DatabaseOperationFailed)?,
        idempotency_key: row
            .try_get("idempotency_key")
            .map_err(|_| IdempotencyRepositoryError::DatabaseOperationFailed)?,
        request_hash: row
            .try_get("request_hash")
            .map_err(|_| IdempotencyRepositoryError::DatabaseOperationFailed)?,
        response_json: row
            .try_get("response_json")
            .map_err(|_| IdempotencyRepositoryError::DatabaseOperationFailed)?,
        created_at,
    })
}

fn validate_idempotency_key(key: &str) -> Result<(), IdempotencyRepositoryError> {
    if key.is_empty()
        || key.len() > MAX_IDEMPOTENCY_KEY_LEN
        || key
            .as_bytes()
            .iter()
            .any(|byte| !matches!(*byte, b'!'..=b'~'))
    {
        return Err(IdempotencyRepositoryError::InvalidKey);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::DateTime;
    use serde_json::json;

    fn timestamp() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-07-01T00:00:00Z")
            .expect("fixture timestamp should parse")
            .with_timezone(&Utc)
    }

    fn hash(ch: &str) -> Sha256 {
        Sha256::parse(&format!("sha256:{}", ch.repeat(64))).expect("fixture hash should parse")
    }

    fn row(request_hash: Sha256) -> IdempotencyRecordRow {
        IdempotencyRecordRow {
            adapter_id: "iphone-anna".to_owned(),
            idempotency_key: "iphone:iphone-anna:op-001".to_owned(),
            request_hash: request_hash.to_string(),
            response_json: json!({ "status": "accepted", "seq": 12_381 }),
            created_at: timestamp(),
        }
    }

    #[test]
    fn compares_matching_and_different_request_fingerprints() {
        let stored_hash = hash("a");
        let matching = row(stored_hash);
        let different_hash = hash("b");

        assert_eq!(
            compare_request_fingerprint(&matching, &stored_hash).unwrap(),
            IdempotencyRequestComparison::SameRequest
        );
        assert_eq!(
            compare_request_fingerprint(&matching, &different_hash).unwrap(),
            IdempotencyRequestComparison::DifferentRequest
        );
    }

    #[test]
    fn invalid_stored_request_hash_is_safe_error() {
        let mut record = row(hash("a"));
        record.request_hash = "not-a-hash".to_owned();

        assert_eq!(
            compare_request_fingerprint(&record, &hash("a")).unwrap_err(),
            IdempotencyRepositoryError::InvalidStoredRequestHash
        );
    }

    #[test]
    fn record_input_rejects_invalid_keys() {
        let adapter_id = AdapterId::parse("iphone-anna").unwrap();
        let request_hash = hash("a");

        assert_eq!(
            IdempotencyRecordInput::new(adapter_id.clone(), "", request_hash, json!({})).unwrap_err(),
            IdempotencyRepositoryError::InvalidKey
        );
        assert_eq!(
            IdempotencyRecordInput::new(
                adapter_id.clone(),
                "contains space",
                request_hash,
                json!({})
            )
            .unwrap_err(),
            IdempotencyRepositoryError::InvalidKey
        );
        assert_eq!(
            IdempotencyRecordInput::new(adapter_id, "bad\0key", request_hash, json!({}))
                .unwrap_err(),
            IdempotencyRepositoryError::InvalidKey
        );
    }

    #[test]
    fn existing_record_outcome_preserves_replay_and_conflict_semantics() {
        let stored_hash = hash("a");
        let replay = outcome_from_existing_record(row(stored_hash), &stored_hash).unwrap();
        let conflict = outcome_from_existing_record(row(stored_hash), &hash("b")).unwrap();

        assert!(matches!(
            replay,
            IdempotencyRepositoryOutcome::ReplaySameRequest { .. }
        ));
        assert!(matches!(
            conflict,
            IdempotencyRepositoryOutcome::ConflictDifferentRequest { .. }
        ));
    }
}
