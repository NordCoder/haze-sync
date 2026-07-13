# W1-SRV-P7B5-MANUAL-AVAILABILITY-FIX — Authoritative manual-state projection

Before starting, name this worker chat exactly:

`server — W1 SRV-P7B5 Manual Availability Fix`

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: fixer-worker
Phase: SRV-P7B5-MANUAL-AVAILABILITY-FIX

Do not merge, rewrite history, modify sibling branches, or begin API-P8/CLI-P6A/Deployment work. Do not perform stylistic cleanup.

## Reviewed candidate

- code-bearing SHA: `78f4e4525327ff03fa1af1e8387e9e3ea07091d6`;
- functional review report commit: `8082bc6fdd65053f66945fba5ed9047b369442e1`;
- report blob: `2f43f88adb950f59100aad80d860b268a54f4f73`;
- review status: `CLEAN_NEEDS_FIX`;
- authoritative DB-capable CI: run `29283227887`, number `1901`, success.

## Defect

The Server-owned manual availability atomic is toggled around every hosted-runtime poll. It can:

- report false Busy during idle DryRun polling;
- overwrite a newer accepted manual request with Available when an older poll completes;
- duplicate Worktree-owned manual gate/accounting instead of projecting authoritative state.

## Required correction

1. Remove unconditional Busy-before-poll and Available-after-poll transitions.
2. Preserve Worktree as sole owner of manual gating, no-overlap and request lifecycle.
3. Make Server manual availability a race-safe projection tied to accepted typed submission/request completion state.
4. The projection must not allow an older poll/request completion to clear a newer accepted request.
5. Idle DryRun polling with no manual request must remain Available.
6. Accepted manual request must remain Busy until that exact request reaches a terminal outcome or lifecycle shutdown/failure overrides it.
7. Shutdown, Cancelling and Failed categories must override stale availability state.
8. Do not probe availability by submitting requests.
9. Add no task, poller, runtime, retry, public route or DTO.

A Server-only generation/token protocol is allowed if it is a pure projection and cannot mutate or replace the accepted Worktree gate. If the accepted Worktree contract cannot provide enough completion identity/state for a correct projection, stop and report `FIX_BLOCKED_BY_CONTRACT` with the precise minimal owner extension required; do not create another approximate gate.

## Tests

Add deterministic tests for:

- idle DryRun ticker/poll remains Available without a manual request;
- accepted request maps Busy until its own completion;
- concurrent newer submission cannot be cleared by older completion;
- Busy remains Ready;
- lifecycle Cancelling/Shutdown/Failed overrides any stale manual projection;
- passive snapshot reads remain side-effect free;
- no accepted Worktree/Storage or public surface changes.

## Scope

Allowed:

- `crates/haze-sync-server/src/worktree_host.rs`;
- `crates/haze-sync-server/src/worktree_status.rs`;
- focused Server tests/internal test seams;
- Server control report.

Forbidden:

- accepted Worktree or Storage source changes unless reporting `FIX_BLOCKED_BY_CONTRACT` without product workaround;
- migrations/schema;
- Core/API/CLI/Deployment product files;
- public routes/DTO/OpenAPI/readiness payload;
- new tasks/pollers/runtimes/retries;
- workflows, sibling control files or unrelated cleanup.

Formatting/style alone is non-blocking when exact-SHA CI is green.

## Completion

Create a real code-bearing fix commit without CI skip. Obtain authoritative DB-capable Component CI on the exact final code-bearing SHA.

Write `crates/haze-sync-server/control/report.md` with:

- `REPORT_TYPE: FIX`;
- `phase_id: SRV-P7B5-MANUAL-AVAILABILITY-FIX`;
- `chat_name: server — W1 SRV-P7B5 Manual Availability Fix`;
- honest status `FIX_COMPLETE`, `FIX_NEEDS_MORE_WORK`, `FIX_BLOCKED_BY_CONTRACT`, `FIX_BLOCKED_BY_SCOPE`, or `FIX_BLOCKED_BY_TOOLING`.

Record the authoritative projection design, race-safety argument, tests, changed paths, final SHA and exact CI evidence.

Do not claim CLEAN_ACCEPT or begin API-P8. A final focused verification follows.
