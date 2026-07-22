# Component Contract: obsidian-plugin

## Responsibility

`apps/haze-obsidian-plugin` owns the Obsidian-side client adapter for Haze Sync.

The plugin runs inside Obsidian and synchronizes the local Obsidian vault with the authoritative Haze Sync Server through the public Core API contract.

The plugin is responsible for future behavior such as:

- user-facing settings for server URL, adapter identity, auth token entry, sync mode, and safety toggles;
- safe local vault scanning through Obsidian APIs;
- pending local change queue;
- upload/download/delete requests through Haze Sync HTTP API;
- base revision tracking for local files;
- conflict display and user-facing conflict actions;
- safe status and notice UI;
- offline/error/backoff behavior;
- plugin lifecycle integration with Obsidian load/unload events;
- compatibility with API JSON/header fixtures.

The plugin must not call Google Drive directly, write the server database, or decide Core sync policy locally.

Current code state is a minimal plugin shell that shows a load notice. The component contract therefore defines intended V1 ownership before real runtime behavior is added.

## Public interfaces

Current public/runtime interface:

```text
Obsidian plugin manifest: manifest.json
Plugin entrypoint: src/main.ts
Plugin lifecycle: HazeSyncPlugin.onload(), HazeSyncPlugin.onunload()
npm scripts: typecheck, build
```

Current behavior:

- loads as Obsidian plugin shell;
- displays a notice: `Haze Sync plugin loaded`;
- does not persist settings;
- does not call server;
- does not scan vault;
- does not mutate files;
- does not implement sync.

Target internal interfaces may include:

```text
PluginSettings
SettingsTab
CoreApiClient
AuthTokenStore
VaultScanner
PendingChangeQueue
BaseRevisionStore
SyncPlanner
SyncRunner
ConflictCenter
NoticeReporter
BackoffPolicy
StatusView
```

Names are indicative. Implementation workers should use final names consistent with Obsidian/TypeScript style and current code.

## Input contracts

Plugin inputs come from:

- Obsidian vault file events and explicit scans;
- Obsidian file reads/writes through safe APIs;
- plugin settings entered by the user;
- Haze Sync Server API responses;
- user actions in conflict/status/settings UI;
- plugin lifecycle events.

Required input rules:

- vault-relative paths must be normalized to the same semantics expected by `VaultPath` and the API contract;
- local file bytes must be hashed before upload;
- write/delete requests must include idempotency key and explicit base revision or explicit null base;
- server responses must be parsed through API-compatible DTO shapes or generated/fixture-backed TypeScript equivalents;
- auth token values must be treated as secrets and never displayed/logged;
- local vault events are hints, not complete correctness guarantees; scans/reconciliation are required for correctness.

## Output contracts

Plugin outputs include:

- HTTP requests to Haze Sync Server;
- Obsidian file writes/deletes only when confirmed by Core/API outcomes;
- local plugin settings/state;
- user-facing notices/status/conflict UI;
- safe diagnostic summaries.

Required output rules:

- all server writes use public API headers and DTOs;
- plugin must not silently overwrite local files when server reports conflict/stale base or local dirty state;
- plugin must not hard-delete local notes without Core tombstone/delete semantics and user-safe behavior;
- UI must represent conflicts clearly and preserve both versions by default;
- status/errors shown to the user must not include tokens, raw request bodies, stack traces, database URLs, or server internals.

## Error contracts

Plugin errors must be safe for notices, UI panels, logs, and support reports.

Errors must not expose:

- bearer/auth tokens;
- token hashes;
- OAuth tokens;
- Idempotency-Key values;
- database URLs;
- raw provider payloads;
- raw server stack traces;
- raw request/response bodies containing file content or secrets;
- local absolute OS paths where Obsidian vault-relative context is sufficient.

Network/server errors should be mapped to user-understandable categories such as offline, unauthorized, forbidden, server unavailable, conflict, rejected, rate/backoff, and internal error.

## Persistence/runtime ownership

Plugin owns Obsidian-local state only.

Plugin may own:

- plugin settings stored through Obsidian APIs;
- local base revision metadata for vault files;
- local pending queue metadata;
- local sync status/cache metadata;
- local conflict UI state;
- local backoff/retry state;
- local notices/status views.

Plugin must not own:

- Core server database schema;
- Storage repositories;
- Google Drive mapping state;
- Google Drive OAuth/token files;
- Worktree state;
- server runtime configuration;
- sync conflict/delete/revision policy;
- provider API calls outside Haze Sync Server.

## Security and secrecy rules

- Do not commit auth tokens, OAuth credentials, server URLs with embedded credentials, user vault contents, debug dumps, or generated plugin bundles unless explicitly accepted.
- Store tokens only through accepted Obsidian/plugin settings behavior and avoid logging/displaying them.
- Never send tokens to non-configured origins.
- Never call Google Drive directly from the plugin.
- Avoid exposing local absolute paths in public diagnostics.
- Do not upload hidden/internal plugin metadata files as vault content.
- Do not use localStorage or insecure global state for secrets without explicit review.
- Do not silently overwrite or delete user notes.

## Non-goals

The Obsidian plugin must not implement:

- Google Drive API integration;
- direct database or Storage access;
- Server route handlers;
- Core policy decisions;
- Worktree adapter runtime behavior;
- CRDT/block-level sync;
- semantic Markdown merge;
- background sync guarantees beyond what Obsidian/mobile runtime permits;
- production mobile background execution guarantees;
- hard delete without Core/user-safe retention behavior;
- hidden telemetry or external reporting.

## Dependencies

See `dependency-map.md`.

## Dependents

See `dependency-map.md`.

## Invariants

- Server/Core are authoritative; plugin is a client adapter.
- Plugin talks to Haze Sync Server API, not Google Drive.
- Every upload/delete uses base revision or explicit null base semantics.
- Unknown/stale base does not permit overwrite.
- Conflict UI must preserve both sides by default.
- Local Obsidian events are hints; reconciliation scans are required for correctness.
- Tokens and idempotency keys must never appear in UI/logs/support output.
- Plugin unload must stop timers/listeners/network loops to avoid duplicate sync workers.
- Mobile limitations must be represented honestly.

## Test obligations

Plugin tests should eventually cover:

- TypeScript typecheck/build;
- settings validation and token redaction;
- API client request headers and DTO parsing using fixtures from `haze-sync-api` where available;
- vault path normalization and reserved/internal file exclusion;
- pending queue behavior;
- base revision tracking;
- local scan/reconciliation behavior;
- upload/download/delete planner behavior;
- conflict response handling and conflict UI state;
- offline/backoff/retry behavior;
- lifecycle cleanup on unload;
- absence of token/idempotency/server-internal leaks in user-facing errors.

Expected checks when shell or CI is available:

```bash
cd apps/haze-obsidian-plugin && npm run typecheck
cd apps/haze-obsidian-plugin && npm run build
```

## Contract change protocol

Request a contract change instead of silently broadening scope when implementation requires:

- plugin calling Google Drive directly;
- plugin writing server DB/storage directly;
- changing public HTTP API DTO/header semantics;
- changing Core conflict/delete/base-revision semantics;
- storing secrets in an unsafe location;
- adding hard delete behavior;
- relying on mobile background execution as a correctness guarantee;
- exposing tokens, idempotency keys, raw file content, or raw server internals in UI/logs.
