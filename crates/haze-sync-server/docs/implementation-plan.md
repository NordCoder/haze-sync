# Implementation Plan: server

## Current state

Server is a post-W3 integration shell with real route wiring for the current Core/API/Storage surfaces.

Implemented responsibilities:

- Axum router composition for `/health`, `/ready`, and `/v1` routes.
- Dependency-free router construction for route-shell tests.
- Explicit `ServerAppState` carrying optional PostgreSQL pool, local object store, config, and auth state.
- Safe config parsing and redaction primitives.
- Safe readiness checks for database and object-store dependencies.
- Database migration/readiness helpers.
- Bearer-token authentication execution through disabled/static/database auth states.
- Runtime route wiring for server-info, file PUT/GET, changes feed, conflicts, DELETE tombstones, and read-only admin/status.
- Transaction orchestration for PUT and DELETE routes that need multi-step Storage writes.
- Safe internal-to-public error mapping at the route boundary.

Server still intentionally does not start a production listener, own provider runtimes, run adapter loops, call Google Drive/Obsidian/worktree providers, or hard-delete data.

## Target state

The server component should remain the runtime composition boundary for Haze Sync:

- construct and run the Axum HTTP server from explicit configuration;
- wire API parsers/DTOs to Core policy services and Storage repositories;
- keep route-level transaction boundaries clear and testable;
- expose safe health/readiness/admin status surfaces;
- execute adapter/admin auth without leaking tokens or token hashes;
- report all internal failures through stable public error responses;
- avoid embedding provider runtime behavior or Core policy decisions.

Server is V1-ready when:

- production startup/listener/graceful shutdown behavior exists and is documented;
- config loading creates explicit runtime state without leaking secrets;
- health/readiness/server-info/admin/status outputs are safe and honest;
- file PUT/GET/changes routes preserve API/Core/Storage semantics under transaction discipline;
- conflict/delete/idempotency routes are transaction-safe and policy-aligned;
- worktree runtime composition, if hosted by Server, is integrated through the Worktree component contract;
- adapter mode/status enforcement at runtime is explicit;
- route tests and cross-component E2E tests cover safety-critical behavior.

As later components mature, Server should integrate them only through explicit fan-in prompts and component contracts.

## Implementation phases

### SRV-P1 — Component contract and planning normalization

Status: completed by T0-P3 plus this Architect planning pass.

Goal:

```text
Keep Server docs accurate enough that future fan-in workers can modify runtime
wiring without moving Core policy, API DTO ownership, or Storage schema behavior
into Server.
```

Allowed scope:

```text
crates/haze-sync-server/docs/**
crates/haze-sync-server/control/state.md when explicitly assigned by Orchestrator
```

Completed deliverables:

- current server component contract;
- dependency map;
- implementation log entry;
- baseline server process-test decision;
- expanded phased implementation plan.

Non-goals:

- no product code changes;
- no route refactor;
- no runtime behavior changes;
- no sibling component changes;
- no CI/workflow edits.

Acceptance:

- docs describe current route/runtime surface;
- docs distinguish Server runtime ownership from API/Core/Storage/adapters;
- future server phases are implementable as explicit fan-in/integration work.

### SRV-P2 — Router, state, auth, and safe error audit

Goal:

```text
Audit current Server runtime shell and route behavior against component contracts,
then harden tests/docs around passive-safe dependency-free behavior and public
error sanitization.
```

Allowed scope:

```text
crates/haze-sync-server/src/routes/**
crates/haze-sync-server/src/state.rs
crates/haze-sync-server/src/http/**
crates/haze-sync-server/src/config/**
crates/haze-sync-server/src/readiness/**
crates/haze-sync-server/docs/**
```

Likely work:

- verify router construction remains explicit and free of hidden globals;
- verify dependency-free router routes fail safely without mutation;
- test auth states: disabled, static principal, database lookup failure, role checks;
- audit `Debug` implementations for redaction;
- test public error bodies for absence of tokens, token hashes, idempotency keys, DB URLs, object-store roots, local paths, stack traces, raw SQLx errors, and request bodies;
- document any route that is intentionally placeholder or partial.

Non-goals:

- no production listener;
- no provider runtime;
- no broad route decomposition unless scoped;
- no Core policy changes;
- no Storage schema changes.

Contract-change triggers:

- adding hidden global runtime state;
- changing API public error shapes inside Server;
- moving token creation/rotation into Server routes;
- exposing raw runtime internals in public responses.

Acceptance:

- route shell behavior is safe and documented;
- auth/error boundaries are tested;
- public outputs remain sanitized.

### SRV-P3 — Production startup, config loading, listener, and graceful shutdown

Goal:

```text
Turn the server binary from scaffold construction into explicit production startup
without changing route semantics or introducing hidden behavior.
```

Allowed scope:

```text
crates/haze-sync-server/src/main.rs
crates/haze-sync-server/src/config/**
crates/haze-sync-server/src/db/**
crates/haze-sync-server/src/state.rs
crates/haze-sync-server/docs/**
```

Likely work:

- load runtime config from environment/files according to accepted config contract;
- create PostgreSQL pool and local object store from config;
- build `ServerAppState` explicitly;
- bind and serve Axum listener;
- implement graceful shutdown signal handling;
- define startup failure messages that are useful but secret-safe;
- keep migrations policy explicit: disabled/manual/auto only if accepted by contract.

Non-goals:

- no adapter loops;
- no provider calls;
- no worktree runtime unless separately scoped;
- no deployment scripts unless deployment component scopes them;
- no route behavior rewrite.

Contract-change triggers:

- auto-running migrations without accepted policy;
- changing configuration variable names/schema;
- introducing production secrets into repo/tests;
- starting background adapter jobs implicitly.

Acceptance:

- server can start from explicit config in a local/prod-like environment;
- startup logs/errors are sanitized;
- route behavior remains compatible with existing API contracts.

### SRV-P4 — File PUT/GET/changes transaction fan-in hardening

Goal:

```text
Harden normal file flow route integration across API/Core/Storage/ObjectStore with
clear transaction boundaries and no silent overwrite behavior.
```

Allowed scope:

```text
crates/haze-sync-server/src/routes/v1/** or current file/changes route modules
crates/haze-sync-server/src/state.rs
crates/haze-sync-server/src/http/**
crates/haze-sync-server/docs/**
```

Likely work:

- ensure PUT uses API header/path parsers and Core revision outcomes;
- acquire per-path PostgreSQL advisory lock before mutation;
- coordinate object-store write, content-blob metadata, sync object/current revision, file revision, operation log, and idempotency response storage;
- ensure same-content and hash-mismatch outcomes map to safe API responses;
- implement GET file metadata/content path using Storage/ObjectStore safely;
- implement changes feed from operation-log repository with API DTO mapping;
- add DB-backed route tests when test support is available.

Non-goals:

- no conflict resolution route behavior beyond conflict-saved preservation fan-in if scoped;
- no adapter loops;
- no provider runtime;
- no API DTO redesign;
- no Storage schema change unless explicitly scoped.

Contract-change triggers:

- bypassing Core base-revision semantics;
- using route-local overwrite policy;
- storing idempotency responses with unsafe raw values;
- leaking DB/object-store internals in route errors.

Acceptance:

- normal file flow works transactionally in server integration tests;
- stale/unknown-base writes do not overwrite current content;
- changes feed is stable and API-compatible.

### SRV-P5 — Conflict, delete, and idempotency route fan-in hardening

Goal:

```text
Harden Server integration for conflict preservation, conflict resolution metadata,
delete/tombstone semantics, mass-delete guard, and durable idempotency.
```

Allowed scope:

```text
crates/haze-sync-server/src/routes/conflicts/**
crates/haze-sync-server/src/routes/delete/**
crates/haze-sync-server/src/routes/v1/** if file-write conflict preservation is there
crates/haze-sync-server/src/http/**
crates/haze-sync-server/docs/**
```

Likely work:

- persist `conflict_saved` outcomes from Core as conflict-copy content/revision/row/operation entries without overwriting current revision;
- wire conflict list/status filtering through Storage repositories and API DTOs;
- implement only accepted conflict resolution actions and honestly return partial/not-implemented behavior where needed;
- ensure DELETE requires base/null-base and idempotency metadata;
- acquire path locks for delete mutations;
- evaluate Core delete guard before tombstone persistence;
- persist tombstones, clear current state, append operation-log entries, and store idempotency responses atomically;
- test idempotent replay and same-key different-request conflicts.

Non-goals:

- no hard delete;
- no retention cleanup job;
- no provider/worktree trash side effects;
- no Web UI conflict center;
- no policy expansion such as latest-wins/incoming-wins.

Contract-change triggers:

- implementing hard delete;
- adding route-local conflict/delete policy;
- changing public conflict resolution vocabulary;
- exposing raw conflict bytes or idempotency keys publicly.

Acceptance:

- conflict and delete routes preserve Core/API/Storage semantics;
- all mutation paths are transaction-bound and locked where needed;
- public responses remain sanitized.

### SRV-P6 — Admin/status, readiness, doctor, and observability hardening

Goal:

```text
Expose honest, read-only operational surfaces that help operators without leaking
secrets, raw cursors, DB URLs, local paths, provider payloads, or stack traces.
```

Allowed scope:

```text
crates/haze-sync-server/src/routes/admin/**
crates/haze-sync-server/src/readiness/**
crates/haze-sync-server/src/db/**
crates/haze-sync-server/src/http/**
crates/haze-sync-server/docs/**
```

Likely work:

- harden `/ready` and DB/object-store readiness checks;
- map adapter summaries, cursor presence, pause support, and mode/status summaries safely;
- add doctor route or server-side doctor inputs only when Core/API contracts exist;
- ensure skipped/not-run checks are represented honestly;
- add structured logs/tracing with redaction guarantees if scoped;
- add metrics endpoint only if system scope accepts it.

Non-goals:

- no admin mutations by default;
- no repair execution;
- no token rotation;
- no provider calls unless a provider component contract supplies safe checks;
- no raw cursor/status payload exposure.

Contract-change triggers:

- adding pause/resume/admin mutation routes;
- exposing raw runtime/provider diagnostics;
- claiming live checks were run when they were skipped;
- adding metrics/logging dependencies that leak secrets.

Acceptance:

- operational outputs are safe and useful;
- read-only admin/status semantics remain clear;
- doctor/readiness claims are honest.

### SRV-P7 — Worktree runtime composition fan-in

Goal:

```text
Integrate the Worktree runtime into Server only as composition, while keeping
scanner/writer/materializer/repair logic owned by the Worktree component.
```

Allowed scope:

```text
crates/haze-sync-server/src/**
crates/haze-sync-server/docs/**
crates/haze-sync-worktree/** only if a dedicated cross-component fan-in explicitly scopes it
```

Likely work:

- add config flags for worktree runtime mode only after config contract exists;
- instantiate Worktree component services from explicit Server config/state;
- ensure worktree import/export respects adapter mode and Core/API safety semantics;
- expose safe status/readiness summaries;
- implement graceful startup/shutdown coordination if background tasks are accepted;
- preserve Worktree logic ownership outside Server.

Non-goals:

- no Worktree scanner/materializer logic inside Server route modules;
- no provider/GDrive behavior;
- no hidden background jobs by default;
- no hard delete/trash behavior outside Worktree contract.

Contract-change triggers:

- moving Worktree business logic into Server;
- starting background tasks without config/ops docs;
- bypassing Core/API write semantics;
- changing deployment topology.

Acceptance:

- Server can host Worktree runtime if configured;
- boundaries remain clean;
- startup/shutdown/status behavior is testable and documented.

### SRV-P8 — Adapter mode enforcement and external adapter integration surfaces

Goal:

```text
Provide runtime integration points for adapter modes, adapter auth, and external
adapter clients without embedding provider runtimes in generic Server routes.
```

Allowed scope:

```text
crates/haze-sync-server/src/**
crates/haze-sync-server/docs/**
```

Likely work:

- enforce adapter role/mode at route/runtime boundaries;
- expose read-only adapter status summaries;
- support safe adapter registration/token lookup only if accepted by auth contract;
- provide provider-neutral integration endpoints/metadata consumed by GDrive/Obsidian/Worktree clients;
- ensure dry-run/import-only/export-only/bidirectional semantics are represented where Server must know them.

Non-goals:

- no Google Drive API calls;
- no Obsidian plugin internals;
- no provider webhook runtime unless explicitly scoped;
- no direct adapter DB ownership changes.

Contract-change triggers:

- adding adapter mutation/admin APIs;
- changing token model;
- embedding provider SDKs in Server;
- exposing raw external cursors or provider payloads.

Acceptance:

- adapter clients can authenticate and receive consistent mode/status behavior;
- Server remains provider-neutral;
- public surfaces remain safe.

### SRV-P9 — Server E2E and failure-injection hardening

Goal:

```text
Prove Server integration behavior under realistic local failures before real-vault
rollout.
```

Allowed scope:

```text
crates/haze-sync-server/**
tests/e2e/** only if explicitly scoped as cross-component integration
crates/haze-sync-storage/test support only through accepted interfaces
```

Likely work:

- DB-backed route integration tests for PUT/GET/changes/conflict/delete/admin/status;
- failure-injection tests for object-store write before DB commit, DB failure after object-store write, duplicate idempotency, concurrent same-path writes, stale base conflicts, and mass delete blocks;
- ensure readiness/doctor/admin status reflect failure modes honestly;
- verify no public response leaks secrets or raw internals.

Non-goals:

- no live provider/GDrive tests;
- no production deployment automation;
- no real vault mutation;
- no broad route redesign.

Contract-change triggers:

- test requirements revealing missing Core/Storage/API contract support;
- needing deployment/CI changes outside Server scope;
- discovering unsafe partial commit behavior requiring architecture review.

Acceptance:

- Server behavior is covered by local integration tests;
- failure modes preserve data and secrecy;
- remaining production rollout blockers are explicit.

## Dependency gates

Server may integrate only stable upstream contracts:

- API route parsers, DTOs, header constants, auth primitives, and public error formats.
- Core revision/conflict/delete/tombstone/idempotency primitives.
- Storage migrations, repositories, locks, object-store primitives, and test support.
- Common value/domain types.

Server implementation phases that wire multiple components should run after the relevant component contracts and plans are stable.

Server must stop or request a contract change when a requested change requires:

- modifying another component contract;
- inventing Core policy in route code;
- changing API DTO/error shapes outside API ownership;
- changing Storage schema/repository contracts outside Storage ownership;
- adding provider runtime behavior;
- adding hard-delete behavior;
- moving Worktree/GDrive/Obsidian logic into Server without explicit fan-in scope.

## Known risks

- Route modules can become oversized because Server is an integration boundary. Refactors should preserve route contracts and remain server-local unless fan-in explicitly allows cross-component cleanup.
- Runtime transaction boundaries must stay aligned with Storage repository behavior and Core policy outcomes.
- Auth/database lookup failures must remain sanitized while still being operationally diagnosable through safe logs/metrics in later phases.
- Dependency-free route-shell behavior must not be mistaken for production readiness.
- Admin/status surfaces must remain read-only until mutation support is explicitly scoped.
- Object-store writes that happen before database commit require future failure-injection coverage and repair/doctor follow-up.
- Startup/migration policy can cause data loss or downtime if hidden inside server boot without operational docs.
- Worktree runtime hosting can collapse boundaries if Server starts owning scanner/materializer logic.

## Deferred work

Deferred outside this Architect documentation/planning pass:

- run shell checks or observe CI for this branch;
- execute SRV-P2 through SRV-P9 implementation/clean-code/CI phases;
- production listener startup and graceful shutdown;
- runtime config loading in `main`;
- explicit migration runner policy at startup or CLI boundary;
- structured tracing/logging initialization with secret redaction guarantees;
- Prometheus/metrics route if product scope requires it;
- full adapter mode enforcement at server/runtime boundaries;
- Worktree runtime composition;
- provider adapter orchestration outside Server until scoped fan-in;
- broader route-module decomposition after behavior is stabilized and covered by tests.

## Completion criteria for the component

`haze-sync-server` is V1-ready when:

- startup, config, state, readiness, and graceful shutdown are explicit and tested;
- route wiring preserves API/Core/Storage semantics;
- mutation routes are transaction-bound, locked where necessary, idempotent where required, and safe under stale/unknown bases;
- conflict/delete/admin/status behavior is honest and sanitized;
- worktree hosting and adapter mode enforcement are explicit fan-in work, not hidden route behavior;
- E2E/failure-injection tests cover critical data-loss and secrecy risks;
- no provider runtime, hard-delete behavior, or Core policy duplication enters Server without explicit contract change.
