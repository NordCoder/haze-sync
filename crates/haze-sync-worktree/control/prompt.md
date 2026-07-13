# W1-WT-P11-HOSTED-RUNTIME-CONTRACT — Add production watcher and scheduler-accounted manual cycles

Before starting, name this worker chat exactly:

`worktree — W1 WT-P11 Hosted Runtime Contract Extension`

Component: worktree
Path: crates/haze-sync-worktree
Branch: component/worktree
PR: #49
Role: implementation-worker
Phase: WT-P11-HOSTED-RUNTIME-CONTRACT

Work through the GitHub connector. Do not merge PR #49, change draft state, rewrite history, modify sibling branches, or implement Server hosting/API/CLI/Deployment behavior.

## Reactivation reason

Worktree is reactivated from accepted hold because Server SRV-P7B4 found a concrete owner-contract blocker.

Server blocker evidence:

- Server report commit: `03e15367e1363bc4718581ca2c8993be52c2231e`;
- Server report blob: `7172be029d2480eb151be2151f36718a985d089d`;
- status: `BLOCKED_BY_CONTRACT`;
- accepted Worktree baseline: `1942946331e8362f19907ab6ad4eb779da70fd57`.

The accepted architecture already assigns filesystem watcher and scheduler/no-overlap semantics to Worktree. No architect redesign is required.

## Goal

Extend the Worktree-owned runtime contract so a Server host can honestly construct and drive a production runtime without implementing Worktree filesystem-watcher semantics or bypassing Worktree scheduler accounting.

## Required deliverables

### 1. Production watcher boundary

Provide one Worktree-owned production path-free watcher implementation or factory/channel boundary with explicit lifecycle semantics.

Requirements:

- implements or constructs the accepted `WorktreeWatcher` contract;
- lifecycle covers start, non-blocking/bounded hint polling and shutdown;
- emitted hints contain sequence/count information only, never paths or file payloads;
- filesystem event details remain inside Worktree;
- overflow, backend closure and backend failure degrade safely to coarse typed outcomes and preserve authoritative full-scan correctness;
- no unmanaged background task, hidden runtime or detached thread owned by the library unless the lifecycle contract explicitly owns and joins it;
- Debug/Display/errors must not expose absolute roots or backend internals;
- platform/backend dependency changes must be minimal and justified.

A channel/factory design is acceptable only if it is production-complete: it must include the Worktree-owned filesystem producer lifecycle, not merely a consumer with no producer.

### 2. Scheduler-accounted manual cycle

Add a `WorktreeRuntimeService` manual-cycle entrypoint that accepts an explicit `WorktreeRuntimeCycleRequest` or a narrower dedicated request.

It must:

- preserve at-most-one-cycle semantics;
- reject or coarsely report busy, cancelling, shutdown and not-started states;
- validate mode, mutation permissions, full-scan requirement and budgets fail-closed;
- execute through the same accepted executor and cancellation token;
- update cycle counters, last-cycle cause/result, lifecycle and status consistently with automatic cycles;
- support manual DryRun planning without mutation, checkpoint or automatic scheduling;
- not let Server call `executor_mut().run_cycle(...)` to bypass runtime accounting;
- return typed coarse outcomes safe for an internal host consumer;
- avoid introducing public API/DTO semantics.

### 3. Tests

Add focused tests for:

- production watcher start/poll/shutdown lifecycle;
- path-free hints and redaction;
- event coalescing/overflow/fallback-to-full-scan behavior;
- backend close/failure handling;
- manual cycle success and status accounting;
- manual busy rejection/no overlap;
- manual DryRun full-scan planning and non-mutation;
- cancellation, not-started and shutdown states;
- exact preservation of automatic startup/periodic/watcher behavior;
- no detached execution or lifecycle leak.

## Scope

Allowed:

- Worktree runtime/watcher modules and focused tests;
- Worktree `lib.rs` exports;
- minimal Worktree manifest/lock changes required by the production watcher backend;
- Worktree docs/control report.

Forbidden:

- Server, Storage, Core, API, CLI or Deployment product files;
- migrations/schema;
- HTTP, public DTOs, Server task hosting or startup wiring;
- provider behavior, hard delete or automatic destructive repair;
- sibling control files or workflows;
- redesign of accepted filesystem/scanner/planner/materializer ownership beyond what this contract extension requires.

## Compatibility

Preserve the accepted WT-P10 contract and behavior:

- awaitable boxed `Send` cycle future;
- async poll executes at most one cycle;
- cooperative cancellation;
- bounded budgets and count-only summaries;
- full-scan correctness and watcher hints as latency hints only;
- mutation-free non-automatic DryRun;
- safe status and Debug output.

If the production watcher backend requires an architectural choice that cannot be bounded within Worktree ownership, report `BLOCKED_BY_CONTRACT` with exact alternatives and evidence rather than inventing Server behavior.

## CI and completion

Create a real code-bearing commit without CI skip. Require normal authoritative Component CI on the exact final code-bearing SHA.

Write `crates/haze-sync-worktree/control/report.md` using `report-template.md` with:

- `REPORT_TYPE: IMPLEMENTATION`;
- `phase_id: WT-P11-HOSTED-RUNTIME-CONTRACT`;
- `chat_name: worktree — W1 WT-P11 Hosted Runtime Contract Extension`.

Use one honest status: `SELF_ACCEPT`, `NEEDS_FIX`, `BLOCKED_BY_CONTRACT`, `BLOCKED_BY_SCOPE`, or `BLOCKED_BY_TOOLING`.

The report must include changed paths, watcher/backend lifecycle, manual-cycle semantics, compatibility assessment, tests, secrecy/redaction, exact final code-bearing SHA and exact CI evidence.

Do not claim `CLEAN_ACCEPT` or reactivate Server. Mandatory Worktree clean-code review and explicit exact-SHA fan-in are required afterward.
