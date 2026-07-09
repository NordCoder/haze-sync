# W1-WT-P4 — Import planner and Core/API submission boundary

Component: worktree
Path: crates/haze-sync-worktree
Branch: component/worktree
PR: #49
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

WT-P3 implementation, CI fixer, and clean-code review are accepted. Component CI evidence for the accepted source state is green.

- workflow: Component CI
- workflow_run_id: 29011302368
- run_number: 533
- conclusion: success

The next implementation phase is WT-P4 from crates/haze-sync-worktree/docs/implementation-plan.md.

## Read

Read before editing:

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- implementation-worker-prompt.md from ChatGPT Project Sources
- chatgpt-gh-connector.md from ChatGPT Project Sources
- crates/haze-sync-worktree/docs/component-contract.md
- crates/haze-sync-worktree/docs/implementation-plan.md
- crates/haze-sync-worktree/docs/implementation-log.md
- crates/haze-sync-worktree/docs/dependency-map.md
- crates/haze-sync-worktree/control/prompt.md
- crates/haze-sync-worktree/control/report.md
- relevant current repository code and PR diff

Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Implement WT-P4: Import planner and Core/API submission boundary.

Follow the implementation plan:

- represent worktree state needed to identify local changes against last applied Core revision;
- produce import plans for new, modified, and deleted local files;
- attach current known base revision or explicit null base;
- include content hash and bytes only for stable file imports;
- submit through an abstract API/Core client trait, not direct DB writes;
- handle outcomes such as accepted, same-content, conflict-saved, rejected, tombstoned, and not-found;
- update local state only after accepted outcomes.

## Allowed files

- crates/haze-sync-worktree/src/**
- crates/haze-sync-worktree/docs/**
- crates/haze-sync-worktree/control/report.md

## Non-goals

- No direct SQLx or Storage writes.
- No route handler implementation.
- No provider/GDrive behavior.
- No conflict policy decisions inside Worktree.
- No hard delete.
- No sibling component changes.
- No workflow changes.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and implementation-worker-prompt.md. Product, test, dependency, contract, workflow, and implementation commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-worktree/control/report.md.

Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION.
