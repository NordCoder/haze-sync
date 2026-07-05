# Dependency Map: core

## Component role in dependency graph

`haze-sync-core` is the safety and decision layer of Haze Sync.

It sits above `haze-sync-common` primitives and below runtime/integration components. Core should expose semantic outcomes and pure policy decisions that downstream components can persist, map, render, or execute without reimplementing overwrite/conflict/delete safety rules.

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

Core relies on `haze-sync-common` for shared value validation and public DTO-safe primitives. Core should not duplicate path, ID, hash, or adapter-role parsing rules that already belong to Common.

### External crate dependencies

Allowed current external dependencies:

- `chrono`
  - UTC timestamps for revisions, conflicts, tombstones, doctor summaries, and operation-log metadata.
- `serde`
  - public serialization/deserialization for safe Core DTO/value models.
- `serde_json`
  - safe JSON metadata and stored idempotency response/fingerprint support.
- `sha2`
  - SHA-256 request/content fingerprinting where the pure Core algorithm computes hashes.

Additional external dependencies require contract review when they affect public behavior, determinism, runtime ownership, or safety semantics.

### Caller-provided runtime dependencies

Core defines traits and value models for dependencies that are implemented outside this component:

- revision repository lookups and inserts;
- content-store writes/metadata;
- operation-log append;
- storage transactions and locks;
- durable idempotency storage;
- conflict/tombstone repository persistence;
- doctor live checks;
- route extraction and HTTP response mapping;
- adapter/provider execution.

These are not direct Core dependencies and must not be silently introduced as concrete dependencies without a contract change.

## Disallowed direct dependencies

Core must not directly depend on:

```text
haze-sync-api
haze-sync-storage
haze-sync-server
haze-sync-worktree
haze-gdrive-adapter
haze-sync-cli
apps/haze-obsidian-plugin
axum
sqlx
tokio runtime ownership
reqwest/google provider SDKs
filesystem watcher crates
CLI parser crates
deployment/config loading as runtime behavior
```

Rationale:

```text
Core decisions must stay deterministic and side-effect-free. Runtime effects are
owned by Server, Storage, CLI, Worktree, GDrive, Obsidian, and deployment layers.
```

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
- Core accepts common value types as trusted parsed inputs but may still validate Core-specific constraints such as conflict-area recursion, delete-run scope, or request fingerprint rules.
- Common must not implement Core overwrite/conflict/delete policy.

### Core → API

- Core returns semantic outcomes and safe errors.
- API owns request extraction, auth/role checks, HTTP status mapping, route registration, and response DTO shaping.
- API must preserve Core semantics, especially no silent overwrite, conflict preservation, tombstone/delete safety, idempotency replay/conflict behavior, and safe public errors.
- API must not expose raw incoming conflict bytes, idempotency keys, raw file content, or storage/provider errors through Core mappings.

### Core → Storage

- Core defines decision metadata and trait boundaries.
- Storage owns SQLx models/repositories, migrations, connection pools, transactions, advisory locks, persisted idempotency records, operation-log rows, tombstone rows, and conflict rows.
- Storage must preserve Core safety outcomes transactionally.
- Storage must not expose raw SQLx errors through Core-facing or public outputs.

### Core → Server

- Server owns runtime wiring and lifecycle.
- Server may compose Core services with Storage/API/auth/config, but Core must not open pools, run migrations, spawn tasks, or start HTTP routes.
- Server must provide transaction/per-path-lock/idempotency boundaries when mapping live requests to Core decisions.

### Core → Worktree

- Worktree submits local file facts through API/Core-facing contracts.
- Worktree owns filesystem scan/materialization/trash/watcher/echo behavior.
- Worktree must not decide stale overwrite or mass-delete policy locally.

### Core → GDrive adapter

- GDrive adapter submits provider facts through API/Core-facing contracts.
- GDrive owns provider calls, OAuth/token loading, mapping semantics, echo guard, change feed, import/export, delete candidate detection, and reconciliation.
- GDrive must not silently overwrite or tombstone data based only on provider state.

### Core → Obsidian plugin

- Obsidian plugin observes Core outcomes through API responses.
- Plugin owns UI, local state, pending queue, sync planner, local scanner, and conflict UI.
- Plugin must not claim conflict/delete resolution that Core/API did not confirm.

### Core → CLI/doctor

- Core doctor helpers produce passive, redacted result models.
- CLI/server components that perform live checks must supply already-redacted facts and must honestly mark checks as skipped when live checks were not performed.
- CLI must not use Core models as permission to perform destructive operations without Server/Storage/delete-guard support.

## Integration/fan-in dependencies

The following work belongs to fan-in or downstream component phases, not Core leaf phases:

- route registration and HTTP status mapping;
- durable idempotency repository wiring;
- DB transaction/per-path lock enforcement;
- object-store temp write/fsync/rename behavior;
- conflict row and conflict-copy persistence;
- tombstone row persistence and adapter trash behavior;
- retention cleanup execution;
- provider import/export loops;
- Worktree runtime loop;
- Obsidian UI and pending queue;
- production E2E tests.

## Contract-change notes

Current known contract questions:

1. Conflict path and `VaultPath` compatibility
   - Core generates conflict materialization paths under `_haze_conflicts/open/**`.
   - Common currently determines which reserved runtime paths are rejected.
   - If `_haze_conflicts/**` policy changes, Core conflict path generation and adapter scan rules must be reviewed together.

2. Full `accept_conflict` semantics
   - Current system has partial conflict resolution behavior outside Core.
   - If Core owns pure resolution plans, API/Storage/Server must consume those plans instead of implementing divergent resolution policy.

3. Tombstone ID consistency
   - Tombstone identifiers appear in tombstone and operation-log contexts.
   - Future Storage/API fan-in should keep naming and conversion explicit.

4. Idempotency stored response shape
   - Core owns safe fingerprint/replay classification.
   - Server/API/Storage must agree on what exact response snapshot is persisted and replayed.

Any future proposal to add concrete SQLx, Axum, provider SDK, object-store filesystem, background job, or adapter runtime dependencies to `haze-sync-core` requires an explicit contract-change request and Orchestrator/Architect review.
