# Dependency Map: api

## Component role in dependency graph

`haze-sync-api` is the public contract layer for Haze Sync HTTP surfaces.

It sits between domain/safety semantics and runtime execution:

```text
common/core semantic values
  -> api public DTO/header/error/helper contracts
  -> server runtime route wiring
  -> adapters/plugin/CLI clients through HTTP JSON
```

API owns public vocabulary. It does not own runtime behavior.

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

Contract expectation: Common owns canonical domain validation. API may wrap these values for JSON contract stability and may provide conversions, but must not fork validation semantics.

### `crates/haze-sync-core`

Used only as pure domain/context input where current route helpers map stable Core operation-log page values, conflict/delete semantics, or other already-produced Core values into API DTOs.

Contract expectation: Core owns sync policy, operation-log semantics, conflict resolution behavior, delete/tombstone safety, idempotency behavior, and state transitions. API may expose DTO vocabulary or pure mapping from already-produced Core values, but must not execute Core behavior.

### Third-party crates

Allowed current dependencies:

- `serde` — JSON serialization/deserialization contracts.
- `sha2` and `hex` — pure token hash helper implementation.
- `subtle` — constant-time token hash comparison.

These dependencies must not be used to introduce runtime storage, provider, filesystem, network, or background behavior.

## Disallowed direct dependencies

API must not directly depend on:

```text
haze-sync-storage
haze-sync-server
haze-sync-worktree
haze-gdrive-adapter
haze-sync-cli
apps/haze-obsidian-plugin
axum route/router runtime ownership
sqlx
provider SDKs
filesystem watcher crates
background job/runtime orchestration crates
config/env loading as runtime behavior
```

API may use pure libraries when needed for serialization, parsing, validation, or safe cryptographic comparison helpers, but any dependency addition must be contract-reviewed when it affects public behavior or introduces runtime coupling.

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

API must not bypass Server by registering routes or owning runtime state.

### `crates/haze-sync-cli`

May consume public admin/status models directly or indirectly through server API responses.

CLI owns command UX and command execution. API only owns stable DTO vocabulary.

### Adapter and plugin components

Expected future consumers:

- `crates/haze-sync-worktree`
- `crates/haze-gdrive-adapter`
- `apps/haze-obsidian-plugin`

Adapters/plugins consume the public HTTP JSON/header contract. They must not rely on private API internals. API must not implement adapter loops, provider calls, worktree scans, Obsidian behavior, or client-specific policy.

### E2E and compatibility tests

E2E and compatibility tests may use API DTOs and fixtures to assert public behavior. They must not treat API helper tests as proof that Server runtime behavior or Storage durability exists.

## Cross-component contracts

### API ↔ Common

- API DTO wrappers preserve public JSON strings while converting to/from Common value types.
- Common remains the canonical owner for vault path, identifier, content hash, role, and mode validation.
- If Common validation changes public acceptance/rejection behavior, API docs/tests must be updated.

### API ↔ Core

- API exposes request/response vocabulary for Core outcomes such as accepted writes, same-content no-ops, conflict-saved results, tombstoned deletes, unsafe delete rejection, idempotency replay/conflict, operation-log changes, and conflict resolution.
- Core owns whether those outcomes occur.
- API must not make overwrite, preserve-both, tombstone-retention, mass-delete, idempotency-storage, operation-log persistence, or conflict-resolution decisions.
- API mappings from Core models must remain pure and safe.

### API ↔ Server

- API owns passive helper functions and DTO/error shapes.
- Server owns Axum extraction, route registration, middleware, auth lookup, Core/storage calls, and final HTTP response construction.
- API errors must be safe enough for Server to return publicly.
- Server may decide HTTP status codes using API error/DTO vocabulary, but API must not start listening or register routes.

### API ↔ Storage

No direct dependency is allowed under the current contract. Storage queries, models, migrations, transactions, and repository errors must not enter API directly.

Storage-originated failures must be mapped by Server/API boundary code into sanitized public errors without exposing raw SQLx/database details.

### API ↔ Worktree/GDrive/Obsidian

No direct provider or adapter runtime dependency is allowed.

API may expose provider-neutral public DTOs such as adapter identifiers, adapter roles, adapter modes, cursor-presence summaries, capability flags, conflict metadata, and safe status summaries.

Adapter-specific behavior remains in the adapter/plugin components.

### API ↔ CLI

CLI may render API DTOs and status payloads. CLI must not assume DTO existence grants permission to execute destructive actions.

Mutating CLI behavior requires corresponding Core, Server, Storage, and operational contracts.

## Integration/fan-in ownership

The following work belongs outside API leaf phases:

- Axum route registration;
- HTTP middleware and runtime auth lookup;
- DB/object-store/Core service calls;
- transaction boundaries;
- idempotency record persistence;
- operation-log queries;
- conflict row and tombstone row persistence;
- provider import/export;
- Worktree scanning/materialization;
- Obsidian local vault behavior;
- CLI command execution;
- production E2E tests.

## Dependency rules

- Keep API dependencies pure and contract-oriented.
- Do not add DB, filesystem, HTTP server runtime, provider SDK, OAuth, watcher, or background-job dependencies to `haze-sync-api`.
- Do not expose storage/provider/Core internal error types through public DTOs.
- Do not serialize raw external cursors, token hashes, bearer tokens, idempotency keys, request bodies, file bytes, database URLs, local filesystem paths, or provider payloads.
- Any dependency addition must be justified in the implementation report and reflected here.

## Contract-change notes

Current known contract questions:

1. Null base representation
   - Current contract requires `X-Base-Revision-Id: null` to represent explicit unknown/no safe base for write/delete routes.
   - If clients cannot reliably send this value, API/Core/Server/adapter contracts must be changed together.

2. Public conflict resolution vocabulary
   - Current public actions are `accept_current`, `accept_conflict`, `keep_both`, and `mark_resolved`.
   - Adding policy-like actions such as latest-wins or incoming-wins requires Core and system safety review.

3. Capability advertisement
   - API can define capability DTO fields, but Server must avoid overclaiming production behavior that is not actually wired.

4. Cross-language fixtures
   - Obsidian plugin and adapters need stable JSON expectations.
   - API should likely add fixtures before heavy client integration.

No immediate blocking contract change is required for the current documentation/planning pass.

The current documentation codifies an existing design decision: API remains passive and `haze-sync-server` owns runtime route wiring.
