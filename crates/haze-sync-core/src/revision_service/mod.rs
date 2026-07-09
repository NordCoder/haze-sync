//! Pure service-layer foundation for normal file revision upserts.
//!
//! The service is intentionally storage-agnostic. Callers own the repository,
//! content store, and operation-log implementations, so the same algorithm can
//! later be executed inside a database transaction without this crate creating
//! hidden pools, running migrations, or wiring server runtime state.

use haze_sync_common::{AdapterId, ContentHash, OperationId, RevisionId, VaultPath};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256 as Sha256Hasher};
use std::error::Error;
use std::fmt;

/// Request accepted by the normal file upsert algorithm.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpsertFileRequest {
    /// Vault-relative, already validated path.
    pub path: VaultPath,
    /// Adapter principal submitting the write.
    pub adapter_id: AdapterId,
    /// Explicit base revision. `None` represents explicit `base_revision_id = null`.
    pub base_revision_id: Option<RevisionId>,
    /// SHA-256 the caller expects for `content`.
    pub expected_hash: ContentHash,
    /// Raw file bytes for the initial pure implementation.
    pub content: Vec<u8>,
    /// Placeholder for future idempotency integration. This phase does not
    /// implement idempotency lookup, comparison, storage, or replay behavior.
    pub idempotency_key: Option<String>,
}

impl UpsertFileRequest {
    /// Build an upsert request from validated value types and raw bytes.
    #[must_use]
    pub fn new(
        path: VaultPath,
        adapter_id: AdapterId,
        base_revision_id: Option<RevisionId>,
        expected_hash: ContentHash,
        content: Vec<u8>,
    ) -> Self {
        Self {
            path,
            adapter_id,
            base_revision_id,
            expected_hash,
            content,
            idempotency_key: None,
        }
    }

    /// Attach a future idempotency key placeholder without enabling idempotency behavior.
    #[must_use]
    pub fn with_idempotency_key(mut self, idempotency_key: impl Into<String>) -> Self {
        self.idempotency_key = Some(idempotency_key.into());
        self
    }
}

/// Immutable file revision snapshot used at the Core service boundary.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StoredRevision {
    pub revision_id: RevisionId,
    pub path: VaultPath,
    pub parent_revision_id: Option<RevisionId>,
    pub content_hash: ContentHash,
    pub size_bytes: u64,
    pub created_by: AdapterId,
}

/// Content blob metadata returned by the content-store boundary.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StoredContent {
    pub hash: ContentHash,
    pub size_bytes: u64,
}

/// Minimal conflict policy marker exposed by W3-P2 for later fan-in wiring.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictPolicyHint {
    /// Preserve current content and save incoming content for a future conflict copy.
    PreserveBoth,
}

/// Incoming content preserved after a stale, unknown, or explicit-null base conflict.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IncomingConflictContent {
    /// Original vault path submitted by the adapter.
    pub path: VaultPath,
    /// Adapter that submitted the incoming content.
    pub adapter_id: AdapterId,
    /// Verified incoming content hash.
    pub content_hash: ContentHash,
    /// Verified incoming content size.
    pub size_bytes: u64,
    /// Raw incoming bytes retained in-memory for future fan-in materialization.
    ///
    /// This intentionally skips serde so public JSON serialization does not leak file content.
    #[serde(skip, default)]
    pub content: Vec<u8>,
}

/// Safe Core conflict-saved planning data for later fan-in phases.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConflictSavedOutcome {
    /// Current revision that must remain current until explicit conflict resolution.
    pub current_revision: StoredRevision,
    /// Base revision provided by the caller, or null when the caller explicitly had no base.
    pub provided_base_revision_id: Option<RevisionId>,
    /// Incoming content retained without overwriting or writing the current revision.
    pub incoming_content: IncomingConflictContent,
    /// Policy the future conflict fan-in should materialize.
    pub policy_hint: ConflictPolicyHint,
}

/// Metadata required to insert an accepted normal revision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InsertRevisionRequest {
    pub path: VaultPath,
    pub adapter_id: AdapterId,
    pub parent_revision_id: Option<RevisionId>,
    pub content_hash: ContentHash,
    pub size_bytes: u64,
}

/// Operation-log entry returned after a successful append.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OperationLogEntry {
    pub operation_id: OperationId,
    pub seq: i64,
}

/// Operation-log append request for accepted file upserts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppendOperationRequest {
    pub adapter_id: AdapterId,
    pub kind: OperationKind,
    pub path: VaultPath,
    pub revision_id: RevisionId,
}

/// Operation kinds needed by the normal revision service.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationKind {
    UpsertFile,
}

/// Safe result returned by the normal upsert algorithm.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum UpsertOutcome {
    AcceptedNewFile {
        revision: StoredRevision,
        operation: OperationLogEntry,
    },
    AcceptedNewRevision {
        revision: StoredRevision,
        operation: OperationLogEntry,
    },
    IgnoredDuplicateSameContent {
        current_revision: StoredRevision,
    },
    RejectedHashMismatch {
        expected: ContentHash,
        actual: ContentHash,
    },
    /// Stale/unknown-base safety outcome.
    ///
    /// The Rust variant name is retained for existing W2 route-source
    /// compatibility. When `conflict_saved` is `Some`, the Core outcome is a
    /// conflict_saved result and the current revision was not overwritten.
    /// When `conflict_saved` is `None`, there was no current file to preserve and
    /// the non-null base was rejected safely without creating an unexpected file.
    RejectedStaleOrUnknownBase {
        current_revision: Option<StoredRevision>,
        provided_base_revision_id: Option<RevisionId>,
        #[serde(skip_serializing_if = "Option::is_none")]
        conflict_saved: Option<Box<ConflictSavedOutcome>>,
    },
}

impl UpsertOutcome {
    /// Return conflict_saved planning data when this outcome preserved incoming content.
    #[must_use]
    pub fn conflict_saved(&self) -> Option<&ConflictSavedOutcome> {
        match self {
            Self::RejectedStaleOrUnknownBase { conflict_saved, .. } => conflict_saved.as_deref(),
            _ => None,
        }
    }

    /// Public semantic status for callers that need to distinguish safe rejects from conflicts.
    #[must_use]
    pub fn public_status(&self) -> &'static str {
        match self {
            Self::AcceptedNewFile { .. } => "accepted_new_file",
            Self::AcceptedNewRevision { .. } => "accepted_new_revision",
            Self::IgnoredDuplicateSameContent { .. } => "same_content",
            Self::RejectedHashMismatch { .. } => "rejected_hash_mismatch",
            Self::RejectedStaleOrUnknownBase {
                conflict_saved: Some(_),
                ..
            } => "conflict_saved",
            Self::RejectedStaleOrUnknownBase {
                conflict_saved: None,
                ..
            } => "rejected_stale_or_unknown_base",
        }
    }
}

/// Caller-owned repository boundary for current revision lookup and revision insertion.
pub trait RevisionRepository {
    fn current_revision(
        &mut self,
        path: &VaultPath,
    ) -> Result<Option<StoredRevision>, RevisionServiceError>;

    fn insert_revision(
        &mut self,
        request: InsertRevisionRequest,
    ) -> Result<StoredRevision, RevisionServiceError>;
}

/// Caller-owned content store boundary.
pub trait ContentStore {
    fn put_content(
        &mut self,
        expected_hash: ContentHash,
        bytes: &[u8],
    ) -> Result<StoredContent, RevisionServiceError>;
}

/// Caller-owned operation-log append boundary.
pub trait OperationLog {
    fn append_operation(
        &mut self,
        request: AppendOperationRequest,
    ) -> Result<OperationLogEntry, RevisionServiceError>;
}

/// Pure normal-upsert service over caller-owned storage boundaries.
#[derive(Debug)]
pub struct RevisionService<R, C, L> {
    repository: R,
    content_store: C,
    operation_log: L,
}

impl<R, C, L> RevisionService<R, C, L> {
    /// Build a service from caller-owned dependencies.
    #[must_use]
    pub const fn new(repository: R, content_store: C, operation_log: L) -> Self {
        Self {
            repository,
            content_store,
            operation_log,
        }
    }

    /// Return owned dependencies, useful for pure tests and future composition.
    #[must_use]
    pub fn into_inner(self) -> (R, C, L) {
        (self.repository, self.content_store, self.operation_log)
    }
}

impl<R, C, L> RevisionService<R, C, L>
where
    R: RevisionRepository,
    C: ContentStore,
    L: OperationLog,
{
    /// Apply normal file upsert rules with W3 stale/unknown-base conflict preservation.
    pub fn upsert_file(
        &mut self,
        request: UpsertFileRequest,
    ) -> Result<UpsertOutcome, RevisionServiceError> {
        let actual_hash = compute_content_hash(&request.content);
        if actual_hash != request.expected_hash {
            return Ok(UpsertOutcome::RejectedHashMismatch {
                expected: request.expected_hash,
                actual: actual_hash,
            });
        }

        let current_revision = self.repository.current_revision(&request.path)?;

        match current_revision {
            None => self.upsert_missing_file(request),
            Some(current_revision) => self.upsert_existing_file(request, current_revision),
        }
    }

    fn upsert_missing_file(
        &mut self,
        request: UpsertFileRequest,
    ) -> Result<UpsertOutcome, RevisionServiceError> {
        if request.base_revision_id.is_some() {
            return Ok(UpsertOutcome::RejectedStaleOrUnknownBase {
                current_revision: None,
                provided_base_revision_id: request.base_revision_id,
                conflict_saved: None,
            });
        }

        let revision = self.store_and_insert_revision(request, None)?;
        let operation = self.append_upsert_operation(&revision)?;

        Ok(UpsertOutcome::AcceptedNewFile {
            revision,
            operation,
        })
    }

    fn upsert_existing_file(
        &mut self,
        request: UpsertFileRequest,
        current_revision: StoredRevision,
    ) -> Result<UpsertOutcome, RevisionServiceError> {
        if request.expected_hash == current_revision.content_hash {
            return Ok(UpsertOutcome::IgnoredDuplicateSameContent { current_revision });
        }

        let current_revision_id = current_revision.revision_id.clone();
        if request.base_revision_id.as_ref() != Some(&current_revision_id) {
            return Self::save_incoming_conflict(request, current_revision);
        }

        let revision = self.store_and_insert_revision(request, Some(current_revision_id))?;
        let operation = self.append_upsert_operation(&revision)?;

        Ok(UpsertOutcome::AcceptedNewRevision {
            revision,
            operation,
        })
    }

    fn save_incoming_conflict(
        request: UpsertFileRequest,
        current_revision: StoredRevision,
    ) -> Result<UpsertOutcome, RevisionServiceError> {
        let size_bytes = u64::try_from(request.content.len())
            .map_err(|_error| RevisionServiceError::content_store("content_size"))?;
        let provided_base_revision_id = request.base_revision_id;
        let conflict_saved = ConflictSavedOutcome {
            current_revision: current_revision.clone(),
            provided_base_revision_id: provided_base_revision_id.clone(),
            incoming_content: IncomingConflictContent {
                path: request.path,
                adapter_id: request.adapter_id,
                content_hash: request.expected_hash,
                size_bytes,
                content: request.content,
            },
            policy_hint: ConflictPolicyHint::PreserveBoth,
        };

        Ok(UpsertOutcome::RejectedStaleOrUnknownBase {
            current_revision: Some(current_revision),
            provided_base_revision_id,
            conflict_saved: Some(Box::new(conflict_saved)),
        })
    }

    fn store_and_insert_revision(
        &mut self,
        request: UpsertFileRequest,
        parent_revision_id: Option<RevisionId>,
    ) -> Result<StoredRevision, RevisionServiceError> {
        let stored_content = self.store_content(request.expected_hash, &request.content)?;

        self.repository.insert_revision(InsertRevisionRequest {
            path: request.path,
            adapter_id: request.adapter_id,
            parent_revision_id,
            content_hash: stored_content.hash,
            size_bytes: stored_content.size_bytes,
        })
    }

    fn store_content(
        &mut self,
        expected_hash: ContentHash,
        bytes: &[u8],
    ) -> Result<StoredContent, RevisionServiceError> {
        let stored_content = self.content_store.put_content(expected_hash, bytes)?;
        if stored_content.hash != expected_hash {
            return Err(RevisionServiceError::content_store(
                "put_content_returned_unexpected_hash",
            ));
        }

        Ok(stored_content)
    }

    fn append_upsert_operation(
        &mut self,
        revision: &StoredRevision,
    ) -> Result<OperationLogEntry, RevisionServiceError> {
        self.operation_log.append_operation(AppendOperationRequest {
            adapter_id: revision.created_by.clone(),
            kind: OperationKind::UpsertFile,
            path: revision.path.clone(),
            revision_id: revision.revision_id.clone(),
        })
    }
}

/// Compute the canonical SHA-256 content hash used by Core verification.
#[must_use]
pub fn compute_content_hash(bytes: &[u8]) -> ContentHash {
    let digest = Sha256Hasher::digest(bytes);
    let mut hash_bytes = [0_u8; 32];
    hash_bytes.copy_from_slice(&digest);
    ContentHash::from_bytes(hash_bytes)
}

/// Safe, path-free service boundary errors for dependency failures.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RevisionServiceError {
    Repository { operation: &'static str },
    ContentStore { operation: &'static str },
    OperationLog { operation: &'static str },
}

impl RevisionServiceError {
    #[must_use]
    pub const fn repository(operation: &'static str) -> Self {
        Self::Repository { operation }
    }

    #[must_use]
    pub const fn content_store(operation: &'static str) -> Self {
        Self::ContentStore { operation }
    }

    #[must_use]
    pub const fn operation_log(operation: &'static str) -> Self {
        Self::OperationLog { operation }
    }

    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Repository { .. } => "revision_repository_error",
            Self::ContentStore { .. } => "content_store_error",
            Self::OperationLog { .. } => "operation_log_error",
        }
    }
}

impl fmt::Display for RevisionServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Repository { .. } => "revision repository operation failed",
            Self::ContentStore { .. } => "content store operation failed",
            Self::OperationLog { .. } => "operation log operation failed",
        })
    }
}

impl Error for RevisionServiceError {}

#[cfg(test)]
mod tests {
    use super::*;

    const CURRENT_BYTES: &[u8] = b"current content";
    const INCOMING_BYTES: &[u8] = b"incoming content";

    #[derive(Debug)]
    struct RecordingRepository {
        current_revision: Option<StoredRevision>,
        current_revision_calls: usize,
        inserted_requests: Vec<InsertRevisionRequest>,
        next_revision_id: RevisionId,
    }

    impl RecordingRepository {
        fn with_current(current_revision: Option<StoredRevision>) -> Self {
            Self {
                current_revision,
                current_revision_calls: 0,
                inserted_requests: Vec::new(),
                next_revision_id: revision_id("rev_01JNEW"),
            }
        }
    }

    impl RevisionRepository for RecordingRepository {
        fn current_revision(
            &mut self,
            _path: &VaultPath,
        ) -> Result<Option<StoredRevision>, RevisionServiceError> {
            self.current_revision_calls += 1;
            Ok(self.current_revision.clone())
        }

        fn insert_revision(
            &mut self,
            request: InsertRevisionRequest,
        ) -> Result<StoredRevision, RevisionServiceError> {
            self.inserted_requests.push(request.clone());
            Ok(StoredRevision {
                revision_id: self.next_revision_id.clone(),
                path: request.path,
                parent_revision_id: request.parent_revision_id,
                content_hash: request.content_hash,
                size_bytes: request.size_bytes,
                created_by: request.adapter_id,
            })
        }
    }

    #[derive(Debug, Default)]
    struct RecordingContentStore {
        put_calls: Vec<(ContentHash, Vec<u8>)>,
    }

    impl ContentStore for RecordingContentStore {
        fn put_content(
            &mut self,
            expected_hash: ContentHash,
            bytes: &[u8],
        ) -> Result<StoredContent, RevisionServiceError> {
            self.put_calls.push((expected_hash, bytes.to_vec()));
            Ok(StoredContent {
                hash: expected_hash,
                size_bytes: bytes.len() as u64,
            })
        }
    }

    #[derive(Debug, Default)]
    struct RecordingOperationLog {
        append_calls: Vec<AppendOperationRequest>,
    }

    impl OperationLog for RecordingOperationLog {
        fn append_operation(
            &mut self,
            request: AppendOperationRequest,
        ) -> Result<OperationLogEntry, RevisionServiceError> {
            self.append_calls.push(request);
            Ok(OperationLogEntry {
                operation_id: OperationId::parse("op_01JTEST")
                    .expect("fixture operation id should parse"),
                seq: self.append_calls.len() as i64,
            })
        }
    }

    fn adapter_id() -> AdapterId {
        AdapterId::parse("gdrive-adapter").expect("fixture adapter id should parse")
    }

    fn revision_id(value: &str) -> RevisionId {
        RevisionId::parse(value).expect("fixture revision id should parse")
    }

    fn vault_path() -> VaultPath {
        VaultPath::parse("Notes/a.md").expect("fixture path should parse")
    }

    fn current_revision() -> StoredRevision {
        StoredRevision {
            revision_id: revision_id("rev_01JCURRENT"),
            path: vault_path(),
            parent_revision_id: None,
            content_hash: compute_content_hash(CURRENT_BYTES),
            size_bytes: CURRENT_BYTES.len() as u64,
            created_by: adapter_id(),
        }
    }

    fn request(base_revision_id: Option<RevisionId>, content: &[u8]) -> UpsertFileRequest {
        UpsertFileRequest::new(
            vault_path(),
            adapter_id(),
            base_revision_id,
            compute_content_hash(content),
            content.to_vec(),
        )
    }

    fn service_with_current(
        current_revision: Option<StoredRevision>,
    ) -> RevisionService<RecordingRepository, RecordingContentStore, RecordingOperationLog> {
        RevisionService::new(
            RecordingRepository::with_current(current_revision),
            RecordingContentStore::default(),
            RecordingOperationLog::default(),
        )
    }

    fn assert_no_accepted_write_side_effects(
        repository: &RecordingRepository,
        content_store: &RecordingContentStore,
        operation_log: &RecordingOperationLog,
    ) {
        assert!(repository.inserted_requests.is_empty());
        assert!(content_store.put_calls.is_empty());
        assert!(operation_log.append_calls.is_empty());
    }

    #[test]
    fn file_missing_base_null_accepts_new_file_and_records_insert_and_operation() {
        let incoming_hash = compute_content_hash(INCOMING_BYTES);
        let mut service = service_with_current(None);

        let outcome = service
            .upsert_file(request(None, INCOMING_BYTES))
            .expect("upsert should not fail");

        let UpsertOutcome::AcceptedNewFile {
            revision,
            operation,
        } = outcome
        else {
            panic!("expected accepted new file outcome");
        };
        assert_eq!(revision.revision_id, revision_id("rev_01JNEW"));
        assert_eq!(revision.parent_revision_id, None);
        assert_eq!(revision.content_hash, incoming_hash);
        assert_eq!(revision.size_bytes, INCOMING_BYTES.len() as u64);
        assert_eq!(operation.seq, 1);

        let (repository, content_store, operation_log) = service.into_inner();
        assert_eq!(repository.current_revision_calls, 1);
        assert_eq!(repository.inserted_requests.len(), 1);
        assert_eq!(repository.inserted_requests[0].parent_revision_id, None);
        assert_eq!(repository.inserted_requests[0].content_hash, incoming_hash);
        assert_eq!(content_store.put_calls, vec![(incoming_hash, INCOMING_BYTES.to_vec())]);
        assert_eq!(operation_log.append_calls.len(), 1);
        assert_eq!(operation_log.append_calls[0].kind, OperationKind::UpsertFile);
        assert_eq!(operation_log.append_calls[0].revision_id, revision_id("rev_01JNEW"));
    }

    #[test]
    fn file_missing_non_null_base_rejects_without_accepted_side_effects() {
        let old_base = revision_id("rev_01JOLD");
        let mut service = service_with_current(None);

        let outcome = service
            .upsert_file(request(Some(old_base.clone()), INCOMING_BYTES))
            .expect("upsert should not fail");

        let UpsertOutcome::RejectedStaleOrUnknownBase {
            current_revision,
            provided_base_revision_id,
            conflict_saved,
        } = outcome
        else {
            panic!("expected stale or unknown base rejection");
        };
        assert!(current_revision.is_none());
        assert_eq!(provided_base_revision_id, Some(old_base));
        assert!(conflict_saved.is_none());

        let (repository, content_store, operation_log) = service.into_inner();
        assert_eq!(repository.current_revision_calls, 1);
        assert_no_accepted_write_side_effects(&repository, &content_store, &operation_log);
    }

    #[test]
    fn existing_file_current_base_different_content_accepts_new_revision() {
        let current = current_revision();
        let current_revision_id = current.revision_id.clone();
        let incoming_hash = compute_content_hash(INCOMING_BYTES);
        let mut service = service_with_current(Some(current));

        let outcome = service
            .upsert_file(request(Some(current_revision_id.clone()), INCOMING_BYTES))
            .expect("upsert should not fail");

        let UpsertOutcome::AcceptedNewRevision {
            revision,
            operation,
        } = outcome
        else {
            panic!("expected accepted new revision outcome");
        };
        assert_eq!(revision.revision_id, revision_id("rev_01JNEW"));
        assert_eq!(revision.parent_revision_id, Some(current_revision_id.clone()));
        assert_eq!(revision.content_hash, incoming_hash);
        assert_eq!(operation.seq, 1);

        let (repository, content_store, operation_log) = service.into_inner();
        assert_eq!(repository.current_revision_calls, 1);
        assert_eq!(repository.inserted_requests.len(), 1);
        assert_eq!(
            repository.inserted_requests[0].parent_revision_id,
            Some(current_revision_id)
        );
        assert_eq!(repository.inserted_requests[0].content_hash, incoming_hash);
        assert_eq!(content_store.put_calls, vec![(incoming_hash, INCOMING_BYTES.to_vec())]);
        assert_eq!(operation_log.append_calls.len(), 1);
        assert_eq!(operation_log.append_calls[0].revision_id, revision_id("rev_01JNEW"));
    }

    #[test]
    fn existing_file_same_content_ignores_old_and_null_base_without_side_effects() {
        for base_revision_id in [None, Some(revision_id("rev_01JOLD"))] {
            let current = current_revision();
            let mut service = service_with_current(Some(current.clone()));

            let outcome = service
                .upsert_file(request(base_revision_id.clone(), CURRENT_BYTES))
                .expect("upsert should not fail");

            let UpsertOutcome::IgnoredDuplicateSameContent { current_revision } = outcome else {
                panic!("expected same-content outcome");
            };
            assert_eq!(current_revision, current);
            assert_eq!(outcome_public_status(&current_revision), "same_content");

            let (repository, content_store, operation_log) = service.into_inner();
            assert_eq!(repository.current_revision_calls, 1);
            assert_no_accepted_write_side_effects(&repository, &content_store, &operation_log);
        }
    }

    #[test]
    fn existing_file_stale_base_different_content_returns_conflict_saved_without_side_effects() {
        let old_base = revision_id("rev_01JOLD");
        let current = current_revision();
        let incoming_hash = compute_content_hash(INCOMING_BYTES);
        let mut service = service_with_current(Some(current.clone()));

        let outcome = service
            .upsert_file(request(Some(old_base.clone()), INCOMING_BYTES))
            .expect("upsert should not fail");

        assert_eq!(outcome.public_status(), "conflict_saved");
        let UpsertOutcome::RejectedStaleOrUnknownBase {
            current_revision,
            provided_base_revision_id,
            conflict_saved,
        } = outcome
        else {
            panic!("expected conflict_saved stale-base outcome");
        };
        assert_eq!(current_revision, Some(current.clone()));
        assert_eq!(provided_base_revision_id, Some(old_base.clone()));
        let conflict_saved = conflict_saved.expect("stale base should preserve incoming content");
        assert_eq!(conflict_saved.current_revision, current);
        assert_eq!(conflict_saved.provided_base_revision_id, Some(old_base));
        assert_eq!(conflict_saved.incoming_content.content_hash, incoming_hash);
        assert_eq!(conflict_saved.incoming_content.size_bytes, INCOMING_BYTES.len() as u64);
        assert_eq!(conflict_saved.incoming_content.content, INCOMING_BYTES);
        assert_eq!(conflict_saved.policy_hint, ConflictPolicyHint::PreserveBoth);

        let (repository, content_store, operation_log) = service.into_inner();
        assert_eq!(repository.current_revision_calls, 1);
        assert_no_accepted_write_side_effects(&repository, &content_store, &operation_log);
    }

    #[test]
    fn existing_file_null_base_different_content_returns_conflict_saved_without_side_effects() {
        let current = current_revision();
        let incoming_hash = compute_content_hash(INCOMING_BYTES);
        let mut service = service_with_current(Some(current.clone()));

        let outcome = service
            .upsert_file(request(None, INCOMING_BYTES))
            .expect("upsert should not fail");

        assert_eq!(outcome.public_status(), "conflict_saved");
        let conflict_saved = outcome
            .conflict_saved()
            .expect("null base with existing different content should save conflict");
        assert_eq!(conflict_saved.current_revision, current);
        assert_eq!(conflict_saved.provided_base_revision_id, None);
        assert_eq!(conflict_saved.incoming_content.content_hash, incoming_hash);
        assert_eq!(conflict_saved.incoming_content.content, INCOMING_BYTES);

        let (repository, content_store, operation_log) = service.into_inner();
        assert_eq!(repository.current_revision_calls, 1);
        assert_no_accepted_write_side_effects(&repository, &content_store, &operation_log);
    }

    #[test]
    fn hash_mismatch_rejects_before_repository_lookup_or_side_effects() {
        let expected = compute_content_hash(b"declared content");
        let actual = compute_content_hash(INCOMING_BYTES);
        let mut request = request(Some(revision_id("rev_01JCURRENT")), INCOMING_BYTES);
        request.expected_hash = expected;
        let mut service = service_with_current(Some(current_revision()));

        let outcome = service
            .upsert_file(request)
            .expect("hash mismatch should be a safe outcome");

        let UpsertOutcome::RejectedHashMismatch {
            expected: reported_expected,
            actual: reported_actual,
        } = outcome
        else {
            panic!("expected hash mismatch outcome");
        };
        assert_eq!(reported_expected, expected);
        assert_eq!(reported_actual, actual);

        let (repository, content_store, operation_log) = service.into_inner();
        assert_eq!(repository.current_revision_calls, 0);
        assert_no_accepted_write_side_effects(&repository, &content_store, &operation_log);
    }

    #[test]
    fn conflict_saved_serialization_skips_incoming_content_bytes() {
        let current_revision = current_revision();
        let incoming_bytes = b"secret provider bytes never public".to_vec();
        let incoming_hash = compute_content_hash(&incoming_bytes);
        let outcome = UpsertOutcome::RejectedStaleOrUnknownBase {
            current_revision: Some(current_revision.clone()),
            provided_base_revision_id: Some(revision_id("rev_01JOLD")),
            conflict_saved: Some(Box::new(ConflictSavedOutcome {
                current_revision,
                provided_base_revision_id: Some(revision_id("rev_01JOLD")),
                incoming_content: IncomingConflictContent {
                    path: vault_path(),
                    adapter_id: adapter_id(),
                    content_hash: incoming_hash,
                    size_bytes: incoming_bytes.len() as u64,
                    content: incoming_bytes,
                },
                policy_hint: ConflictPolicyHint::PreserveBoth,
            })),
        };

        assert_eq!(outcome.public_status(), "conflict_saved");

        let serialized = serde_json::to_string(&outcome).expect("upsert outcome should serialize");
        assert!(serialized.contains("conflict_saved"));
        assert!(!serialized.contains("secret provider bytes"));
        assert!(!serialized.contains("\"content\":"));

        let decoded: UpsertOutcome =
            serde_json::from_str(&serialized).expect("upsert outcome should deserialize");
        let decoded_conflict = decoded
            .conflict_saved()
            .expect("conflict_saved should survive serde roundtrip");
        assert_eq!(
            decoded_conflict.incoming_content.content_hash,
            incoming_hash
        );
        assert!(decoded_conflict.incoming_content.content.is_empty());
    }

    fn outcome_public_status(current_revision: &StoredRevision) -> &'static str {
        UpsertOutcome::IgnoredDuplicateSameContent {
            current_revision: current_revision.clone(),
        }
        .public_status()
    }
}
