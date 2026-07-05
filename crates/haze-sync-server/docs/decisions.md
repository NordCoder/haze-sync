# Decisions: server

## 2026-07-05 — T0-P3 remains documentation plus tiny cleanup

Decision:
T0-P3 is treated as a component documentation and process-control test with at most one behavior-preserving server-local route cleanup. It is not a broad route refactor or runtime behavior phase.

Rationale:
The T0-P3 implementation prompt asked to replace generic scaffold docs with useful current-state server documentation and permitted only one tiny safe cleanup if obvious. Server route modules already contain W2/W3 wiring across file, changes, conflict, delete, and admin surfaces, so a broad cleanup would be higher risk and belongs in a separately scoped implementation or fan-in phase.

Alternatives:
- Refactor route registration or split oversized route modules now; rejected because the T0-P3 implementation prompt explicitly forbade rewriting `routes/mod.rs` or `routes/v1.rs` and framed the work as a process test.
- Make no source cleanup; allowed, but a small route doc-comment clarification was safe and behavior-preserving.

Consequences:
The component docs now describe the current runtime wiring, ownership boundaries, safety rules, and deferred work without changing public behavior. Any substantial route decomposition, startup wiring, observability, or provider-runtime integration remains deferred to future scoped phases.

Affected contracts:
Server component contract only. No cross-component contract changes requested.

## 2026-07-05 — Server is runtime composition, not sync policy

Decision:

`haze-sync-server` owns runtime route composition, auth execution, transaction orchestration, readiness, app state, and safe internal-to-public error mapping. It does not own Core overwrite/conflict/delete/idempotency policy, API DTO vocabulary, Storage schema, or adapter/provider semantics.

Rationale:

Server is the component where real dependencies meet. If Server route code invents policy or public vocabulary, the system loses the separation that lets Core, API, and Storage remain testable and independently reviewable.

Alternatives:

- Put sync policy directly in route handlers.
- Let API own runtime handlers.
- Let Storage repositories decide route outcomes.

Consequences:

- Server fan-in phases must compose upstream contracts explicitly.
- Server tests should focus on integration behavior and safe mapping.
- Policy or DTO changes discovered during Server work require contract-change requests.

Affected contracts:

- component contract;
- dependency map;
- Core/API/Storage/Server fan-in phases;
- route implementation phases.

## 2026-07-05 — Runtime state is explicit; hidden globals are forbidden

Decision:

Server runtime dependencies are carried through explicit state such as `ServerAppState`, not hidden globals or implicit singleton initialization. Dependency-free router construction remains supported for safe route-shell tests.

Rationale:

Explicit state makes tests deterministic, keeps readiness honest, and prevents route construction from silently connecting to databases, object stores, or providers.

Alternatives:

- Global DB pool/object store initialization.
- Environment-driven singleton state inside route modules.
- Route construction that always requires production dependencies.

Consequences:

- Startup code must explicitly construct state from config.
- Dependency-free routes must fail safely rather than mutate state.
- Tests can verify route surfaces without production dependencies.

Affected contracts:

- state;
- routes;
- readiness;
- startup/config phases;
- server tests.

## 2026-07-05 — Server owns transaction orchestration for live mutations

Decision:

For live write/delete/conflict/idempotency flows, Server owns the transaction choreography across API inputs, Core decisions, Storage repositories, object store, path locks, operation log, and public response persistence.

Rationale:

Neither Core nor Storage alone owns enough runtime context to make multi-step durability decisions. Server is the correct fan-in point for request-scoped transactions and dependency coordination.

Alternatives:

- Let Core own SQL transactions.
- Let Storage repository helpers hide full service flows.
- Split one HTTP mutation across multiple independent transactions.

Consequences:

- Server mutation routes must be carefully tested.
- Path locks and idempotency records must be coordinated consistently.
- Failure injection around object-store-before-DB-commit and DB-after-object-write remains important.

Affected contracts:

- file PUT/GET/changes routes;
- conflict/delete routes;
- Storage repository usage;
- Core decision mapping;
- E2E/failure-injection tests.

## 2026-07-05 — Dependency-free router behavior is not production readiness

Decision:

`routes::build_router()` may expose route surfaces without runtime dependencies for tests and scaffolding, but protected routes must fail safely and this mode must not be described as production-ready sync behavior.

Rationale:

Dependency-free construction is useful for verifying route registration and safe errors. It does not prove that DB/object-store/auth/idempotency/operation-log behavior is live.

Alternatives:

- Remove dependency-free router construction.
- Treat route-shell success as production readiness.
- Hide missing dependencies behind mock mutations.

Consequences:

- Tests and docs must distinguish route-shell coverage from runtime-backed integration tests.
- `/ready` must report disabled/missing dependencies honestly.
- Future production startup must construct explicit runtime state.

Affected contracts:

- routes;
- readiness;
- tests;
- README/system docs;
- deployment/runbook planning.

## 2026-07-05 — Provider runtimes are not embedded in generic Server routes

Decision:

Server must not contain Google Drive provider calls, Obsidian plugin internals, or Worktree scanner/materializer logic in generic Core API routes. Worktree may be hosted by Server only through a dedicated composition fan-in that preserves Worktree ownership.

Rationale:

Server is the HTTP/runtime fan-in boundary, not the owner of provider-specific logic. Embedding provider behavior in route modules would couple unrelated components and make adapter safety harder to review.

Alternatives:

- Put all adapter runtimes inside Server immediately.
- Let Server route modules call Google Drive directly.
- Move Worktree scanner/materializer code into Server.

Consequences:

- Adapter components need their own contracts and plans.
- Server can expose provider-neutral status and compose accepted runtimes later.
- Worktree hosting requires explicit startup/shutdown/status design.

Affected contracts:

- Worktree component;
- GDrive adapter component;
- Obsidian plugin component;
- Server startup/runtime phases;
- deployment topology.

## 2026-07-05 — Admin/status surfaces are read-only until explicitly scoped

Decision:

Current admin/status routes are read-only operational surfaces. Mutations such as pause/resume, adapter mode changes, delete unlocks, token rotation, repair, or cleanup require explicit future contracts.

Rationale:

Admin mutation routes can change propagation, auth, or data safety. They need clear Core/API/Storage/Server/CLI contracts and operational docs before implementation.

Alternatives:

- Add admin mutations opportunistically to status routes.
- Let CLI mutate server state without API/server contracts.
- Hide mutation behavior behind query parameters or status endpoints.

Consequences:

- Status/admin DTOs can be stabilized safely first.
- Mutating operations are deferred to dedicated phases.
- Public status output remains less risky and easier to audit.

Affected contracts:

- admin routes;
- API admin/status DTOs;
- CLI status/doctor/admin commands;
- Core delete unlock/repair semantics;
- operational runbook.
