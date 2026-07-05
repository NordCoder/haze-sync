# Component Contract: server

## Responsibility

The server component owns HTTP runtime wiring for Haze Sync. It composes Axum routers, executes adapter/admin authentication, carries explicit runtime state, bridges public API route helpers to Core/Storage services, and maps internal failures to safe public responses.

Server is an integration component, not the source of sync policy. Core remains the conflict/delete/revision arbiter. API remains the owner of DTO shape and route parsing contracts. Storage remains the owner of database repositories, migrations, locks, and object-store primitives.

## Public interfaces

The server currently exposes these HTTP route surfaces through Axum router composition:

- `GET /health` — dependency-free process/router health.
- `GET /ready` — sanitized database/object-store readiness.
- `GET /v1/server-info` — static protocol and capability metadata.
- `GET /v1/changes` — authenticated operation-log changes feed wiring.
- `GET /v1/files/{path}` — authenticated file download wiring.
- `PUT /v1/files/{path}` — authenticated normal upsert/conflict-saved wiring.
- `DELETE /v1/files/{path}` — authenticated tombstone delete wiring.
- `GET /v1/conflicts` — authenticated conflict listing wiring.
- `POST /v1/conflicts/{id}/resolve` — authenticated conflict resolution wiring.
- `GET /v1/admin/status` — admin-only read-only server status summary.
- `GET /v1/admin/adapters` — admin-only sanitized adapter summary.

The server crate also exposes Rust construction points used by tests and future startup wiring:

- `routes::build_router()` for dependency-free route shell construction.
- `routes::build_router_with_state(ServerAppState)` for caller-supplied runtime dependencies.
- `routes::build_router_with_readiness(ReadinessState)` for readiness-shell tests.
- `ServerAppState` for DB pool, object store, config, and auth state.
- `ReadinessState` and readiness response types.
- config parsing/redaction types under `config`.
- database readiness/migration helpers under `db`.

## Input contracts

Server accepts already-normalized runtime dependencies from startup/fan-in code:

- optional PostgreSQL pool;
- optional local content-addressed object store;
- optional server configuration;
- explicit auth state: disabled, static test principal, or database-backed token lookup.

For HTTP requests, Server delegates canonical request validation to API route helpers wherever those helpers exist. It must preserve these API-owned contracts rather than reimplementing divergent validation:

- bearer authorization header shape;
- idempotency key extraction;
- content SHA-256 header extraction;
- base revision header extraction;
- vault path validation;
- changes query validation;
- delete request validation;
- conflict/admin DTO response shape.

Database-backed runtime routes require configured storage dependencies. Dependency-free construction is allowed for route-shell tests and must return safe authentication/storage/readiness failures instead of mutating state.

## Output contracts

Server responses must use API DTOs and stable public error payloads. Internal route wiring may call Core and Storage, but public responses must remain sanitized and deterministic.

For runtime-backed operations, Server is responsible for applying transaction boundaries around multi-step state changes:

- PUT file operations lock the vault path, inspect current revision state, execute Core upsert planning, persist content/revision/object/operation-log state, store successful idempotency responses, and commit atomically.
- DELETE file operations lock the vault path, check current revision/base semantics, evaluate delete guard input, create tombstone state, clear the current revision, append delete operation-log state, store idempotency responses, and commit atomically.
- Conflict-saved PUT outcomes materialize the incoming copy, insert conflict metadata, and append a conflict-created operation without overwriting the original current revision.

Server must not silently overwrite files, bypass Core conflict/delete policy, or expose adapter/provider-specific implementation details in generic route responses.

## Error contracts

Public errors must be safe and must not expose secrets, raw provider payloads, database URLs, local absolute paths, stack traces, bearer tokens, OAuth tokens, token hashes, Idempotency-Key values, request bodies, object-store roots, SQL text with secrets, or raw SQLx/provider error strings.

Server owns safe internal-to-public error mapping at the runtime boundary:

- authentication failures map to missing/invalid token or forbidden-role responses;
- unavailable runtime dependencies map to sanitized service-unavailable/internal responses;
- repository/object-store/Core failures map to stable public errors without raw internals;
- validation errors from API route helpers retain API-owned public error format.

## Persistence/runtime ownership

Server may own runtime wiring and transaction orchestration. It may call:

- `haze-sync-api` for auth primitives, DTOs, route parsers, and public error shapes;
- `haze-sync-core` for revision, conflict, delete guard, tombstone, and idempotency primitives;
- `haze-sync-storage` for database repositories, path locks, migrations/readiness helpers, and local object store access;
- `haze-sync-common` for shared domain/value types.

Server must not own database schema design, provider state models, adapter cursor semantics beyond safe read-only status summaries, or Core conflict/delete/revision policy decisions.

## Security and secrecy rules

- Do not commit secrets, production `.env` files, OAuth tokens, bearer tokens, token hashes, database URLs, generated dumps, or local logs.
- Do not expose raw token hashes, bearer tokens, OAuth tokens, idempotency values, database URLs, object-store paths, local absolute paths, stack traces, raw SQLx errors, or provider payloads in public HTTP responses, debug output, docs reports, or tests.
- Auth execution must treat configured token lookup failures as safe authentication/internal failures.
- Debug implementations for runtime state must redact database pools, object-store roots, and config values.
- Admin/status routes must remain read-only unless mutation is explicitly scoped.

## Non-goals

- No Google Drive, Obsidian, or worktree provider runtime behavior.
- No provider API calls.
- No adapter sync loops, watchers, exporters, importers, webhook handlers, or background jobs unless explicitly scoped.
- No production listener/bootstrap/deployment automation unless explicitly scoped.
- No hard delete of database rows, content blobs, object-store blobs, worktree files, or provider files unless explicitly scoped by a later contract.
- No route-wide refactor, public API redesign, or cross-component contract change without Orchestrator/Architect direction.
- No token creation/rotation CLI behavior inside the server component unless explicitly scoped.

## Dependencies

See `dependency-map.md`.

## Dependents

See `dependency-map.md`.

## Invariants

- Core is the only conflict/delete/revision policy arbiter.
- Server wires and persists Core outcomes; it does not invent replacement policy.
- Adapters never silently overwrite through Server routes.
- Every write route must require `base_revision_id` semantics as defined by API/Core contracts.
- Unknown/stale base revisions must not overwrite different current content.
- Delete routes create tombstones and retention metadata; they must not hard-delete.
- Conflict handling must preserve both sides by default where Core returns conflict-saved outcomes.
- Dependency-free router construction must be safe and non-mutating.
- Public outputs and debug output must remain sanitized.
- Runtime dependencies must be explicit; hidden globals must not be introduced.

## Test obligations

Server tests should cover:

- dependency-free router construction;
- `/health`, `/ready`, and `/v1/server-info` safe responses;
- auth-required route behavior without runtime dependencies;
- role authorization for file, conflict, delete, and admin routes;
- sanitized error bodies for internal/auth/storage failures;
- transaction-backed PUT/GET/changes behavior when a real test database is available;
- stale/unknown-base conflict-saved behavior at the server integration boundary;
- tombstone DELETE behavior, idempotent delete replay, and mass-delete guard behavior;
- admin/status sanitization and read-only behavior;
- absence of raw secrets, database URLs, local paths, token hashes, bearer tokens, and stack traces in public/debug output.

Connector-only workers must not claim shell checks passed unless those checks were actually run or CI metadata was observed.

## Contract change protocol

If implementation would require serious hacks, unsafe behavior, or cross-component changes, the worker must report `CONTRACT_CHANGE_REQUESTED` instead of silently broadening scope.
