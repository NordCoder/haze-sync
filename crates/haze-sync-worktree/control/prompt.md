# W1-WT-P12-MANUAL-STATUS-CONTRACT — Add authoritative read-only manual status

Before starting, name this worker chat exactly:

`worktree — W1 WT-P12 Manual Status Contract`

Component: worktree
Path: crates/haze-sync-worktree
Branch: component/worktree
PR: #49
Role: implementation-worker
Phase: WT-P12-MANUAL-STATUS-CONTRACT

Do not merge, rewrite history, modify sibling branches, or change Server/API product files.

## Accepted baseline

- accepted WT-P11 code-bearing SHA: `b38264ce2b09632a4c0bab0dd77319e1db239a3b`;
- WT-P11 final clean report commit: `b76f88369d079a9cb264bb4cdb9b6c62e361a9bc`;
- clean report blob: `84fb7a399d708088c3b545efd0e2adbaad3f909e`;
- authoritative Component CI: run `29272964159`, number `1881`, success.

## Downstream blocker

Server SRV-P7B5 cannot safely expose manual availability because the accepted Worktree contract exposes neither:

- a passive authoritative lifecycle/busy view of the existing manual gate; nor
- stable request identity carried through Accepted submission and Manual completion.

Server-only mirrors produce false Busy and stale-completion races. Worktree remains the sole owner of manual gating/accounting.

## Goal

Add the smallest authoritative read-only status contract over the existing Worktree manual gate.

Preferred design:

- a cloneable `WorktreeRuntimeManualStatusHandle`, or equivalent passive status method;
- shares the existing accepted gate/state rather than duplicating it;
- exposes only coarse typed fields:
  - lifecycle: Created | Running | Cancelling | Shutdown;
  - busy: bool;
- reads are synchronous, lock-free or bounded/non-blocking, side-effect free;
- no request submission, polling, I/O, path, payload, token, cursor, backend detail or request identity exposure;
- no new task, runtime, poller, watcher or retry.

The status must be authoritative across:

- accepted submission;
- Busy rejection;
- dequeue/execution;
- normal completion;
- cancellation-by-drop;
- start/cancelling/shutdown transitions;
- dropped handles/tickets.

Do not weaken current typed `Accepted`, `Busy`, `NotStarted`, `Cancelling`, `Shutdown`, `Cancelled` behavior.

An opaque generation alternative is allowed only if materially smaller and assigned/cleared by the existing gate owner, with the same generation carried through Accepted and Manual completion. Prefer the read-only status handle.

## Tests

Add deterministic tests for:

- Created/not-started lifecycle and not busy;
- Running idle => not busy;
- accepted request => busy immediately;
- Busy rejection preserves busy;
- busy remains true while pending/executing;
- normal completion clears busy;
- cancellation-by-drop clears busy and ticket receives typed Cancelled;
- newer accepted request cannot be cleared by older completion;
- Cancelling and Shutdown lifecycle are authoritative;
- passive reads trigger no work and expose no sensitive details;
- existing WT-P11 scheduling/no-overlap/accounting tests remain green.

## Scope

Allowed:

- `crates/haze-sync-worktree/src/hosted_runtime.rs`;
- minimal exports in `src/lib.rs`;
- focused Worktree tests;
- Worktree control report.

Forbidden:

- Server, Storage, Core, API, CLI or Deployment product changes;
- migrations/schema/workflows;
- new task/poller/runtime/retry;
- public HTTP/DTO surfaces;
- payload/path/backend detail exposure;
- unrelated cleanup.

Formatting/style alone is non-blocking when exact-SHA CI is green.

## Completion

Create a real code-bearing commit without CI skip. Obtain authoritative Component CI on the exact final code-bearing SHA.

Write `crates/haze-sync-worktree/control/report.md` with:

- `REPORT_TYPE: IMPLEMENTATION`;
- `phase_id: WT-P12-MANUAL-STATUS-CONTRACT`;
- `chat_name: worktree — W1 WT-P12 Manual Status Contract`;
- honest status `SELF_ACCEPT`, `NEEDS_FIX`, `BLOCKED_BY_CONTRACT`, `BLOCKED_BY_SCOPE`, or `BLOCKED_BY_TOOLING`.

Record the shared authoritative state design, lifecycle/busy transition evidence, tests, changed paths, final SHA and exact CI evidence.

Do not claim CLEAN_ACCEPT or modify Server. A focused functional review and exact-SHA Server fan-in follow.
