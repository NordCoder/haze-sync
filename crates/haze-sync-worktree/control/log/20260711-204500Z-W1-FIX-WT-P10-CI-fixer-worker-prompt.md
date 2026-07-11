# W1-FIX-WT-P10-CI — Awaitable Worktree runtime CI correction

Before starting, name this worker chat exactly:

`worktree — W1 WT-P10 CI Fix`

Component: worktree
Path: crates/haze-sync-worktree
Branch: component/worktree
PR: #49
Role: fixer-worker

Work only through the GitHub connector. Do not merge the PR, change its draft state, rebase, reset, rewrite history, force-push, modify `main`, or modify sibling branches.

## Trigger and accepted implementation

WT-P10 completed the Worktree-owned awaitable runtime contract implementation, but its authoritative code-bearing Component CI is red.

Implementation evidence:

- implementation phase: `WT-P10`
- implementation status: `BLOCKED_BY_TOOLING`
- final code-bearing/source/docs SHA: `b3c62d4e1655d0a595290a87c593974a5ed1a3dc`
- workflow: `Component CI`
- workflow run id: `29165931603`
- run number: `1737`
- attempt: `1`
- conclusion: `failure`
- visible passing steps: cargo fmt, cargo check, cargo test, cargo clippy
- visible failing stage: Finalize CI diagnostics

Archived phase evidence:

- prompt snapshot: `crates/haze-sync-worktree/control/log/20260711-201500Z-W1-WT-P10-implementation-worker-prompt.md`
- report snapshot: `crates/haze-sync-worktree/control/log/20260711-201500Z-W1-WT-P10-implementation-worker-report.md`
- exact implementation report blob: `a513838ba1abc1c7d62135498d0fd12dca196d88`

The accepted implementation introduced:

- an awaitable boxed `Send` cycle future;
- async-compatible `WorktreeRuntimeService::poll` awaiting at most one cycle;
- cooperative cancellation with post-await normalization;
- explicit inert/planning-only DryRun semantics;
- preserved full-scan, watcher, budget, summary-validation and no-overlap guarantees;
- deterministic std-only async tests;
- safe path/payload-free status and Debug output.

Do not redesign or revert this accepted contract unless the exact diagnostics artifact proves that a narrowly scoped correction is required.

## Exact diagnostics artifact

Use only this artifact:

- artifact name: `ci-diag__component-worktree__wf-component-ci__run-29165931603__attempt-1`
- artifact id: `8252153523`
- artifact head SHA: `b3c62d4e1655d0a595290a87c593974a5ed1a3dc`
- artifact digest: `sha256:e69d46bb8077a0ec7496df3480e270d95761191d5b7e612c9d06c45275924fc9`
- artifact size bytes: `2977`
- created at: `2026-07-11T19:51:15Z`
- expires at: `2026-07-12T19:51:15Z`
- status when assigned: available and unexpired

## Required reads

Before editing, read:

- project implementation manifest, report template, fixer-worker prompt and GitHub connector guidance;
- current Worktree control state and this prompt;
- archived WT-P10 prompt/report snapshots and exact report blob;
- final WT-P10 changed source/tests/docs at the code-bearing SHA;
- the exact diagnostics artifact above.

## Diagnostics protocol

Through the GitHub connector:

1. fetch artifact `8252153523` from workflow run `29165931603`;
2. verify its head SHA exactly equals `b3c62d4e1655d0a595290a87c593974a5ed1a3dc`;
3. read `summary.md` and `manifest.json` at their actual archive locations;
4. read every failure marker and log named by `failed_checks`;
5. apply only the complete minimum artifact-proven correction.

Raw GitHub job logs are not authorized as fallback. If the artifact is missing, expired, malformed, mismatched or unreadable, report `FIX_BLOCKED_BY_LOGS` without guessing.

## Preservation requirements

Preserve unless the artifact directly proves otherwise:

- awaitable `WorktreeRuntimeCycle` contract and boxed `Send` future;
- async `poll` awaiting at most one cycle;
- no overlap and no later cycle after cancellation;
- cooperative cancellation before/after phases and bounded items;
- explicit DryRun with no automatic execution or mutation permission;
- Disabled inertness;
- startup/watcher/periodic full-scan correctness;
- watcher-failure periodic fallback;
- import/delete/export budgets and result validation;
- lifecycle misuse protections;
- safe count/category-only status and Debug output;
- synchronous scanner/planner/materializer/echo/trash behavior;
- no Server, Storage, SQLx, HTTP or provider behavior;
- no nested runtime, `Handle::block_on`, detached task, hidden task or fabricated summary;
- no new dependency unless the artifact makes it strictly unavoidable and the report explains why.

## Allowed files

- `crates/haze-sync-worktree/src/runtime.rs` only if artifact-proven;
- `crates/haze-sync-worktree/src/runtime_tests.rs` only if artifact-proven;
- `crates/haze-sync-worktree/src/lib.rs` only if artifact-proven;
- WT-P10 Worktree docs only if the correction changes a documented fact;
- `crates/haze-sync-worktree/Cargo.toml` only if artifact-proven and unavoidable;
- `crates/haze-sync-worktree/control/report.md`.

Do not modify Worktree scanner/planner/materializer/delete/trash/doctor behavior, sibling components, workflows, public HTTP/API contracts, schemas or migrations. Do not archive control files.

## CI and report

If product source, tests, Cargo or docs change, commit normally and require a new code-bearing Component CI run. CI skip is allowed only for the final report-only commit.

Write `crates/haze-sync-worktree/control/report.md` using `report-template.md` with:

- `REPORT_TYPE: FIX`
- `phase_id: FIX-WT-P10-CI`
- `chat_name: worktree — W1 WT-P10 CI Fix`

Use an honest status:

- `FIX_COMPLETE`
- `FIX_NEEDS_MORE`
- `FIX_BLOCKED_BY_LOGS`
- `FIX_BLOCKED_BY_SCOPE`
- `FIX_BLOCKED_BY_CONTRACT`
- `FIX_BLOCKED_BY_TOOLING`

The report must include artifact verification, exact artifact-proven cause, changed files, behavior-change assessment, final code-bearing SHA, post-fix CI run and conclusion, preserved WT-P10 invariants, and whether the phase is ready for mandatory clean-code review.