# Dependency Map: storage

## Component role in dependency graph

`haze-sync-storage` is the durable persistence implementation component.

It sits below Server runtime composition and beside Core policy:

```text
common primitives
core decisions
  -> server transaction orchestration
  -> storage repositories/object store/locks
  -> API-safe outputs through server mapping
```

Storage owns how accepted facts are persisted. It does not own whether a sync operation should be accepted, conflicted, tombstoned, replayed, rejected, exported, or rendered publicly.

## Upstream dependencies

### Internal workspace dependencies

- `crates/haze-sync-common`
  - `VaultPath` for path-lock keys and repository/helper inputs where practical;
  - `ContentHash`/SHA-256 representation for object store and blob metadata;
  - IDs and validation primitives where repository APIs use typed inputs.

Contract expectation: Common owns validation of shared primitives. Storage may persist normalized string forms, but must not fork primitive validation semantics.

### External crate dependencies

Allowed current dependencies:

- `chrono`
  - timestamps in passive row models.
- `serde`
  - row/model serialization for tests/internal values.
- `serde_json`
  - JSON metadata fields such as cursor/response/metadata rows.
- `sha2`
  - object-store verification and path advisory-lock key derivation.
- `sqlx`
  - PostgreSQL repository helpers and transaction-scoped advisory locks.

Allowed current dev dependency:

- `tokio`
  - async tests where SQLx/test support requires runtime.

### Caller-provided dependencies

Storage helpers expect callers to provide:

- database pool/connection/transaction/executor;
- transaction boundaries;
- retry strategy;
- per-path lock discipline;
- object-store root configuration;
- Core decisions to persist;
- API/Server sanitization before public output;
- adapter/provider runtime behavior outside Storage.

## Disallowed direct dependencies

Storage must not directly depend on:

```text
haze-sync-api
haze-sync-server
haze-sync-worktree
haze-gdrive-adapter
haze-sync-cli
apps/haze-obsidian-plugin
axum route/router runtime ownership
provider SDKs
filesystem watcher crates
CLI parser crates
background job/runtime orchestration crates as application lifecycle
Google OAuth/token loading libraries as adapter behavior
```

Storage may use filesystem APIs inside `object_store` because local object-store implementation is Storage-owned. It must not use filesystem APIs to implement Worktree scanning/materialization behavior.

## Downstream dependents

Expected downstream components:

### `crates/haze-sync-server`

Consumes:

- repositories;
- object-store trait/implementation;
- advisory locks;
- safe storage errors;
- test support in integration contexts.

Server owns:

- database pool lifecycle;
- transaction orchestration;
- per-route request handling;
- Core/API/Storage mapping;
- HTTP status mapping;
- readiness checks and runtime configuration.

### `crates/haze-sync-core`

Core should not depend directly on Storage under the current contract.

Core may define traits/value models that Server or integration layers adapt to Storage repositories. Storage must not force Core to import SQLx or DB row models.

### `crates/haze-sync-api`

API should not depend directly on Storage.

Server maps Storage results/errors into API-safe DTOs and public errors.

### `crates/haze-sync-cli`

CLI may use Storage indirectly through Server APIs or explicit local diagnostic commands when scoped. CLI must not expose raw Storage row models, raw SQLx errors, database URLs, or local object-store paths publicly.

### `crates/haze-sync-worktree`

Worktree may need persisted worktree state through Server/Storage integration. Worktree owns filesystem scan/materialization semantics, not Storage.

### `crates/haze-gdrive-adapter`

GDrive may need persisted mapping/cursor state through Server/API/Storage integration. GDrive owns provider identity interpretation, OAuth, Drive API calls, echo guard, import/export, and delete-candidate semantics.

Direct GDrive adapter DB access requires an explicit architecture decision.

### `apps/haze-obsidian-plugin`

The plugin should not depend on Storage directly. It consumes Server/API behavior.

## Cross-component contracts

### Common ↔ Storage

- Common owns shared primitive validation.
- Storage may persist string forms of common values and should use typed values at helper boundaries where practical.
- Object-store path derivation uses `ContentHash`, not vault paths or provider payloads.
- Path locks use normalized `VaultPath` values.

### Core ↔ Storage

- Core owns semantic decisions and storage-agnostic plan/value models.
- Storage owns durable rows/repositories and object-store persistence.
- Server/integration layers adapt Core decisions to Storage writes inside transactions.
- Storage must not independently decide overwrite, conflict, tombstone, delete guard, idempotency conflict, or operation semantics.

### API ↔ Storage

- API owns public DTOs and errors.
- Storage owns row/repository values and safe storage errors.
- Server maps Storage outputs into API outputs.
- Storage rows must not be returned directly as public DTOs without sanitization.

### Server ↔ Storage

- Server owns pool creation, migrations execution when implemented, app state, routes, auth, transaction boundaries, idempotency orchestration, and error mapping.
- Storage owns reusable SQLx helpers, object-store operations, and locks.
- Server must provide transaction discipline for multi-step flows such as PUT, conflict preservation, delete/tombstone, idempotency, and cursor updates.

### Worktree/GDrive/Obsidian ↔ Storage

- Adapter/plugin components do not own Storage internals.
- Mapping/state persistence can be provided by Storage, but adapter semantics stay in adapter components.
- Provider payloads and external cursors must be sanitized before public output.

## Integration/fan-in ownership

The following work belongs to Server or dedicated integration/fan-in phases, not Storage-only leaf phases:

- mapping Core outcomes into multi-row transactions;
- route handlers;
- HTTP error/status mapping;
- object-store + metadata transaction choreography;
- idempotency replay middleware;
- provider import/export loops;
- Worktree scan/materialization runtime;
- Obsidian local client behavior;
- production migration execution;
- deployment directory provisioning;
- backup/restore orchestration.

## Dependency rules

- Keep Storage repository helpers transaction-friendly and caller-executor based.
- Keep Storage policy-free: persist facts, not sync decisions.
- Do not expose raw SQLx/database/object-store internals through public error formatting.
- Do not add Server/API/adapter dependencies to Storage.
- Do not add provider SDKs or OAuth behavior to Storage.
- Do not add Worktree scanner/materializer behavior to Storage.
- Any schema/migration/repository shape change must be reflected in component docs and reviewed before downstream fan-in depends on it.

## Contract-change notes

Current known contract questions:

1. GDrive mapping access boundary
   - Storage owns the `gdrive_mapping` table and may own repository helpers.
   - GDrive adapter should not gain direct DB ownership without a system decision.
   - Server/API may need to mediate mapping persistence for clean boundaries.

2. Worktree state access boundary
   - Storage owns persisted `worktree_state` shape if used.
   - Worktree owns scan/materialization/dirty/echo semantics.
   - Server may mediate runtime access.

3. Migration runner ownership
   - Storage owns migration metadata and migration files, but runtime migration execution may belong to Server/Deployment.
   - A future decision is required before adding a migration runner here.

4. Retention cleanup and hard delete
   - Storage can persist tombstone/retention metadata.
   - Physical cleanup/hard delete requires explicit future contract and operational guardrails.

No immediate blocking contract change is required for the current documentation/planning pass.
