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

As later components mature, Server should integrate them only through explicit fan-in prompts and component contracts.

## Waves

### T0-P3 — Server component docs plus tiny route cleanup

- Replace placeholder component documentation with current post-W3 server responsibilities.
- Record Server ownership of runtime wiring and safe internal-to-public error mapping.
- Record dependency boundaries against API/Core/Storage/Common.
- Record non-goals around provider runtimes, background jobs, and hard deletes.
- Add one process-test decision and one implementation-log entry.
- Perform at most one behavior-preserving server-local route cleanup.

### Existing baseline through W3

The codebase already contains server-local wiring from earlier wave outputs:

- W1/W2 server shell and readiness/config scaffolding.
- W2 normal file PUT/GET/changes route fan-in.
- W3 conflict API, DELETE tombstone, admin/status, and safety route fan-in.

T0-P3 documents this current state. It does not broaden runtime behavior.

### Future fan-in work

Future server work should be scheduled as fan-in/integration phases when other components are ready:

- production startup/listener wiring;
- explicit migration-run policy at startup or CLI boundary;
- full config-to-runtime bootstrap;
- worktree runtime integration;
- adapter mode enforcement in route/runtime composition;
- observability/metrics/logging hardening;
- deployment/runtime consistency checks;
- E2E acceptance wiring.

## Dependency gates

Server may integrate only stable upstream contracts:

- API route parsers, DTOs, header constants, auth primitives, and public error formats.
- Core revision/conflict/delete/tombstone/idempotency primitives.
- Storage migrations, repositories, locks, object-store primitives, and test support.
- Common value/domain types.

Server must stop or request a contract change when a requested change requires:

- modifying another component contract;
- inventing Core policy in route code;
- changing API DTO/error shapes outside API ownership;
- changing Storage schema/repository contracts outside Storage ownership;
- adding provider runtime behavior;
- adding hard-delete behavior.

## Known risks

- Route modules can become oversized because Server is an integration boundary. Refactors should preserve route contracts and remain server-local unless fan-in explicitly allows cross-component cleanup.
- Runtime transaction boundaries must stay aligned with Storage repository behavior and Core policy outcomes.
- Auth/database lookup failures must remain sanitized while still being operationally diagnosable through safe logs/metrics in later phases.
- Dependency-free route-shell behavior must not be mistaken for production readiness.
- Admin/status surfaces must remain read-only until mutation support is explicitly scoped.
- Object-store writes that happen before database commit require future failure-injection coverage and repair/doctor follow-up.

## Deferred work

- Production listener startup and graceful shutdown.
- Runtime config loading in `main`.
- Explicit migration runner policy at startup or CLI boundary.
- Structured tracing/logging initialization with secret redaction guarantees.
- Prometheus/metrics route if product scope requires it.
- Full adapter mode enforcement at server/runtime boundaries.
- Worktree runtime composition.
- Provider adapter orchestration remains outside Server until scoped fan-in.
- Broader route-module decomposition after behavior is stabilized and covered by tests.
- CI-backed verification for connector-only documentation updates.
