# Component Contract: storage

## Responsibility

`haze-sync-storage` owns durable storage implementation boundaries for Haze Sync.

The component is responsible for:

- storage schema metadata and migration filename ordering;
- passive database row models matching the Haze Sync metadata schema;
- SQLx repository helpers over caller-owned executors/transactions;
- safe repository error classification that hides raw SQLx/database details;
- content-addressed local object-store primitives;
- transaction-scoped PostgreSQL advisory locks for normalized vault paths;
- gated test support for storage/integration harnesses.

Storage persists and retrieves facts. It does not decide sync policy.

Core owns overwrite/conflict/delete/idempotency semantics. Server owns runtime composition and transaction orchestration. API owns public HTTP DTOs/errors. Adapters own provider/filesystem/plugin behavior.

## Public interfaces

The public crate surface is exported from `src/lib.rs`.

Current public modules:

```text
locks
models
object_store
repositories
schema

test_support   cfg(test) or feature = "test-support"
```

Current public re-exports:

```text
LocalObjectStore
ObjectMetadata
ObjectStore
ObjectStoreError
ObjectStoreResult
RepositoryError
RepositoryResult
```

Current owned surfaces:

- `schema::table_names` constants for metadata tables;
- `schema::INITIAL_MIGRATIONS` ordered migration filenames;
- passive row structs for all initial storage tables;
- repository helper modules for adapters/cursors, conflicts, content blobs, idempotency, objects, operation log, revisions, and tombstones;
- repository error codes/messages that do not expose raw DB internals;
- object-store trait and local filesystem-backed object store;
- deterministic `PathLockKey` derivation and `lock_vault_path` helper.

## Input contracts

Storage inputs must already be validated, scoped, and policy-decided by upstream components unless a storage helper explicitly validates a narrow storage-level constraint.

Required input rules:

- vault paths should use `haze-sync-common::VaultPath` at public helper boundaries wherever practical;
- content hashes should use `haze-sync-common::ContentHash` wherever practical;
- Core semantic decisions must be supplied by callers; Storage must not infer whether a write should be accepted, conflicted, tombstoned, or rejected;
- SQLx repository helpers execute against caller-owned executors or transactions;
- callers own database pool lifecycle, transaction boundaries, retries, request auth, idempotency request selection, and per-route orchestration;
- object-store writes must include expected content hash and raw bytes;
- local object-store roots are caller-configured and must not be exposed in public errors;
- advisory lock helpers require a caller-owned transaction-scoped PostgreSQL executor.

Storage may validate storage-level constraints such as non-negative sequences, page-limit bounds, representable `size_bytes`, known operation-kind/status strings, and cursor monotonicity.

## Output contracts

Storage outputs are internal/service-layer Rust values, not direct public API responses.

Required output rules:

- repository helpers return row/value models or safe `RepositoryError` values;
- object-store helpers return verified `ObjectMetadata` or safe `ObjectStoreError` values;
- object-store paths and local absolute paths must not appear in public error formatting;
- database errors are mapped to safe storage error variants before leaving repository helpers;
- repository outputs must preserve Core decisions transactionally when used by Server fan-in;
- cursor, idempotency, conflict, tombstone, and operation-log rows must not expose raw provider cursor JSON, idempotency keys, token hashes, or unsafe runtime details to public callers without a sanitizing API/Server layer.

Storage row models may contain raw persisted values because they represent database rows. Those row models must not be returned directly as public API/admin/status output unless sanitized by API/Server.

## Error contracts

Storage errors must be safe across component boundaries.

`RepositoryError` must not expose:

- raw SQL;
- SQLx error messages;
- database URLs;
- credentials;
- bearer tokens;
- OAuth tokens;
- token hashes;
- Idempotency-Key values;
- provider payloads;
- raw external cursors;
- local absolute paths;
- stack traces;
- file bytes.

`ObjectStoreError` formatting must not expose local absolute paths or temporary file paths.

Higher-level mapping into HTTP status codes or CLI output belongs to Server/API/CLI, not Storage.

## Persistence/runtime ownership

Storage owns persistence implementation primitives, but not application runtime orchestration.

Storage owns:

- SQLx query helper code inside repository modules;
- table/row model shapes;
- object-store filesystem implementation;
- transaction-scoped advisory-lock helper;
- storage test support behind `test-support` or `cfg(test)`.

Storage does not own:

- Axum route handlers or route registration;
- server startup, runtime state, config loading, or background jobs;
- Core decision policy;
- adapter/provider calls;
- Google Drive OAuth or Drive API calls;
- Obsidian plugin behavior;
- Worktree scanner/watcher/materializer loops;
- CLI command parsing or UX;
- migration runner/execution unless explicitly scoped later;
- hard-delete cleanup jobs unless explicitly scoped later.

## Security and secrecy rules

- Do not commit secrets.
- Do not expose tokens or token hashes in public outputs.
- Do not expose local absolute paths or database URLs.
- Do not serialize raw provider payloads into public outputs unless explicitly allowed by a higher-level API/Server contract.
- Map raw SQLx errors to safe storage errors before returning them.
- Object-store errors must not include filesystem paths in `Display` output.
- Stored row models may contain sensitive persisted fields, but public surfaces must sanitize before rendering.
- Test fixtures must not use real tokens, real OAuth credentials, production DB URLs, or real provider payloads.

## Non-goals

Storage must not implement:

- Core overwrite/conflict/delete/idempotency policy;
- API HTTP DTO ownership or public error envelopes;
- Axum runtime wiring;
- server app state or dependency injection;
- adapter loops or provider calls;
- Google Drive mapping semantics beyond persisted row/repository support;
- Worktree scanner/materializer behavior beyond persisted state support;
- Obsidian plugin local behavior;
- CLI commands;
- physical retention cleanup or hard delete unless explicitly scoped;
- migration execution/runtime orchestration unless explicitly scoped;
- public status/doctor rendering without API/Server sanitization.

## Dependencies

See `dependency-map.md`.

## Dependents

See `dependency-map.md`.

## Invariants

Storage component invariants:

- Storage persists facts; Core decides safety policy.
- Repository helpers execute only caller-requested operations against caller-owned executors/transactions.
- Storage does not create database pools or own runtime lifecycle.
- Storage does not expose raw SQLx/database details outside safe error boundaries.
- Object-store paths derive from validated content hashes, not vault paths or provider payloads.
- Object-store reads verify content hash before returning bytes.
- Object-store writes verify expected hash before committing blobs.
- Advisory locks are transaction-scoped PostgreSQL locks; callers must provide transaction scope.
- Test support remains gated and must not be required by production code.

## Test obligations

Storage tests should cover:

- schema table-name and migration-order metadata;
- row model serialization/deserialization where used as contract/test fixtures;
- repository validation helpers for sequences, limits, sizes, statuses, cursor regression, and safe error mapping;
- repository SQL helpers using test database support where scoped;
- object-store put/get/stat/exists behavior;
- object-store hash mismatch, stored blob mismatch, missing blob, duplicate write, and path-free error formatting;
- deterministic advisory-lock key generation for normalized paths;
- lock helper behavior with PostgreSQL transactions where integration DB support is available;
- test-support utilities behind the correct feature/cfg gates.

Checks expected for Storage changes when shell or CI is available:

```bash
cargo fmt --check
cargo check -p haze-sync-storage
cargo test -p haze-sync-storage
cargo test -p haze-sync-storage --features test-support
```

## Contract change protocol

Request a contract change instead of silently broadening scope when implementation requires:

- adding Core policy decisions to Storage;
- adding Axum/Server route behavior;
- adding provider SDK calls;
- changing schema/table ownership or migration order after consumers depend on it;
- exposing raw SQLx errors or local filesystem paths;
- changing object-store path layout;
- adding direct GDrive adapter DB ownership beyond persisted row/repository support;
- adding migration runner/runtime lifecycle behavior;
- adding physical hard-delete/retention cleanup jobs.
