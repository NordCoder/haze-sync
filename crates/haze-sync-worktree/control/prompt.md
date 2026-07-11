# W1-WT-P10-COMPLETE-FAN-IN — Awaitable Worktree runtime contract accepted

Component: worktree
Path: crates/haze-sync-worktree
Branch: component/worktree
PR: #49
Role: none

This is a hold notice, not an executable worker prompt.

## Accepted lifecycle

WT-P10 is fully accepted:

- implementation: `SELF_ACCEPT`
- fixer: `FIX_COMPLETE`
- clean-code review: `CLEAN_ACCEPT`
- accepted code-bearing SHA: `1942946331e8362f19907ab6ad4eb779da70fd57`
- Component CI run: `29167289593`
- run number: `1799`
- conclusion: `success`

The accepted contract includes:

- an awaitable boxed `Send` `WorktreeRuntimeCycle` future;
- async `WorktreeRuntimeService::poll` awaiting at most one cycle;
- cooperative cancellation before, during, and after a cycle;
- explicit non-automatic, mutation-free DryRun semantics;
- preserved full-scan, watcher fallback, mode, budget, summary-validation, lifecycle, no-overlap, and safe-status invariants;
- unchanged synchronous scanner, planner, materializer, echo, trash, and doctor contracts.

## Hold reason

The component is waiting for the parallel owner phases:

- `STOR-P10` durable Worktree state and cursor persistence;
- `SRV-P7B2` reusable Server application services.

`SRV-P7B3` must not begin until WT-P10, STOR-P10, and SRV-P7B2 are all clean-accepted and their exact accepted SHAs are synchronized into the Server integration scope.

## Unblock condition

Only an explicit Orchestrator fan-in/integration prompt pinned to this accepted SHA may reactivate Worktree-owned files. Do not launch an agent from this hold notice.