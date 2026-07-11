# W1-CLI-P6-BLOCKED-BY-RUNTIME-FAN-IN — Operator/runtime acceptance gate

Component: cli
Path: crates/haze-sync-cli
Branch: component/cli
PR: #48
Role: none

This is a hold notice, not an executable worker prompt.

## Accepted state

CLI-P5 implementation, CI correction, and clean-code review are accepted with green CI.

The following dependencies are now accepted:

- GDrive Adapter component-local lifecycle through GDA-P8C and its CI correction;
- Storage component-local lifecycle and Server-owned production feature isolation;
- Worktree WT-P9 implementation and post-fix code-bearing CI.

WT-P9C clean-code review remains active. Server/API do not yet expose accepted bootstrap/import/export/sync-once operator contracts or concrete Worktree/GDrive runtime composition.

## Blocked next phase

CLI-P6 owns bootstrap and sync operation commands. It may call only accepted Server/API operator endpoints or an explicitly accepted orchestration contract.

The remaining blockers are:

- WT-P9C clean acceptance and final Worktree hosting boundary;
- accepted Server composition of Worktree and GDrive runtime operations;
- accepted API/Server bootstrap, import/export, sync-once, progress, result, and safe failure contracts;
- an explicit ownership decision for any local orchestration that is not Server-hosted.

CLI must not invent direct database access, provider calls, hidden daemon ownership, or destructive repair execution.

## Unblock condition

WT-P9C `CLEAN_ACCEPT` with green final code-bearing CI plus an accepted Server/API operator and runtime fan-in contract, or a dedicated explicit CLI fan-in contract. Until then, do not launch a worker from this hold notice.
