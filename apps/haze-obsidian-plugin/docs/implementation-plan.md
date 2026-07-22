# Implementation Plan: obsidian-plugin

## Current state

`apps/haze-obsidian-plugin` is currently a minimal Obsidian plugin scaffold.

Implemented current-state surface:

- `manifest.json` with plugin metadata;
- `package.json` with `typecheck` and `build` scripts;
- `src/main.ts` plugin class extending `Plugin`;
- `onload()` displays a load notice;
- `onunload()` is empty.

There is no settings tab, API client, token handling, vault scanner, pending queue, base revision store, sync planner, conflict UI, offline/backoff behavior, or real sync implementation yet.

The component docs were scaffold-level before this planning pass.

## Target state

The target state for the Obsidian plugin is a safe client adapter that syncs an Obsidian vault with Haze Sync Server through the public API contract.

The component is V1-ready when:

- users can configure server URL, adapter identity/mode, and auth token safely;
- plugin secrets are redacted and not logged;
- local vault paths are normalized and mapped to API-compatible vault paths;
- local events and scans produce safe pending changes;
- uploads/deletes include idempotency key and base/null-base semantics;
- downloads/materialization protect local dirty files;
- conflict-saved and conflict-list responses are represented in a user-facing conflict UI;
- offline/backoff/retry behavior is deterministic and user-visible;
- plugin lifecycle cleans up listeners/timers/network loops;
- TypeScript DTO compatibility is checked against API fixtures or generated types where available.

## Implementation phases

### OBS-P1 — Component contract and planning normalization

Status: completed by this Architect planning pass.

Goal:

```text
Replace scaffold Obsidian plugin docs with a real contract, dependency map,
implementation plan, decisions, and baseline implementation log.
```

Allowed scope:

```text
apps/haze-obsidian-plugin/docs/**
```

Completed deliverables:

- complete `component-contract.md`;
- complete `dependency-map.md`;
- complete `implementation-plan.md`;
- add initial component decisions;
- update implementation log with planning baseline.

Non-goals:

- no product code changes;
- no TypeScript implementation changes;
- no API contract changes;
- no CI/workflow changes;
- no generated client pipeline.

Acceptance:

- docs define plugin as Haze Sync Server client adapter;
- docs explicitly forbid Google Drive direct integration;
- future plugin phases are implementable without local sync policy ownership.

### OBS-P2 — Settings, secret handling, and lifecycle foundation

Goal:

```text
Implement safe plugin settings and lifecycle cleanup before adding network or sync
behavior.
```

Allowed scope:

```text
apps/haze-obsidian-plugin/src/**
apps/haze-obsidian-plugin/docs/**
```

Likely work:

- define `PluginSettings` for server URL, adapter identity, auth token, sync mode, and safety toggles;
- add settings load/save through Obsidian APIs;
- add settings tab with token redaction/masking;
- validate server URL and mode values;
- ensure `onunload()` removes listeners/timers/status items;
- introduce safe notice/status reporter;
- add unit-testable pure helpers where possible.

Non-goals:

- no server sync calls;
- no vault scanner;
- no conflict UI;
- no Google Drive integration;
- no token rotation endpoint.

Contract-change triggers:

- storing tokens in unsafe global state;
- logging/displaying tokens;
- changing API auth scheme;
- relying on mobile background behavior for correctness.

Acceptance:

- settings are validated and redacted;
- lifecycle cleanup is explicit;
- token values do not appear in logs/UI except masked input behavior.

### OBS-P3 — API client and DTO compatibility foundation

Goal:

```text
Implement a small Haze Sync API client layer that uses public API headers/DTOs and
is compatibility-tested against API fixtures when available.
```

Allowed scope:

```text
apps/haze-obsidian-plugin/src/**
apps/haze-obsidian-plugin/docs/**
```

Likely work:

- implement typed API client for server-info, changes, file upload/download/delete, conflicts, and status as scoped;
- centralize auth header construction and token redaction;
- attach `Idempotency-Key`, `X-Content-SHA256`, and `X-Base-Revision-Id` where required;
- parse safe public error responses;
- add DTO TypeScript types manually or from fixtures if generation is not accepted;
- add tests for request construction and response parsing without real server dependency.

Non-goals:

- no Obsidian vault mutation;
- no sync loop;
- no public API redesign;
- no generated client unless accepted;
- no provider/GDrive calls.

Contract-change triggers:

- needing API field/header changes;
- exposing raw request/response bodies with secrets;
- sending tokens to non-configured origins;
- bypassing idempotency/base semantics.

Acceptance:

- API client requests match server/API contract;
- public errors map to safe user-facing categories;
- DTO compatibility drift is test-visible.

### OBS-P4 — Vault path mapping, scan, and pending queue foundation

Goal:

```text
Observe local vault files safely and queue pending changes without sending or
applying sync mutations yet.
```

Allowed scope:

```text
apps/haze-obsidian-plugin/src/**
apps/haze-obsidian-plugin/docs/**
```

Likely work:

- map Obsidian `TFile` paths to API-compatible vault paths;
- exclude plugin metadata, internal directories, temporary files, and unsupported resources;
- implement explicit scan/reconcile entrypoint;
- capture local file facts: path, mtime/revision marker if available, hash, size;
- maintain pending queue for new/modified/deleted files;
- distinguish local event hints from full scans;
- make queue state safe to inspect in UI/status.

Non-goals:

- no upload/download execution;
- no server writes;
- no conflict resolution;
- no direct filesystem APIs outside Obsidian abstractions unless justified;
- no Google Drive behavior.

Contract-change triggers:

- uploading plugin metadata files;
- relying only on file events for correctness;
- treating local vault state as authoritative over Core;
- exposing local absolute paths.

Acceptance:

- scanner/queue is deterministic and safe;
- local events are hints, scans are correctness;
- no network mutation occurs in this phase.

### OBS-P5 — Base revision store and upload/delete planner

Goal:

```text
Track per-file base revision state and convert pending local changes into API
upload/delete requests with no-silent-overwrite semantics.
```

Allowed scope:

```text
apps/haze-obsidian-plugin/src/**
apps/haze-obsidian-plugin/docs/**
```

Likely work:

- store per-vault-path last known Core revision/hash metadata;
- generate idempotency keys for local operations;
- compute content hashes before upload;
- prepare upload requests with current base revision or explicit null base;
- prepare delete requests with current base revision or explicit null base;
- handle same-content, accepted, conflict-saved, rejected, unauthorized, and server-unavailable outcomes;
- update local base state only after accepted/server-confirmed outcomes.

Non-goals:

- no automatic conflict resolution;
- no hard delete;
- no direct DB access;
- no provider behavior;
- no full background sync loop unless scoped.

Contract-change triggers:

- upload/delete without base/null-base metadata;
- changing API idempotency semantics;
- local overwrite decisions without server confirmation;
- persisting secrets in base revision state.

Acceptance:

- outgoing mutations preserve Core/API safety contracts;
- accepted outcomes update base revision state;
- rejected/conflict outcomes remain visible and non-destructive.

### OBS-P6 — Remote changes pull and safe materialization

Goal:

```text
Pull server changes and apply them to the Obsidian vault without silently
overwriting dirty local files.
```

Allowed scope:

```text
apps/haze-obsidian-plugin/src/**
apps/haze-obsidian-plugin/docs/**
```

Likely work:

- fetch `/v1/changes` with cursor state;
- download file metadata/content for relevant revisions;
- verify content hash before applying;
- write through Obsidian APIs with local dirty checks;
- update base revision/cursor only after successful apply;
- represent tombstones/deletes safely;
- queue conflicts instead of overwriting dirty local files;
- prevent echo loops from plugin-applied changes.

Non-goals:

- no provider calls;
- no semantic markdown merge;
- no hard delete by default;
- no server route changes;
- no Worktree behavior.

Contract-change triggers:

- applying remote changes without hash verification;
- overwriting dirty local notes;
- advancing cursor before successful local apply;
- deleting local notes irreversibly.

Acceptance:

- remote changes materialize only after verification;
- dirty files are preserved;
- cursor/base metadata remains consistent.

### OBS-P7 — Conflict center and user actions

Goal:

```text
Expose conflicts clearly to users and submit only supported conflict actions to
Server/API.
```

Allowed scope:

```text
apps/haze-obsidian-plugin/src/**
apps/haze-obsidian-plugin/docs/**
```

Likely work:

- list open conflicts from Server;
- show original path, conflict path/materialized copy, status, source adapter, and safe timestamps;
- provide actions for supported vocabulary: `accept_current`, `accept_conflict`, `keep_both`, `mark_resolved`;
- explain destructive implications before actions where necessary;
- refresh local base state after server-confirmed resolution;
- avoid rendering raw server internals or tokens.

Non-goals:

- no local-only conflict resolution bypassing Server;
- no semantic merge editor unless scoped;
- no new conflict policy vocabulary;
- no server route changes.

Contract-change triggers:

- adding unsupported conflict actions;
- resolving conflicts locally without Server confirmation;
- exposing raw file bytes or secrets in conflict UI;
- changing public conflict DTO shape.

Acceptance:

- user can inspect and act on conflicts safely;
- plugin only sends supported API actions;
- both sides remain preserved unless server confirms resolution.

### OBS-P8 — Sync runner, offline/backoff, and status UX

Goal:

```text
Coordinate push/pull work into an explicit sync runner with safe retry, backoff,
status, and lifecycle behavior.
```

Allowed scope:

```text
apps/haze-obsidian-plugin/src/**
apps/haze-obsidian-plugin/docs/**
```

Likely work:

- implement manual sync trigger;
- add optional interval/event-triggered sync with lifecycle cleanup;
- add offline/backoff state;
- expose status bar or settings/status view summaries;
- avoid concurrent overlapping sync runs;
- stop work on unload;
- make mobile/background limitations explicit in UI/docs.

Non-goals:

- no guaranteed mobile background sync;
- no hidden telemetry;
- no provider integration;
- no server runtime changes;
- no repair/destructive automation by default.

Contract-change triggers:

- claiming reliable background sync on platforms where Obsidian cannot guarantee it;
- overlapping sync loops causing duplicate mutations;
- hiding failures from user;
- leaking token/server details in status.

Acceptance:

- sync runner is explicit and lifecycle-safe;
- offline/backoff behavior is predictable;
- user-facing status is useful and sanitized.

### OBS-P9 — Compatibility, packaging, and E2E readiness

Goal:

```text
Prepare the plugin for integration testing and safe local packaging without
committing generated artifacts unless explicitly accepted.
```

Allowed scope:

```text
apps/haze-obsidian-plugin/**
```

Likely work:

- add fixture-based DTO compatibility tests with `haze-sync-api` examples;
- add mocked API client tests for scan/push/pull/conflict flows;
- verify `npm run typecheck` and `npm run build`;
- document local installation/testing procedure;
- decide whether generated `main.js`/bundles are tracked or excluded;
- prepare E2E scenarios with a local test vault and test server.

Non-goals:

- no production plugin marketplace release unless scoped;
- no real user vault contents in tests;
- no secrets in fixtures;
- no generated artifacts committed unless policy accepts them.

Contract-change triggers:

- needing API fixture changes;
- requiring generated files in repo against project policy;
- exposing real vault data/secrets in test artifacts;
- changing plugin manifest compatibility unexpectedly.

Acceptance:

- plugin behavior is testable without real secrets;
- API compatibility drift is visible;
- packaging/install docs are honest.

## Dependency gates

Plugin implementation depends on API/Core/Server/Common contracts:

- Common semantics must define path/hash/ID behavior that TypeScript mirrors.
- API must stabilize JSON DTOs and headers before deep client work.
- Core owns base revision, conflict, tombstone, idempotency, and changes semantics through API outcomes.
- Server must expose runtime routes and safe errors.
- Storage, Worktree, and GDrive remain server-side/non-plugin concerns.

API fixture work should happen before or alongside plugin DTO-heavy phases.

## Known risks

- TypeScript DTOs can drift from Rust API DTOs without fixtures/generated types.
- Obsidian mobile/background behavior may not support reliable continuous sync.
- Local file events may be missed; scans/reconciliation are required.
- Token handling in plugin settings is leak-prone.
- Conflict UI can accidentally imply local-only resolution authority.
- Applying remote changes can overwrite local dirty notes if not guarded.
- Idempotency/base revision semantics can be lost in client abstractions.
- Generated plugin bundles can accidentally be committed without policy.

## Deferred work

Deferred outside this Architect documentation/planning pass:

- run npm checks or observe CI;
- execute OBS-P2 through OBS-P9 implementation/clean-code/CI phases;
- implement settings/API client/scanner/sync UI;
- create TypeScript DTO fixtures or generated client pipeline;
- integrate with real Server routes;
- implement conflict center;
- implement E2E tests with a local test vault;
- decide packaging/artifact tracking policy.

## Completion criteria for the component

`haze-obsidian-plugin` is V1-ready when:

- plugin settings and token handling are safe;
- API client is compatible with server DTO/header contracts;
- scan/queue/push/pull workflows preserve base revision and idempotency semantics;
- remote materialization protects dirty local notes;
- conflict center exposes supported server actions safely;
- sync runner is lifecycle-safe and honest about offline/mobile limits;
- TypeScript checks and fixture tests guard API drift;
- no Google Drive, DB, Storage, Worktree, or Core policy ownership enters the plugin.
