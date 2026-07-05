# Dependency Map: core

## Upstream dependencies

### Internal workspace dependencies

- `crates/haze-sync-common`
  - `VaultPath`
  - `AdapterId`
  - `RevisionId`
  - `OperationId`
  - `ConflictId`
  - `ContentHash`
  - `Sha256`
  - `ValidationError`
  - adapter role/mode primitives where downstream callers need consistent semantics

Core relies on `haze-sync-common` for shared value validation and public DTO-safe primitives. Core should not duplicate path, ID, hash, or adapter-role parsing rules that already belong to common.

### External crate dependencies

- `chrono`
  - UTC timestamps for revisions, conflicts, tombstones, doctor summaries, and operation-log metadata.
- `serde`
  - public serialization/deserialization for safe Core DTO/value models.
- `serde_json`
  - safe JSON metadata and stored idempotency response/fingerprint support.
- `sha2`
  - SHA-256 request/content fingerprinting where the pure Core algorithm computes hashes.

### Caller-provided runtime dependencies

Core defines traits and value models for dependencies that are implemented outside this component:

- revision repository lookups and inserts;
- content-store writes/metadata;
- operation-log append;
- storage transactions and locks;
- durable idempotency storage;
- doctor live checks;
- route extraction and HTTP response mapping;
- adapter/provider execution.

These are not direct Core dependencies and must not be silently introduced as concrete dependencies without a contract change.

## Downstream dependents

Expected downstream components:

- `crates/haze-sync-api`
  - maps Core decisions into route DTOs and safe HTTP errors;
  - must not reimplement conflict/delete policy.
- `crates/haze-sync-server`
  - wires runtime state, config, auth, DB pools, routes, readiness, and service composition;
  - owns server lifecycle and background runtime behavior, not Core.
- `crates/haze-sync-storage`
  - implements SQLx repositories, schema-backed persistence, transactions, locks, idempotency records, operation-log storage, tombstone rows, and conflict rows;
  - must preserve Core safety outcomes transactionally.
- `crates/haze-sync-cli`
  - may render Core doctor/status summaries and invoke server/storage operations;
  - must not expose secrets, token hashes, raw SQLx errors, or local absolute paths.
- `crates/haze-sync-worktree`
  - submits local filesystem facts to Core/API and obeys Core decisions;
  - owns scanning, materialization, trash, watcher, and echo-guard behavior outside Core.
- `crates/haze-gdrive-adapter`
  - submits Drive facts to Core/API and obeys Core decisions;
  - owns Drive API calls, OAuth/token loading, mapping, echo guard, import/export, and delete reconciliation outside Core.
- `apps/haze-obsidian-plugin`
  - uses Core API outcomes through the API client;
  - owns local vault scanning, UI, pending queue, and conflict center rendering outside Core.
- E2E and integration tests
  - may use Core public modules to assert safety behavior;
  - must avoid assuming side effects that Core does not own.

## Cross-component contracts

### Common → Core

- Common owns validation for shared paths, IDs, hashes, roles, modes, and validation errors.
- Core accepts common value types as trusted parsed inputs but may still validate Core-specific constraints such as conflict-area recursion or delete-run scope.

### Core → API

- Core returns semantic outcomes and safe errors.
- API owns request extraction, auth/role checks, HTTP status mapping, route registration, and response DTO shaping.
- API must preserve Core semantics, especially no silent overwrite, conflict preservation, tombstone/delete safety, and idempotency conflict behavior.

### Core → Storage

- Core defines decision metadata and trait boundaries.
- Storage owns SQLx models/repositories, migrations, connection pools, transactions, advisory locks, persisted idempotency records, operation-log rows, tombstone rows, and conflict rows.
- Storage must not expose raw SQLx errors through Core-facing or public outputs.

### Core → Server

- Server owns runtime wiring and lifecycle.
- Server may compose Core services with storage/API/auth/config, but Core must not open pools, run migrations, spawn tasks, or start HTTP routes.

### Core → Adapters

- Adapters submit observed facts: path, base revision, content hash, content bytes, delete candidates, cursors, and adapter identity.
- Core decides overwrite/conflict/delete safety through API/server/storage integration.
- Adapters must not silently overwrite and must not implement independent conflict/delete policy that contradicts Core.

### Core → CLI/doctor

- Core doctor helpers produce passive, redacted result models.
- CLI/server components that perform live checks must supply already-redacted facts and must honestly mark checks as skipped when live checks were not performed.

## Contract-change notes

No contract changes requested in T0-P1.

Any future proposal to add concrete SQLx, Axum, provider SDK, object-store filesystem, background job, or adapter runtime dependencies to `haze-sync-core` requires an explicit contract-change request and Orchestrator/Architect review.
