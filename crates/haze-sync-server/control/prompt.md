# W1-SRV-API-P8-HTTP-FINAL-REVIEW — Verify authoritative sync-once fix

Before starting, name this worker chat exactly:

`server — W1 API-P8 Worktree HTTP Final Review`

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: clean-code-reviewer
Phase: SRV-API-P8-HTTP-FINAL-REVIEW

Do not merge, change draft state, rewrite history, modify sibling branches, begin CLI-P6A, or perform unrelated cleanup.

## Candidate

- original HTTP fan-in SHA: `be2b1c16fa6c4919d446b76b1f15dca5767b2482`;
- previous review: `CLEAN_NEEDS_FIX`;
- previous review report blob: `04ba2d51a041b57c677a2a742d56719a181eaf67`;
- fixed final code-bearing SHA: `50461354c18ddc4d2e47202d9303b4358a27ee45`;
- fixer report blob: `a80e3c365d218d3228907c638cd23b70d6cc9b23`;
- authoritative DB-capable Component CI run: `29326558901`, number `1940`, attempt `1`, success.

Formatting, rustfmt, naming taste and stylistic matters are out of scope and must not block acceptance.

## Focused review

Verify the fixed POST sync-once control flow and confirm prior accepted areas remain intact:

1. Weak host upgrade occurs once and failed upgrade maps safely to Unavailable.
2. Exactly one passive snapshot is read.
3. Snapshot-only early return is limited to:
   - host Failed;
   - configured/manual Unavailable.
4. Snapshot categories Available, Busy, NotStarted, Cancelling and Shutdown all proceed to exactly one authoritative `submit_manual` call.
5. The bounded DryRun request uses the already validated stored action budget.
6. Public result comes only from the typed Worktree submission result for submit-capable states.
7. No retry, second submit, probe, polling, completion wait, task, watcher or runtime was introduced.
8. Stale Busy can become authoritative Accepted; stale lifecycle observations cannot bypass submit.
9. Authoritative Busy/NotStarted/Cancelling/Shutdown remain mapped correctly.
10. Accepted ticket is dropped without cancelling queued work and without waiting for completion.
11. Weak ownership and unique shutdown/join ownership remain unchanged.
12. API-P8 product blobs and public vocabulary remain unchanged.
13. GET status, Admin auth, strict body parsing, HTTP mappings, secrecy and absent-control behavior remain correct.
14. Focused deterministic tests prove stale-snapshot resolution and exactly-once submission.
15. No sibling product, migration or workflow changes occurred.
16. Exact final SHA has green DB-capable CI and later commits before review are control-only.

Do not modify code unless a concrete functional, concurrency, authorization, secrecy or scope defect remains. No formatting-only corrections.

## Report

Write `crates/haze-sync-server/control/report.md` with:

- `REPORT_TYPE: CLEAN_CODE_REVIEW`;
- `phase_id: SRV-API-P8-HTTP-FINAL-REVIEW`;
- `chat_name: server — W1 API-P8 Worktree HTTP Final Review`;
- status `CLEAN_ACCEPT`, `CLEAN_NEEDS_FIX`, `CLEAN_BLOCKED_BY_CONTRACT`, `CLEAN_BLOCKED_BY_SCOPE`, or `CLEAN_BLOCKED_BY_TOOLING`.

If no substantive blocker remains, use `CLEAN_ACCEPT` and authorize CLI-P6A control-slot resolution. Do not implement CLI work or claim merge readiness.
