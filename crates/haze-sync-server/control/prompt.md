# W1-SRV-P7B5-STATUS-READINESS — Implement Server internal status/readiness contract

Before starting, name this worker chat exactly:

`server — W1 SRV-P7B5 Status Readiness`

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: implementation-worker
Phase: SRV-P7B5-STATUS-READINESS

Do not merge PR #45, change draft state, rewrite history, modify sibling branches, or begin API-P8/CLI-P6A/Deployment work.

## Accepted baseline

- accepted SRV-P7B4 code-bearing SHA: `55ed0d6c6ab9b78a953b954fcf5a9a68a6a708fe`;
- SRV-P7B4 clean-review report commit: `5304a2733d7519a2699352686e2599ebde7d0474`;
- SRV-P7B4 clean-review report blob: `2658e8e0bd85462ce5614c7395f1f1a7274b7258`;
- status: `CLEAN_ACCEPT`;
- authoritative DB-capable Component CI: run `29281569666`, number `1893`, success.

## Goal

Define and implement the Server-owned internal status/readiness contract over the accepted hosted Worktree runtime. This phase prepares a stable passive contract for later API-P8 exposure, but must not add public HTTP routes or API DTOs yet.

## Required behavior

1. Add a compact Server-owned snapshot type for hosted Worktree status using only coarse, secret-safe fields:
   - configured mode/category;
   - lifecycle category;
   - readiness category and coarse reason code;
   - cycles completed/failed;
   - cycle-in-progress;
   - pending watcher hint count;
   - manual submission availability category;
   - optional coarse last-cycle cause/result category if already safely available.

2. Define deterministic readiness semantics:
   - Disabled is ready/inert and must not make the Server unready by itself;
   - Starting is not ready;
   - Running is ready unless the accepted host reports a terminal failure condition;
   - Cancelling/Shutdown are not ready for new work;
   - Failed is not ready;
   - readiness must not depend on raw paths, backend messages, SQLx/notify/I/O text, payloads or secrets;
   - counters alone must not make readiness fail unless an explicit current terminal/degraded state requires it.

3. Preserve separation of concerns:
   - Worktree remains owner of scheduler, watcher, manual gating, counters and no-overlap semantics;
   - Server maps accepted internal status into Server readiness vocabulary only;
   - no executor direct access or duplicate lifecycle state machine;
   - no polling loop, background task or retry is added in this phase.

4. Provide a passive in-process read boundary suitable for later API-P8:
   - immutable snapshot/read method or watch-like receiver already owned by Server;
   - no public route, serialization schema or HTTP status code yet;
   - reads must be bounded, non-blocking and side-effect free;
   - no cycle trigger on status read.

5. Integrate the snapshot with existing Server readiness/health composition only if a current internal composition point already exists. Do not invent a public endpoint. If no shared readiness aggregator exists, expose a narrow internal contract and tests for later composition.

6. Manual availability mapping must remain typed and coarse:
   - Available;
   - Busy;
   - NotStarted;
   - Cancelling;
   - Shutdown;
   - Unavailable/Failed where required.

Do not submit a manual request merely to determine availability.

7. Status transitions must be monotonic with the accepted host lifecycle where applicable and must accurately reflect startup acknowledgement, task failure, shutdown timeout and successful shutdown.

## Tests

Add focused tests for:

- Disabled => ready/inert;
- Starting => not ready;
- Running => ready;
- Busy cycle does not make the service unready;
- Failed => not ready with coarse reason;
- Cancelling and Shutdown => not ready for new work;
- startup failure/status transition mapping;
- shutdown timeout/Failed mapping;
- counters and watcher hints remain count-only;
- status read is passive and does not trigger work;
- no paths, URLs, backend errors, payloads, tokens or idempotency data in Debug/display/serialized test representations;
- exact integration with accepted `ServerWorktreeRuntimeHost` status transitions.

## Scope

Allowed:

- Server internal status/readiness modules, types, tests and exports;
- narrow wiring into the accepted Server host;
- minimal existing internal readiness composition if already present;
- Server control report.

Forbidden:

- accepted Worktree or Storage source changes;
- migrations/schema;
- Core/API/CLI/Deployment product files;
- public HTTP routes, public API DTOs, OpenAPI, CLI output or deployment probes;
- new tasks/runtimes/pollers/retries;
- raw path/error/detail exposure;
- sibling control files or workflows.

Formatting/style alone is non-blocking when exact-SHA CI is green.

## Completion

Create a real code-bearing commit without CI skip. Obtain authoritative DB-capable Component CI on the exact final code-bearing SHA.

Write `crates/haze-sync-server/control/report.md` with:

- `REPORT_TYPE: IMPLEMENTATION`;
- `phase_id: SRV-P7B5-STATUS-READINESS`;
- `chat_name: server — W1 SRV-P7B5 Status Readiness`;
- honest status `SELF_ACCEPT`, `NEEDS_FIX`, `BLOCKED_BY_CONTRACT`, `BLOCKED_BY_SCOPE`, or `BLOCKED_BY_TOOLING`.

Record readiness semantics, status fields, passive read boundary, transitions, tests, secrecy, changed paths, final SHA and exact CI evidence.

Do not claim `CLEAN_ACCEPT`, begin API-P8 or change merge readiness. A focused functional review follows.
