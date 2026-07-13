# W1-SRV-P7B4-LIFECYCLE-VERIFY — Final functional verification

Before starting, name this worker chat exactly:

`server — W1 SRV-P7B4 Lifecycle Verification`

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: clean-code-reviewer
Phase: SRV-P7B4-LIFECYCLE-VERIFY

Do not merge, rewrite history, modify sibling branches, or begin SRV-P7B5/API-P8.

## Candidate

- previous reviewed SHA: `5536d260bb4f95ec11c0cd07501c23903a72757d`;
- final lifecycle-fix SHA: `55ed0d6c6ab9b78a953b954fcf5a9a68a6a708fe`;
- FIX report commit: `a0dc770a14e2a9d1a9a0aab2c30aef8e0c4e5ea5`;
- FIX report blob: `7963d704e7dfba89ef8d7e26178e772d7f3a947a`;
- authoritative DB-capable Component CI: run `29281569666`, number `1893`, success.

Formatting, rustfmt, naming taste and stylistic matters are out of scope and must not block acceptance.

Verify only:

1. enabled start waits for bounded acknowledgement after accepted Worktree runtime/watcher startup and Running status publication;
2. watcher/runtime startup failure returns a coarse error before Server can treat the host as started;
3. every failed-start path joins or abort-and-awaits the retained task and cannot detach/leak it;
4. task errors publish Failed rather than leaving Starting stale;
5. Disabled remains inert, task-free and acknowledgement-free;
6. real Server host-loop Tokio tests cover successful startup, failure propagation, startup/periodic/watcher/manual/Busy/no-overlap/DryRun, pending shutdown cancellation, timeout abort-and-await, mandatory join and final status;
7. one joined task and accepted Worktree scheduling/manual/accounting boundaries remain intact;
8. errors/status remain coarse and secret-safe;
9. no accepted Worktree/Storage, migration, downstream product, public DTO/readiness or workflow scope changed;
10. exact-SHA DB-capable CI is green and no later product/tooling commit invalidates the candidate.

Do not modify code unless a concrete functional, lifecycle, concurrency, safety or scope defect exists. No formatting-only corrections.

Write `crates/haze-sync-server/control/report.md` with:

- `REPORT_TYPE: CLEAN_CODE_REVIEW`;
- `phase_id: SRV-P7B4-LIFECYCLE-VERIFY`;
- `chat_name: server — W1 SRV-P7B4 Lifecycle Verification`;
- status `CLEAN_ACCEPT`, `CLEAN_NEEDS_FIX`, `CLEAN_BLOCKED_BY_CONTRACT`, `CLEAN_BLOCKED_BY_SCOPE`, or `CLEAN_BLOCKED_BY_TOOLING`.

If the listed invariants hold, use `CLEAN_ACCEPT` and state that Orchestrator may begin SRV-P7B5 status/readiness work.
