# W1-WT-P5 — Materializer and atomic writer

Component: worktree
Path: crates/haze-sync-worktree
Branch: component/worktree
PR: #49
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

WT-P4 implementation, fixer, and clean-code review are accepted. Component CI evidence for the accepted source state is green.

- workflow: Component CI
- workflow_run_id: 29034738311
- run_number: 679
- conclusion: success

The next implementation phase is WT-P5 from crates/haze-sync-worktree/docs/implementation-plan.md.

## Read

Read implementation-manifest.md, report-template.md, implementation-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, relevant current code, and PR diff.

Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Implement WT-P5: Materializer and atomic writer.

Follow the plan:

- represent materialization requests from Core/API changes feed or Server-hosted state;
- verify incoming bytes against expected content hash;
- write through temp files under reserved runtime area;
- use fsync/rename or an accepted platform-safe equivalent where practical;
- avoid overwriting dirty local files and create a conflict/import plan instead where necessary;
- update worktree state after successful materialization;
- add an echo-guard marker for adapter-written files;
- test partial-write cleanup where practical.

## Allowed files

- crates/haze-sync-worktree/src/**
- crates/haze-sync-worktree/docs/**
- crates/haze-sync-worktree/control/report.md

## Non-goals

No Core conflict policy, API DTO redesign, provider calls, watcher requirement, hard delete, workflow changes, or sibling component changes.

## CI trigger policy

Product/source/docs commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-worktree/control/report.md. Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION and phase_id to WT-P5.
