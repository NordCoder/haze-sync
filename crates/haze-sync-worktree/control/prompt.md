# W1-WT-P12-FUNCTIONAL-REVIEW — Review authoritative manual status contract

Before starting, name this worker chat exactly:

`worktree — W1 WT-P12 Manual Status Review`

Component: worktree
Path: crates/haze-sync-worktree
Branch: component/worktree
PR: #49
Role: clean-code-reviewer
Phase: WT-P12-FUNCTIONAL-REVIEW

Do not merge, rewrite history, modify sibling branches, or begin Server fan-in.

## Candidate

- accepted WT-P11 baseline: `b38264ce2b09632a4c0bab0dd77319e1db239a3b`;
- final WT-P12 code-bearing SHA: `526714cdfe185713a09af68fd5bddcb967a7902e`;
- implementation report commit: `f7ad06d4ddec016c35555388634926bda5f963b0`;
- implementation report blob: `a561a094346abbdf58df88d1da51b28ba93aac69`;
- authoritative Component CI: run `29287214701`, number `1905`, success.

Formatting, rustfmt, naming taste and stylistic matters are out of scope and must not block acceptance.

Verify only substantive correctness:

1. `WorktreeRuntimeManualStatusHandle` or equivalent reads the same existing authoritative Gate used by submission/execution/completion/cancellation/shutdown.
2. No duplicate manual gate, mirror, generation approximation, request probe, task, poller, runtime, watcher or retry was added.
3. Status exposes only coarse typed lifecycle and busy fields and is passive, bounded, synchronous, side-effect free and secret-safe.
4. Created/Running/Cancelling/Shutdown lifecycle transitions match the accepted gate exactly.
5. Accepted submission makes busy true before returning Accepted; Busy rejection preserves true.
6. Busy remains true through queued/pending/executing manual work and clears on that request's normal completion.
7. Cancellation-by-drop sends typed Cancelled and clears busy through the existing RAII owner path.
8. Older ticket/completion/drop cannot clear a newer accepted request.
9. Automatic cycles using the shared no-overlap gate do not create false manual availability semantics for downstream readers; document the precise intended meaning of `busy` as the authoritative shared gate if applicable.
10. Dropping status handles or tickets alone does not mutate gate state.
11. Existing WT-P11 cancellation, scheduling, accounting and no-overlap behavior remains intact.
12. Exact-SHA CI is green and no later product/tooling commit invalidates the candidate.
13. No Server/Storage/Core/API/CLI/Deployment, migration, workflow or public HTTP/DTO scope changed.

Do not modify code unless there is a concrete functional, concurrency, lifecycle, secrecy or scope defect. No formatting-only corrections.

Write `crates/haze-sync-worktree/control/report.md` with:

- `REPORT_TYPE: CLEAN_CODE_REVIEW`;
- `phase_id: WT-P12-FUNCTIONAL-REVIEW`;
- `chat_name: worktree — W1 WT-P12 Manual Status Review`;
- status `CLEAN_ACCEPT`, `CLEAN_NEEDS_FIX`, `CLEAN_BLOCKED_BY_CONTRACT`, `CLEAN_BLOCKED_BY_SCOPE`, or `CLEAN_BLOCKED_BY_TOOLING`.

If no substantive blocker exists, use `CLEAN_ACCEPT` and authorize exact-SHA fan-in of the accepted WT-P12 product files to component/server. Do not modify Server yourself.
