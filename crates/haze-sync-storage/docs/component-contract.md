# Component Contract: storage

## Responsibility

`haze-sync-storage` owns durable storage implementation boundaries for Haze Sync:

- schema and migration content/metadata;
- passive internal row models;
- SQLx repositories over caller-owned executors and transactions;
- safe repository error classification;
- verified content-addressed local object storage;
- transaction-scoped PostgreSQL advisory locks;
- versioned Worktree instance/path state and exact export checkpoints;
- gated test support.

Storage persists and retrieves caller-decided facts. Core owns
revision/conflict/delete/idempotency policy. Server owns runtime composition,
transactions and operation timing. API owns public DTOs/errors. Adapters own
provider/filesystem/plugin semantics.

## Public interfaces

Public modules:

```text
locks
models
object_store
repositories
schema

test_support   cfg(test) or feature = "test-support"
```

Public re-exports include the object-store surface plus `RepositoryError` and
`RepositoryResult`. Current owned repository modules cover cursors, conflicts,
content blobs, GDrive mapping, idempotency, objects, operation log, revisions,
tombstones and Worktree durable state.

`schema::INITIAL_MIGRATIONS` is the ordered current migration list. The historical
name remains source-compatible even though migration 0010 extends the initial
schema.

## Input contracts

- Public path boundaries use `VaultPath` where practical.
- Content/root fingerprints use Common SHA-256 types.
- Adapter/revision/operation/conflict identities use Common validated types.
- Callers supply Core/adapter decisions; Storage does not infer accepted,
  conflicted, tombstoned, replayed, imported, exported, dirty or repaired states.
- Callers own pools, transaction boundaries, retries, authorization,
  idempotency-key selection and runtime orchestration.
- Object-store writes include expected hash and bytes.
- Advisory-lock helpers require caller-owned transaction scope.
- Worktree root fingerprints are supplied for already-normalized roots. Storage
  never receives or derives the raw root.
- Worktree path-state writes are already-decided present/tombstoned facts.
- Exact Worktree cursor advancement is requested only after one authoritative
  operation is safely materialized inside the same caller-owned transaction.

Storage may validate narrow persistence constraints: canonical values, nonnegative
sequences/sizes, bounded limits, known vocabularies, supported state versions,
instance-binding equality, path-state consistency and cursor transition shape.

## Output contracts

Storage outputs are internal/service-layer values, not direct API/status DTOs.

- Database and object-store failures cross the boundary only as safe errors.
- Raw SQLx errors, local paths and database URLs are never rendered.
- Raw external cursors, idempotency keys, token hashes, provider identifiers,
  Worktree root fingerprints and operational metadata require higher-layer
  sanitization.
- `AdapterCursorSummary` excludes raw external cursor JSON.
- Worktree instance rows/bindings use redacted `Debug`; root fingerprints are not
  serde/public-status surfaces.
- Repository outputs preserve caller decisions transactionally when composed by
  Server.

## Worktree durable-state contract

Storage owns:

- `worktree_instances`, keyed by stable Worktree `adapter_id`;
- canonical non-public root fingerprint and `state_format_version = 1`;
- create-or-verify binding with fail-closed root/version mismatch;
- `worktree_state`, keyed by `(adapter_id, path)`;
- explicit `present` or `tombstoned` kind;
- required authoritative revision;
- required content hash only for present state;
- optional versioned, bounded reconciliation observation fields;
- deterministic adapter-scoped path-ordered snapshot pagination;
- guarded observation updates bound to expected revision and content hash;
- no hard-delete repository operation;
- transaction-only lock/read and exact-contiguous cursor advancement.

The strict cursor transition is `persisted == expected` and
`next == expected + 1`. Missing cursors, negative values, equality/regression,
gaps, stale expected values and overflow fail safely. The compatibility broad
monotonic cursor API remains for other adapters; Worktree export uses the strict
API.

Server must persist Worktree path state and strict cursor advancement in one
transaction. Rollback leaves both unchanged. Storage does not materialize files or
claim that filesystem work succeeded.

See `worktree-durable-state.md`.

## Migration contract

Migration 0010 replaces the legacy path-only Worktree table only when it is
empty. Non-empty legacy rows fail before destructive SQL because Storage cannot
invent adapter/root identity. PostgreSQL migration atomicity preserves the old
schema/data on failure.

Production migration execution belongs to Server/Deployment. Storage owns the
migration content and test-support validation for fresh, exact pre-P10 and exact
current schemas.

## Error contract

`RepositoryError`, `ObjectStoreError` and `TestSupportError` must not expose:

- SQL or SQLx/driver details;
- database URLs or credentials;
- bearer/OAuth/token hashes or idempotency keys;
- provider payloads or raw cursors;
- raw Worktree roots or root fingerprints;
- local absolute/temporary paths;
- stack traces or file bytes.

Higher-level HTTP/CLI mapping belongs outside Storage.

## Persistence/runtime ownership

Storage owns query helpers, migrations, row shapes, object-store implementation,
advisory-lock primitives and gated test support. It does not own:

- Axum routes, startup, config loading, background jobs or hosted loops;
- Core policy;
- provider/OAuth calls;
- Worktree root normalization, scanning, watching, reconciliation semantics,
  materialization, echo/trash/repair behavior or scheduling;
- public status/doctor rendering;
- production migration invocation;
- retention cleanup or physical hard delete.

## Security and secrecy

- Never commit secrets or real production configuration.
- Store only the Worktree root fingerprint, never the raw root.
- Keep Storage rows internal and sanitize before public rendering.
- Map raw database/filesystem errors before returning.
- Object-store errors remain path-free.
- Test fixtures use dedicated test databases and synthetic credentials/data.
- Production crates must not enable `test-support` in normal dependencies.

## Invariants

- Storage persists facts; Core/Worktree decide semantics.
- Repositories use caller-owned executors/transactions and create no pools.
- Object-store layout derives only from content hashes and verifies reads/writes.
- Advisory path locks are transaction-scoped.
- Worktree binding is stable per adapter and fails closed on mismatch/version.
- Worktree path state is adapter-scoped and versioned.
- A tombstoned path has no content hash or reconciliation observation.
- A present path has a canonical content hash.
- Observation updates cannot overwrite a newer revision/hash.
- Worktree export checkpoint never advances by more than one or beyond a stale
  expected value.
- Test support remains gated and production-independent.

## Test obligations

Storage tests cover:

- migration ordering and current/pre-P10 schema metadata;
- migration refusal for non-empty legacy Worktree rows;
- row validation and redaction;
- instance bind replay/mismatch and per-adapter isolation;
- present/tombstoned state, observations, pagination and rollback;
- exact cursor regression/gap/stale/missing/overflow and race behavior;
- existing repository validation, object store and advisory locks;
- test-support configuration/schema/cleanup safety.

Expected checks:

```bash
cargo fmt --check
cargo check -p haze-sync-storage
cargo test -p haze-sync-storage
cargo test -p haze-sync-storage --features test-support
HAZE_SYNC_TEST_DATABASE_URL='<dedicated test DB>' \
  cargo test -p haze-sync-storage --features test-support -- --ignored
```

The last command is mandatory STOR-P10 PostgreSQL evidence; ordinary Component CI
without PostgreSQL cannot substitute for it.

## Contract-change protocol

Stop and request the owner phase rather than silently broadening scope when work
requires Core policy, Server/application/runtime behavior, provider SDKs,
Worktree filesystem semantics, API DTOs, raw-secret/path exposure, object-store
layout changes, production migration execution or physical cleanup jobs.
