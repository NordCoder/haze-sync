# Recovery and Mutation Contracts — Scope and Compatibility

Status: normative V1 contract part  
Contract version: `haze-sync.recovery-and-mutation.v1`  
Authority: this file is normative only as part of the ordered contract set listed in `../recovery-and-mutation-contracts.md`.

## 1. Normative language and scope

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHOULD**, **SHOULD NOT**, and **MAY** are normative.

This document defines behavior and ownership boundaries. It does not authorize executable changes by itself.

The contract covers:

- coordinated backup and restore of PostgreSQL, the Haze Sync object store, and an optional Worktree recovery artifact;
- safe upgrade and rollback orchestration;
- the atomic mutation required to promote preserved conflict content through `accept_conflict`;
- the evidence required for later recovery acceptance and failure-injection work.

The contract does not redefine ordinary revision, conflict creation, tombstone, delete-retention, or metadata-only conflict-resolution semantics.

## 2. Compatibility baseline

Implementations MUST preserve these accepted boundaries:

- Core is the only revision/conflict/delete policy owner.
- Server owns application-service composition, SQL transaction choreography, path locking, object-store coordination, idempotency persistence, operation-log append, and safe API mapping.
- Storage owns passive repositories, migrations, caller-transaction-scoped persistence helpers, advisory-lock primitives, and verified content-addressed object-store primitives.
- Deployment owns host-level service ordering, migration execution, backup destinations, restore targets, release activation, and recovery-helper privileges.
- CLI is an authenticated operator client. It MUST use accepted public Server/operation surfaces and MUST NOT compensate for missing contracts through direct infrastructure access.
- `accept_current`, `keep_both`, and `mark_resolved` remain metadata-only with respect to the authoritative file revision.
- `accept_conflict` creates one new authoritative revision from the already preserved incoming conflict content. The previous current revision remains immutable history and becomes the parent of the new revision.
- Existing V1 changes-feed vocabulary remains valid. Conflict resolution continues to append `conflict_resolved`; this contract does not require a new changes-feed operation kind.
- Object-store blobs are immutable, content-addressed by SHA-256, verified before use, and are not authoritative merely because bytes exist.
- Recovery operations are exact specializations of the current Stage 10 operational-control contract: they use `haze-sync.operational-job.v1`, `haze-sync.audit-event.v1`, namespace-specific CAS tokens, inventory-complete quiescence evidence, executor fencing/leases, execution slots, and operational idempotency. This document MUST NOT be implemented as a parallel job, idempotency, confirmation, audit, lease, or concurrency subsystem.
- Ordinary product-data restore MUST preserve the live operational control plane. It MUST NOT overwrite the current `maintenance_generation`, adapter inventory/control generations, adapter desired/effective state, principals, credentials, operational jobs, executor fences/leases, execution slots, operational idempotency records, or audit events.
- `accept_conflict` effects MUST be derived from the current Core planner. The new revision preserves incoming-content provenance through `created_by = conflict.incoming_adapter_id`; the resolving principal is recorded separately as resolver/audit actor; and the conflict-copy disposition is `MarkConflictCopyResolved`.

Materially relevant accepted surfaces include:

- `crates/haze-sync-core/docs/component-contract.md`;
- `crates/haze-sync-core/src/conflict_service/mod.rs`;
- `crates/haze-sync-core/docs/idempotency-operation-log-persistence.md`;
- `crates/haze-sync-storage/docs/component-contract.md`;
- `crates/haze-sync-storage/src/object_store/mod.rs`;
- `crates/haze-sync-server/docs/component-contract.md`;
- `crates/haze-sync-server/src/routes/conflicts_impl.rs`;
- `crates/haze-sync-api/src/dto/conflicts.rs`;
- `crates/haze-sync-cli/docs/component-contract.md`;
- `deploy/docs/migrations-backup-restore.md`.
