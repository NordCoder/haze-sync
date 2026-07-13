# W1-SRV-P7B4-LIFECYCLE-FIX — Fail-closed startup and host lifecycle coverage

Before starting, name this worker chat exactly:

`server — W1 SRV-P7B4 Lifecycle Fix`

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: fixer-worker
Phase: SRV-P7B4-LIFECYCLE-FIX

Do not merge, rewrite history, modify sibling branches, or begin SRV-P7B5/API-P8. Do not perform stylistic or formatting cleanup.

## Reviewed candidate

- code-bearing SHA: `5536d260bb4f95ec11c0cd07501c23903a72757d`;
- functional review report commit: `de06e35bb687448ea61fe6eacc0d1dd76ad04766`;
- report blob: `07e298a0622c6e5b1210160dc70b04c714a858e7`;
- review status: `CLEAN_NEEDS_FIX`;
- authoritative DB-capable CI: run `29278756276`, number `1888`, success.

## Required corrections

### 1. Fail-closed enabled startup

Add a bounded startup acknowledgement from the hosted task to `ServerWorktreeRuntimeHost::start`.

For enabled modes:

- do not return the host until `WorktreeHostedRuntime::start` and watcher startup have succeeded;
- publish a valid initial running status before acknowledging success;
- propagate startup failure through a coarse `ServerWorktreeHostError`;
- publish Failed status on every task error path, including runtime/watcher startup failure;
- join or abort-and-await the task before returning an enabled-start error;
- leave no retained or detached task after failed startup;
- preserve Disabled inert/task-free behavior.

The acknowledgement path must be bounded and cancellation-safe. Do not expose raw watcher, path, database, SQLx, notify or I/O errors.

### 2. Server-owned lifecycle tests

Add focused Tokio tests around the real Server host loop, using narrow fake Worktree clock/watcher/executor seams where necessary, for:

- enabled successful startup acknowledgement;
- watcher/runtime startup failure propagation and no leaked task;
- status becomes Failed rather than remaining Starting on task error;
- startup, periodic and watcher/debounce polling through the Server host;
- manual accepted/completed and Busy behavior through the host boundary;
- no overlapping cycles;
- DryRun request behavior remains non-mutating;
- cooperative cancellation and shutdown during pending work;
- bounded shutdown timeout aborts and awaits the retained task;
- mandatory join and final Shutdown/Failed status;
- coarse redacted status and errors.

Do not merely cite lower-level Worktree tests. The tests must exercise Server-owned orchestration, acknowledgement, shutdown and join behavior.

## Scope

Allowed:

- `crates/haze-sync-server/src/worktree_host.rs`;
- focused Server tests and minimal internal test seams;
- minimal Server exports/config wiring directly required by the fix;
- Server control report.

Forbidden:

- accepted Worktree or Storage source changes;
- migrations/schema;
- Core/API/CLI/Deployment product files;
- public routes, DTOs or readiness surface;
- multiple/detached tasks, nested runtime, `block_on`, internal HTTP;
- workflows, sibling control files or unrelated cleanup.

Formatting/style alone is non-blocking when exact-SHA CI is green and formatting-only edits are out of scope.

## Completion

Create a real code-bearing fix commit without CI skip. Obtain authoritative DB-capable Component CI on the exact final code-bearing SHA.

Write `crates/haze-sync-server/control/report.md` with:

- `REPORT_TYPE: FIX`;
- `phase_id: SRV-P7B4-LIFECYCLE-FIX`;
- `chat_name: server — W1 SRV-P7B4 Lifecycle Fix`;
- honest status `FIX_COMPLETE`, `FIX_NEEDS_MORE_WORK`, `FIX_BLOCKED_BY_CONTRACT`, `FIX_BLOCKED_BY_SCOPE`, or `FIX_BLOCKED_BY_TOOLING`.

Record startup acknowledgement design, failure cleanup/join semantics, status transitions, host-owned tests, changed paths, final SHA and exact CI evidence.

Do not claim CLEAN_ACCEPT or begin SRV-P7B5/API-P8. A final functional verification follows.
