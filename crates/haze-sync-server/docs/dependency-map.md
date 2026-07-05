# Dependency Map: server

## Component role in dependency graph

`haze-sync-server` is the runtime composition and HTTP execution boundary.

It sits above API/Core/Storage/Common and below external clients/adapters:

```text
API contracts + Core decisions + Storage persistence + Common values
  -> Server runtime wiring, auth execution, transactions, readiness, errors
  -> HTTP clients: Obsidian plugin, GDrive adapter, Worktree/runtime, CLI, E2E
```

Server owns runtime route composition, authentication execution, transaction boundaries, dependency readiness, and safe internal-to-public error mapping.

Server does not own public DTO vocabulary, Core policy, Storage schema/repositories, adapter/provider behavior, or CLI UX.

## Upstream dependencies

### `haze-sync-api`

Server depends on API for public HTTP contract surfaces:

- auth primitives: bearer token parsing, adapter principal, and role capability checks;
- header constants: Authorization, Idempotency-Key, X-Content-SHA256, X-Base-Revision-Id, response headers;
- route parsers for file, changes, delete, conflict, and admin surfaces;
- DTOs for server-info, changes, file upload/download/delete responses, conflicts, admin/status, and safe error responses;
- stable public error-code semantics.

Server must not fork or silently diverge from API parsing/DTO contracts.

### `haze-sync-core`

Server depends on Core for sync policy primitives:

- revision upsert planning and outcomes;
- conflict-saved planning and conflict policy representation;
- tombstone creation primitives;
- delete guard evaluation;
- idempotency request fingerprint and stored response wrappers;
- operation/change/cursor semantics where mapped into runtime behavior;
- passive doctor/status models where accepted.

Server must not replace Core conflict/delete/revision/idempotency policy with route-local policy.

### `haze-sync-storage`

Server depends on Storage for persistence and runtime data access:

- PostgreSQL repositories for sync objects, revisions, operation log, tombstones, conflicts, idempotency, adapter cursors, and admin/status reads;
- PostgreSQL advisory/path locks;
- migration/readiness helpers when accepted;
- local content-addressed object-store primitives;
- test database support where enabled.

Server owns transaction orchestration around Storage calls, but not schema design or repository contracts.

### `haze-sync-common`

Server depends on Common for shared domain/value types:

- adapter identifiers and roles;
- revision, operation, tombstone, conflict, content-hash, and vault-path identifiers;
- adapter mode values used in config/status surfaces;
- safe validation primitives where needed.

Server must preserve Common validation semantics.

### External crates

Server uses external crates for HTTP/runtime glue and serialization:

- `axum` for router/handler composition;
- `sqlx` for PostgreSQL pools, transactions, and query execution;
- `serde`/`serde_json` for safe JSON responses and idempotency payload storage;
- `chrono` for timestamps and tombstone retention dates;
- `sha2` for deterministic internal identifiers and idempotency metadata;
- `tower`, `tokio`, and `http-body-util` in server tests.

External dependencies must not be used to smuggle provider runtime behavior, hard delete behavior, or public secret exposure into Server.

## Downstream dependents

Current and future dependents of Server behavior include:

- API clients used by adapters and tests;
- Obsidian plugin HTTP client code;
- Google Drive adapter Core-client code;
- Worktree runtime integration if Server hosts the Worktree loop;
- CLI/admin/doctor commands that call or inspect server status;
- E2E tests and deployment/runbook procedures.

These dependents rely on Server to keep public responses stable, sanitized, and aligned with API DTOs.

## Cross-component contracts

### API ↔ Server

- API owns request/response DTO shape, header parsing semantics, auth value types, and public error vocabulary.
- Server owns Axum extraction, route registration, middleware/auth execution, app state, Core/Storage calls, and final HTTP response construction.
- Server must not change public API vocabulary without updating API docs/tests.

### Core ↔ Server

- Core owns conflict/delete/revision/idempotency/operation semantics.
- Server owns orchestration: extracting API inputs, checking auth, opening transactions, locking paths, calling Core, persisting outcomes, and mapping responses.
- Server route code must not implement alternative overwrite, conflict, delete, or idempotency policy.

### Storage ↔ Server

- Storage owns migrations, repository helpers, object-store primitives, and locks.
- Server owns database pool lifecycle, runtime state, transaction boundaries, error mapping, and readiness checks.
- Server must not expose Storage row models directly as public DTOs.
- Server must map raw Storage/SQLx failures to safe public API errors.

### Common ↔ Server

- Common owns shared primitive validation.
- Server uses Common value types directly or through API/Core/Storage helpers.
- Server must not fork path/hash/ID/role/mode parsing semantics.

### Worktree ↔ Server

- Worktree owns scanner/writer/materializer/echo/trash/repair logic.
- Server may host or compose Worktree runtime only through explicit fan-in.
- Server must not move Worktree business logic into generic route modules.

### GDrive adapter ↔ Server

- GDrive adapter owns provider API, OAuth/token loading, mapping semantics, echo guard, import/export, change feed, and delete-candidate behavior.
- Server exposes provider-neutral Core/API endpoints and may mediate persistence/auth/runtime status.
- Server must not import Google Drive provider SDKs unless a future architecture decision explicitly changes the boundary.

### Obsidian plugin ↔ Server

- Obsidian plugin owns UI, local vault state, pending queue, local scanner, apply engine, and conflict UI.
- Server exposes HTTP API behavior and safe public status.
- Server must not implement Obsidian plugin internals.

### CLI ↔ Server

- CLI may consume Server APIs for status, doctor, bootstrap, or safe operator commands.
- Server owns runtime HTTP behavior; CLI owns command UX and local/operator workflows.
- Mutating CLI behavior requires corresponding Server/Core/API/Storage contracts.

## Forbidden dependency directions

Server must not depend on provider runtime internals for normal Core API route behavior:

- no direct Google Drive provider calls;
- no Obsidian plugin runtime logic;
- no worktree watcher/importer/exporter loops unless explicitly scoped as Server-hosted Worktree fan-in;
- no background adapter jobs unless a future server/runtime fan-in owns them;
- no CLI command parsing as server behavior.

Server must not introduce hard-delete dependencies or filesystem/provider deletion side effects unless a future contract explicitly scopes retention-expiry behavior with operational guardrails.

## Integration/fan-in ownership

Server is allowed to own fan-in work when the prompt explicitly scopes it, especially:

- route registration and HTTP status mapping;
- runtime config/state construction;
- DB pool and object-store composition;
- transaction boundaries across Core and Storage;
- path advisory lock discipline;
- idempotency replay orchestration;
- readiness/admin/status mapping;
- Worktree runtime composition if accepted by Worktree/server contracts.

Server should not hide fan-in in unrelated leaf phases when the change modifies API/Core/Storage/adapter contracts.

## Contract-change notes

Current known contract questions:

1. Production startup and migration policy
   - Server currently has scaffold `main` behavior and DB helpers.
   - Whether Server auto-runs migrations, refuses startup, or delegates to CLI/deployment must be decided before production startup work.

2. Worktree runtime hosting
   - System docs allow Worktree runtime to run inside Server in V1.
   - Server may host composition, but Worktree owns scanner/materializer logic.
   - A dedicated fan-in phase is required.

3. Adapter mode enforcement
   - Common/API can represent modes, and adapters/Server must enforce them.
   - Exact Server-side enforcement points need future integration design.

4. Admin mutations
   - Current admin/status surfaces are read-only.
   - Pause/resume, mode changes, token rotation, repair, or delete unlocks require explicit contracts.

5. Observability
   - Logging/metrics/tracing must be secret-safe.
   - Adding metrics/logs may involve deployment and CI/runbook updates.

No immediate blocking contract change is required for the current documentation/planning pass.
