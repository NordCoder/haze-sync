# W1-SRV-P7A-FIX — Activate the Server-owned Worktree composition boundary

Before starting, name this worker chat exactly:

`server — W1 SRV-P7A Wiring Fix`

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: implementation-worker

Work only through the GitHub connector. Do not merge the PR, change its draft state, rebase, reset, rewrite history, force-push, modify `main`, modify `component/worktree`, or change sibling components.

## Why this correction is required

The accepted Worktree snapshot fan-in is complete and exact, but the Server-owned SRV-P7A composition file is not part of the active crate:

- accepted Worktree source SHA: `4f7bc748d9b901d7d5c3e43c845ba407c0c36e59`
- synchronized Worktree files: `32/32` exact blob identity
- SRV-P7A product/source/docs head: `71fd46ceb8b50f2523cacd70165dcca63881aa82`
- Component CI run: `29158883879`
- run number: `1701`
- conclusion: `success`

That green run is insufficient for acceptance because `crates/haze-sync-server/src/worktree_runtime.rs` is not declared by `main.rs`. Its implementation and six tests were therefore not compiled or executed. Production startup also does not construct the boundary from `ServerConfig.worktree`, call `start`, retain it for the server lifetime, or call `shutdown`.

The authoritative recovery report is archived at:

`crates/haze-sync-server/control/log/20260711-163000Z-W1-SRV-P7A-REPORT-RECOVERY-implementation-worker-report.md`

It concluded `SELF_NEEDS_FIX`. This prompt corrects only that verified Server wiring defect.

## Required reads

Before editing, read:

- project implementation manifest, report template, implementation-worker prompt, and GitHub connector guidance;
- current Server control state/prompt;
- the archived SRV-P7A implementation prompt;
- the archived SRV-P7A report-recovery prompt and report;
- Server contract, SRV-P7 plan section, dependency map, decisions, implementation log, Cargo manifest, `src/main.rs`, config types, state, readiness, routes, and `src/worktree_runtime.rs`;
- accepted Worktree runtime contract/docs and public runtime types already synchronized into this branch.

## Task

Complete the already-scoped Server-owned composition boundary without implementing SRV-P7B execution.

Required behavior:

1. Include `worktree_runtime.rs` in the active Server crate module graph so its implementation and tests compile.
2. Construct `ServerWorktreeRuntime` from the existing `ServerConfig.worktree.mode` and `ServerConfig.worktree.root` before `ServerConfig` is moved into `ServerAppState`.
3. Call `start` exactly once through the production startup path.
4. Retain the composition boundary for the full HTTP server lifetime.
5. Call `shutdown` exactly once after the serve future completes, including the serve-error path; do not skip lifecycle closure merely because serving failed.
6. Preserve disabled mode as completely inert.
7. Preserve enabled modes as honestly `Unavailable` with `CycleExecutorNotWired`; do not create a fake watcher, executor, polling loop, scan, import, export, mutation, or background task.
8. Preserve `DryRun` as unsupported rather than silently remapping it.
9. Map any lifecycle/startup error to a stable secret-safe `StartupError` without exposing the configured root, database URL, tokens, raw filesystem errors, or internal debug payloads.
10. Keep dependency-free router/state behavior and all existing routes unchanged.
11. Ensure the focused mode/lifecycle/inertness/redaction tests in `worktree_runtime.rs` are part of the compiled test target and actually run in Component CI.
12. Correct the SRV-P7A implementation-log entry only if needed so it precisely matches active behavior after this fix.

Prefer a small explicit composition helper if it makes start/serve/shutdown ordering testable without opening a real listener or requiring a live database. Do not broaden this into a runtime framework.

## Acceptance checks

The implementation is acceptable only if:

- `worktree_runtime.rs` is in the crate graph;
- its focused tests compile and execute;
- production startup constructs the boundary from existing config;
- start and shutdown ownership is explicit and deterministic;
- shutdown is attempted after both successful and failed serving completion where testable;
- disabled behavior remains inert;
- enabled behavior remains honestly unavailable rather than fake-operational;
- no accepted Worktree product file changes;
- no route/API/storage/provider/deployment behavior changes;
- a new code-bearing Component CI run is observed green.

## Allowed files

- `crates/haze-sync-server/src/main.rs`
- `crates/haze-sync-server/src/worktree_runtime.rs`
- `crates/haze-sync-server/docs/implementation-log.md` only for factual correction/update
- `crates/haze-sync-server/control/report.md`

If a compile-proven correction requires another Server-local source file, stop and report the exact need as `BLOCKED_BY_CONTRACT` rather than silently expanding scope.

## Forbidden scope

- no changes under `crates/haze-sync-worktree/**`;
- no Cargo dependency or feature changes;
- no Core/API/Storage/Common/GDrive/Obsidian/CLI/Deployment changes;
- no new HTTP routes or public DTOs;
- no real Worktree cycle executor, watcher implementation, import/export loop, repair executor, or persistence wiring;
- no hidden globals or unbounded/background tasks;
- no provider calls;
- no hard delete or automatic destructive repair;
- no workflow changes;
- no test deletion or assertion weakening;
- no unrelated cleanup.

## CI and report

Source/test/docs commits must run CI normally. CI skip is permitted only for the final report-only commit.

Do not self-accept based on the old run `29158883879`. A new code-bearing Component CI run must compile and test the activated module and conclude successfully.

Write only `crates/haze-sync-server/control/report.md` using `report-template.md`.

Set:

- `REPORT_TYPE: IMPLEMENTATION`
- `phase_id: SRV-P7A-FIX`
- `chat_name: server — W1 SRV-P7A Wiring Fix`

The report must include exact changed files, lifecycle ordering, tests proving module activation, final code-bearing SHA, authoritative CI run metadata, preserved non-goals, and an honest status. Recommend clean-code review only if the corrected implementation is `SELF_ACCEPT` with green CI.
