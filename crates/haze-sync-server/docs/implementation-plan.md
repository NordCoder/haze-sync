# Implementation Plan: server

## Current accepted baseline

Server already provides production startup, explicit state, auth, readiness, normal file routes, conflict/delete/idempotency fan-in and read-only admin status. SRV-P7A is accepted at code-bearing SHA `37706634fd8dd2d9b299a1c453718f2de63981d0` with Component CI run `29161721748` successful.

The accepted Worktree product snapshot is `component/worktree@4f7bc748d9b901d7d5c3e43c845ba407c0c36e59` and remains unchanged by this architecture phase.

SRV-P7B1 proved that real Worktree execution is blocked by contract: the Worktree scheduler/cycle boundary is synchronous while authoritative Server/Storage operations are async; correct write/delete/conflict/idempotency behavior is route-private; durable Worktree state/export checkpoint and runtime policy are incomplete.

## Preferred target

The preferred architecture is:

1. Worktree owns an async-compatible scheduler and cycle-executor contract while retaining synchronous filesystem primitives.
2. Storage owns versioned durable Worktree instance/path state and monotonic export checkpoint primitives.
3. Server owns reusable async application services used by both HTTP routes and Worktree execution.
4. Server owns one explicit bounded, cancellable, joined Worktree host task.
5. API owns any new public operator/status DTOs; CLI and Deployment consume accepted Server/API/config contracts later.

Contract-owner phases precede consumer phases. No two branches may independently define the same contract.

## Ordered implementation phases

### 1. WT-P10 — Async runtime contract

- Suggested chat: `worktree — W1 WT-P10 Async Runtime Contract`
- Owner: worktree
- Branch: `component/worktree`
- Role: implementation-worker
- Prerequisites: accepted SRV-P7B architecture decision; accepted Worktree snapshot.
- Allowed files: `crates/haze-sync-worktree/src/runtime.rs`, runtime tests, `src/lib.rs`, Worktree-owned docs/control files explicitly assigned by Orchestrator.
- Forbidden: Server/Storage/API source, DB code, HTTP, provider behavior, nested runtimes, blocking bridges, hidden task spawning.
- Deliverables:
  - make `WorktreeRuntimeCycle::run_cycle` awaitable with cooperative cancellation;
  - make `WorktreeRuntimeService::poll` awaitable and still execute at most one cycle;
  - preserve synchronous watcher/start/status/shutdown where no async authority is needed;
  - preserve full-scan correctness, hint coalescing, budgets, count-only status and no overlap;
  - represent DryRun explicitly without changing filesystem/import/export behavior outside the runtime contract;
  - migrate existing tests to async tests and add cancellation/no-overlap coverage.
- Test/CI: fmt, check, test and clippy for Worktree and workspace through normal Component CI.
- Clean gate: mandatory clean-code review after green CI.
- Unblocks: SRV-P7B3 executor and SRV-P7B4 host.

### 2. STOR-P10 — Durable Worktree instance, path state and cursor

- Suggested chat: `storage — W1 STOR-P10 Worktree Durable State`
- Owner: storage
- Branch: `component/storage`
- Role: implementation-worker
- Prerequisites: accepted SRV-P7B architecture decision; no dependency on Server implementation.
- Allowed files: Storage migrations/schema/models/repositories/tests and Storage-owned docs/control files explicitly assigned.
- Forbidden: Core policy, Server orchestration, Worktree filesystem behavior, API DTOs, direct runtime loops.
- Deliverables:
  - versioned runtime-instance binding keyed by Worktree `adapter_id`, including non-public root fingerprint;
  - path state keyed by `(adapter_id, path)` with explicit present/tombstoned kind, last-applied revision/hash and reconciliation fields;
  - migration from or replacement of the path-only `worktree_state` schema without losing valid rows silently;
  - typed repositories for bind/load/upsert/delete/scan snapshot operations;
  - locked, monotonic, contiguous export checkpoint operations using `adapter_cursors.last_core_seq`;
  - safe repository errors and row models;
  - tests for regression rejection, instance mismatch, version mismatch and transaction rollback.
- Test/CI: Storage unit/DB integration tests, migration tests, workspace Component CI.
- Clean gate: mandatory clean-code review after green CI.
- Unblocks: SRV-P7B3 executor and deployment migration runbook.

### 3. SRV-P7B2 — Reusable application services

- Suggested chat: `server — W1 SRV-P7B2 Application Services`
- Owner: server
- Branch: `component/server`
- Role: implementation-worker
- Prerequisites: this architecture decision; existing route behavior and SRV-P7A remain accepted.
- Allowed files: `crates/haze-sync-server/src/application/**`, route modules needed only to delegate, state/tests/docs/control files explicitly assigned.
- Forbidden: Worktree cycle execution, hosted loop, schema/migration changes, public DTO changes, route behavior changes, provider behavior.
- Deliverables:
  - `ServerApplicationServices` and typed actor/command/outcome/error contracts;
  - async file apply, guarded delete, bounded changes retrieval and verified revision-content retrieval;
  - deterministic Worktree idempotency derivation helper without logging keys;
  - extract route-private planning/persistence/idempotency/delete transaction logic;
  - routes become thin parsing/auth/DTO adapters over the services;
  - preserve public HTTP status/body/header behavior and dependency-free router tests.
- Test/CI: parity tests for existing PUT/DELETE/GET/changes behavior, DB-backed transaction/idempotency/conflict tests, normal Component CI.
- Clean gate: mandatory clean-code review after green CI.
- Unblocks: SRV-P7B3 executor and later non-HTTP internal consumers.

### 4. SRV-P7B3 — Real bounded Worktree cycle executor

- Suggested chat: `server — W1 SRV-P7B3 Bounded Worktree Executor`
- Owner: server
- Branch: `component/server`
- Role: implementation-worker
- Prerequisites: WT-P10 clean-accepted and synchronized; STOR-P10 clean-accepted and synchronized; SRV-P7B2 clean-accepted.
- Allowed files: Server Worktree executor/application composition/tests/docs/control; synchronized accepted owner-component product files only when Orchestrator explicitly scopes fan-in.
- Forbidden: scheduler ownership changes, route-policy duplication, internal HTTP, unbounded work, hidden task, public DTO changes, provider behavior.
- Deliverables:
  - `ServerWorktreeCycleExecutor` implementing the async Worktree contract;
  - bounded state load, full scan, reconciliation, import, guarded delete, export query, revision fetch, materialization and checkpoint flow;
  - application-service use for every authoritative mutation;
  - Storage repository use for durable state/cursor;
  - awaited bounded blocking-pool use only for synchronous filesystem phases;
  - cooperative cancellation and count-only summaries;
  - deterministic replay/crash tests for import and export checkpoints.
- Test/CI: focused unit tests, DB/object-store/temp-worktree integration tests, failure injection, workspace Component CI.
- Clean gate: mandatory clean-code review after green CI.
- Unblocks: SRV-P7B4 host and manual one-cycle operation.

### 5. SRV-P7B4 — Hosted scheduling, startup and shutdown

- Suggested chat: `server — W1 SRV-P7B4 Hosted Worktree Runtime`
- Owner: server
- Branch: `component/server`
- Role: implementation-worker
- Prerequisites: SRV-P7B3 clean-accepted.
- Allowed files: Server config, startup, `worktree_runtime.rs` or replacement host module, state/tests/docs/control.
- Forbidden: API DTO expansion, CLI/deploy files, detached tasks, multiple concurrent cycles, implicit migrations, unbounded retry loops.
- Deliverables:
  - `ServerWorktreeRuntimeHost` with one joined Tokio task;
  - explicit policy config and validated defaults;
  - startup identity binding before first cycle;
  - periodic and watcher-hint scheduling plus explicit manual request channel;
  - at-most-one-cycle guarantee and busy/coalesced manual behavior;
  - graceful cancellation, bounded shutdown and task join;
  - exact mode behavior including manual-only DryRun.
- Test/CI: Tokio paused-time scheduler tests, startup/shutdown/error tests, no-overlap tests, Component CI.
- Clean gate: mandatory clean-code review after green CI.
- Unblocks: public status/readiness and Deployment hosting.

### 6. API-P8 — Passive Worktree runtime status contract

- Suggested chat: `api — W1 API-P8 Worktree Runtime Status Contract`
- Owner: api
- Branch: `component/api`
- Role: implementation-worker
- Prerequisites: SRV-P7B4 internal status vocabulary accepted; no Server/API parallel DTO invention.
- Allowed files: API passive DTO/parser/error tests and API-owned docs/control.
- Forbidden: Server runtime, database, filesystem, provider calls, mutation execution.
- Deliverables:
  - safe DTOs for configured mode, lifecycle, readiness category, in-progress flag, pending hint count, since-start counters, last-cycle cause/result category and cursor presence;
  - passive manual one-cycle request/response contract if Orchestrator accepts a public operator route;
  - no raw root, cursor, key, SQLx/I/O error or payload fields.
- Test/CI: serde/backward-compatibility/redaction tests and normal Component CI.
- Clean gate: mandatory clean-code review after green CI.
- Unblocks: SRV-P7B5 and CLI operator work.

### 7. SRV-P7B5 — Safe status and readiness integration

- Suggested chat: `server — W1 SRV-P7B5 Worktree Status and Readiness`
- Owner: server
- Branch: `component/server`
- Role: implementation-worker
- Prerequisites: SRV-P7B4 and API-P8 clean-accepted and synchronized.
- Allowed files: Server readiness/admin/operator route mapping, state/tests/docs/control.
- Forbidden: API shape invention, repair/destructive actions, raw diagnostics, provider behavior.
- Deliverables:
  - Worktree readiness component and safe admin status mapping;
  - enabled-mode readiness requires config, durable binding and live non-failed host;
  - Disabled does not make Server unready;
  - cycle failure degrades Worktree readiness/status but not `/health`;
  - optional explicit one-cycle endpoint delegates to the single host and cannot overlap;
  - dependency-free router behavior remains unchanged.
- Test/CI: route/status/redaction/readiness tests and normal Component CI.
- Clean gate: mandatory clean-code review after green CI.
- Unblocks: CLI-P6A and Deployment fan-in.

### 8. CLI-P6A — Worktree status and explicit sync-once operator commands

- Suggested chat: `cli — W1 CLI-P6A Worktree Sync Once`
- Owner: cli
- Branch: `component/cli`
- Role: implementation-worker
- Prerequisites: API-P8 and SRV-P7B5 clean-accepted.
- Allowed files: CLI source/tests/docs/control.
- Forbidden: direct DB/filesystem mutation, local runtime hosting, provider calls, bypassing Server/API, automatic repair.
- Deliverables:
  - read-only Worktree runtime status;
  - explicit `sync once`/DryRun invocation through Server API only;
  - safe busy/not-ready/cancelled/failure mapping;
  - no raw token, cursor, root or internal error output.
- Test/CI: parser/client/output/redaction tests and Component CI.
- Clean gate: mandatory clean-code review after green CI.
- Unblocks: operator-controlled rollout and bootstrap checks.

### 9. DEP-P5A — Worktree runtime/config fan-in

- Suggested chat: `deployment — W1 DEP-P5A Worktree Runtime Fan-In`
- Owner: deployment
- Branch: `component/deployment`
- Role: implementation-worker
- Prerequisites: STOR-P10, SRV-P7B4 and SRV-P7B5 clean-accepted; migration execution policy accepted.
- Allowed files: `deploy/**`, `.env.example` only for placeholder alignment, Deployment docs/control.
- Forbidden: product source, real secrets, automatic bidirectional enablement, hidden migration execution, destructive cleanup.
- Deliverables:
  - documented env/config keys and defaults;
  - stable Worktree adapter identity and root binding procedure;
  - directory permissions and service lifecycle;
  - migration/backup/rollback steps;
  - staged disabled -> export/import -> bidirectional rollout;
  - shutdown/readiness/status runbook.
- Test/CI: compose/config syntax validation and normal applicable Component CI.
- Clean gate: mandatory clean-code review after green CI.
- Unblocks: deployment-ready hosted Worktree runtime.

## Cross-phase acceptance rules

- Every code-bearing phase runs normal Component CI; no CI skip.
- Each owner phase receives mandatory clean-code review after green CI.
- Consumer phases may start only from exact accepted owner-component SHAs synchronized by an explicit fan-in prompt.
- No phase may independently redesign another component's contract.
- Source behavior accepted in SRV-P7A remains closed unless the relevant phase explicitly changes it.
- No phase may add nested runtimes, blocking async bridges, detached execution, internal HTTP self-calls, fake production state, hard delete or automatic destructive repair.

## Completion criteria for SRV-P7B

SRV-P7B is complete only when:

- Worktree cycle execution is natively awaitable and no-overlap;
- routes and Worktree use one reusable Server application-service layer;
- Worktree instance/path state and export cursor are durable and crash-recoverable;
- the real executor is bounded, cancellable and policy-aligned;
- the hosted loop is explicit, joined, configurable and observable;
- status/readiness are safe and honest;
- CLI and Deployment consume accepted contracts without bypassing Server/Core/Storage ownership.