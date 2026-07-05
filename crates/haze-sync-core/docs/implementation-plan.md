# Implementation Plan: core

## Current state

The Core component is past the generic scaffold stage. Its current code is a storage-agnostic pure/service-layer crate with public modules for the post-W3 Core safety foundation.

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

Downstream storage, API, server, CLI, and adapter components should map Core decisions into concrete persistence rows, HTTP responses, CLI output, or adapter actions without reimplementing Core safety policy.

## Waves / phases

### T0-P1 — Core component contract audit

Type: documentation/process test.

Scope:

- replace scaffold component docs with current-state Core documentation;
- document current public modules and behavior boundaries;
- document non-goals and cross-component ownership boundaries;
- record the process-test decision;
- add one implementation-log entry;
- update control state and report.

Non-goals:

- no Rust source changes;
- no feature behavior changes;
- no route/storage/adapter wiring;
- no PR or merge.

### Future Core implementation passes

Future Core passes should be scheduled by the Orchestrator through the component control slot. Any future prompt should identify whether it is expanding pure Core behavior or wiring a downstream component.

Likely future Core-owned work, if not already covered by current code, belongs in narrow prompts such as:

- conflict-resolution decision primitives, if resolution logic is kept pure inside Core;
- restore decision primitives, if restore is modeled before being wired to storage/API;
- retention eligibility classification, without physical deletion;
- additional doctor-check summary models, without performing live checks;
- additional unit and property tests for existing pure behavior.

Work that should not be assigned to Core unless the contract is explicitly changed:

- Axum routes;
- SQLx repositories;
- migrations;
- concrete object-store implementation;
- server startup/readiness;
- provider calls;
- adapter loops;
- CLI command parsing;
- deployment files.

## Dependency gates

See `dependency-map.md` for current upstream/downstream relationships.

Important gates:

- Core depends on `haze-sync-common` for path, ID, hash, validation, and adapter value types.
- API/server/storage components must not depend on undocumented Core internals; they should map through public module types and functions.
- Adapter components must not duplicate Core conflict/delete policy; they should submit facts and obey Core/API outcomes.
- Storage fan-in must provide transactional safety, persistence, repository implementations, operation-log append, object-store durability, and path locking outside Core.

## Known risks

- Documentation can drift from public module behavior unless every Core-affecting prompt updates these docs when behavior changes.
- `revision_service` currently keeps incoming conflict bytes in memory for conflict-saved planning while skipping serialization; storage fan-in must preserve bytes safely without exposing them in public outputs.
- Tombstone identifiers currently appear in both `tombstone_service` and `operation_log` models; future fan-in should keep conversion/contract consistency explicit.
- The pure `RevisionService` trait boundaries do not themselves enforce database transactions or per-path locks. Downstream storage/server wiring must provide those guarantees.
- Doctor models are passive; a downstream CLI/server checker must not claim live DB/object-store/provider checks were performed unless it actually performed them.
- Idempotency primitives classify replay behavior only; durable idempotency repository logic remains outside Core.

## Deferred work

Deferred outside this T0-P1 documentation task:

- run shell checks or observe CI for this branch;
- wire Core decisions into API routes and server state;
- implement SQLx repositories and transactional fan-in where needed;
- persist conflict records and conflict-copy revisions from `conflict_saved` plans;
- implement conflict-resolution API/service wiring;
- implement restore behavior if required by a future contract;
- implement retention cleanup eligibility and cleanup jobs outside Core;
- integrate passive doctor models with real CLI/server checks;
- add or expand Core tests only when a future prompt changes behavior or explicitly asks for test hardening.
