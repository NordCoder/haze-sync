# W1-FIX-SRV-API-P8-SUBMISSION-RACE — Use authoritative Worktree submit result

Before starting, name this worker chat exactly:

`server — W1 API-P8 Sync-Once Race Fix`

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: fixer-worker
Phase: FIX-SRV-API-P8-SUBMISSION-RACE

Do not merge, change draft state, rewrite history, modify sibling branches, begin CLI-P6A, or perform unrelated cleanup.

## Candidate and finding

- current code-bearing SHA: `be2b1c16fa6c4919d446b76b1f15dca5767b2482`;
- implementation status: `SELF_ACCEPT`;
- functional review status: `CLEAN_NEEDS_FIX`;
- review report blob: `04ba2d51a041b57c677a2a742d56719a181eaf67`;
- authoritative DB-capable CI run `29321038276`, number `1938`, success.

Blocking defect:

`ServerWorktreeHttpControl::submit_sync_once` reads a preliminary snapshot and returns `Busy`, `NotStarted`, `Cancelling`, or `Shutdown` without calling the authoritative Worktree manual submit API. Those observations may become stale before response and can bypass a valid typed submission result.

## Required fix

1. Preserve snapshot precheck only for Server-only conditions that the Worktree typed submit boundary cannot express:
   - host `Failed`;
   - configured/manual mode `Unavailable`.
2. For snapshot categories `Available`, `Busy`, `NotStarted`, `Cancelling`, and `Shutdown`, construct exactly one bounded `WorktreeRuntimeManualRequest::dry_run(...)` from the validated stored budget and call `submit_manual` exactly once.
3. Map the authoritative typed result only:
   - Accepted -> Accepted;
   - Busy -> Busy;
   - NotStarted -> NotStarted;
   - Cancelling -> Cancelling;
   - Shutdown -> Shutdown.
4. Do not retry, poll, wait for completion, submit a probe, or add any task/runtime/watcher.
5. Preserve weak host ownership, unique shutdown/join ownership, ticket-drop semantics and all accepted HTTP/auth/body mappings.
6. Do not change API-P8 source blobs or public vocabulary.
7. Add deterministic focused tests proving:
   - a stale Busy snapshot cannot bypass a later authoritative Accepted result;
   - a stale NotStarted/Cancelling/Shutdown observation cannot be returned without typed submit evaluation;
   - authoritative Busy/Cancelling/Shutdown results still map correctly;
   - submit is invoked exactly once;
   - Failed and manual/mode Unavailable remain legitimate snapshot-only Server outcomes;
   - no completion wait/poll/retry is introduced.

Use the smallest Server-only change, expected primarily in `crates/haze-sync-server/src/worktree_http.rs` and its focused tests.

Forbidden:

- Worktree runtime/gate/watcher/executor changes;
- API semantic edits;
- Storage/Core/CLI/Deployment product changes;
- migrations/workflows;
- broad app-state or route refactor;
- formatting-only cleanup beyond touched lines.

Formatting/style alone is non-blocking when exact-SHA CI is green.

## Completion

Create a real code-bearing commit without CI skip and obtain authoritative DB-capable Component CI on the exact final SHA.

Write `crates/haze-sync-server/control/report.md` with:

- `REPORT_TYPE: FIX`;
- `phase_id: FIX-SRV-API-P8-SUBMISSION-RACE`;
- `chat_name: server — W1 API-P8 Sync-Once Race Fix`;
- status `FIX_COMPLETE`, `FIX_NEEDS_MORE_WORK`, `FIX_BLOCKED_BY_CONTRACT`, `FIX_BLOCKED_BY_SCOPE`, or `FIX_BLOCKED_BY_TOOLING`.

Record changed paths, exact authoritative-submit flow, focused race tests, final SHA and exact DB-capable CI evidence. Do not claim CLEAN_ACCEPT or begin CLI-P6A; a final focused review follows.
