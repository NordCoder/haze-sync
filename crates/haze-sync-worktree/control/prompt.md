# W1-WT-P11-CANCELLATION-FIX — Make hosted runtime cancellation-by-drop safe

Before starting, name this worker chat exactly:

`worktree — W1 WT-P11 Cancellation Safety Fix`

Component: worktree
Path: crates/haze-sync-worktree
Branch: component/worktree
PR: #49
Role: fixer-worker
Phase: WT-P11-CANCELLATION-FIX

Do not merge, rewrite history, modify sibling branches, begin Server fan-in, or perform stylistic/formatting cleanup.

## Sole blocker

Reviewed SHA: `9cce5f5a34597f13a506fadad49a7ab46972fa98`.
Functional review report commit: `7e761ccf1b3ba0036d10067a9d6e4bf395db4bf1`.
Report blob: `db35ddba7c77518f6c04216a52907634b451eeef`.

`WorktreeHostedRuntime::poll()` is not cancellation-by-drop safe:

- busy state is cleared only after awaited execution returns;
- dropping a pending poll future can leave the shared busy gate permanently set;
- a dequeued manual request can lose its response sender and produce `Closed` instead of typed cancellation.

## Required fix

1. Add an RAII in-flight guard whose `Drop` always releases the shared busy gate.
2. For dequeued manual work, cancellation-by-drop must complete the ticket with the existing coarse typed cancelled outcome.
3. Do not commit completed-cycle accounting when execution was dropped before completion.
4. Add deterministic pending-future tests for:
   - accepted manual request;
   - hosted poll reaches Pending;
   - poll future is dropped;
   - ticket receives typed cancellation, not Pending/Closed;
   - subsequent request is accepted, not permanently Busy;
   - counters and last-cycle state remain consistent;
   - equivalent dropped automatic-poll path releases the gate and allows later work.
5. Preserve all already accepted WT-P11 behavior and scope.

Formatting/style/naming-only changes are out of scope. Exact-SHA green CI is authoritative.

Write `crates/haze-sync-worktree/control/report.md` with:

- `REPORT_TYPE: FIX`;
- `phase_id: WT-P11-CANCELLATION-FIX`;
- `chat_name: worktree — W1 WT-P11 Cancellation Safety Fix`;
- honest status `FIX_COMPLETE`, `FIX_NEEDS_MORE_WORK`, `FIX_BLOCKED_BY_CONTRACT`, `FIX_BLOCKED_BY_SCOPE`, or `FIX_BLOCKED_BY_TOOLING`.

Record changed paths, RAII/ticket semantics, tests, final SHA and exact CI evidence. Do not claim CLEAN_ACCEPT or begin Server fan-in.
