# W1-SRV-P7-BLOCKED — Worktree runtime acceptance gate

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: none

This is a hold notice, not an executable worker prompt.

## Accepted state

SRV-P6 implementation, clean-code review, and CI correction are accepted with green CI.

Worktree WT-P8 is now implemented. Its host-driven runtime service, watcher-hint scheduling, mode handling, lifecycle, bounded work, and safe status contracts are present. However, the final WT-P8 source/docs head is formally red and has been routed to `FIX-WT-P8-CI` using artifact `8236750926`. WT-P8 has not yet completed fixer and clean-code acceptance.

## Blocked next phase

The next Server phase is SRV-P7: Worktree runtime composition fan-in.

Server must not mount or duplicate Worktree runtime behavior until:

- WT-P8 completes its fixer and clean-code lifecycle with green CI;
- Worktree config, lifecycle, cancellation, mode, status, and hosting boundaries are accepted;
- WT-P9 provides the explicit Server-hosted doctor/fan-in boundary, or a dedicated equivalent fan-in contract is approved.

## Unblock condition

WT-P8 clean-code/CI acceptance plus the required WT-P9 or dedicated Worktree/Server fan-in contract. Until then, do not launch a worker from this hold notice.
