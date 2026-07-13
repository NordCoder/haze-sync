# W1-SRV-P7B5-MANUAL-STATUS-VERIFY — Final functional verification

Before starting, name this worker chat exactly:

`server — W1 SRV-P7B5 Manual Status Verification`

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: clean-code-reviewer
Phase: SRV-P7B5-MANUAL-STATUS-VERIFY

Do not merge, rewrite history, modify sibling branches, or begin API-P8/CLI-P6A/Deployment work.

## Candidate

- accepted WT-P12 SHA: `526714cdfe185713a09af68fd5bddcb967a7902e`;
- final Server code-bearing SHA: `1d1fc8ca62c97db041cca09dd8316370285dfba1`;
- fan-in implementation report commit: `854084ab642c70ad0abc7ec878817d6410d59f22`;
- report blob: `2bb0637e819bc55530b0429cb728b09c8203cdbd`;
- authoritative DB-capable Component CI: run `29289080020`, number `1912`, success.

Formatting, rustfmt, naming taste and stylistic matters are out of scope and must not block acceptance.

Verify only substantive correctness:

1. accepted WT-P12 product blobs match exactly and no Worktree semantic edits/control files were introduced;
2. the Server Atomic/manual mirror and poll-based Busy/Available toggles are fully removed;
3. Server snapshot reads accepted `WorktreeRuntimeManualStatusHandle` directly and passively;
4. lifecycle mapping is deterministic: Created→NotStarted, Running DryRun idle→Available, Running DryRun busy→Busy, non-DryRun Running→Unavailable, Cancelling→Cancelling, Shutdown→Shutdown, host Failed→Failed;
5. Busy remains Ready because readiness derives from host lifecycle, not gate occupancy;
6. `busy` is interpreted as authoritative shared gate ownership, including automatic cycles, without false availability claims;
7. older ticket/completion/drop cannot clear newer accepted work;
8. lifecycle failure/shutdown overrides stale status safely;
9. status reads submit no probe, perform no I/O and add no task/poller/runtime/retry;
10. focused tests cover idle DryRun, pending Busy lifetime, old/new ticket safety, automatic-cycle semantics, Busy-is-Ready and lifecycle overrides;
11. no public route/DTO/OpenAPI/readiness payload, migration, workflow or downstream product scope changed;
12. exact-SHA DB-capable CI is green and no later product/tooling commit invalidates the candidate.

Do not modify code unless there is a concrete functional, concurrency, readiness, secrecy or scope defect. No formatting-only corrections.

Write `crates/haze-sync-server/control/report.md` with:

- `REPORT_TYPE: CLEAN_CODE_REVIEW`;
- `phase_id: SRV-P7B5-MANUAL-STATUS-VERIFY`;
- `chat_name: server — W1 SRV-P7B5 Manual Status Verification`;
- status `CLEAN_ACCEPT`, `CLEAN_NEEDS_FIX`, `CLEAN_BLOCKED_BY_CONTRACT`, `CLEAN_BLOCKED_BY_SCOPE`, or `CLEAN_BLOCKED_BY_TOOLING`.

If no substantive blocker exists, use `CLEAN_ACCEPT` and state that Orchestrator may begin API-P8 passive status/manual HTTP contract work immediately.
