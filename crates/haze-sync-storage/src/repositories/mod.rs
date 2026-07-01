//! Storage repository helpers for caller-owned database connections.
//!
//! Repositories in this module do not create global pools, run migrations, start
//! server runtime behavior, or apply Core sync policy.

pub mod idempotency;

pub use idempotency::{
    check_or_store_idempotency_record, compare_request_fingerprint, insert_idempotency_record,
    read_idempotency_record, IdempotencyRecordInput, IdempotencyRepositoryError,
    IdempotencyRepositoryOutcome, IdempotencyRequestComparison, IdempotencyStoreOutcome,
};
