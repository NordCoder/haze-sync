# W1-WT-P8 — Watcher latency layer and runtime service

Component: worktree
Path: crates/haze-sync-worktree
Branch: component/worktree
PR: #49
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

WT-P7 implementation, clean-code review, and artifact-based CI correction are accepted. Final source CI is green.

- code_bearing_sha: a3ee10b47ae55878d6d27461774775a0febf4425
- workflow: Component CI
- workflow_run_id: 29113546394
- run_number: 1445
- conclusion: success

The next implementation phase is WT-P8 from crates/haze-sync-worktree/docs/implementation-plan.md.

## Read

Read implementation-manifest.md, report-template.md, implementation-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, current scan/import/materialization/reconciliation/delete code, accepted adapter-mode contracts, and PR diff. Do not read diagnostics artifacts unless a future fixer prompt explicitly instructs it.

## Task

Implement WT-P8: Watcher latency layer and runtime service.

- introduce a hostable runtime-service abstraction without adding Server startup wiring;
- use watcher events only as debounce/scheduling hints;
- keep full scans authoritative for correctness;
- trigger bounded scan/import planning after watcher hints;
- provide explicit startup, cancellation, shutdown, and no-overlap behavior;
- expose safe runtime status summaries without absolute local paths;
- respect disabled, read-only, import-only, export-only, and bidirectional modes where applicable;
- add focused fake-clock/fake-watcher tests for debounce, missed/duplicate/reordered hints, cancellation, shutdown, and mode behavior.

## Allowed files

- crates/haze-sync-worktree/src/**
- crates/haze-sync-worktree/docs/**
- crates/haze-sync-worktree/control/report.md

## Non-goals

No Server startup/composition code, provider behavior, watcher-only correctness, hidden unmanaged background tasks, direct DB mutation, Core policy changes, workflow/dependency changes, or sibling component changes.

## CI trigger policy

Product/source/test/docs commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-sync-worktree/control/report.md. Use report-template.md, REPORT_TYPE IMPLEMENTATION, phase_id WT-P8.
