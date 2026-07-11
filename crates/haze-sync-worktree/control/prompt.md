# W1-CLEAN-WT-P10 — Awaitable Worktree runtime clean-code review

Before starting, name this worker chat exactly:

`worktree — W1 WT-P10 Clean-Code Review`

Component: worktree
Path: crates/haze-sync-worktree
Branch: component/worktree
PR: #49
Role: clean-code-reviewer
Phase: WT-P10-CLEAN

Work only through the GitHub connector. Do not merge the PR, change draft state, rebase, reset, rewrite history, force-push, modify `main`, or modify sibling branches.

## Accepted review baseline

Review the complete WT-P10 contract change, including its artifact-based formatting correction.

- pre-phase accepted Worktree SHA: `4f7bc748d9b901d7d5c3e43c845ba407c0c36e59`
- WT-P10 implementation SHA before fixer: `b3c62d4e1655d0a595290a87c593974a5ed1a3dc`
- final reviewed code-bearing candidate SHA: `1942946331e8362f19907ab6ad4eb779da70fd57`
- authoritative Component CI run: `29167289593`
- run number: `1799`
- conclusion: `success`
- passed: cargo fmt, cargo check, cargo test, cargo clippy, Finalize CI diagnostics
- fixer status: `FIX_COMPLETE`
- fixer changed only formatting in `src/runtime.rs` and `src/runtime_tests.rs`

Archived evidence:

- WT-P10 implementation prompt/report under `control/log/20260711-201500Z-W1-WT-P10-*`
- WT-P10 fixer prompt/report under `control/log/20260711-204500Z-W1-FIX-WT-P10-CI-*`

Do not read CI diagnostics artifacts. The final code-bearing CI is green.

## Review scope

Review every WT-P10 product, test, and documentation change between the accepted baseline and final candidate, especially:

- `crates/haze-sync-worktree/src/runtime.rs`
- `crates/haze-sync-worktree/src/runtime_tests.rs`
- `crates/haze-sync-worktree/src/lib.rs`
- `crates/haze-sync-worktree/docs/runtime-service.md`
- `crates/haze-sync-worktree/docs/component-contract.md`

Assess correctness, clean code, API clarity, ownership, test quality, and downstream usability. Do not treat green CI as sufficient by itself.

## Mandatory review questions

1. Is `WorktreeRuntimeCycle::run_cycle` genuinely awaitable and `Send` without nested runtimes, blocking bridges, hidden tasks, fabricated summaries, or new unnecessary dependencies?
2. Does async `WorktreeRuntimeService::poll` await at most one cycle and preserve no-overlap through type/borrow/lifecycle design rather than timing assumptions?
3. Is cancellation observable before and during a cycle, normalized safely after await, and guaranteed to prevent later cycles?
4. Are startup, watcher-hint, periodic, manual DryRun, Disabled, cancellation, and shutdown transitions explicit and internally consistent?
5. Are watcher hints still latency-only while authoritative full scans remain mandatory for local-observing cycles?
6. Are import/delete/export budgets, mode permissions, summary-count validation, and full-scan validation preserved exactly?
7. Is DryRun explicit, non-automatic and incapable of authorizing mutation, cursor advancement, materialization, trash movement, echo writes, or durable state updates?
8. Do public/request/status/failure types expose the minimum stable contract required by future Server code without leaking implementation details?
9. Are Debug/Display/status outputs path-, payload-, token-, cursor- and executor-internal-free?
10. Are deterministic futures/tests sound, non-flaky, and strong enough to prove cancellation and no-overlap rather than merely exercising happy paths?
11. Are synchronous scanner/planner/materializer/echo/trash/doctor contracts unchanged?
12. Are docs accurate about Worktree ownership versus future Server executor/host ownership?
13. Did the phase introduce avoidable complexity, confusing names, broad visibility, dead code, unsafe generic bounds, pinning/lifetime hazards, or weak assertions?
14. Does the final diff contain only Worktree-owned changes plus orchestrator control files?

## Corrections

You may make focused Worktree-local clean-code/correctness corrections when necessary. Any product/test/docs correction must preserve the accepted architecture and run a new normal Component CI.

Allowed files:

- WT-P10 runtime source, exports and tests;
- WT-P10 Worktree-owned docs;
- `crates/haze-sync-worktree/Cargo.toml` only if correcting a demonstrable WT-P10 dependency defect;
- `crates/haze-sync-worktree/control/report.md`.

Forbidden:

- Server, Storage, Core, API, Common, provider, CLI or Deployment changes;
- concrete executor, SQLx, HTTP, hosted loop or watcher implementation;
- nested runtime, `block_on`, detached/hidden task;
- scanner/planner/materializer/delete/trash/repair redesign;
- schema, migration, workflow or public HTTP changes;
- hard delete, automatic destructive repair;
- unrelated cleanup;
- archiving control files.

Do not weaken or delete tests to obtain acceptance.

## CI policy

If source, test, Cargo or docs change, require a new code-bearing Component CI with fmt/check/test/clippy/finalizer green. A report-only commit may skip CI but is never code-bearing evidence.

## Report

Write `crates/haze-sync-worktree/control/report.md` using `report-template.md`.

Set:

- `REPORT_TYPE: CLEAN_CODE_REVIEW`
- `phase_id: WT-P10-CLEAN`
- `chat_name: worktree — W1 WT-P10 Clean-Code Review`

Use one honest status:

- `CLEAN_ACCEPT`
- `CLEAN_NEEDS_FIX`
- `CLEAN_BLOCKED_BY_CONTRACT`
- `CLEAN_BLOCKED_BY_SCOPE`
- `CLEAN_BLOCKED_BY_TOOLING`

The report must state the exact reviewed code-bearing SHA, complete reviewed range/files, findings and corrections, architecture-invariant assessment, test-quality assessment, final CI evidence when corrections were made, secrecy assessment, and whether WT-P10 is clean-accepted for downstream SHA synchronization.