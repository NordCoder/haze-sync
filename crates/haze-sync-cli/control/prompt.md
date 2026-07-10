# W1-CLI-P6-BLOCKED — Operator endpoint and runtime acceptance gate

Component: cli
Path: crates/haze-sync-cli
Branch: component/cli
PR: #48
Role: none

This is a hold notice, not an executable worker prompt.

## Accepted state

CLI-P5 implementation, CI correction, and clean-code review are accepted with green CI.

Worktree WT-P8 and GDrive GDA-P7C are now implemented, but both final heads are formally red and have been routed through artifact-based fixer slots. Their runtime and export boundaries are not yet clean-code/CI accepted for fan-in use.

## Blocked next phase

CLI-P6 owns bootstrap and sync operation commands. It may call only accepted Server/API operator endpoints or an explicitly accepted local orchestration contract.

The following remain unavailable:

- accepted bootstrap/import/export/sync-once operator endpoints;
- accepted Worktree runtime hosting boundary;
- accepted GDrive export/runtime and persistence fan-in;
- stable safe progress/result and failure contracts for command rendering.

CLI must not invent local orchestration, direct database access, provider calls, or hidden runtime ownership.

## Unblock condition

Green WT-P8 and GDA-P7C lifecycle acceptance plus accepted Server/API operator contracts, or a dedicated cross-component CLI fan-in contract. Until then, do not launch a worker from this hold notice.
