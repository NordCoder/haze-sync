# W1-CLI-P6-BLOCKED — Bootstrap/operator endpoint dependency gate

Component: cli
Path: crates/haze-sync-cli
Branch: component/cli
PR: #48
Role: none

This is a hold notice, not an executable worker prompt.

## Accepted state

CLI-P5 implementation, CI fixer, and clean-code review are accepted. Final code/docs CI is green.

- code_bearing_sha: 3130df7c6ceeb735baf186d57cee931f973a4cd3
- workflow: Component CI
- workflow_run_id: 29084418994
- run_number: 1047
- conclusion: success

## Blocked next phase

The next plan phase is CLI-P6: bootstrap and sync operation commands.

CLI-P6 may only call accepted Server/API operator endpoints or an explicitly accepted local orchestration contract. The required bootstrap/import/export/sync-once endpoints and safe orchestration boundaries do not yet exist. Worktree and GDrive are also not at their runtime/fan-in completion phases.

## Unblock condition

Unblock after Server/API expose accepted bootstrap and sync-operation contracts, with required Worktree/GDrive runtime boundaries and safety gates available, or after a dedicated cross-component fan-in contract is approved.

Until then, do not run a worker for this component and do not add placeholder mutation commands.
