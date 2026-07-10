# W1-SRV-P7-BLOCKED — Worktree runtime dependency gate

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: none

This is a hold notice, not an executable worker prompt.

## Accepted state

SRV-P6 implementation, clean-code review, and CI fixer are complete. Post-fix Component CI is green.

- code_bearing_sha: ef615905b996f9eaac249a663ec8c945b508a653
- workflow: Component CI
- workflow_run_id: 29079795947
- run_number: 890
- conclusion: success

Worktree WT-P7 implementation, clean-code review, and CI correction are accepted. Worktree is now queued for WT-P8, which owns the hostable watcher/runtime-service abstraction.

## Blocked next phase

The next Server plan phase is SRV-P7: Worktree runtime composition fan-in.

Do not start SRV-P7 while WT-P8 is only queued. Server must not duplicate Worktree-owned scan/watcher/runtime logic or invent lifecycle, mode, status, cancellation, or configuration contracts before they are implemented and reviewed.

WT-P9 owns the doctor/repair and explicit Server-hosted fan-in boundary unless a dedicated cross-component contract is approved earlier.

## Unblock condition

Orchestrator may replace this hold only after:

- WT-P8 runtime-service abstraction is implemented, clean-code reviewed, and green in CI;
- the required WT-P9 Server-hosted boundary is accepted, or a dedicated cross-component fan-in prompt defines the equivalent contract;
- Worktree config, lifecycle, cancellation, status, mode, and safe diagnostics boundaries needed by Server are concrete and readable.

Until then, do not modify Server source or report for SRV-P7.
