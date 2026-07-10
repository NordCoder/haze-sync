# W1-SRV-P7-BLOCKED — Worktree runtime acceptance gate

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: none

This is a hold notice, not an executable worker prompt.

## Accepted state

SRV-P6 implementation, clean-code review, and CI correction are accepted with green CI.

Worktree WT-P8 runtime implementation and its artifact-based CI correction are now green. WT-P8C clean-code review is queued, so the runtime lifecycle, cancellation, mode, status, and hosting surface are not yet finally accepted for Server composition.

## Blocked next phase

The next Server phase is SRV-P7: Worktree runtime composition fan-in.

Server must not mount or duplicate Worktree runtime behavior until:

- WT-P8C is clean-code accepted with green CI;
- Worktree config, lifecycle, cancellation, mode, status, and hosting contracts are final;
- WT-P9 provides the explicit Server-hosted doctor/fan-in boundary, or a dedicated equivalent contract is approved.

## Unblock condition

WT-P8C acceptance plus the required WT-P9 or dedicated Worktree/Server fan-in contract. Until then, do not launch a worker from this hold notice.
