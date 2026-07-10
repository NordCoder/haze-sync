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

## Blocked next phase

The next Server plan phase is SRV-P7: Worktree runtime composition fan-in.

Do not start SRV-P7 yet. The Worktree implementation plan places the hostable runtime-service abstraction in WT-P8 and the explicit Server-hosted fan-in/doctor boundary in WT-P9. Worktree is currently only entering WT-P5 clean-code review.

Starting SRV-P7 now would either duplicate Worktree-owned runtime logic inside Server or invent an unaccepted cross-component contract.

## Unblock condition

Orchestrator may replace this hold only after:

- WT-P8 runtime-service abstraction is implemented and clean-code/CI accepted;
- any required WT-P9 Server-hosted boundary is explicitly available or a dedicated cross-component fan-in prompt is approved;
- Worktree config, lifecycle, status, and mode contracts needed by Server are concrete and readable.

Until then, do not modify Server source or report for SRV-P7.
