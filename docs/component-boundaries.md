# Component boundaries

## Boundary principle

Haze Sync uses component-bounded development.

A component owns a coherent responsibility area. Implementation phases may modify any file inside the component when that is necessary for a clean implementation, but they must not silently change sibling components.

Cross-component changes require an explicit contract change or a dedicated integration/fan-in phase.

## Dependency direction

Preferred dependency direction:

```text
common
  -> core
  -> api/server/storage/cli/adapters by contract

api
  -> server/clients by public DTO contract

storage
  -> server/core through repository interfaces or explicit composition

server
  -> api + core + storage + common

adapters/plugin/cli
  -> public API/client contracts, not server internals
```

The dependency graph should stay as flat as possible. Parallel component work is allowed because components depend on contracts, mocks, and local fakes rather than unfinished sibling implementations.

## Global forbidden dependency patterns

These patterns should be treated as architecture violations unless an Architect explicitly changes the contract:

```text
core -> sqlx / axum / filesystem object-store implementation
api -> storage / provider SDKs / runtime service state
storage -> core policy decisions / adapter provider logic
server -> Google Drive provider internals
server -> Obsidian plugin internals
worktree -> server route internals
gdrive-adapter -> Obsidian internals
obsidian-plugin -> Google Drive API
gdrive-adapter -> direct DB writes without an explicit system decision
cli -> destructive behavior without backup/doctor/delete guard contracts
adapters -> silent overwrite/delete decisions
```

## Component ownership

### common

Owns shared primitives used across the repository:

- normalized vault paths;
- IDs;
- content hashes;
- adapter IDs, roles, and modes;
- safe public error primitives;
- security value types that do not expose secrets.

Must not own runtime behavior, persistence, HTTP routing, provider calls, or sync policy.

### core

Owns deterministic sync decisions:

- normal revision semantics;
- stale and unknown base behavior;
- conflict policy decisions;
- tombstone/delete guard semantics;
- idempotency semantics;
- operation outcome models;
- doctor summary models;
- safety invariants.

Core should be deterministic and side-effect-light. It must not own Axum routes, SQLx repositories, provider SDK calls, filesystem watchers, deployment, or plugin behavior.

### api

Owns public HTTP/API contract shapes:

- DTOs;
- header extraction contracts;
- safe error payloads;
- passive route helper contracts;
- public action vocabulary.

API must not own runtime wiring, database transactions, provider behavior, Core policy, or long-running jobs.

### storage

Owns durable persistence implementation:

- migrations;
- PostgreSQL models and repositories;
- object-store implementation;
- test support for real database/storage checks;
- storage-specific error classification before Server maps it outward.

Storage must not decide conflict policy, delete safety, adapter modes, or public HTTP behavior.

### server

Owns runtime composition:

- Axum router construction;
- app state;
- auth execution;
- DB/object-store readiness;
- error mapping;
- transaction boundaries;
- service wiring;
- route registration;
- runtime configuration loading when scoped.

Server may compose Worktree runtime later, but should not own Worktree scanner/writer/importer internals. Server should not contain Google Drive provider code or Obsidian plugin logic.

### worktree

Owns local filesystem adapter logic:

- layout and ignore rules;
- full scanner;
- stable file detection;
- atomic writer;
- echo guard;
- trash path handling;
- materializer/importer primitives;
- repair dry-run models.

Worktree must not treat the filesystem as authoritative state. It reports local facts to Core/API-facing contracts and obeys Core outcomes.

### gdrive-adapter

Owns Google Drive provider adapter behavior:

- config and OAuth token loading interface;
- Drive client abstraction;
- full scan and changes feed consumers;
- import/export planners;
- mapping semantics;
- echo guard;
- provider delete candidate detection;
- reconciliation and dry-run reports.

Google Drive adapter must not implement Core overwrite/conflict/delete policy. It should not expose raw provider payloads or secrets in public output.

A dedicated integration decision is required before allowing direct persistent DB access from this component. The cleaner default is to interact through public Core/API contracts and keep storage repositories behind Server composition.

### obsidian-plugin

Owns Obsidian-side client behavior:

- settings UI;
- token storage through Obsidian plugin data;
- API client;
- local state;
- local scanner;
- pending queue;
- sync planner;
- apply engine;
- event watcher and debounce;
- conflict UI.

The plugin must not call Google Drive directly and must not claim iOS background reliability. It must treat server responses as authoritative and keep offline work pending rather than inventing policy locally.

### cli

Owns operator-facing commands:

- status;
- adapters list/mode visibility;
- doctor;
- dry-run scans;
- bootstrap wrappers;
- backup/restore wrappers when scoped.

CLI commands must be safe by default. Destructive or propagation-changing commands require explicit contract support, guardrails, and operational docs.

### deployment

Owns deployment scaffolding:

- Compose files;
- reverse proxy skeleton;
- directory layout;
- environment templates;
- healthcheck configuration;
- operational deployment docs.

Deployment must not contain secrets, real OAuth tokens, production environment files, or hidden automation that mutates provider state.

### github-ci

Owns CI workflow behavior:

- Rust checks;
- TypeScript plugin checks;
- Docker Compose config validation;
- component branch signal quality;
- artifact/log accessibility where useful.

CI must not require production secrets, live Google Drive OAuth, live deployment credentials, or external provider access.

### docs-process

Owns process documentation if process docs are versioned in the repository.

Product architecture docs should remain separate from process docs. Process docs may describe component/control-folder development mechanics, but should not redefine product runtime behavior.

## Boundary decisions

### Worktree built into Server runtime

V1 may run Worktree runtime inside `haze-sync-server`, but implementation ownership should remain split:

```text
worktree component
  owns scanner/writer/importer/materializer logic

server component
  owns runtime composition, state, route/service integration, and lifecycle
```

This preserves deployment simplicity without collapsing component boundaries.

### Google Drive mapping

Google Drive mapping has three distinct concerns:

```text
gdrive-adapter
  mapping semantics, provider file identity, reconciliation decisions

storage
  persisted table/repository shape

server/api
  runtime access boundary and safe public/service interface
```

The adapter should not become a database service unless a later architecture decision explicitly accepts that coupling.

### API route helpers vs Server runtime

API helpers define public shape and validation contracts. Server owns live route composition and service execution.

If an API module needs a database pool, object store, runtime state, or provider client, it has crossed into Server/component integration scope.

### Storage vs Core policy

Storage may enforce database constraints and transactional integrity. It must not independently decide whether a stale write is accepted, whether a conflict should be preserved, or whether a delete is safe.

### CLI safety boundary

Read-only commands may appear early. Mutating commands should be introduced only when the corresponding Core policy, Server route, doctor check, rollback path, and docs exist.

## Contract-change triggers

A component should request a contract change when the current contract would cause:

- impossible implementation;
- serious internal contortions;
- hidden sibling dependency;
- duplicated Core policy;
- unsafe overwrite/delete behavior;
- provider secrets or payloads leaking into public output;
- test claims that cannot be honestly supported;
- fan-in work hidden inside a leaf/component-local phase.
