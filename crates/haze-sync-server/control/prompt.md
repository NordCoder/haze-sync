# W1-SRV-P7B4-HOSTED-WORKTREE-RUNTIME — Implement Server-hosted Worktree runtime

Before starting, name this worker chat exactly:

`server — W1 SRV-P7B4 Hosted Worktree Runtime`

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: implementation-worker
Phase: SRV-P7B4-HOSTED-WORKTREE-RUNTIME

Do not merge PR #45, change draft state, rewrite history, modify sibling branches, or begin API/CLI/Deployment phases.

## Accepted baselines

- Server bounded executor SHA: `f8475af72b3e1795c5b11fa39f4625191eff59b1`;
- accepted Worktree WT-P11 source SHA: `b38264ce2b09632a4c0bab0dd77319e1db239a3b`;
- accepted Server fan-in SHA: `1b2b572a1a200f2968d005e48e9c0674f9db8bc0`;
- fan-in clean-review report commit: `0d2d6038fb84805e3844cd539c88fcb409a36cfc`;
- fan-in report blob: `ef03533a379c842d93beb4dc072958978da2991a`;
- authoritative DB-capable Component CI: run `29275250064`, number `1882`, success.

The prior owner-contract blocker is resolved. Do not re-open it unless the accepted integrated contract is demonstrably insufficient.

## Goal

Implement `ServerWorktreeRuntimeHost`: exactly one explicit joined cancellable Tokio task that owns and drives the accepted Worktree hosted runtime and accepted Server bounded cycle executor.

## Required behavior

1. Add validated Server runtime-host configuration for:
   - mode;
   - debounce;
   - periodic interval;
   - watcher hint budget;
   - action budgets;
   - bounded shutdown timeout;
   - bounded manual request capacity.

Defaults must be safe. Never silently enable bidirectional mutation.

2. Bind and verify durable Worktree adapter/root identity before the first cycle.

3. Construct and own the accepted Worktree components:
   - `ProductionWorktreeWatcher` from validated Worktree config/root capability;
   - `WorktreeRuntimeService` / `WorktreeHostedRuntime`;
   - accepted `ServerWorktreeCycleExecutor`.

Do not duplicate Worktree watcher, scheduler, no-overlap, cancellation or accounting policy in Server.

4. Host exactly one joined task:
   - no detached task;
   - no nested runtime;
   - no `block_on`;
   - no internal HTTP;
   - no concurrent cycle executor ownership.

5. Drive startup, periodic and watcher-hint polling through the accepted hosted runtime contract.

6. Expose an internal bounded manual one-cycle submission boundary for later API integration:
   - use the accepted Worktree manual handle/ticket contract;
   - preserve typed Busy/NotStarted/Cancelling/Shutdown/Cancelled outcomes;
   - no public DTO or route yet;
   - no executor bypass.

7. Mode semantics:
   - Disabled: inert and not unready by itself;
   - ReadOnly / ImportOnly / ExportOnly / Bidirectional: accepted automatic permissions only;
   - DryRun: manual-only, full-scan, no mutation and no cursor/checkpoint advancement.

8. Shutdown:
   - cooperative cancellation;
   - bounded graceful shutdown;
   - mandatory task join;
   - no lifecycle leak;
   - safe coarse status if timeout/failure occurs.

9. Add internal count/category-only, secret-safe runtime status suitable for later Server readiness/status work. Do not expose paths, database URLs, backend errors or payloads.

10. Preserve at-most-one-cycle across startup, periodic, watcher and manual triggers using only the accepted Worktree runtime boundary.

## Tests

Add focused Tokio tests, using paused time where useful, for:

- Disabled inert behavior;
- startup cycle;
- periodic cadence;
- watcher hint/debounce path;
- manual accepted/completed and Busy outcomes;
- DryRun non-mutation;
- cooperative cancellation;
- dropped/timeout shutdown path;
- mandatory task join;
- no overlapping cycles;
- safe redacted status;
- DB/object-store/temp-worktree integration where required.

## Scope

Allowed:

- Server runtime-host modules, focused tests and exports;
- minimal Server config/dependency wiring;
- narrow composition with accepted Worktree and Server executor contracts;
- Server control report.

Forbidden:

- accepted Worktree or Storage source changes;
- migrations/schema;
- Core/API/CLI/Deployment product files;
- public status/readiness/manual DTOs or routes;
- multiple/detached tasks, hidden runtimes or unbounded retry;
- provider behavior, repair, hard delete or unrelated cleanup;
- sibling control files or workflows.

Formatting/style alone is non-blocking when exact-SHA CI is green.

## Completion

Create a real code-bearing commit without CI skip. Obtain authoritative DB-capable Component CI on the exact final code-bearing SHA.

Write `crates/haze-sync-server/control/report.md` with:

- `REPORT_TYPE: IMPLEMENTATION`;
- `phase_id: SRV-P7B4-HOSTED-WORKTREE-RUNTIME`;
- `chat_name: server — W1 SRV-P7B4 Hosted Worktree Runtime`;
- honest status `SELF_ACCEPT`, `NEEDS_FIX`, `BLOCKED_BY_CONTRACT`, `BLOCKED_BY_SCOPE`, or `BLOCKED_BY_TOOLING`.

Record changed paths, task ownership/join semantics, config defaults, accepted Worktree composition, manual boundary, tests, secrecy, final SHA and exact CI evidence.

Do not claim `CLEAN_ACCEPT`, begin API-P8/SRV-P7B5 or change merge readiness. A focused functional review follows.
