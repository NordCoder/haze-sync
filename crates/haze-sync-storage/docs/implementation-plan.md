# Implementation Plan: storage

## Current state

`haze-sync-storage` is already more than a scaffold in code, but its component docs were scaffold-level before this planning pass.

Current implemented surfaces:

- `schema`
  - table-name constants for the initial metadata schema;
  - ordered initial migration filename list.
- `models`
  - passive database row structs for adapters, content blobs, sync objects, file revisions, operation log, tombstones, conflicts, adapter cursors, idempotency records, Google Drive mapping, worktree state, and audit events.
- `object_store`
  - `ObjectStore` trait;
  - filesystem-backed `LocalObjectStore`;
  - canonical content-addressed blob layout;
  - hash verification on write/read/stat;
  - safe object-store errors.
- `repositories`
  - passive SQLx repository modules for adapter cursors, conflicts, content blobs, idempotency, objects, operation log, revisions, and tombstones;
  - safe `RepositoryError` boundary;
  - validation helpers for sequences, page limits, sizes, operation kinds/statuses, and cursor regression.
- `locks`
  - deterministic path lock key derivation from `VaultPath`;
  - PostgreSQL transaction-scoped advisory-lock helper.
- `test_support`
  - gated test support available only under `cfg(test)` or `feature = "test-support"`.

The component intentionally does not own Core policy, API DTOs, Server runtime wiring, adapter loops, provider calls, migration runner lifecycle, CLI commands, or public status rendering.

## Target state

The target state for Storage is a durable, safe, transaction-friendly persistence component that can be composed by Server and Core-facing integration phases without leaking runtime details or reimplementing policy.

Storage is V1-ready when:

- schema metadata and migrations match the accepted V1 storage model;
- row models and repository helpers cover required Core/API/Server fan-in paths;
- repository helpers are transaction-friendly and caller-executor based;
- object-store implementation is hash-addressed, verified, path-safe, and idempotent for duplicate writes;
- path advisory locks are deterministic and transaction-scoped;
- idempotency, operation-log, cursor, conflict, tombstone, object, revision, and content-blob repositories support the required server flows;
- storage errors are safe and do not expose SQLx/database/object-store internals publicly;
- real-DB tests are gated and do not require production credentials.

## Implementation phases

### STOR-P1 — Component contract and planning normalization

Status: completed by this Architect planning pass.

Goal:

```text
Replace scaffold storage docs with a real contract, dependency map,
implementation plan, decisions, and baseline implementation log.
```

Allowed scope:

```text
crates/haze-sync-storage/docs/**
```

Completed deliverables:

- complete `component-contract.md`;
- complete `dependency-map.md`;
- complete `implementation-plan.md`;
- add initial storage decisions;
- update implementation log with planning baseline.

Non-goals:

- no product code changes;
- no migration changes;
- no repository behavior changes;
- no sibling component changes;
- no CI/workflow changes.

Acceptance:

- docs describe current Storage module surface;
- docs preserve Storage/Core/API/Server/adapters boundaries;
- future phases are implementable without making Storage own sync policy or runtime orchestration.

### STOR-P2 — Schema and row-model audit

Goal:

```text
Audit schema metadata, migration filenames, and row models against the accepted
system storage model before more repository and fan-in work depends on them.
```

Allowed scope:

```text
crates/haze-sync-storage/src/schema/**
crates/haze-sync-storage/src/models/**
crates/haze-sync-storage/docs/**
migrations/** only if explicitly scoped and reviewed as a schema change
```

Likely work:

- verify all expected tables are represented in `schema::table_names`;
- verify `INITIAL_MIGRATIONS` order matches actual migration files;
- audit row models for field names, optionality, timestamps, and JSON metadata fields;
- clarify which row fields may contain sensitive values and must not be returned publicly;
- add row serialization tests where useful for fixtures;
- document any schema drift or missing row model.

Non-goals:

- no migration runner;
- no Core policy;
- no API DTO mapping;
- no SQL query fan-in unless scoped separately.

Contract-change triggers:

- changing migration order;
- adding/removing tables;
- changing persisted field meaning after repositories or server flows depend on it;
- exposing row models directly as public API output.

Acceptance:

- schema metadata and row models match current migrations;
- sensitive persisted fields are identified;
- downstream repository/server phases have a stable storage baseline.

### STOR-P3 — Object-store hardening

Goal:

```text
Harden the local content-addressed object store as a verified, path-safe,
content-hash-addressed blob store for Server/Core fan-in.
```

Allowed scope:

```text
crates/haze-sync-storage/src/object_store/**
crates/haze-sync-storage/docs/**
```

Required behavior to preserve or harden:

- object paths derive only from validated content hashes;
- writes verify expected hash before commit;
- committed reads/stat verify stored hash;
- duplicate writes are idempotent for same hash/content;
- temporary files are cleaned after failures where possible;
- object-store errors do not format local absolute paths.

Likely work:

- add tests for duplicate writes, missing blobs, hash mismatch, stored blob corruption, non-file object entries, and path-free error formatting;
- review atomicity assumptions around temp write, fsync, hard-link/commit, and cleanup;
- document object-store root ownership and deployment expectations;
- decide whether cross-filesystem rename/link behavior needs fallback support.

Non-goals:

- no object-store HTTP API;
- no garbage collection;
- no retention cleanup;
- no provider blob storage;
- no encryption layer unless a future contract changes V1.

Contract-change triggers:

- changing blob path layout;
- adding compression/encryption;
- exposing local file paths in errors;
- changing duplicate-write semantics;
- adding background cleanup jobs.

Acceptance:

- object-store behavior is deterministic, verified, and tested;
- Server can compose it without adding safety checks outside the store;
- failures remain safe and path-redacted.

### STOR-P4 — Repository validation and safe error boundary hardening

Goal:

```text
Make repository helpers safe and predictable at the storage boundary before
Server and Core fan-in depend on them.
```

Allowed scope:

```text
crates/haze-sync-storage/src/repositories/**
crates/haze-sync-storage/docs/**
```

Likely work:

- audit every repository module for caller-owned executor/transaction usage;
- test validation helpers for sequences, limits, size conversions, status parsing, operation kind parsing, and cursor regression;
- verify `map_sqlx_error` never exposes raw SQLx messages;
- ensure repository errors have stable codes/messages;
- document which repository functions are transaction-sensitive.

Non-goals:

- no DB pool creation;
- no server route wiring;
- no Core policy decisions;
- no public HTTP status mapping;
- no provider behavior.

Contract-change triggers:

- exposing raw database errors;
- adding runtime pool/config ownership;
- returning API DTOs from repositories;
- adding policy decisions such as accepted/conflict/tombstone outcomes.

Acceptance:

- repository error boundary is safe;
- repository helpers remain passive and executor-based;
- repository tests cover storage-level validation.

### STOR-P5 — Normal file flow repository support

Goal:

```text
Ensure Storage can support normal file PUT/GET/changes Server fan-in while Core
continues to own write decisions.
```

Allowed scope:

```text
crates/haze-sync-storage/src/repositories/content_blobs/**
crates/haze-sync-storage/src/repositories/objects/**
crates/haze-sync-storage/src/repositories/revisions/**
crates/haze-sync-storage/src/repositories/operation_log/**
crates/haze-sync-storage/src/locks.rs
crates/haze-sync-storage/docs/**
```

Likely work:

- verify content blob metadata insertion/read behavior;
- verify sync object current-path/current-revision behavior;
- verify immutable file revision insertion/read behavior;
- verify operation-log append and changes-page repository behavior;
- verify per-path advisory lock helper use in integration examples/tests;
- provide repository outputs that Server can map to Core/API models.

Non-goals:

- no Core upsert decision implementation;
- no HTTP handler;
- no content upload streaming runtime;
- no adapter loop;
- no conflict/delete behavior beyond current flow dependencies.

Contract-change triggers:

- needing schema changes;
- adding hidden transactions inside helpers when caller-owned transaction is required;
- changing operation-log semantics consumed by API/adapters;
- bypassing Core base-revision policy.

Acceptance:

- Storage supports normal file flow persistence under caller-owned transactions;
- operation-log changes feed can be queried safely;
- repository helpers do not make sync-policy decisions.

### STOR-P6 — Conflict, tombstone, and delete repository support

Goal:

```text
Support durable persistence for conflict preservation and safe delete flows while
keeping Core as the policy owner.
```

Allowed scope:

```text
crates/haze-sync-storage/src/repositories/conflicts/**
crates/haze-sync-storage/src/repositories/tombstones/**
crates/haze-sync-storage/src/repositories/operation_log/**
crates/haze-sync-storage/src/models/** if row-model adjustment is scoped
crates/haze-sync-storage/docs/**
```

Likely work:

- verify conflict row insertion/list/read/update-status behavior;
- verify tombstone insertion/read/restore metadata behavior;
- verify operation-log rows can represent conflict/delete/tombstone events;
- verify storage rejects unsupported conflict statuses or invalid sizes safely;
- document that full `accept_conflict` content replacement needs Core/API/Server fan-in.

Non-goals:

- no conflict resolution policy;
- no hard delete;
- no filesystem/provider trash moves;
- no API route handlers;
- no retention cleanup job.

Contract-change triggers:

- adding conflict policy decisions to Storage;
- changing conflict/tombstone schema;
- implying hard delete from storage repository helpers;
- exposing raw conflict bytes publicly.

Acceptance:

- durable conflict/delete storage primitives are available for Server fan-in;
- Core decisions remain source of policy;
- repository outputs remain safe and internal.

### STOR-P7 — Idempotency and cursor repository support

Goal:

```text
Stabilize durable idempotency and adapter cursor persistence before adapter and
server retry flows expand.
```

Allowed scope:

```text
crates/haze-sync-storage/src/repositories/idempotency/**
crates/haze-sync-storage/src/repositories/adapter_cursors/**
crates/haze-sync-storage/src/models/** if row-model adjustment is scoped
crates/haze-sync-storage/docs/**
```

Likely work:

- test idempotency record insert/read/check-or-store behavior;
- verify same-key same-request and same-key different-request repository outcomes match Core idempotency primitives;
- ensure stored response snapshots remain safe and public-output-compatible;
- test cursor read/update behavior and cursor regression rejection;
- ensure raw external cursor JSON is not exposed directly through public admin/status surfaces.

Non-goals:

- no HTTP replay middleware;
- no adapter polling loop;
- no provider changes-feed calls;
- no public admin/status rendering.

Contract-change triggers:

- storing raw public idempotency keys in unsafe public reports;
- changing stored response shape without API/Server coordination;
- permitting cursor regression;
- exposing raw provider cursor payloads publicly.

Acceptance:

- Server can implement durable retry/idempotency flow;
- adapters can persist progress safely;
- public surfaces can summarize cursor presence without leaking values.

### STOR-P8 — Adapter mapping and worktree state repository support

Goal:

```text
Provide persistence support for Google Drive mapping and Worktree state without
making Storage own adapter semantics.
```

Allowed scope:

```text
crates/haze-sync-storage/src/repositories/** if mapping/state modules exist or are added
crates/haze-sync-storage/src/models/**
crates/haze-sync-storage/docs/**
```

Likely work:

- verify or add repository helpers for `gdrive_mapping` if Storage is the accepted persistence owner;
- verify or add repository helpers for `worktree_state`;
- preserve adapter semantics outside Storage:
  - GDrive owns provider identity interpretation, echo guard, delete candidates, and import/export decisions;
  - Worktree owns scan/materialization/dirty/echo behavior.
- add tests for storage-level mapping/state persistence only.

Non-goals:

- no Google Drive API calls;
- no OAuth/token loading;
- no Worktree filesystem scan/materialization;
- no adapter mode enforcement;
- no sync policy decisions.

Contract-change triggers:

- allowing GDrive adapter direct DB ownership without system decision;
- encoding provider-specific policy in Storage repositories;
- exposing raw provider payloads or raw cursors publicly;
- changing mapping schema after adapter consumers exist.

Acceptance:

- mapping/state persistence is available if needed by Server/adapter integration;
- adapter semantics remain outside Storage;
- storage row/repository behavior is safe and testable.

### STOR-P9 — Storage test support and integration harness hardening

Goal:

```text
Make storage test support useful for component and fan-in tests without leaking
production credentials or requiring live provider services.
```

Allowed scope:

```text
crates/haze-sync-storage/src/test_support/**
crates/haze-sync-storage/docs/**
```

Likely work:

- audit `test-support` feature gates;
- provide clear helpers for test DB setup/cleanup where scoped;
- ensure tests use local/test-only database URLs supplied by CI or environment;
- document skipped behavior when DB is unavailable;
- avoid production secret assumptions.

Non-goals:

- no production migrations deployment;
- no provider credentials;
- no external vault access;
- no CI workflow edits unless explicitly scoped to github-ci.

Contract-change triggers:

- requiring production DB URLs or secrets;
- making test-support part of production dependency graph;
- hiding failing DB setup as passing tests.

Acceptance:

- storage integration tests are honest and reproducible;
- feature gating is clear;
- CI can choose whether to run storage test-support checks.

## Dependency gates

Storage planning follows Common/Core/API because it must persist common primitives and Core semantic outcomes while avoiding API/runtime leakage.

Important gates:

- Common owns path/hash/ID validation and wire-safe primitive representations.
- Core owns accepted/conflict/tombstone/idempotency/delete-guard decisions.
- API owns public DTOs and sanitized public errors.
- Server owns transaction orchestration, runtime state, route handlers, and mapping across API/Core/Storage.
- Worktree/GDrive/Obsidian own adapter/provider/client behavior.

Storage implementation phases that change schema or repository output shape should be reviewed before Server/adapters depend on those shapes.

## Known risks

- Storage can accidentally absorb Core policy by naming repository helpers after semantic outcomes rather than persisted facts.
- Row models may contain sensitive persisted fields and must not be returned directly as public DTOs.
- Raw SQLx errors or database URLs could leak if error mapping is bypassed.
- Object-store error formatting could leak local absolute paths if IO errors are formatted directly.
- GDrive mapping persistence can blur boundaries between Storage and GDrive adapter semantics.
- Worktree state persistence can blur boundaries between Storage and Worktree scanner/materializer behavior.
- Advisory locks only protect callers that use the same PostgreSQL transaction discipline.
- Migration/schema changes after Server/adapters consume repositories are high-risk.

## Deferred work

Deferred outside this Architect documentation/planning pass:

- run shell checks or observe CI;
- execute STOR-P2 through STOR-P9 implementation/clean-code/CI phases;
- schema migration changes, unless explicitly scoped;
- Server transaction fan-in;
- Core policy changes;
- API public DTO changes;
- adapter runtime behavior;
- production migration runner/deployment;
- hard-delete cleanup and retention execution.

## Completion criteria for the component

`haze-sync-storage` is V1-ready when:

- schema metadata, migrations, row models, and repositories support accepted V1 flows;
- object-store primitives are verified and path-safe;
- repository helpers are safe, transaction-friendly, and policy-free;
- advisory locks support per-path transaction discipline;
- idempotency/cursor/conflict/tombstone/operation-log persistence supports Server fan-in;
- adapter mapping/worktree state persistence is available without owning adapter semantics;
- storage tests cover pure validation, object store behavior, and DB-backed helpers where scoped;
- no public output leaks raw DB errors, filesystem paths, credentials, tokens, cursors, provider payloads, idempotency keys, or file bytes.
