# Dependency Map: worktree

## Component role in dependency graph

`haze-sync-worktree` is the built-in filesystem adapter for the VPS worktree.

Conceptual position:

```text
Core/API authoritative state
  -> Worktree materializer
  -> local filesystem worktree
  -> Worktree scanner/import planner
  -> Core/API write/delete facts
```

The worktree is a materialized replica, not source of truth.

## Upstream dependencies

### Required conceptual dependencies

- `haze-sync-common`
  - `VaultPath`;
  - content hash values;
  - adapter id/mode primitives;
  - validation vocabulary.
- `haze-sync-api`
  - public write/delete/change/conflict/status DTOs;
  - headers such as idempotency key, content hash, base revision;
  - safe public error vocabulary.
- `haze-sync-core`
  - authoritative safety semantics observed through API outcomes;
  - no-silent-overwrite and conflict/delete behavior.
- `haze-sync-storage`
  - persisted `worktree_state` shape/repositories if integration chooses Storage-backed state.
- `haze-sync-server`
  - runtime hosting/composition if Worktree runs inside Server in V1;
  - API endpoint provider if Worktree communicates over HTTP/internal client boundary.

### Current direct dependencies

Current code is placeholder-only and has no crate dependencies beyond workspace/lints.

Future direct dependencies should be added only inside scoped Worktree implementation phases and reflected in this map.

### External dependency categories that may be needed later

Potential future dependencies, subject to implementation review:

- filesystem walking;
- temporary directory/test filesystem support;
- file watching as latency hint;
- async runtime traits if runtime service is implemented;
- hashing/IO helpers;
- HTTP client only if Worktree communicates with Server over public API rather than internal service traits.

These must not introduce provider SDKs, direct DB ownership, or hidden background lifecycle.

## Disallowed direct dependencies

Worktree must not directly depend on:

```text
haze-gdrive-adapter
apps/haze-obsidian-plugin
Google Drive provider SDKs
Obsidian plugin internals
SQLx/Storage DB access unless explicitly accepted
Axum route registration/server startup unless explicitly accepted
CLI parser/command execution as Worktree logic
```

Worktree may use filesystem APIs because local filesystem behavior is the component's core responsibility. Those APIs must be root-contained and path-safe.

## Downstream dependents

Expected downstream dependents:

### `haze-sync-server`

Server may host Worktree runtime in V1.

Server owns:

- startup/shutdown composition;
- config loading;
- app lifecycle;
- status/readiness routing;
- integration with Storage/API/Core runtime dependencies.

Worktree owns:

- scanner;
- importer/planner;
- materializer/writer;
- echo guard;
- local trash/repair behavior;
- worktree-specific status/doctor facts.

### `haze-sync-cli`

CLI may eventually trigger or inspect worktree status, doctor, repair plans, or one-shot scan/materialize operations through Server or direct component APIs when scoped.

CLI owns command UX and confirmation flows.

### E2E tests and deployment/runbooks

E2E tests and runbooks depend on Worktree to materialize one local filesystem view of the vault under the configured VPS path.

## Cross-component contracts

### Common ↔ Worktree

- Common owns `VaultPath`, hash, adapter id/mode, and validation semantics.
- Worktree maps filesystem paths to/from `VaultPath` and must not fork Common validation.
- Worktree should represent public/internal paths as vault-relative values whenever possible.

### API ↔ Worktree

- API owns request/response/header vocabulary for file writes, deletes, changes, conflicts, and status.
- Worktree submits facts through API-compatible contracts or internal client traits modeled on API contracts.
- Worktree must preserve idempotency key, content hash, and base/null-base semantics.

### Core ↔ Worktree

- Core owns conflict/delete/revision policy.
- Worktree submits local facts and obeys Core/API outcomes.
- Worktree must not decide overwrite/conflict/delete policy locally.

### Storage ↔ Worktree

- Storage may own persisted `worktree_state` row/repository support.
- Worktree owns the meaning and use of scan/materialization/dirty/echo state.
- Direct DB writes from Worktree require explicit architecture approval; Server-mediated access is preferred unless a later decision changes this.

### Server ↔ Worktree

- Server may host Worktree runtime through explicit fan-in.
- Server owns process lifecycle, config loading, and route/status exposure.
- Worktree owns runtime behavior internals.
- Server must not reimplement Worktree scanner/materializer logic.

### GDrive/Obsidian ↔ Worktree

- Worktree must not depend on Google Drive or Obsidian internals.
- Worktree and those adapters meet through Core/API authoritative state, not direct peer synchronization.

## Integration/fan-in ownership

The following work belongs to Server or cross-component fan-in phases, not Worktree leaf phases alone:

- hosting Worktree runtime inside Server;
- wiring Worktree status into `/ready` or admin/status routes;
- persisting worktree state through Storage repositories;
- E2E tests across Server/Core/API/Storage/Worktree;
- deployment path provisioning and permissions;
- operational runbook updates for worktree repair or rollback.

## Dependency rules

- Worktree may own filesystem logic only inside the configured root.
- Worktree must not write directly to Core database tables unless explicitly accepted.
- Worktree must not call Google Drive or Obsidian APIs.
- Worktree must not decide conflict/delete policy locally.
- Watchers may improve latency but scans remain the correctness mechanism.
- Any dependency addition must be justified by the implementation phase and reflected here.

## Contract-change notes

Current known contract questions:

1. Worktree state persistence boundary
   - Storage has `worktree_state` row shape.
   - Worktree may need repository access through Server or direct internal traits.
   - Direct DB ownership is not assumed by default.

2. Server-hosted runtime boundary
   - V1 architecture allows Worktree runtime inside Server.
   - Worktree logic must remain in Worktree crate.
   - Server integration requires a dedicated fan-in phase.

3. Reserved path policy
   - Worktree needs reserved directories for temp/trash/echo/runtime metadata.
   - Core conflict materialization uses `_haze_conflicts/**` and should not be accidentally ignored if it must be materialized.

4. Symlink policy
   - Default should be conservative: do not follow symlinks unless explicitly accepted.
   - This affects scanner and path escape safety.

5. Local trash/retention semantics
   - Worktree delete behavior must align with Core tombstones and system retention policy.
   - Immediate hard delete is out of scope without future contract.

No immediate blocking contract change is required for the current documentation/planning pass.
