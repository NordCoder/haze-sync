# W1-CLI-P6-BLOCKED — Operator endpoint and runtime acceptance gate

Component: cli
Path: crates/haze-sync-cli
Branch: component/cli
PR: #48
Role: none

This is a hold notice, not an executable worker prompt.

## Accepted state

CLI-P5 implementation, CI correction, and clean-code review are accepted with green CI.

GDrive GDA-P7C is now fully accepted with green CI. Worktree WT-P8 implementation and fixer are green, but WT-P8C clean-code review is only queued. GDA-P8 delete-guard implementation is also queued and does not supply operator endpoints.

## Blocked next phase

CLI-P6 owns bootstrap and sync operation commands. It may call only accepted Server/API operator endpoints or an explicitly accepted local orchestration contract.

The following remain unavailable:

- accepted bootstrap/import/export/sync-once operator endpoints;
- final accepted Worktree runtime hosting boundary;
- accepted Server composition of Worktree/GDrive runtime operations;
- stable safe progress/result and failure contracts for command rendering.

CLI must not invent local orchestration, direct database access, provider calls, or hidden runtime ownership.

## Unblock condition

Green WT-P8C acceptance plus accepted Server/API bootstrap and sync contracts and required runtime fan-in, or a dedicated CLI fan-in contract. Until then, do not launch a worker from this hold notice.
