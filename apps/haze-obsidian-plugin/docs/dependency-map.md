# Dependency Map: obsidian-plugin

## Component role in dependency graph

`apps/haze-obsidian-plugin` is the Obsidian client adapter for Haze Sync.

Conceptual position:

```text
Obsidian vault
  -> Obsidian plugin scanner/pending queue/API client
  -> Haze Sync Server public API
  -> Core/Storage authoritative state
```

The plugin is a client of Haze Sync Server. It is not a peer-to-peer sync engine, Google Drive adapter, database client, or Core policy engine.

## Upstream dependencies

### Runtime/platform dependencies

- Obsidian plugin API
  - plugin lifecycle;
  - vault file reads/writes;
  - settings storage;
  - notices/status/UI surfaces;
  - file events and explicit vault access.

### Project contract dependencies

- `haze-sync-api`
  - HTTP routes;
  - headers;
  - request/response DTO JSON vocabulary;
  - public error shapes;
  - auth semantics.
- `haze-sync-common`
  - conceptual path/hash/ID semantics to mirror in TypeScript;
  - adapter roles/modes and validation vocabulary.
- `haze-sync-core`
  - authoritative semantics observed through API responses:
    - base revision;
    - conflict-saved;
    - tombstones/deletes;
    - idempotency;
    - operation changes.
- `haze-sync-server`
  - public HTTP runtime endpoints;
  - safe error/status behavior.

### Current direct dependencies

Current package dependencies:

- `obsidian` as dev dependency for plugin API types;
- `typescript`;
- `@types/node`.

The current plugin scaffold has no real network/client/state dependencies yet.

## Disallowed direct dependencies

The plugin must not directly depend on:

```text
haze-sync-storage
haze-sync-server internals
haze-sync-core Rust internals
haze-gdrive-adapter
haze-sync-worktree
Google Drive APIs/provider SDKs
PostgreSQL/SQLx/database clients
Node filesystem APIs for vault mutation when Obsidian API should be used
server-side secret files
```

The plugin may consume public HTTP API JSON and generated/fixture-backed TypeScript types if accepted.

## Downstream dependents

Expected dependents:

- Obsidian users/operators;
- local test vaults and E2E tests;
- API compatibility fixture checks;
- documentation/runbooks for client setup.

No server-side component should depend on plugin internals for Core correctness.

## Cross-component contracts

### API ↔ Obsidian plugin

- API owns public route/header/DTO/error vocabulary.
- Plugin owns TypeScript client code that consumes those public contracts.
- Plugin must not rely on private Rust internals.
- Fixture or generated type compatibility is required before heavy client work.

### Core ↔ Obsidian plugin

- Core owns sync semantics and conflict/delete/revision/idempotency decisions.
- Plugin observes outcomes through Server/API responses and obeys them.
- Plugin must not implement local alternative conflict/delete policy.

### Common ↔ Obsidian plugin

- Common owns canonical Rust primitive validation.
- Plugin must mirror relevant path/hash/ID constraints in TypeScript or rely on server validation plus safe client pre-validation.
- Divergence in path normalization or hash formatting can break sync safety and must be tested.

### Server ↔ Obsidian plugin

- Server exposes HTTP API and safe errors.
- Plugin sends authenticated requests to configured Server URL.
- Plugin must never send auth tokens to non-configured origins.
- Server status/capability outputs should inform plugin behavior without overclaiming offline/mobile guarantees.

### Storage ↔ Obsidian plugin

No direct dependency is allowed.

Plugin never writes Storage repositories or database rows directly.

### GDrive ↔ Obsidian plugin

No direct dependency is allowed.

Plugin must not call Google Drive API. Google Drive sync is mediated by Server/Core and the separate GDrive adapter.

### Worktree ↔ Obsidian plugin

No direct dependency is allowed.

Worktree and Obsidian plugin are separate replicas/adapters coordinated through Core/API, not through direct local filesystem synchronization.

## Integration/fan-in ownership

The following work belongs outside plugin-only leaf phases:

- Rust API fixture generation or publishing, unless explicitly scoped here with API coordination;
- Server route changes;
- Core policy changes;
- Storage schema/repository changes;
- Google Drive adapter runtime;
- Worktree runtime;
- deployment server configuration;
- production plugin release process.

Plugin phases may add TypeScript code, UI, and tests only within `apps/haze-obsidian-plugin` unless a cross-component fan-in prompt explicitly expands scope.

## Dependency rules

- Consume Haze Sync through public Server/API only.
- Do not call Google Drive.
- Do not access database/storage directly.
- Do not import Rust crate internals into plugin behavior.
- Keep secrets/token values redacted.
- Use Obsidian APIs for vault operations where practical.
- Keep generated bundles/artifacts out of commits unless project policy explicitly accepts them.
- Any dependency addition must be justified by a plugin phase and reflected here.

## Contract-change notes

Current known contract questions:

1. TypeScript DTO source of truth
   - API Rust DTOs are current source of truth.
   - Plugin needs fixtures or generated TypeScript types to avoid drift.
   - Exact mechanism is deferred.

2. Mobile/background behavior
   - Obsidian mobile/background execution cannot be treated as guaranteed correctness mechanism.
   - Plugin UI/docs must be honest about limitations.

3. Secret storage
   - Token storage through Obsidian settings is convenient but must be reviewed for acceptable secrecy expectations.
   - Alternative OS/keychain storage is not currently designed.

4. Generated artifact policy
   - Plugin build output may be needed for installation.
   - Whether generated `main.js`/bundles are tracked requires explicit project policy.

No immediate blocking contract change is required for the current documentation/planning pass.
