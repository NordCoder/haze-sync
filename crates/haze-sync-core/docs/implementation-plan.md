# Implementation Plan: core

## Current state

The Core component is past the generic scaffold stage. Its current code is a storage-agnostic pure/service-layer crate with public modules for the post-W2/W3 Core safety foundation.

Implemented surfaces currently documented by this plan:

- `revision_service`
  - normal upsert decision flow over caller-owned repository/content-store/operation-log traits;
  - expected-hash verification;
  - accepted new-file and current-base revision outcomes;
  - duplicate same-content ignore outcome;
  - stale, unknown, and explicit-null base safety outcome;
  - `conflict_saved` semantic status when incoming different content must be preserved without overwriting current content.
- `conflict_service`
  - conflict policy names;
  - current and incoming conflict metadata;
  - conflict record input/output shapes;
  - conflict-copy path generation under `_haze_conflicts/open/**`;
  - recursive conflict-area guard.
- `policy_engine`
  - pure conflict policy application;
  - preserve-both and current-wins-with-incoming-backup metadata outcomes.
- `conflict_saved_planner`
  - conversion from revision-service `conflict_saved` outcomes into conflict preservation plans suitable for future storage/API fan-in.
- `tombstone_service`
  - tombstone id validation;
  - tombstone creation metadata;
  - retention validation;
  - restore-ready metadata shape without restore behavior.
- `delete_guard`
  - conservative default delete thresholds;
  - count and ratio blocking;
  - scoped manual unlock model.
- `idempotency`
  - idempotency key validation;
  - safe request fingerprinting;
  - stored response validation;
  - same-request replay and different-request conflict classification.
- `operation_log`
  - operation sequence and changes limit validation;
  - changes query and changes page models;
  - operation kind parsing;
  - adapter cursor regression classification.
- `doctor`
  - passive doctor check result models;
  - redacted DB/object-store/missing-blob/token sanity summaries;
  - aggregate report summary derivation.

The current crate intentionally does not wire routes, repositories, adapters, providers, migrations, object-store filesystem writes, background jobs, or runtime side effects.

## Target state

The target state for the Core component is a stable, documented, side-effect-free decision layer that downstream components can rely on for safety-critical sync semantics.

Core should remain:

- deterministic;
- storage-agnostic;
- API-neutral;
- provider-neutral;
- free of concrete runtime resources;
- explicit about unsafe/deferred behavior;
- conservative around conflicts and deletes;
- safe to serialize at public boundaries.

Downstream Storage, API, Server, CLI, Worktree, GDrive, and Obsidian components should map Core decisions into concrete persistence rows, HTTP responses, CLI output, provider actions, filesystem actions, or UI state without reimplementing Core safety policy.

Core is V1-ready when:

- every overwrite-capable write path has explicit current/stale/null-base semantics;
- conflict preservation can be represented as safe metadata and byte-preservation plans without exposing raw content in public serialization;
- delete/tombstone/retention policy is represented without physical deletion side effects;
- idempotency and operation-log/cursor value models are stable enough for Storage and Server fan-in;
- passive doctor models are safe, redacted, and honest about skipped checks;
- Core tests cover safety invariants without a database, server, provider, or filesystem runtime.

## Implementation phases

### CORE-P1 — Component contract and planning normalization

Status: completed by T0-P1 plus this Architect planning pass.

Goal:

```text
Keep Core component docs accurate enough that future implementation workers can
operate inside the Core scope without rediscovering system boundaries.
```

Allowed scope:

```text
crates/haze-sync-core/docs/**
crates/haze-sync-core/control/state.md when explicitly assigned by Orchestrator
```

Completed deliverables:

- current component contract;
- dependency map;
- implementation log entry;
- baseline decisions;
- expanded phased implementation plan.

Non-goals:

- no product code changes;
- no route/storage/server wiring;
- no CI/workflow edits;
- no sibling component changes.

Acceptance:

- docs describe the current public Core module surface;
- docs distinguish Core policy from Storage/API/Server/adapters;
- future phases are implementable without turning Core into a runtime crate.

### CORE-P2 — Public module audit and behavior/test alignment

Goal:

```text
Audit every public Core module against the component contract, then add small
rustdoc/test hardening where current behavior is underdocumented or undertested.
```

Allowed scope:

```text
crates/haze-sync-core/src/**
crates/haze-sync-core/tests/** if present or added inside the component
crates/haze-sync-core/docs/**
```

Likely work:

- verify `src/lib.rs` exports only intended Core modules;
- audit each public type/function for safe serialization and public output;
- add missing tests for current behavior without changing semantics;
- improve rustdoc where a type may be mistaken for persistence/runtime ownership;
- mark any discovered public API mismatch as a contract-change request.

Non-goals:

- no new sync behavior;
- no downstream wiring;
- no sibling crate edits;
- no SQLx/Axum/provider/filesystem dependencies.

Acceptance:

- public module surface matches `component-contract.md`;
- no raw content/secrets/provider payloads are serialized publicly;
- Core tests pass when runnable;
- docs are updated for any clarified behavior.

### CORE-P3 — Revision service safety hardening

Goal:

```text
Finalize pure normal upsert semantics so Storage/API/Server can wire file writes
without duplicating overwrite/conflict policy.
```

Allowed scope:

```text
crates/haze-sync-core/src/revision_service/**
crates/haze-sync-core/src/conflict_saved_planner/** when needed for outcome compatibility
crates/haze-sync-core/docs/**
```

Required behavior to preserve or harden:

- file missing + base=null -> accepted new file;
- file missing + non-null base -> safe stale/unknown-base reject;
- existing file + base=current + different content -> accepted new revision;
- existing file + same content -> ignored same-content outcome;
- existing file + base=old/null/unknown + different content -> `conflict_saved` semantic outcome;
- hash mismatch -> safe rejection with expected/actual hashes only;
- incoming conflict bytes must not serialize in public JSON.

Likely work:

- add tests for all base/content matrix cases;
- clarify `RejectedStaleOrUnknownBase` vs public `conflict_saved` naming in docs/tests;
- verify repository/content-store/operation-log trait boundaries are still storage-agnostic;
- ensure the algorithm never appends an operation or inserts a revision for unsafe stale overwrites unless explicitly accepted by policy.

Contract-change triggers:

- renaming public outcome variants in a way that breaks API/Server mapping;
- changing base=null existing-file semantics;
- moving repository transactions/locks into Core;
- removing incoming bytes from conflict-saved plans before a storage fan-in alternative exists.

Acceptance:

- revision matrix tests are complete;
- public status strings are stable and documented;
- downstream components can map outcomes without guessing policy.

### CORE-P4 — Conflict preservation and resolution primitives

Goal:

```text
Complete the pure Core side of conflict preservation and conflict resolution
vocabulary without implementing route wiring or storage writes.
```

Allowed scope:

```text
crates/haze-sync-core/src/conflict_service/**
crates/haze-sync-core/src/policy_engine/**
crates/haze-sync-core/src/conflict_saved_planner/**
crates/haze-sync-core/docs/**
```

Likely work:

- test conflict-copy path generation for nested paths, extensions, unsafe adapter IDs, timestamps, and recursive conflict-area input;
- verify `_haze_conflicts/open/**` materialization remains compatible with `VaultPath` rules;
- define pure decision primitives for resolution actions if Core owns them:
  - `accept_current`;
  - `accept_conflict`;
  - `keep_both`;
  - `mark_resolved`;
- represent resolution effects as storage/API-neutral plans, not persisted writes;
- clarify which actions are metadata-only and which require new current revision creation.

Non-goals:

- no `GET /v1/conflicts` or `POST /v1/conflicts/{id}/resolve` wiring;
- no conflict row repository implementation;
- no object-store writes;
- no API DTO ownership;
- no Obsidian conflict UI behavior.

Contract-change triggers:

- changing conflict path layout;
- making latest-wins/incoming-wins policy available;
- moving conflict persistence into Core;
- allowing recursive conflict materialization;
- making resolution actions mutate storage directly.

Acceptance:

- Core can express conflict preservation/resolution plans safely;
- route/storage/server components can wire them without inventing policy;
- raw conflict bytes remain non-public.

### CORE-P5 — Tombstone, delete guard, restore, and retention classifiers

Goal:

```text
Finalize pure delete safety semantics before Worktree/GDrive/Obsidian delete
propagation expands.
```

Allowed scope:

```text
crates/haze-sync-core/src/tombstone_service/**
crates/haze-sync-core/src/delete_guard/**
crates/haze-sync-core/docs/**
```

Likely work:

- add or harden tests for tombstone metadata validation;
- test retention-window validation and restore-ready metadata;
- test delete guard thresholds with zero totals, exact thresholds, over-threshold counts, ratio overflow safety, and scoped unlock mismatch;
- add pure restore/retention eligibility classifiers if needed before Storage/CLI/Server implement restore or cleanup;
- keep physical deletion and trash movement outside Core.

Non-goals:

- no hard delete cleanup job;
- no filesystem trash movement;
- no provider delete calls;
- no DB tombstone repository;
- no CLI command parsing.

Contract-change triggers:

- changing default delete thresholds;
- allowing deletes without manual unlock after mass-delete threshold breach;
- adding physical cleanup behavior to Core;
- changing tombstone ID ownership.

Acceptance:

- delete/tombstone behavior is safe and fully represented as pure plans/models;
- adapters can use Core/API outcomes without inventing delete policy;
- future restore/cleanup components have explicit Core-owned classifiers if required.

### CORE-P6 — Idempotency, operation-log, and cursor primitive hardening

Goal:

```text
Stabilize pure request fingerprinting, replay classification, operation metadata,
changes-query bounds, and cursor update rules before durable Storage/Server fan-in.
```

Allowed scope:

```text
crates/haze-sync-core/src/idempotency/**
crates/haze-sync-core/src/operation_log/**
crates/haze-sync-core/docs/**
```

Likely work:

- test idempotency key validation and redaction boundaries;
- test request fingerprint determinism and same/different request classification;
- validate stored response snapshot limitations and safe replay shape;
- test operation kind parsing/serialization;
- test changes query limit boundaries;
- test cursor monotonicity and regression rejection;
- document what durable idempotency/storage components must persist.

Non-goals:

- no durable idempotency repository;
- no operation-log database append implementation;
- no HTTP replay middleware;
- no adapter polling loop.

Contract-change triggers:

- changing fingerprint algorithm or stored response shape;
- adding raw idempotency key to public output;
- changing operation sequence type semantics;
- moving durable record storage into Core.

Acceptance:

- Storage/Server can implement durable idempotency and operation-log persistence from stable Core primitives;
- public outputs remain safe;
- adapters can rely on cursor classification rules.

### CORE-P7 — Passive doctor and safety-report models

Goal:

```text
Keep doctor models useful for CLI/Server while preserving the rule that Core does
not perform live checks itself.
```

Allowed scope:

```text
crates/haze-sync-core/src/doctor/**
crates/haze-sync-core/docs/**
```

Likely work:

- audit doctor statuses and aggregate summary rules;
- test missing-blob, DB, object-store, cursor, mapping, token-sanity, and worktree-drift summary models as they are added;
- ensure doctor details are pre-redacted and public-safe;
- expose skipped/not-run state honestly for connector/tooling-limited environments;
- avoid embedding local absolute paths, database URLs, provider payloads, or raw errors.

Non-goals:

- no actual DB connection;
- no filesystem/object-store probing;
- no provider/OAuth validation;
- no CLI command implementation;
- no repair behavior.

Contract-change triggers:

- adding live checks to Core;
- exposing raw diagnostics in public models;
- making doctor mutate state or repair data.

Acceptance:

- CLI/Server can render honest doctor reports using Core models;
- skipped/live/not-run semantics are explicit;
- outputs are safe for logs and operator reports.

### CORE-P8 — Core compatibility fixtures and integration contract examples

Goal:

```text
Provide minimal examples/fixtures that downstream components can use to preserve
Core semantic compatibility without depending on Core internals.
```

Allowed scope:

```text
crates/haze-sync-core/**
```

Possible deliverables:

- sample serialized outcomes for accepted write, same-content, conflict_saved, hash mismatch, tombstone, delete guard block, idempotency replay/conflict, and doctor summary;
- tests asserting fixture stability;
- docs explaining which fields are semantic contract and which are internal implementation details.

Non-goals:

- no API DTO duplication;
- no TypeScript generation unless explicitly accepted;
- no Server route tests;
- no Storage repository tests;
- no adapter runtime tests.

Contract-change triggers:

- needing to edit API/Server/Storage/adapter components to make fixtures useful;
- discovering that Core public models are too storage/API-specific;
- changing serialized outcome shapes after downstream components consume them.

Acceptance:

- downstream components can verify semantic mapping against stable examples;
- fixtures do not expose raw bytes/secrets;
- compatibility risk is reduced before fan-in work.

## Dependency gates

See `dependency-map.md` for current upstream/downstream relationships.

Important gates:

- Core depends on `haze-sync-common` for path, ID, hash, validation, and adapter value types.
- API/server/storage components must not depend on undocumented Core internals; they should map through public module types and functions.
- Adapter components must not duplicate Core conflict/delete policy; they should submit facts and obey Core/API outcomes.
- Storage fan-in must provide transactional safety, persistence, repository implementations, operation-log append, object-store durability, and path locking outside Core.
- Server fan-in must map Core outcomes into routes, auth, state, readiness, and public errors without making Core own runtime state.

## Known risks

- Documentation can drift from public module behavior unless every Core-affecting prompt updates these docs when behavior changes.
- `revision_service` currently keeps incoming conflict bytes in memory for conflict-saved planning while skipping serialization; storage fan-in must preserve bytes safely without exposing them in public outputs.
- Tombstone identifiers currently appear in both `tombstone_service` and `operation_log` models; future fan-in should keep conversion/contract consistency explicit.
- The pure `RevisionService` trait boundaries do not themselves enforce database transactions or per-path locks. Downstream Storage/Server wiring must provide those guarantees.
- Doctor models are passive; a downstream CLI/server checker must not claim live DB/object-store/provider checks were performed unless it actually performed them.
- Idempotency primitives classify replay behavior only; durable idempotency repository logic remains outside Core.
- Conflict-copy path generation depends on `_haze_conflicts/**` remaining accepted by `VaultPath` while still avoiding recursive materialization.
- Adding too many DTO-like structures to Core can blur ownership with API.

## Deferred work

Deferred outside this Architect documentation/planning pass:

- run shell checks or observe CI for this branch;
- execute CORE-P2 through CORE-P8 implementation/clean-code/CI phases;
- wire Core decisions into API routes and server state;
- implement SQLx repositories and transactional fan-in where needed;
- persist conflict records and conflict-copy revisions from `conflict_saved` plans;
- implement conflict-resolution API/service wiring;
- implement restore behavior if required by a future contract;
- implement retention cleanup eligibility and cleanup jobs outside Core;
- integrate passive doctor models with real CLI/server checks.

## Completion criteria for the component

`haze-sync-core` is V1-ready when:

- all public modules have documented stable semantics;
- Core safety invariants are tested without runtime dependencies;
- no silent overwrite path exists in pure decisions;
- conflict preservation and delete safety are represented as explicit plans/models;
- idempotency and operation-log primitives are stable enough for durable Storage/Server implementation;
- doctor models are safe and passive;
- Core remains free of Axum, SQLx, provider SDKs, filesystem watchers, deployment, and CLI command parsing.
