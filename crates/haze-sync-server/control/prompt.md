# W1-SRV-P7B4-HOSTED-WORKTREE-RUNTIME — Implement hosted scheduling, startup and shutdown

Before starting, name this worker chat exactly:

`server — W1 SRV-P7B4 Hosted Worktree Runtime`

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: implementation-worker
Phase: SRV-P7B4-HOSTED-WORKTREE-RUNTIME

Work through the GitHub connector. Do not merge PR #45, change draft state, rewrite history, modify sibling branches, or begin API-P8 / SRV-P7B5 / CLI / Deployment work.

## Accepted prerequisite

SRV-P7B3 is clean-accepted:

- final executor code-bearing SHA: `f8475af72b3e1795c5b11fa39f4625191eff59b1`;
- final clean-review report commit: `b9e22533091a9477876b91729c04682266a1b718`;
- final clean-review report blob: `18d6ac03c77498570b21db0768155cf31ad978d1`;
- status: `CLEAN_ACCEPT`;
- authoritative Component CI: run `29257244778`, number `1860`, attempt `1`, success.

Accepted owner snapshots remain authoritative and immutable:

- Worktree WT-P10: `1942946331e8362f19907ab6ad4eb779da70fd57`;
- Storage STOR-P10: `66b6a1f554aae1d1b774cc88560d46dd140c7a54`;
- Server application services: `647dce7b624d67663632808906896cb6745ea7e7`;
- Server bounded executor: `f8475af72b3e1795c5b11fa39f4625191eff59b1`.

## Goal

Implement `ServerWorktreeRuntimeHost`: one explicit, joined, cancellable Tokio host task that owns hosted Worktree runtime scheduling and lifecycle around the accepted bounded executor.

This phase adds hosting only. It must not invent public API DTOs, readiness/status vocabulary, CLI behavior or Deployment behavior.

## Required implementation

1. Add explicit validated runtime-host configuration for mode, debounce, periodic interval, watcher hint budget, action budgets and bounded shutdown timeout. Defaults must be safe and must not silently enable bidirectional behavior.
2. Bind and verify durable Worktree adapter/root identity before the first automatic or manual cycle can run.
3. Construct the accepted `WorktreeRuntimeService` and accepted `ServerWorktreeCycleExecutor` without duplicating their policy or filesystem logic.
4. Own exactly one joined Tokio task. No detached task, nested runtime, `block_on`, internal HTTP or second concurrent cycle.
5. Drive startup cycle, periodic scheduling and watcher-hint polling through the accepted Worktree runtime contract.
6. Provide an internal bounded manual one-cycle request channel for later Server/API integration. Requests must not overlap; busy/coalesced/rejected behavior must be explicit and safe. Do not expose a public route or API DTO in this phase.
7. Preserve exact mode behavior:
   - Disabled: inert and not unready by itself;
   - ReadOnly / ImportOnly / ExportOnly / Bidirectional: only accepted automatic permissions;
   - DryRun: manual-only planning, never automatic mutation or cursor advance.
8. Provide cooperative cancellation, bounded graceful shutdown and mandatory task join. Shutdown must stop new work, cancel in-flight work cooperatively, shut down watcher state and return an honest coarse result.
9. Keep status internal and count/category-only. No roots, fingerprints, cursors, tokens, DB URLs, SQLx/I/O strings or idempotency material.
10. Preserve at-most-one-cycle behavior under startup, periodic, watcher and manual triggers.
11. Add focused Tokio paused-time tests for startup, periodic cadence, debounce/coalescing, manual busy behavior, DryRun manual-only behavior, cancellation, bounded shutdown, join completion and no overlap.
12. Add DB/object-store/temp-worktree integration coverage where required to prove host-to-executor composition without fake production repositories.

## Scope

Allowed:

- Server config and startup composition directly required by hosted Worktree runtime;
- `crates/haze-sync-server/src/worktree_runtime.rs` or a Server-owned host module/replacement;
- Server state/composition and focused tests;
- narrow Server docs and manifest/lock changes directly required;
- Server control report.

Forbidden:

- accepted Worktree or Storage source changes;
- migrations or schema redesign;
- Core/API/CLI/Deployment product files;
- public status/readiness/manual-cycle DTOs or routes;
- multiple runtime tasks, detached tasks, unbounded retries, automatic repair, provider behavior or hard delete;
- sibling control files or workflows.

If an accepted owner contract is insufficient, do not modify owner snapshots here. Report the exact blocker with path/SHA/evidence for Orchestrator routing.

## CI and completion

Create a real code-bearing commit without CI skip. Require authoritative DB-capable Component CI on the exact final code-bearing SHA.

Write `crates/haze-sync-server/control/report.md` using `report-template.md` with:

- `REPORT_TYPE: IMPLEMENTATION`;
- `phase_id: SRV-P7B4-HOSTED-WORKTREE-RUNTIME`;
- `chat_name: server — W1 SRV-P7B4 Hosted Worktree Runtime`.

Use one honest status: `SELF_ACCEPT`, `NEEDS_FIX`, `BLOCKED_BY_CONTRACT`, `BLOCKED_BY_SCOPE`, or `BLOCKED_BY_TOOLING`.

The report must include changed paths, exact final code-bearing SHA, runtime/task ownership, configuration/defaults, lifecycle/mode/manual semantics, cancellation/shutdown behavior, tests, secrecy assessment, exact CI evidence and whether the phase is ready for mandatory clean-code review.

Do not claim `CLEAN_ACCEPT`, activate API-P8 or begin SRV-P7B5. Only Orchestrator rotates the next gate.
