# W1-SRV-P7B3-BOUNDED-WORKTREE-EXECUTOR — Implement the real bounded Worktree cycle executor

Before starting, name this worker chat exactly:

`server — W1 SRV-P7B3 Bounded Worktree Executor`

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: implementation-worker
Phase: SRV-P7B3-BOUNDED-WORKTREE-EXECUTOR

Work through the GitHub connector. Do not merge PR #45 into main, change draft state, rebase, reset, rewrite history, force-push, modify sibling branches, or begin the hosted-runtime phase.

## Accepted prerequisites

This phase is unblocked by the following exact accepted evidence:

- fan-in clean review report commit: `05250a460f9865d00a822e795ec510866f52bd0a`;
- fan-in clean report blob: `558bd7b62ac08dbdc06efacceace84764217de70`;
- fan-in status: `CLEAN_ACCEPT`;
- accepted integration code-bearing SHA: `4f8b3d9219961409847b12e393d9a38dc6377dea`;
- authoritative Component CI: run ID `29235942761`, run number `1840`, attempt `1`, conclusion `success` on that exact SHA;
- Worktree WT-P10 owner SHA: `1942946331e8362f19907ab6ad4eb779da70fd57`;
- Storage STOR-P10 owner SHA: `66b6a1f554aae1d1b774cc88560d46dd140c7a54`;
- Server SRV-P7B2 application-services SHA: `647dce7b624d67663632808906896cb6745ea7e7`.

The accepted Worktree and Storage snapshots on `component/server` are authoritative and immutable for this phase.

## Goal

Implement a real `ServerWorktreeCycleExecutor` that satisfies the accepted async `WorktreeRuntimeCycle` contract for exactly one bounded cycle.

The executor must connect existing Worktree filesystem/reconciliation primitives to existing Server application services and accepted Storage durable Worktree state/cursor repositories without duplicating Core policy or creating a hosted scheduler.

This phase implements the executor only. It does not start or host it.

## Required architecture

Preserve these ownership boundaries:

- Core is the only conflict, overwrite and guarded-delete policy authority.
- Server application services are the only authoritative internal mutation/read boundary used by the executor.
- Worktree owns scanning, reconciliation, filesystem writing, trash, echo and scheduler semantics.
- Storage owns schema and passive repositories; transaction timing remains Server-owned.
- API owns future public status/manual-cycle DTOs.
- Server hosted task lifecycle belongs to SRV-P7B4, not this phase.

Do not call Server HTTP routes from the executor. Do not duplicate route-private policy. Do not use internal HTTP.

## Required implementation

### 1. Executor contract

Implement a Server-owned executor type, expected name:

`ServerWorktreeCycleExecutor`

It must implement:

`haze_sync_worktree::WorktreeRuntimeCycle`

Its `run_cycle` must:

- return the accepted boxed `Send` future;
- await all asynchronous authority directly;
- execute at most the single requested cycle;
- perform no task detachment or hidden scheduling;
- observe cooperative cancellation before work, between bounded stages and between individual actions;
- return only accepted `WorktreeRuntimeCycleSummary` counts or accepted coarse `WorktreeRuntimeCycleFailure` categories.

### 2. Dependencies and construction

Use explicit injected dependencies. They may include:

- `ServerApplicationServices`;
- the Worktree adapter identity;
- validated Worktree configuration/root;
- PostgreSQL pool or narrowly scoped transaction/repository access required for accepted Storage repositories;
- synchronous Worktree scanner/reconciler/materializer/trash/echo primitives;
- bounded configuration required by one cycle.

Do not introduce global state, fake production repositories, nested runtimes or blocking async bridges.

The Worktree instance/root binding is an accepted Storage contract. The executor must fail safely on a missing or mismatched required binding. Do not silently rebind a different root. Explicit startup binding/host orchestration remains SRV-P7B4.

### 3. Bounded local-state load and scan

For a request requiring a full scan:

- load the durable Worktree path-state snapshot through accepted paginated Storage repository operations;
- keep every page and total action count bounded;
- execute synchronous filesystem scanning/reconciliation through awaited bounded blocking-pool work only;
- do not run synchronous filesystem work on the async executor thread;
- preserve Worktree ignore/path-safety/stability semantics;
- treat watcher hints only as latency hints; do not replace required full scans with hinted paths.

If `full_scan_required` is false, do not claim a full scan was completed.

### 4. Local import and guarded delete

When `request.import_enabled` is true and the mode permits imports:

- derive local import/delete actions through accepted Worktree planning/reconciliation primitives;
- process no more than `max_import_actions` imports and `max_delete_candidates` delete candidates;
- submit every authoritative file mutation through `ServerApplicationServices::apply_file`;
- submit every authoritative delete through `ServerApplicationServices::apply_delete`;
- use the accepted Worktree adapter actor identity;
- use deterministic, domain-separated replay/idempotency material without logging it;
- preserve base-revision semantics from durable Worktree state;
- record accepted resulting revision/hash/tombstone facts through accepted Storage Worktree state repositories;
- never directly write authoritative revision/conflict/tombstone/operation-log tables from executor code;
- never turn a missing local file directly into an unsafe hard delete.

Crash/retry behavior must be safe when an application-service mutation succeeds before durable Worktree path state is updated. Re-running the cycle must converge through deterministic idempotency rather than duplicate authoritative operations.

### 5. Ordered Core-to-Worktree export

When `request.export_enabled` is true and the mode permits exports:

- initialize/read the accepted adapter cursor without exposing raw cursor details;
- retrieve authoritative changes through `ServerApplicationServices::authoritative_changes` using a bounded limit derived from `max_export_actions`;
- process changes in exact ascending contiguous sequence order;
- retrieve authoritative revision bytes through `ServerApplicationServices::revision_content` when required;
- materialize files or tombstones through accepted Worktree filesystem/trash/echo primitives;
- persist corresponding durable Worktree path state through accepted Storage repositories;
- advance `adapter_cursors.last_core_seq` only through `advance_exact_contiguous`;
- advance a sequence only after its filesystem and durable path-state effects are successfully complete;
- never skip a failed sequence or advance beyond an unprocessed change;
- stop safely at the first non-recoverable export failure.

A crash after a filesystem effect but before durable state/cursor commit must be replay-safe and converge without corrupting or silently skipping the authoritative sequence.

### 6. Transactions and blocking boundaries

Storage repositories remain passive and caller-transaction-owned.

Use Server-owned transactions where atomic Storage state/cursor updates are required. Do not hold a database transaction open across unbounded filesystem work or application-service calls.

Only synchronous filesystem phases may use `tokio::task::spawn_blocking` or an equivalent awaited bounded blocking-pool boundary. Every spawned blocking operation must be joined before `run_cycle` returns. Do not use `block_on`.

### 7. Cancellation, summaries and failures

Cancellation must:

- prevent starting later stages/actions;
- avoid cursor advancement for incomplete export work;
- avoid false durable-state claims;
- return `WorktreeRuntimeCycleFailure::Cancelled`.

Summaries must be truthful and count-only:

- `full_scan_completed` reflects an actually completed required scan;
- `scanned_files` and `skipped_entries` reflect observed scan results;
- planned/submitted/applied counts must not exceed request budgets;
- `submitted_imports` counts actual completed authoritative import/delete submissions according to the accepted summary contract;
- `applied_exports` counts only fully applied and checkpointed export actions.

Do not place paths, roots, content, cursor values, tokens, database URLs or raw internal errors in public/coarse failures, Debug output or report text.

## Required tests

Add focused Server-owned tests proving at minimum:

1. a bounded full-scan cycle imports a new or modified local file through application services;
2. deterministic replay after mutation-before-state interruption does not duplicate authoritative operations;
3. local deletion uses guarded application-service delete behavior and remains bounded;
4. ordered export materializes authoritative content and advances only the exact contiguous cursor;
5. export failure before completion leaves the failed sequence unadvanced;
6. replay after materialization-before-checkpoint converges safely;
7. cancellation before and during action processing stops later work and leaves incomplete cursor/state unclaimed;
8. import, delete and export budgets are enforced;
9. required-scan and mode permissions are preserved rather than bypassed;
10. no cycle overlap, task detachment, nested runtime, internal HTTP or fake production repository is introduced;
11. errors, Debug and summaries do not expose private root/path/DB/idempotency material.

Use real PostgreSQL, real object-store/temp-worktree integration where required. Test-only failure-injection seams are allowed when narrowly scoped and impossible to activate in production.

## Allowed files

Primary allowed scope:

- `crates/haze-sync-server/src/worktree_executor/**` or an equivalently clear Server-owned executor module;
- `crates/haze-sync-server/src/lib.rs` or internal module declarations required to expose the executor inside the Server crate;
- `crates/haze-sync-server/src/application/**` only for narrowly required internal visibility or reusable typed helpers, without changing accepted route behavior;
- `crates/haze-sync-server/src/worktree_runtime.rs` only for executor-facing Server-local composition types, not hosted scheduling;
- Server-owned tests and test support required for this phase;
- `crates/haze-sync-server/docs/component-contract.md`;
- `crates/haze-sync-server/docs/implementation-plan.md`;
- `crates/haze-sync-server/docs/implementation-log.md`;
- `crates/haze-sync-server/docs/dependency-map.md`;
- `crates/haze-sync-server/Cargo.toml` and `Cargo.lock` only when directly required by this implementation;
- `crates/haze-sync-server/control/report.md`.

Any broader Server-internal change must be necessary for a clean executor implementation and must be explained in the report.

## Protected and forbidden files

Do not modify:

- `crates/haze-sync-worktree/**`;
- `crates/haze-sync-storage/**`;
- `migrations/**`;
- `crates/haze-sync-core/**`;
- `crates/haze-sync-api/**`;
- `crates/haze-sync-cli/**`;
- `deploy/**`;
- sibling component `control/**`;
- `.github/workflows/**` unless the Orchestrator issues a separate tooling/fixer prompt.

Do not implement:

- `ServerWorktreeRuntimeHost`;
- startup/shutdown host task wiring;
- watcher or periodic scheduling ownership;
- manual request channels or public manual-cycle routes;
- API-P8 DTOs or lifecycle vocabulary;
- SRV-P7B5 readiness/status behavior;
- CLI or Deployment behavior;
- provider behavior;
- schema redesign;
- hard delete or automatic destructive repair.

If an accepted Worktree or Storage contract is insufficient or defective, do not silently change the owner snapshot. Report `BLOCKED_BY_CONTRACT` with exact owner path, owner SHA, required change and evidence.

## Verification and CI

Create a real code-bearing implementation commit without CI skip.

Authoritative completion requires DB-capable Component CI on the exact final code-bearing SHA with:

- cargo fmt success;
- cargo check success;
- isolated Server PostgreSQL tests success;
- isolated Storage PostgreSQL tests including mandatory STOR-P10 evidence success;
- remaining workspace tests success;
- cargo clippy with warnings denied success;
- diagnostics finalizer success.

If CI fails, do not guess from wrapper summaries. Record the failed run/artifact metadata honestly and leave CI diagnostics work to a fixer slot unless this prompt is explicitly changed.

## Mandatory report

Write `crates/haze-sync-server/control/report.md` using `report-template.md`.

Set:

- `REPORT_TYPE: IMPLEMENTATION`;
- `phase_id: SRV-P7B3-BOUNDED-WORKTREE-EXECUTOR`;
- `chat_name: server — W1 SRV-P7B3 Bounded Worktree Executor`.

Use one honest status:

- `SELF_ACCEPT`;
- `SELF_ACCEPT_PENDING_CI`;
- `SELF_NEEDS_FIX`;
- `BLOCKED_BY_CONTRACT`;
- `BLOCKED_BY_DEPENDENCY`;
- `BLOCKED_BY_TOOLING`.

The report must include:

- exact base and final code-bearing SHA;
- complete changed paths;
- executor architecture and explicit dependencies;
- import/delete/export/checkpoint flow;
- transaction and blocking-pool boundaries;
- cancellation and replay/crash behavior;
- exact tests added and evidence;
- confirmation that protected owner snapshots were unchanged;
- exact final Component CI run ID, number, attempt, SHA and job conclusions;
- any blocker or deferred host/status work;
- whether the implementation is ready for mandatory SRV-P7B3 clean-code review.

This phase is incomplete until a real code-bearing branch advance and committed report exist. Do not claim that SRV-P7B4 is active; only the Orchestrator may rotate the next slot after mandatory clean review.
