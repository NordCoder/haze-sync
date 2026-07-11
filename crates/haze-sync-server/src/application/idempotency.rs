use super::{
    lower_hex, ApplicationActor, ApplicationError, ApplyDeleteOutcome, ApplyFileOutcome,
};
use chrono::{DateTime, Utc};
use haze_sync_api::{
    dto::{
        common::ConflictPolicyDto,
        files::{
            DeleteFileResponse, DeleteRejectedReasonDto, FileIgnoredReasonDto, PutFileResponse,
        },
    },
    routes::{
        delete::{
            not_found_delete_response, stale_base_delete_response, tombstoned_delete_response,
            unsafe_delete_response,
        },
        files::{
            accepted_upload_response, conflict_saved_upload_response, ignored_same_content_response,
        },
    },
};
use haze_sync_common::{AdapterId, ConflictId, ContentHash, RevisionId, VaultPath};
use haze_sync_core::{
    conflict_service::ConflictPolicy,
    idempotency::{RequestFingerprint, StoredIdempotencyResponse},
};
use haze_sync_storage::repositories::idempotency::{
    compare_request_fingerprint, insert_idempotency_record, read_idempotency_record,
    IdempotencyRecordInput, IdempotencyRequestComparison, IdempotencyStoreOutcome,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Transaction};
use std::fmt;

/// Durable adapter-scoped idempotency metadata.
#[derive(Clone, Eq, PartialEq)]
pub(crate) struct ApplicationIdempotency {
    key: String,
    fingerprint: RequestFingerprint,
}

impl ApplicationIdempotency {
    #[must_use]
    pub(crate) fn new(key: impl Into<String>, fingerprint: RequestFingerprint) -> Self {
        Self {
            key: key.into(),
            fingerprint,
        }
    }

    pub(super) fn key(&self) -> &str {
        &self.key
    }

    pub(super) const fn fingerprint(&self) -> RequestFingerprint {
        self.fingerprint
    }

    #[cfg(test)]
    pub(crate) fn test_key(&self) -> &str {
        &self.key
    }
}

impl fmt::Debug for ApplicationIdempotency {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ApplicationIdempotency")
            .field("key", &"[REDACTED]")
            .field("fingerprint", &"[REDACTED]")
            .finish()
    }
}

#[must_use]
pub(crate) fn file_request_fingerprint(
    actor: &ApplicationActor,
    path: &VaultPath,
    base_revision_id: Option<&RevisionId>,
    content_hash: ContentHash,
    bytes: &[u8],
) -> RequestFingerprint {
    let body_hash = hash_bytes(bytes);
    RequestFingerprint::from_safe_json_metadata(&json!({
        "method": "PUT",
        "path": path.as_str(),
        "base_revision_id": base_revision_id.map(RevisionId::as_str).unwrap_or("null"),
        "content_sha256": content_hash.to_string(),
        "body_sha256": body_hash.to_string(),
        "adapter_id": actor.adapter_id(),
        "scope": "adapter",
    }))
}

#[must_use]
pub(crate) fn delete_request_fingerprint(
    actor: &ApplicationActor,
    path: &VaultPath,
    base_revision_id: Option<&RevisionId>,
    requested_delete_count: u32,
) -> RequestFingerprint {
    RequestFingerprint::from_safe_json_metadata(&json!({
        "method": "DELETE",
        "path": path.as_str(),
        "base_revision_id": base_revision_id.map(RevisionId::as_str).unwrap_or("null"),
        "requested_delete_count": requested_delete_count,
        "adapter_id": actor.adapter_id(),
        "scope": "adapter",
    }))
}

#[must_use]
pub(crate) fn derive_worktree_put_idempotency(
    adapter_id: &AdapterId,
    path: &VaultPath,
    base_revision_id: Option<&RevisionId>,
    content_hash: ContentHash,
    bytes: &[u8],
) -> ApplicationIdempotency {
    let actor = ApplicationActor::new(adapter_id.clone());
    let path_hash = normalized_path_hash(path);
    let base = base_revision_id.map(RevisionId::as_str).unwrap_or("null");
    let key = format!(
        "wt:v1:{}:put:{path_hash}:{base}:{}",
        adapter_id,
        content_hash
    );
    ApplicationIdempotency::new(
        key,
        file_request_fingerprint(&actor, path, base_revision_id, content_hash, bytes),
    )
}

#[must_use]
pub(crate) fn derive_worktree_delete_idempotency(
    adapter_id: &AdapterId,
    path: &VaultPath,
    base_revision_id: Option<&RevisionId>,
    requested_delete_count: u32,
) -> ApplicationIdempotency {
    let actor = ApplicationActor::new(adapter_id.clone());
    let path_hash = normalized_path_hash(path);
    let base = base_revision_id.map(RevisionId::as_str).unwrap_or("null");
    let key = format!("wt:v1:{}:delete:{path_hash}:{base}", adapter_id);
    ApplicationIdempotency::new(
        key,
        delete_request_fingerprint(&actor, path, base_revision_id, requested_delete_count),
    )
}

pub(super) async fn read_file_replay(
    pool: &PgPool,
    actor: &ApplicationActor,
    idempotency: &ApplicationIdempotency,
) -> Result<Option<ApplyFileOutcome>, ApplicationError> {
    let response = read_replay(pool, actor, idempotency).await?;
    response.map(decode_file_response).transpose()
}

pub(super) async fn store_file_outcome(
    transaction: &mut Transaction<'_, Postgres>,
    actor: &ApplicationActor,
    idempotency: &ApplicationIdempotency,
    outcome: &ApplyFileOutcome,
) -> Result<Option<ApplyFileOutcome>, ApplicationError> {
    let response = StoredIdempotencyResponse::json(200, serde_json::to_value(encode_file_outcome(outcome)).map_err(|_| ApplicationError::Internal)?)
        .map_err(|_| ApplicationError::Internal)?;
    store_outcome(transaction, actor, idempotency, response)
        .await?
        .map(decode_file_response)
        .transpose()
}

pub(super) async fn read_delete_replay(
    pool: &PgPool,
    actor: &ApplicationActor,
    idempotency: &ApplicationIdempotency,
) -> Result<Option<ApplyDeleteOutcome>, ApplicationError> {
    let response = read_replay(pool, actor, idempotency).await?;
    response.map(decode_delete_response).transpose()
}

pub(super) async fn store_delete_outcome(
    transaction: &mut Transaction<'_, Postgres>,
    actor: &ApplicationActor,
    idempotency: &ApplicationIdempotency,
    outcome: &ApplyDeleteOutcome,
) -> Result<Option<ApplyDeleteOutcome>, ApplicationError> {
    let response = StoredIdempotencyResponse::json(
        delete_status(outcome),
        serde_json::to_value(encode_delete_outcome(outcome)).map_err(|_| ApplicationError::Internal)?,
    )
    .map_err(|_| ApplicationError::Internal)?;
    store_outcome(transaction, actor, idempotency, response)
        .await?
        .map(decode_delete_response)
        .transpose()
}

async fn read_replay(
    pool: &PgPool,
    actor: &ApplicationActor,
    idempotency: &ApplicationIdempotency,
) -> Result<Option<StoredIdempotencyResponse>, ApplicationError> {
    let mut connection = pool.acquire().await.map_err(|_| ApplicationError::Internal)?;
    let Some(record) = read_idempotency_record(
        &mut connection,
        actor.adapter_id(),
        idempotency.key(),
    )
    .await
    .map_err(|_| ApplicationError::Internal)?
    else {
        return Ok(None);
    };
    match compare_request_fingerprint(&record, idempotency.fingerprint().as_sha256())
        .map_err(|_| ApplicationError::Internal)?
    {
        IdempotencyRequestComparison::SameRequest => serde_json::from_value(record.response_json)
            .map(Some)
            .map_err(|_| ApplicationError::Internal),
        IdempotencyRequestComparison::DifferentRequest => {
            Err(ApplicationError::IdempotencyMismatch)
        }
    }
}

async fn store_outcome(
    transaction: &mut Transaction<'_, Postgres>,
    actor: &ApplicationActor,
    idempotency: &ApplicationIdempotency,
    response: StoredIdempotencyResponse,
) -> Result<Option<StoredIdempotencyResponse>, ApplicationError> {
    let input = IdempotencyRecordInput::new(
        actor.adapter_id().clone(),
        idempotency.key(),
        *idempotency.fingerprint().as_sha256(),
        serde_json::to_value(response).map_err(|_| ApplicationError::Internal)?,
    )
    .map_err(|_| ApplicationError::Internal)?;
    match insert_idempotency_record(&mut **transaction, &input)
        .await
        .map_err(|_| ApplicationError::Internal)?
    {
        IdempotencyStoreOutcome::Stored { .. } => Ok(None),
        IdempotencyStoreOutcome::AlreadyExists { record } => {
            match compare_request_fingerprint(&record, idempotency.fingerprint().as_sha256())
                .map_err(|_| ApplicationError::Internal)?
            {
                IdempotencyRequestComparison::SameRequest => {
                    serde_json::from_value(record.response_json)
                        .map(Some)
                        .map_err(|_| ApplicationError::Internal)
                }
                IdempotencyRequestComparison::DifferentRequest => {
                    Err(ApplicationError::IdempotencyMismatch)
                }
            }
        }
    }
}

fn encode_file_outcome(outcome: &ApplyFileOutcome) -> PutFileResponse {
    match outcome {
        ApplyFileOutcome::Accepted {
            path,
            revision_id,
            seq,
            ..
        } => accepted_upload_response(path.clone(), revision_id.clone(), *seq),
        ApplyFileOutcome::SameContent { path, .. } => ignored_same_content_response(path.clone()),
        ApplyFileOutcome::ConflictSaved {
            path,
            conflict_id,
            materialized_path,
            policy_applied,
            seq,
        } => conflict_saved_upload_response(
            path.clone(),
            conflict_id.clone(),
            materialized_path.clone(),
            policy_to_dto(*policy_applied),
            *seq,
        ),
    }
}

fn decode_file_response(response: StoredIdempotencyResponse) -> Result<ApplyFileOutcome, ApplicationError> {
    let body: PutFileResponse = serde_json::from_value(response.body().clone())
        .map_err(|_| ApplicationError::Internal)?;
    match body {
        PutFileResponse::Accepted { path, revision_id, seq } => Ok(ApplyFileOutcome::Accepted {
            path: VaultPath::try_from(path).map_err(|_| ApplicationError::Internal)?,
            revision_id: RevisionId::try_from(revision_id).map_err(|_| ApplicationError::Internal)?,
            content_hash: None,
            seq,
        }),
        PutFileResponse::Ignored {
            reason: FileIgnoredReasonDto::SameContent,
            path,
        } => Ok(ApplyFileOutcome::SameContent {
            path: VaultPath::try_from(path).map_err(|_| ApplicationError::Internal)?,
            revision_id: None,
            content_hash: None,
        }),
        PutFileResponse::ConflictSaved {
            path,
            conflict_id,
            materialized_path,
            policy_applied,
            seq,
        } => Ok(ApplyFileOutcome::ConflictSaved {
            path: VaultPath::try_from(path).map_err(|_| ApplicationError::Internal)?,
            conflict_id: ConflictId::try_from(conflict_id).map_err(|_| ApplicationError::Internal)?,
            materialized_path: VaultPath::try_from(materialized_path)
                .map_err(|_| ApplicationError::Internal)?,
            policy_applied: policy_from_dto(policy_applied),
            seq,
        }),
        PutFileResponse::Rejected { .. } => Err(ApplicationError::Internal),
    }
}

fn encode_delete_outcome(outcome: &ApplyDeleteOutcome) -> DeleteFileResponse {
    match outcome {
        ApplyDeleteOutcome::Tombstoned {
            path,
            tombstone_id,
            seq,
            retention_until,
        } => tombstoned_delete_response(
            path.clone(),
            tombstone_id,
            *seq,
            retention_until.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        ),
        ApplyDeleteOutcome::NotFound { path } => not_found_delete_response(path.clone()),
        ApplyDeleteOutcome::StaleBase { path } => stale_base_delete_response(path.clone()),
        ApplyDeleteOutcome::GuardRejected { path } => unsafe_delete_response(path.clone()),
    }
}

fn decode_delete_response(
    response: StoredIdempotencyResponse,
) -> Result<ApplyDeleteOutcome, ApplicationError> {
    let body: DeleteFileResponse = serde_json::from_value(response.body().clone())
        .map_err(|_| ApplicationError::Internal)?;
    match body {
        DeleteFileResponse::Tombstoned {
            path,
            tombstone_id,
            seq,
            retention_until,
        } => Ok(ApplyDeleteOutcome::Tombstoned {
            path: VaultPath::try_from(path).map_err(|_| ApplicationError::Internal)?,
            tombstone_id: tombstone_id.as_str().to_owned(),
            seq,
            retention_until: DateTime::parse_from_rfc3339(retention_until.as_str())
                .map_err(|_| ApplicationError::Internal)?
                .with_timezone(&Utc),
        }),
        DeleteFileResponse::NotFound { path } => Ok(ApplyDeleteOutcome::NotFound {
            path: VaultPath::try_from(path).map_err(|_| ApplicationError::Internal)?,
        }),
        DeleteFileResponse::Rejected { reason, path } => {
            let path = VaultPath::try_from(path).map_err(|_| ApplicationError::Internal)?;
            match reason {
                DeleteRejectedReasonDto::StaleBaseRevision => {
                    Ok(ApplyDeleteOutcome::StaleBase { path })
                }
                DeleteRejectedReasonDto::UnsafeDelete => {
                    Ok(ApplyDeleteOutcome::GuardRejected { path })
                }
                DeleteRejectedReasonDto::IdempotencyConflict
                | DeleteRejectedReasonDto::ValidationError => Err(ApplicationError::Internal),
            }
        }
    }
}

const fn delete_status(outcome: &ApplyDeleteOutcome) -> u16 {
    match outcome {
        ApplyDeleteOutcome::Tombstoned { .. } => 200,
        ApplyDeleteOutcome::NotFound { .. } => 404,
        ApplyDeleteOutcome::StaleBase { .. } | ApplyDeleteOutcome::GuardRejected { .. } => 409,
    }
}

const fn policy_to_dto(policy: ConflictPolicy) -> ConflictPolicyDto {
    match policy {
        ConflictPolicy::PreserveBoth => ConflictPolicyDto::PreserveBoth,
        ConflictPolicy::CurrentWinsWithIncomingBackup => {
            ConflictPolicyDto::CurrentWinsWithIncomingBackup
        }
    }
}

const fn policy_from_dto(policy: ConflictPolicyDto) -> ConflictPolicy {
    match policy {
        ConflictPolicyDto::PreserveBoth => ConflictPolicy::PreserveBoth,
        ConflictPolicyDto::CurrentWinsWithIncomingBackup => {
            ConflictPolicy::CurrentWinsWithIncomingBackup
        }
    }
}

fn normalized_path_hash(path: &VaultPath) -> String {
    let digest = Sha256::digest(path.as_str().as_bytes());
    lower_hex(&digest)
}

fn hash_bytes(bytes: &[u8]) -> ContentHash {
    let digest = Sha256::digest(bytes);
    let mut hash = [0_u8; 32];
    hash.copy_from_slice(&digest);
    ContentHash::from_bytes(hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(byte: char) -> ContentHash {
        ContentHash::parse(&format!("sha256:{}", byte.to_string().repeat(64))).unwrap()
    }

    #[test]
    fn worktree_put_keys_are_stable_domain_separated_and_path_hashed() {
        let adapter = AdapterId::parse("worktree").unwrap();
        let path = VaultPath::parse("Notes/a.md").unwrap();
        let base = RevisionId::parse("rev_base").unwrap();
        let first = derive_worktree_put_idempotency(
            &adapter,
            &path,
            Some(&base),
            hash('a'),
            b"bytes",
        );
        let same = derive_worktree_put_idempotency(
            &adapter,
            &path,
            Some(&base),
            hash('a'),
            b"bytes",
        );
        let other_path = derive_worktree_put_idempotency(
            &adapter,
            &VaultPath::parse("Notes/b.md").unwrap(),
            Some(&base),
            hash('a'),
            b"bytes",
        );

        assert_eq!(first, same);
        assert_ne!(first, other_path);
        assert!(first.test_key().starts_with("wt:v1:worktree:put:"));
        assert!(!first.test_key().contains("Notes/a.md"));
        assert!(!format!("{first:?}").contains(first.test_key()));
    }

    #[test]
    fn worktree_delete_keys_do_not_alias_put_or_materially_different_bases() {
        let adapter = AdapterId::parse("worktree").unwrap();
        let path = VaultPath::parse("Notes/a.md").unwrap();
        let base = RevisionId::parse("rev_base").unwrap();
        let delete = derive_worktree_delete_idempotency(&adapter, &path, Some(&base), 1);
        let null_delete = derive_worktree_delete_idempotency(&adapter, &path, None, 1);
        let put = derive_worktree_put_idempotency(
            &adapter,
            &path,
            Some(&base),
            hash('a'),
            b"bytes",
        );

        assert_ne!(delete, null_delete);
        assert_ne!(delete, put);
        assert!(delete.test_key().contains(":delete:"));
        assert!(!delete.test_key().contains("Notes/a.md"));
    }
}
