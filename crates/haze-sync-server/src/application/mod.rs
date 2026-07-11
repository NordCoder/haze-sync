//! Reusable asynchronous Server application services.
//!
//! HTTP routes and future built-in adapter executors consume this boundary instead
//! of duplicating transaction, locking, idempotency, object-store, and repository
//! choreography. Core remains the policy owner and Storage remains passive.

mod changes;
mod deletes;
mod files;
mod idempotency;

pub(crate) use changes::{
    AuthoritativeChange, AuthoritativeChangeBatch, AuthoritativeChangesQuery,
};
pub(crate) use deletes::{ApplyDeleteCommand, ApplyDeleteOutcome};
pub(crate) use files::{
    ApplyFileCommand, ApplyFileOutcome, AuthoritativeRevisionContent, RevisionContentQuery,
};
pub(crate) use idempotency::{
    delete_request_fingerprint, derive_worktree_delete_idempotency,
    derive_worktree_put_idempotency, file_request_fingerprint, ApplicationIdempotency,
};

use haze_sync_common::AdapterId;
use haze_sync_storage::LocalObjectStore;
use sqlx::PgPool;
use std::fmt;

/// Internal actor identity used by reusable application services.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ApplicationActor {
    adapter_id: AdapterId,
}

impl ApplicationActor {
    #[must_use]
    pub(crate) const fn new(adapter_id: AdapterId) -> Self {
        Self { adapter_id }
    }

    #[must_use]
    pub(crate) const fn adapter_id(&self) -> &AdapterId {
        &self.adapter_id
    }
}

/// Stable safe failures from Server application services.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ApplicationError {
    /// Required database/object-store dependencies are not configured.
    DependenciesUnavailable,
    /// Requested authoritative file/revision metadata does not exist.
    NotFound,
    /// Request conflicts with current authoritative state.
    Conflict,
    /// Incoming bytes do not match the declared content hash.
    InvalidContentHash,
    /// One idempotency key was reused for materially different input.
    IdempotencyMismatch,
    /// Stored authoritative blob metadata exists but bytes are unavailable.
    ContentUnavailable,
    /// Stored bytes or metadata are inconsistent with authoritative metadata.
    ContentCorrupt,
    /// Internal persistence, conversion, or policy orchestration failed safely.
    Internal,
}

impl ApplicationError {
    #[must_use]
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::DependenciesUnavailable => "application_dependencies_unavailable",
            Self::NotFound => "application_not_found",
            Self::Conflict => "application_conflict",
            Self::InvalidContentHash => "application_invalid_content_hash",
            Self::IdempotencyMismatch => "application_idempotency_mismatch",
            Self::ContentUnavailable => "application_content_unavailable",
            Self::ContentCorrupt => "application_content_corrupt",
            Self::Internal => "application_internal_failure",
        }
    }

    const fn message(self) -> &'static str {
        match self {
            Self::DependenciesUnavailable => "application dependencies are unavailable",
            Self::NotFound => "authoritative file or revision was not found",
            Self::Conflict => "application command conflicts with current state",
            Self::InvalidContentHash => "application content hash is invalid",
            Self::IdempotencyMismatch => "idempotency key was reused for different input",
            Self::ContentUnavailable => "authoritative content is unavailable",
            Self::ContentCorrupt => "authoritative content metadata is inconsistent",
            Self::Internal => "application operation failed",
        }
    }
}

impl fmt::Display for ApplicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message())
    }
}

impl std::error::Error for ApplicationError {}

/// Cloneable explicit runtime dependencies for application operations.
#[derive(Clone)]
pub(crate) struct ServerApplicationServices {
    pool: PgPool,
    object_store: LocalObjectStore,
}

impl ServerApplicationServices {
    #[must_use]
    pub(crate) fn new(pool: PgPool, object_store: LocalObjectStore) -> Self {
        Self { pool, object_store }
    }

    pub(crate) async fn apply_file(
        &self,
        command: ApplyFileCommand,
    ) -> Result<ApplyFileOutcome, ApplicationError> {
        files::apply_file(&self.pool, &self.object_store, command).await
    }

    pub(crate) async fn apply_delete(
        &self,
        command: ApplyDeleteCommand,
    ) -> Result<ApplyDeleteOutcome, ApplicationError> {
        deletes::apply_delete(&self.pool, command).await
    }

    pub(crate) async fn authoritative_changes(
        &self,
        query: AuthoritativeChangesQuery,
    ) -> Result<AuthoritativeChangeBatch, ApplicationError> {
        changes::authoritative_changes(&self.pool, query).await
    }

    pub(crate) async fn revision_content(
        &self,
        query: RevisionContentQuery,
    ) -> Result<AuthoritativeRevisionContent, ApplicationError> {
        files::revision_content(&self.pool, &self.object_store, query).await
    }
}

impl fmt::Debug for ServerApplicationServices {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ServerApplicationServices")
            .field("pool", &"[REDACTED]")
            .field("object_store", &"[REDACTED]")
            .finish()
    }
}

pub(crate) fn deterministic_identifier(prefix: &str, parts: &[&str]) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update([0]);
    }
    let digest = hasher.finalize();
    format!("{prefix}{}", lower_hex(&digest[..16]))
}

pub(crate) fn lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}
