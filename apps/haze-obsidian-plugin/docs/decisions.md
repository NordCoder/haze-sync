# Decisions: obsidian-plugin

## 2026-07-05 — Plugin is a Server API client, not a Google Drive client

Decision:

The Obsidian plugin communicates with Haze Sync Server through the public HTTP API. It must not call Google Drive directly or depend on the GDrive adapter.

Rationale:

V1 architecture keeps Core as authoritative arbiter and separates replicas/adapters. If the plugin talks to Google Drive directly, it bypasses Core conflict/delete/idempotency semantics and creates a second sync topology.

Alternatives:

- Let the plugin sync directly with Google Drive.
- Let the plugin choose between Server and Drive at runtime.
- Put provider-specific code into the plugin for convenience.

Consequences:

- Plugin implementation must focus on API client, vault scanner, pending queue, and conflict UI.
- GDrive behavior stays in the separate adapter component.
- User setup requires a reachable Haze Sync Server for real sync.

Affected contracts:

- component contract;
- dependency map;
- GDrive adapter contract;
- Server/API client phases.

## 2026-07-05 — Plugin does not own sync policy

Decision:

The plugin submits local facts and user actions to Server/API and obeys Core outcomes. It does not decide overwrite, conflict, tombstone, idempotency, or operation-log policy locally.

Rationale:

The plugin runs in a user-controlled client environment and may be offline/stale. Core must remain the only arbiter for no-silent-overwrite and delete/conflict behavior.

Alternatives:

- Resolve conflicts locally in the plugin.
- Let the plugin decide latest-wins behavior.
- Treat local Obsidian state as authoritative.

Consequences:

- Upload/delete requests must carry base/null-base and idempotency metadata.
- Conflict UI can request supported actions but not invent policies.
- Rejected/conflict responses must be surfaced non-destructively.

Affected contracts:

- API client;
- sync planner;
- conflict center;
- Core/API route semantics.

## 2026-07-05 — Local events are hints; scans/reconciliation are required

Decision:

Obsidian file events may be used to trigger work, but correctness requires explicit scans/reconciliation.

Rationale:

Client runtimes can miss events, especially across plugin reloads, mobile lifecycle limits, and offline periods. Event-only sync would lose changes.

Alternatives:

- Rely entirely on Obsidian file events.
- Require manual sync only.
- Assume mobile/background event delivery is reliable.

Consequences:

- A scanner/reconciliation phase is required.
- Pending queue must tolerate duplicate or missing event hints.
- Status UI should distinguish last scan/sync state from live guarantees.

Affected contracts:

- vault scanner;
- pending queue;
- sync runner;
- status UX;
- mobile/offline behavior.

## 2026-07-05 — Token handling is redacted and origin-scoped

Decision:

Plugin auth tokens are secrets. They must be redacted in UI/logs/errors and sent only to the configured Haze Sync Server origin.

Rationale:

Plugin settings and browser-like request code are leak-prone. A sync token should never appear in notices, debug logs, support reports, or requests to unintended origins.

Alternatives:

- Show token values in status/debug UI.
- Store token in arbitrary global/local state.
- Let redirects or dynamic URLs receive auth headers without checks.

Consequences:

- Settings UI must mask token values.
- API client must centralize URL/origin handling.
- Tests should assert token redaction and origin scoping.

Affected contracts:

- settings;
- API client;
- error reporter;
- status UI;
- security tests.

## 2026-07-05 — Conflict UI preserves both sides by default

Decision:

The plugin conflict UI must show conflicts without discarding either version. User actions must call supported Server/API conflict actions and wait for confirmation before updating local state.

Rationale:

Obsidian users need clear local UX, but the server remains authoritative. Local-only conflict resolution risks data loss and divergent replicas.

Alternatives:

- Auto-resolve conflicts locally.
- Hide conflicts and keep retrying.
- Let plugin invent additional actions like latest-wins.

Consequences:

- Conflict list/resolve API support is required.
- UI should explain what each action does.
- Local base revision state updates only after server-confirmed outcome.

Affected contracts:

- conflict center;
- API conflict DTOs;
- sync planner;
- user-facing notices/status.

## 2026-07-05 — Mobile/background sync limitations must be explicit

Decision:

The plugin must not claim guaranteed background sync across all Obsidian platforms. Mobile/background limitations must be reflected in UI/docs/status where relevant.

Rationale:

Obsidian plugin lifecycle and platform restrictions can prevent continuous background work. Overclaiming reliability would mislead users and hide data freshness risks.

Alternatives:

- Promise continuous realtime sync.
- Hide platform limitations.
- Treat missed background sync as impossible.

Consequences:

- Manual sync and visible status remain important.
- Scans/reconciliation handle missed background work.
- Documentation must distinguish near-realtime from strict realtime.

Affected contracts:

- sync runner;
- status UX;
- docs/runbook;
- mobile support expectations.
