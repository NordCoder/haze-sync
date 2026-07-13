# W1-SRV-WT-P12-FAN-IN — Exact-SHA Worktree manual-status fan-in

Before starting, name this worker chat exactly:

`server — W1 WT-P12 Manual Status Fan-In`

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: implementation-worker
Phase: SRV-WT-P12-MANUAL-STATUS-FAN-IN

Do not merge, change draft state, rewrite history, modify sibling branches, or begin API-P8.

## Accepted source

- Worktree accepted code-bearing SHA: `526714cdfe185713a09af68fd5bddcb967a7902e`;
- Worktree clean-review commit: `c0f636964cdfc4a6686e09736c73431fdc964d51`;
- clean report blob: `7c0d3e739642d01c363316b72a9075f6f876044a`;
- Component CI run `29287214701`, number `1905`, success.

## Server baseline

- current SRV-P7B5 candidate SHA: `78f4e4525327ff03fa1af1e8387e9e3ea07091d6`;
- blocker: Server approximate manual availability mirror is racy and duplicates Worktree gate ownership;
- blocker report commit: `d612196a33c56711e9102f09eaf1340ed7b696e0`.

## Task

Perform an exact-SHA fan-in of the accepted WT-P12 product changes required for Server integration:

- `crates/haze-sync-worktree/src/hosted_runtime.rs`;
- `crates/haze-sync-worktree/src/lib.rs`;
- focused WT-P12 tests only if needed for workspace verification.

Requirements:

1. Product blobs must match accepted Worktree SHA exactly; no semantic edits.
2. Do not copy Worktree control files.
3. Preserve all existing Server product changes.
4. After fan-in, replace the Server-owned approximate manual availability Atomic/mirror with direct mapping from the accepted Worktree manual status handle.
5. Remove poll-based Busy/Available toggles.
6. Map authoritative lifecycle/busy into existing Server categories:
   - Created => NotStarted;
   - Running + busy false => Available only for DryRun, otherwise Unavailable;
   - Running + busy true => Busy for DryRun, otherwise Unavailable;
   - Cancelling => Cancelling;
   - Shutdown => Shutdown;
   - host Failed => Failed.
7. Busy must remain Ready at the readiness layer.
8. Status reads remain passive, bounded and side-effect free.
9. Add deterministic Server tests for idle DryRun availability, authoritative Busy lifetime, automatic-cycle Busy semantics, stale-completion safety, lifecycle overrides and no probe submission.
10. No public route/DTO/OpenAPI/readiness payload, new task/poller/runtime, migration, workflow or sibling component changes.

Create a real code-bearing commit without CI skip and obtain authoritative DB-capable Component CI on the exact final Server SHA.

Write `crates/haze-sync-server/control/report.md` with:

- `REPORT_TYPE: IMPLEMENTATION`;
- `phase_id: SRV-WT-P12-MANUAL-STATUS-FAN-IN`;
- `chat_name: server — W1 WT-P12 Manual Status Fan-In`;
- honest status `SELF_ACCEPT`, `NEEDS_FIX`, `BLOCKED_BY_CONTRACT`, `BLOCKED_BY_SCOPE`, or `BLOCKED_BY_TOOLING`.

Record exact blob matching, removed mirror logic, authoritative mapping, tests, final SHA and exact CI evidence. Do not claim CLEAN_ACCEPT or begin API-P8.
