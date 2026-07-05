# Dependency Map: api

## Upstream dependencies

### `crates/haze-sync-common`

Used for shared value/domain primitives and validation vocabulary:

- `VaultPath`
- `AdapterId`
- `RevisionId`
- `OperationId`
- `ConflictId`
- `ContentHash` / SHA-256 parsing and formatting
- `AdapterRole`
- `AdapterMode`
- `ValidationError`
- secret-redaction helpers used by auth primitives

Contract expectation: common owns canonical domain validation. API may wrap these values for JSON contract stability and may provide conversions, but must not fork validation semantics.

### `crates/haze-sync-core`

Used only as pure domain/context input where current route helpers map stable Core operation-log page values into API DTOs.

Contract expectation: Core owns sync policy, operation-log semantics, conflict resolution behavior, delete/tombstone safety, idempotency behavior, and state transitions. API may expose DTO vocabulary or pure mapping from already-produced Core values, but must not execute Core behavior.

### Third-party crates

- `serde` — JSON serialization/deserialization contracts.
- `sha2` and `hex` — pure token hash helper implementation.
- `subtle` — constant-time token hash comparison.

These dependencies must not be used to introduce runtime storage, provider, filesystem, network, or background behavior.

## Downstream dependents

### `crates/haze-sync-server`

Consumes:

- route-contract helpers;
- public DTOs;
- auth primitives and role metadata;
- safe public error shapes.

Server owns:

- Axum router registration;
- middleware and runtime authentication lookup;
- request handling;
- app state and dependency injection;
- DB/object-store/Core service wiring;
- HTTP status/header/body emission.

API must not bypass server by registering routes or owning runtime state.

### `crates/haze-sync-cli`

May consume public admin/status models directly or indirectly through server API responses.

CLI owns command UX and command execution. API only owns stable DTO vocabulary.

### Adapter and plugin components

Expected future consumers:

- `crates/haze-sync-worktree`
- `crates/haze-gdrive-adapter`
- `apps/haze-obsidian-plugin`

Adapters/plugins consume the public HTTP JSON/header contract. They must not rely on private API internals. API must not implement adapter loops, provider calls, worktree scans, Obsidian behavior, or client-specific policy.

## Cross-component contracts

### API ↔ Common

- API DTO wrappers preserve public JSON strings while converting to/from Common value types.
- Common remains the canonical owner for vault path, identifier, content hash, role, and mode validation.
- If Common validation changes public acceptance/rejection behavior, API docs/tests must be updated.

### API ↔ Core

- API exposes request/response vocabulary for Core outcomes such as accepted writes, same-content no-ops, conflict-saved results, tombstoned deletes, unsafe delete rejection, and conflict resolution.
- Core owns whether those outcomes occur.
- API must not make overwrite, preserve-both, tombstone-retention, mass-delete, idempotency-storage, or operation-log persistence decisions.

### API ↔ Server

- API owns passive helper functions and DTO/error shapes.
- Server owns Axum extraction, route registration, middleware, auth lookup, Core/storage calls, and final HTTP response construction.
- API errors must be safe enough for server to return publicly.

### API ↔ Storage

No direct dependency is allowed under the current contract. Storage queries, models, migrations, transactions, and repository errors must not enter API directly.

### API ↔ Providers/adapters

No direct provider dependency is allowed. API may expose generic adapter identifiers, roles, modes, cursor-presence summaries, and provider-neutral public DTOs only.

## Dependency rules

- Keep API dependencies pure and contract-oriented.
- Do not add DB, filesystem, HTTP server runtime, provider SDK, OAuth, watcher, or background-job dependencies to `haze-sync-api`.
- Do not expose storage/provider/core internal error types through public DTOs.
- Do not serialize raw external cursors, token hashes, bearer tokens, idempotency keys, request bodies, file bytes, database URLs, or local filesystem paths.
- Any dependency addition must be justified in the implementation report and reflected here.

## Contract-change notes

No contract changes requested in T0-P2.

The current documentation codifies an existing design decision: API remains passive and `haze-sync-server` owns runtime route wiring.
