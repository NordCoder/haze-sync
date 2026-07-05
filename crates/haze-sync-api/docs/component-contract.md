# Component Contract: api

## Responsibility

The `haze-sync-api` component owns public HTTP API contract types for Haze Sync:

- JSON DTOs for server-info, changes, files, delete, conflicts, and admin/status surfaces.
- Header contract helpers for `Authorization`, `Idempotency-Key`, `X-Content-SHA256`, and `X-Base-Revision-Id`.
- Passive route-contract helpers that parse request-shaped inputs and build response-shaped metadata.
- Adapter authentication value types, role checks, and token hash verification primitives.
- Sanitized public error response shapes and stable public error codes.

The API component is passive. It does not own Axum runtime wiring, route registration, database queries, object-store access, provider calls, background jobs, adapter execution, or broad Core service execution. Server owns runtime route wiring and request handling. Core owns sync policy and state transitions.

## Public interfaces

Public Rust module surface:

- `auth` — adapter principal, API-local adapter role, bearer-token wrapper, token-hash representation, and pure verifier trait/helper.
- `contracts::headers` — header names and validated header value wrappers.
- `contracts::errors` — safe JSON error response contract.
- `dto` — JSON-serializable DTOs for stable API response/request bodies.
- `routes` — passive route-level helpers for parsing and response/error shaping.

Covered V1 HTTP surfaces:

- `GET /v1/server-info`
- `GET /v1/changes?since=&limit=`
- `GET /v1/files/{path}`
- `PUT /v1/files/{path}`
- `DELETE /v1/files/{path}`
- `GET /v1/conflicts?status=open`
- `POST /v1/conflicts/{conflict_id}/resolve`
- passive admin/status DTOs for status and adapter-list output

The concrete HTTP router, middleware stack, runtime app state, and handler registration belong to `haze-sync-server`.

## Input contracts

API helpers may accept raw request-shaped inputs supplied by a future or existing server handler, but only as data:

- route path strings are normalized through `haze-sync-common::VaultPath` before becoming typed request metadata;
- revision, operation, conflict, adapter, and content-hash identifiers use common domain/value types where available;
- write/delete routes require `Idempotency-Key` metadata at the contract layer;
- write/delete routes require `X-Base-Revision-Id`, including the literal `null` when an adapter explicitly has no safe known base;
- upload routes require `X-Content-SHA256` in `sha256:<64 lowercase/uppercase hex>` shape;
- `Authorization: Bearer <token>` may be parsed or wrapped, but runtime authentication lookup is not performed in this component;
- conflict resolution accepts only the public action vocabulary: `accept_current`, `accept_conflict`, `keep_both`, `mark_resolved`.

Input parsing must not leak raw request bodies, bearer tokens, idempotency keys, provider payloads, database URLs, local filesystem paths, or internal stack traces in errors or debug output.

## Output contracts

DTOs and response helpers must preserve safe, deterministic JSON vocabulary for adapters and UI surfaces:

- server-info exposes only public capability metadata;
- changes responses expose operation sequence, operation kind, path, optional revision/hash/tombstone/conflict metadata, adapter id, and timestamp;
- file metadata exposes vault path, revision id, content hash, size, updater adapter id, and timestamp;
- PUT responses distinguish accepted, conflict-saved, ignored, and rejected outcomes;
- DELETE responses distinguish tombstoned, not-found, and rejected outcomes;
- conflict list/resolve DTOs expose conflict identifiers, original/materialized paths, revisions, source adapter, policy, status, and resolution status;
- admin/status DTOs expose sanitized readiness, adapter, cursor-presence, and pause-support summaries only.

The API component may define response builders from already-validated service output, but it must not create revisions, tombstones, conflict records, operation-log rows, or adapter cursor updates.

## Error contracts

Public errors must be safe and stable:

- top-level shape is `ErrorResponse { error: PublicError }`;
- public codes use `snake_case` JSON values;
- messages and details are sanitized public strings only;
- validation details may identify public field/header/query names, but not secret values.

Public errors must not expose secrets, raw provider payloads, database URLs, local absolute paths, stack traces, bearer tokens, OAuth tokens, token hashes, raw request bodies, file bytes, or `Idempotency-Key` values.

## Persistence/runtime ownership

The API component owns no persistence and no runtime orchestration.

Explicit non-ownership:

- no PostgreSQL queries or repository calls;
- no object-store reads/writes;
- no operation-log persistence;
- no tombstone creation or physical delete;
- no conflict policy execution;
- no adapter cursor mutation;
- no Google Drive, filesystem, Obsidian, or provider calls;
- no Axum router construction, middleware installation, listener startup, or background task spawning.

Runtime wiring is owned by `haze-sync-server`. Sync state transitions and safety policy execution are owned by `haze-sync-core`. Durable storage is owned by `haze-sync-storage`.

## Security and secrecy rules

- Do not commit secrets, production `.env` files, OAuth tokens, bearer tokens, token hashes, provider payloads, local logs/dumps, or generated artifacts.
- Redact token and token-hash formatting output.
- Do not serialize or log raw bearer tokens, idempotency keys, database URLs, local absolute paths, stack traces, raw provider payloads, raw external cursors, request bodies, or file bytes.
- Public admin/status output may indicate that a private external cursor exists, but must not expose the cursor itself.
- Public errors must use safe codes/messages/details only.
- API helpers may model authorization requirements, but token lookup and enforcement are server/runtime responsibilities.

## Non-goals

The API component must not implement:

- Axum route registration or HTTP listener startup;
- database pools, migrations, repositories, or queries;
- Core apply/delete/conflict/operation-log/idempotency service logic;
- content-addressed object-store writes or reads;
- provider integrations, Google OAuth, Drive calls, Obsidian plugin behavior, or worktree filesystem behavior;
- background jobs, watchers, scans, repair execution, or adapter loops;
- production token creation, token persistence, or admin mutation execution;
- broad runtime behavior hidden behind DTO or helper types.

## Dependencies

See `dependency-map.md`.

Primary dependency expectations:

- `haze-sync-common` supplies shared value/domain types and validation vocabulary.
- `haze-sync-core` may supply pure domain page/operation types for DTO mapping where already stable.
- `serde` supplies JSON serialization contracts.
- `sha2`, `hex`, and `subtle` support pure token hash/verification primitives.

## Dependents

See `dependency-map.md`.

Known dependents:

- `haze-sync-server` consumes route-contract helpers, DTOs, auth primitives, and safe error shapes when wiring runtime HTTP handlers.
- `haze-sync-cli` may consume admin/status DTOs indirectly through server APIs or shared output models.
- Adapter components and plugin code depend on stable public JSON semantics, but should not depend on API internals.

## Invariants

- API is passive; server owns runtime route wiring.
- API must not silently perform Core state transitions.
- API must not decide conflict/delete policy beyond public vocabulary and metadata contracts.
- Every write/delete contract preserves `base_revision_id` or explicit `base_revision_id = null` semantics.
- Unknown base revision must remain representable and must not imply overwrite permission.
- Delete output vocabulary must represent tombstone/guard semantics, never immediate hard delete.
- Conflict output vocabulary must preserve both sides and expose safe resolution actions only.
- Safe public errors must never expose internals or secrets.
- DTOs must remain JSON-serializable and deterministic for adapter clients.

## Test obligations

API tests should cover:

- serde roundtrips and stable `snake_case` JSON vocabulary;
- header parsing and rejection of malformed/unsafe header values;
- bearer token and token hash redaction;
- conversion between API DTO wrappers and common domain/value types;
- route helper parsing for path, query, headers, body metadata, and bounds;
- safe error code/status/detail mapping;
- absence of secrets, token hashes, raw cursors, database URLs, local paths, stack traces, provider payloads, request bodies, and raw file bytes in public/debug output;
- passive behavior boundaries: no DB, object-store, provider, router-registration, or background-job behavior inside API tests.

Connector-only workers must honestly report shell checks as not run unless CI metadata is observed.

## Risks

- DTO drift between API, server wiring, and adapter clients can break external clients even if Rust compiles.
- Route helper names can appear active; docs and tests must keep passive boundaries explicit.
- Adding Core conversions can accidentally pull runtime or storage concerns into API if not reviewed.
- Admin/status output is especially leak-prone because it summarizes runtime state.

## Deferred work

- Final cross-component route wiring remains server/fan-in work.
- Full integration tests for live HTTP behavior belong in server or cross-component E2E phases.
- Adapter/client contract tests may be added once adapter components consume these DTOs.
- Any future API contract expansion must update this contract, dependency map, and relevant server/adapter docs.

## Contract change protocol

If implementation would require serious hacks, unsafe behavior, or cross-component changes, the worker must report `CONTRACT_CHANGE_REQUESTED` / `BLOCKED_BY_CONTRACT` instead of silently broadening scope.
