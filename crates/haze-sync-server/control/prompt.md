# W1-SRV-P7B5-FUNCTIONAL-REVIEW — Functional review of Server status/readiness contract

Before starting, name this worker chat exactly:

`server — W1 SRV-P7B5 Status Readiness Review`

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: clean-code-reviewer
Phase: SRV-P7B5-FUNCTIONAL-REVIEW

Do not merge, rewrite history, modify sibling branches, or begin API-P8/CLI-P6A/Deployment work.

## Candidate

- accepted SRV-P7B4 baseline: `55ed0d6c6ab9b78a953b954fcf5a9a68a6a708fe`;
- final SRV-P7B5 code-bearing SHA: `78f4e4525327ff03fa1af1e8387e9e3ea07091d6`;
- implementation report commit: `c70b789111aab2ea4eadc4bc9d7d06b21c6b6b4a`;
- implementation report blob: `1f5d044b9625fe42e87f97fc2cf37743f079ade9`;
- authoritative DB-capable Component CI: run `29283227887`, number `1901`, success.

Formatting, rustfmt, naming taste and stylistic matters are out of scope and must not block acceptance.

Verify only substantive correctness:

1. `ServerWorktreeStatusSnapshot` is passive, bounded, side-effect free and exposes only coarse secret-safe fields.
2. Disabled maps to Ready/DisabledInert and never makes Server unready by itself.
3. Starting, Cancelling, Shutdown and Failed map deterministically to NotReady with coarse reason codes.
4. Running maps to Ready, including while a cycle/manual request is Busy; counters/hints alone never fail readiness.
5. Manual availability mapping is correct without probe submission: Available, Busy, NotStarted, Cancelling, Shutdown, Unavailable and Failed.
6. DryRun is the only Running mode with manual availability; non-DryRun Running modes remain Unavailable unless an accepted contract explicitly says otherwise.
7. Snapshot transitions accurately mirror accepted host startup acknowledgement, task failure, shutdown timeout and successful shutdown.
8. No duplicate lifecycle/scheduler/manual accounting or executor access was introduced.
9. No new task, poller, runtime, retry, I/O or backend access exists in status reads.
10. No public route, DTO, OpenAPI/readiness payload, CLI output or deployment probe was added.
11. Tests cover readiness categories, Busy-with-ready semantics, passive reads, transitions, manual availability and secrecy.
12. Exact-SHA DB-capable CI is green and no later product/tooling commit invalidates the candidate.
13. No accepted Worktree/Storage, migrations, Core/API/CLI/Deployment, workflows or sibling scope changed.

Do not modify code unless there is a concrete functional, lifecycle, readiness, secrecy or scope defect. No formatting-only corrections.

Write `crates/haze-sync-server/control/report.md` with:

- `REPORT_TYPE: CLEAN_CODE_REVIEW`;
- `phase_id: SRV-P7B5-FUNCTIONAL-REVIEW`;
- `chat_name: server — W1 SRV-P7B5 Status Readiness Review`;
- status `CLEAN_ACCEPT`, `CLEAN_NEEDS_FIX`, `CLEAN_BLOCKED_BY_CONTRACT`, `CLEAN_BLOCKED_BY_SCOPE`, or `CLEAN_BLOCKED_BY_TOOLING`.

If no substantive blocker exists, use `CLEAN_ACCEPT` and state that Orchestrator may begin API-P8 passive status/manual HTTP contract work. Do not begin API-P8 yourself.
