# Archived architecture report — ARCH-SRV-P7B-CONTRACTS

This file archives the completed active architecture report by immutable Git object reference and preserves its accepted decision/gate summary.

Authoritative verbatim source:

- repository: `NordCoder/haze-sync`
- branch at completion: `component/server`
- original path: `crates/haze-sync-server/control/report.md`
- report blob SHA: `f2c321d46fe98030544a0e2b92b1f195317c7bdf`
- final report-only commit: `22b5e993431a1c0a858716ac835a3ece8ade6d5c`
- documentation-bearing SHA: `b98f5079ed90ccf7eaec79e617ae591e0c308ff4`
- Component CI run: `29165123142`
- run number: `1719`
- conclusion: `success`

REPORT_TYPE: ARCHITECT_REVIEW
STATUS: ARCHITECT_CHANGED_CONTRACTS
phase_id: ARCH-SRV-P7B-CONTRACTS
chat_name: server — W1 SRV-P7B Architecture Decision

## Accepted architecture

One preferred architecture was accepted:

1. Worktree owns an awaitable runtime scheduler/cycle contract while retaining synchronous filesystem primitives.
2. Server owns reusable async application services shared by HTTP routes and Worktree execution.
3. Storage owns versioned per-adapter Worktree instance/path state and locked exact-contiguous export cursor repositories.
4. Server later owns one explicit joined runtime host with bounded policy, cancellation, shutdown and safe status.

No nested Tokio runtime, `Handle::block_on`, ad hoc blocking, internal HTTP self-call, detached fabricated summary, duplicated route policy, or production in-memory substitute is accepted.

## Contract decision A — Worktree async runtime

- add cooperative cancellation observation;
- `WorktreeRuntimeCycle::run_cycle` returns an awaited `Send` future;
- `WorktreeRuntimeService::poll` becomes async and awaits at most one cycle;
- start/cancel/status/watcher shutdown may remain synchronous;
- Worktree keeps scheduler/no-overlap/full-scan/watcher/budget/result-validation ownership;
- Server later supplies the concrete async executor and joined host;
- DryRun is explicit manual planning only and performs no mutation/materialization/cursor/state change.

## Contract decision B — Server application services

Proposed Server modules/types:

- `src/application/mod.rs`
- `src/application/files.rs`
- `src/application/deletes.rs`
- `src/application/changes.rs`
- `src/application/idempotency.rs`
- `ServerApplicationServices`
- `ApplicationActor`
- typed file/delete/change/revision-content commands and outcomes
- `ApplicationError`

Services own transaction begin/rollback/commit, advisory path locks, authoritative state reads, Core planning, object-store coordination, persistence, operation log, durable idempotency and safe typed outcomes. Routes retain transport parsing/auth/API DTO/status mapping.

Deterministic Worktree idempotency vocabulary:

- `wt:v1:<adapter_id>:put:<path_hash>:<base_or_null>:<content_hash>`
- `wt:v1:<adapter_id>:delete:<path_hash>:<base_or_null>`

Keys remain secret and are never logged or exposed.

## Contract decision C — Storage durable state/cursor

Storage owns:

- versioned Worktree runtime-instance binding keyed by adapter id;
- non-public normalized-root fingerprint;
- per-instance/path present or tombstoned state;
- last-applied revision and present-file content hash;
- bounded/versioned reconciliation observation fields;
- locked/read and exact-contiguous monotonic `adapter_cursors.last_core_seq` advancement.

Server owns transaction timing. Worktree owns semantic planner/materializer state types. Existing path-only `worktree_state` is insufficient and requires a minimum migration/repository phase.

Crash/replay invariants:

- cursor never advances before one operation is safely materialized;
- path state and exact cursor advancement commit together;
- crash after materialization but before checkpoint replays as AlreadyCurrent, then commits;
- deterministic idempotency repairs missing state after accepted import;
- rollback cannot leave false state/cursor.

## Contract decision D — runtime policy/host

Accepted conservative defaults:

- adapter id: `worktree`
- import/delete/export budgets: `100` each
- periodic correctness interval: `60s`
- watcher debounce: `500ms`
- max watcher hints per poll: `256`
- host idle wake interval: `250ms`
- graceful cycle shutdown budget: `30s`

One joined Server host serializes periodic/watcher/manual triggers, owns cancellation and joins on shutdown. Disabled starts no host/state binding. ReadOnly and ExportOnly export only. ImportOnly imports/deletes only. Bidirectional does both. DryRun is explicit manual non-mutating planning only.

## Accepted phase order

May activate independently now:

1. `WT-P10` — `worktree — W1 WT-P10 Async Runtime Contract`
2. `STOR-P10` — `storage — W1 STOR-P10 Worktree Durable State`
3. `SRV-P7B2` — `server — W1 SRV-P7B2 Application Services`

Blocked order after those phases:

4. `SRV-P7B3` — bounded Worktree executor, only after all three exact clean-accepted SHAs are synchronized.
5. `SRV-P7B4` — hosted Worktree runtime, only after SRV-P7B3 clean acceptance.
6. `API-P8` — runtime status contract, after internal hosted status vocabulary exists.
7. `SRV-P7B5` — status/readiness integration after SRV-P7B4 and API-P8.
8. `CLI-P6A` — operator status/sync-once after API-P8 and SRV-P7B5.
9. `DEP-P5A` — deployment fan-in after Storage migration and Server hosted/status phases.

## Scope and safety result

Only Server documentation and the original report changed. Product source, Cargo manifests, migrations, workflows, main and sibling branches were not modified. CI passed cargo fmt/check/test/clippy and diagnostics finalization on the documentation-bearing SHA.

FINAL_VERDICT: ARCHITECT_CHANGED_CONTRACTS. Activate WT-P10, STOR-P10 and SRV-P7B2 independently. Do not activate SRV-P7B3 until all three pass normal CI and mandatory clean-code review and their exact accepted SHAs are synchronized.
