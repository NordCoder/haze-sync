# W1-WT-P10 — Awaitable Worktree runtime contract

Before starting, name this worker chat exactly:

`worktree — W1 WT-P10 Async Runtime Contract`

Component: worktree
Path: crates/haze-sync-worktree
Branch: component/worktree
PR: #49
Role: implementation-worker

Work only through the GitHub connector. Do not merge the PR, change its draft state, rebase, reset, rewrite history, force-push, modify `main`, modify sibling branches, or implement Server/Storage behavior.

## Accepted architecture authority

The Server architecture pass `ARCH-SRV-P7B-CONTRACTS` returned `ARCHITECT_CHANGED_CONTRACTS` and accepted one target architecture.

Authoritative evidence:

- Server architecture documentation SHA: `b98f5079ed90ccf7eaec79e617ae591e0c308ff4`
- Component CI run: `29165123142`
- run number: `1719`
- conclusion: `success`
- final architecture report-only commit: `22b5e993431a1c0a858716ac835a3ece8ade6d5c`
- archived architecture report on `component/server`: `crates/haze-sync-server/control/log/20260711-191500Z-W1-ARCH-SRV-P7B-CONTRACTS-architect-report.md`

The chosen architecture keeps synchronous filesystem primitives in Worktree, but makes the authoritative runtime cycle boundary awaitable. Worktree continues to own scheduler correctness, watcher-hint coalescing, periodic full-scan rules, budget validation, result validation, cancellation vocabulary, count-only status, and no-overlap guarantees. Server will later own the concrete async executor and joined Tokio host.

Accepted Worktree baseline:

- code-bearing SHA: `4f7bc748d9b901d7d5c3e43c845ba407c0c36e59`
- Component CI run: `29152965199`, run number `1665`, conclusion `success`
- existing scanner/planner/materializer/echo/trash/doctor behavior is accepted and must remain unchanged unless required by this runtime-contract phase.

## Required reads

Before editing, read:

- project implementation manifest, report template, implementation-worker prompt, and GitHub connector guidance;
- current Worktree control state and this prompt;
- Worktree component contract, implementation plan, dependency map, decisions, runtime-service docs, implementation log, Cargo manifest, `src/runtime.rs`, `src/runtime_tests.rs`, and exports in `src/lib.rs`;
- accepted scanner, import-planning, delete-guard, reconciliation, materialization, echo, trash, and doctor public contracts;
- Server architecture report and updated Server contract/plan/dependency-map/decisions at exact architecture SHA `b98f5079ed90ccf7eaec79e617ae591e0c308ff4`.

Do not read CI diagnostics artifacts unless a new CI failure is later routed by an exact fixer prompt.

## Task

Implement the Worktree-owner contract changes required for an awaitable runtime boundary. This phase changes Worktree contracts and deterministic scheduler tests only. It does not implement a Server executor, SQLx operations, durable Storage repositories, or a hosted background loop.

### 1. Awaitable cycle contract

Replace or supplement the synchronous `WorktreeRuntimeCycle::run_cycle` boundary with an async-compatible, `Send` future contract that a Server-owned executor can implement without nested runtimes or blocking.

Required properties:

- `run_cycle` accepts the existing bounded `WorktreeRuntimeCycleRequest` plus an explicit cooperative cancellation view/token;
- the returned future is awaitable and `Send` under the supported runtime model;
- no `async-trait` or other dependency may be added casually: use the smallest Rust contract compatible with the workspace rust-version and current generic service design; any dependency change must be justified in the report and remain Worktree-local;
- no nested Tokio runtime, `Handle::block_on`, ad hoc blocking, detached task, callback with fabricated summary, or internal HTTP behavior.

### 2. Async scheduler poll

Make `WorktreeRuntimeService::poll` async-compatible and await at most one cycle.

Preserve:

- at most one cycle in progress;
- no overlapping startup/watcher/periodic/manual cycles;
- watcher hints remain latency hints only;
- startup, periodic, and watcher-triggered local-observing cycles require authoritative full scan;
- watcher failure does not remove periodic correctness;
- all import/delete/export budgets remain hard bounds;
- executor summaries are still validated before becoming accepted status;
- safe count-only failures/status with no paths, bytes, tokens, cursors, raw I/O, or internal payloads.

### 3. Cooperative cancellation contract

Add a Worktree-owned cancellation type or trait with safe `is_cancelled` observation.

Required semantics:

- cancellation can be observed before and after phases and between bounded items by a future Server executor;
- after cancellation is requested, no new cycle may begin;
- one already-running atomic filesystem operation may complete, but the contract must allow the executor to stop before subsequent phases;
- cancellation maps only to the accepted safe `Cancelled` failure/category;
- start, cancellation request, status, and watcher shutdown may remain synchronous when they perform no async authority;
- shutdown and restart-prevention invariants remain explicit.

### 4. DryRun representation

Align the Worktree-owned runtime mode contract with the accepted architecture:

- DryRun is an explicit mode/representation, not silently mapped to ReadOnly or another mode;
- DryRun allows explicit/manual planning only in later Server phases;
- it must not authorize Core mutation, cursor advancement, materialization, trash movement, echo writes, or durable Worktree-state mutation;
- this phase only establishes truthful mode/request/policy semantics and tests; it does not implement the future manual-cycle endpoint or host.

If adding DryRun directly to `WorktreeMode` would break an accepted invariant, document and implement the smallest explicit alternative representation that still lets Server distinguish it without an unsupported-mode placeholder. Do not leave ambiguity.

### 5. Compatibility and public surface

Retain current request, budget, summary, failure, status, lifecycle, watcher, and policy types where practical.

Update exports and documentation so downstream Server code can consume the new contract deliberately. Existing synchronous scanner/planner/materializer/trash/echo APIs remain synchronous and behaviorally unchanged.

## Tests

Add or migrate deterministic tests proving:

- an async executor is awaited and its real summary/failure is returned;
- `poll` runs at most one cycle and cannot overlap cycles;
- cancellation before a due cycle prevents execution;
- cancellation observed by an in-flight executor maps safely and prevents later phases/cycles;
- startup, watcher-hint, and periodic causes remain correct;
- full-scan requirement is preserved for local-observing modes;
- watcher close/failure still falls back to periodic correctness;
- import/delete/export budgets and summary-contract validation remain exact;
- DryRun is explicit and mutation/materialization permissions are false;
- Disabled remains completely inert;
- restart, double-start, shutdown-before-start, and double-shutdown protections remain correct;
- status and Debug output remain path/secret/payload-free.

Use deterministic immediately-ready or manually controlled futures. Do not add sleeping/flaky wall-clock tests or require Server/Storage/database access.

## Allowed files

- `crates/haze-sync-worktree/src/runtime.rs`
- `crates/haze-sync-worktree/src/runtime_tests.rs`
- `crates/haze-sync-worktree/src/lib.rs`
- other Worktree-local runtime test modules only if narrowly required
- `crates/haze-sync-worktree/Cargo.toml` only if a minimal contract/test dependency is demonstrably required and justified
- `crates/haze-sync-worktree/docs/component-contract.md`
- `crates/haze-sync-worktree/docs/runtime-service.md`
- `crates/haze-sync-worktree/docs/dependency-map.md`
- `crates/haze-sync-worktree/docs/decisions.md`
- `crates/haze-sync-worktree/docs/implementation-plan.md`
- `crates/haze-sync-worktree/docs/implementation-log.md`
- `crates/haze-sync-worktree/control/report.md`

If another Worktree file is compile-proven necessary, report the exact need as `BLOCKED_BY_SCOPE` rather than silently expanding.

## Forbidden scope

- no Server, Storage, Core, API, Common, GDrive, Obsidian, CLI, or Deployment changes;
- no SQLx/database/object-store/HTTP/provider behavior;
- no concrete Server executor;
- no watcher implementation or hosted loop;
- no hidden or detached tasks;
- no blocking async bridge;
- no schema/migration/config/public DTO changes;
- no scanner/planner/materializer/delete/trash/repair semantic redesign;
- no hard delete or automatic destructive repair;
- no workflow changes;
- no unrelated cleanup;
- do not archive control files.

## CI policy

All source/test/docs/Cargo changes must run Component CI normally.

Required gates:

- cargo fmt;
- cargo check;
- cargo test;
- cargo clippy with warnings denied;
- Finalize CI diagnostics.

Only the final report-only commit may use `[skip ci]`. If CI fails, record the exact code-bearing SHA and run, do not read raw logs or guess; Orchestrator will assign an artifact-based fixer.

## Report

Write `crates/haze-sync-worktree/control/report.md` using `report-template.md`.

Set:

- `REPORT_TYPE: IMPLEMENTATION`
- `phase_id: WT-P10`
- `chat_name: worktree — W1 WT-P10 Async Runtime Contract`

Use an honest status:

- `SELF_ACCEPT`
- `SELF_ACCEPT_PENDING_CI`
- `SELF_NEEDS_FIX`
- `BLOCKED_BY_CONTRACT`
- `BLOCKED_BY_SCOPE`
- `BLOCKED_BY_TOOLING`

The report must include exact contract signatures, compatibility decisions, DryRun semantics, cancellation/no-overlap behavior, changed files, tests, final code-bearing SHA, authoritative CI evidence, secrecy assessment, and whether WT-P10 is ready for mandatory clean-code review.
