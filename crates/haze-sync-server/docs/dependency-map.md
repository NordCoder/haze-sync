# Dependency Map: server

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
- idempotency request fingerprint and stored response wrappers.

Server must not replace Core conflict/delete/revision policy with route-local policy.

### `haze-sync-storage`

Server depends on Storage for persistence and runtime data access:

- PostgreSQL repositories for sync objects, revisions, operation log, tombstones, conflicts, idempotency, and admin/status reads;
- PostgreSQL advisory/path locks;
- migration runner and database ping helpers;
- local content-addressed object-store primitives;
- test database support where enabled.

Server owns transaction orchestration around Storage calls, but not schema design or repository contracts.

### `haze-sync-common`

Server depends on Common for shared domain/value types:

- adapter identifiers and roles;
- revision, operation, tombstone, conflict, content-hash, and vault-path identifiers;
- adapter mode values used in config/status surfaces.

Server must preserve Common validation semantics.

### External crates

Server uses external crates for HTTP/runtime glue and serialization:

- `axum` for router/handler composition;
- `sqlx` for PostgreSQL pools, transactions, and query execution;
- `serde`/`serde_json` for safe JSON responses and idempotency payload storage;
- `chrono` for timestamps and tombstone retention dates;
- `sha2` for deterministic internal identifiers and idempotency metadata;
- `tower`, `tokio`, and `http-body-util` in server tests.

## Downstream dependents

Current and future dependents of Server behavior include:

- API clients used by adapters and tests;
- Obsidian plugin HTTP client code;
- Google Drive adapter Core-client code;
- worktree runtime integration;
- CLI/admin/doctor commands that call or inspect server status;
- E2E tests and deployment/runbook procedures.

These dependents rely on Server to keep public responses stable, sanitized, and aligned with API DTOs.

## Cross-component contracts

- API owns request/response DTO shape and header parsing semantics.
- Core owns conflict/delete/revision/idempotency policy semantics.
- Storage owns migrations, repository behavior, database locks, and object-store primitives.
- Common owns shared value-type validation.
- Server owns runtime route composition, authentication execution, transaction boundaries, dependency readiness, and safe internal-to-public error mapping.
- Adapter/provider components own provider-specific runtimes and must not be implemented in Server unless a future fan-in explicitly scopes that wiring.

## Forbidden dependency directions

Server must not depend on provider runtime internals for normal Core API route behavior:

- no direct Google Drive provider calls;
- no Obsidian plugin runtime logic;
- no worktree watcher/importer/exporter loops unless explicitly scoped;
- no background adapter jobs unless a future server/runtime fan-in owns them.

Server must not introduce hard-delete dependencies or filesystem/provider deletion side effects unless a future contract explicitly scopes retention-expiry behavior.

## Contract-change notes

No contract changes requested by T0-P3.
