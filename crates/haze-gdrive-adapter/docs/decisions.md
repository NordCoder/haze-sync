# Decisions: gdrive-adapter

## 2026-07-05 — GDrive adapter is an external replica client

Decision:

`haze-gdrive-adapter` treats Google Drive as an external replica of Haze Sync Core state. It submits Drive facts to Server/Core API and applies confirmed Core changes back to Drive.

Rationale:

Core is authoritative. Drive state can be stale, incomplete, permission-filtered, or temporarily inconsistent. Treating Drive as source of truth would allow provider anomalies to overwrite or delete Core data.

Alternatives:

- Treat Drive as authoritative source.
- Let Drive adapter resolve conflicts locally.
- Synchronize Drive directly with Obsidian or Worktree.

Consequences:

- Adapter import/export must preserve Core/API semantics.
- Adapter cannot bypass conflict/delete/idempotency policy.
- GDrive and other replicas meet only through Core/API.

Affected contracts:

- component contract;
- dependency map;
- import/export phases;
- delete-candidate guard phases.

## 2026-07-05 — Full scan is correctness; Drive change feed is latency

Decision:

Drive change feed or future webhook support may reduce latency, but full Drive scans are required for correctness and reconciliation.

Rationale:

Provider change feeds can be invalidated, partial, permission-filtered, delayed, or missed. A full scan is needed to repair drift and validate mapping state.

Alternatives:

- Rely only on Drive change feed.
- Poll full scan only and ignore change feed.
- Treat cursor invalidation as fatal forever.

Consequences:

- Full scan planner must be implemented before relying on incremental change feed.
- Cursor invalidation should fall back to full scan.
- Status/doctor summaries should distinguish full scan and change-feed health.

Affected contracts:

- scan planner;
- change-feed loop;
- cursor state;
- doctor/status behavior.

## 2026-07-05 — Drive disappearance is delete candidate, not immediate tombstone

Decision:

A Drive file missing from scan/change feed becomes a delete candidate first. The adapter must not immediately tombstone Core state on first disappearance.

Rationale:

Drive disappearance can mean user delete, move, permission loss, folder scope change, API inconsistency, or provider lag. Immediate tombstone propagation risks mass data loss.

Alternatives:

- Tombstone immediately when file disappears.
- Ignore Drive disappearance entirely.
- Let adapter hard-delete Core state.

Consequences:

- Delete-candidate state must be tracked with timestamps/confirmation.
- Mass delete guard behavior is required.
- Core/API remains delete arbiter.

Affected contracts:

- delete-candidate planner;
- Core delete/tombstone semantics;
- Storage mapping/cursor state;
- Server/API delete routes.

## 2026-07-05 — Echo guard is required for provider writes

Decision:

The adapter must track its own Drive writes so exported Core changes are not immediately re-imported as Drive edits.

Rationale:

Without echo guard, export/import loops can duplicate operations, waste quota, and create false conflicts.

Alternatives:

- Rely only on same-content Core ignores.
- Sleep after every provider write.
- Disable imports after exports globally.

Consequences:

- Mapping state must record enough provider/core metadata to identify adapter-created echoes.
- Echo guard must be narrow enough not to hide real user changes.
- Tests need fake provider scenarios for echo loops.

Affected contracts:

- mapping model;
- import planner;
- export planner;
- cursor/change-feed handling.

## 2026-07-05 — Mapping/cursor persistence boundary is unresolved by default

Decision:

The adapter needs durable mapping/cursor state, but direct database ownership is not assumed. Server/API-mediated persistence is preferred unless a future architecture decision accepts direct Storage repository access.

Rationale:

Direct DB access from an external adapter can bypass Server/Core/API safety boundaries and complicate deployment/secrets. However, mapping/cursor persistence is essential for correctness.

Alternatives:

- Give adapter unrestricted SQLx access.
- Store mapping only in local files.
- Add Server/API mapping endpoints.

Consequences:

- A dedicated integration decision is required before implementation chooses persistence mechanism.
- Storage may still provide row/repository primitives.
- Deployment must account for whichever persistence boundary is chosen.

Affected contracts:

- dependency map;
- Storage `gdrive_mapping` ownership;
- Server/API adapter integration;
- deployment secret/topology planning.

## 2026-07-05 — Provider scope remains conservative in V1

Decision:

V1 GDrive adapter scope is regular Drive files in a configured folder subtree. Google Docs/Sheets/Slides conversion, shared drives, and shortcuts are excluded unless future contracts explicitly add them.

Rationale:

Those provider features introduce non-file content models, shortcut/path ambiguity, permissions complexity, and conversion fidelity risks. V1 should synchronize file-like objects safely first.

Alternatives:

- Convert Docs/Sheets/Slides to Markdown or exports.
- Follow shortcuts as files.
- Support shared drives immediately.

Consequences:

- Unsupported entries must be skipped and reported safely.
- Users need docs explaining V1 limitations.
- Future support requires product/architecture review.

Affected contracts:

- provider normalization;
- scanner;
- docs/runbook;
- product non-goals.

## 2026-07-05 — OAuth tokens and provider payloads are never public output

Decision:

OAuth tokens, bearer tokens, raw provider payloads, raw request/response bodies, and secret paths must not appear in logs, status, reports, or public errors.

Rationale:

Provider integration is credential-heavy and leak-prone. Safe diagnostics must use redacted summaries and categories, not raw payload dumps.

Alternatives:

- Log raw Google errors for debugging.
- Include OAuth token file paths in status.
- Store provider payload snapshots in reports.

Consequences:

- Error types and status DTOs must sanitize by default.
- Tests should check redaction.
- Debugging may require opt-in local-only tooling outside public reports.

Affected contracts:

- error handling;
- status/doctor;
- logging/observability;
- test fixtures;
- deployment runbooks.
