# W1-WT-P9 — Doctor, repair planning, and hostable fan-in boundary

Component: worktree
Path: crates/haze-sync-worktree
Branch: component/worktree
PR: #49
Role: implementation-worker

Work only through the GitHub connector. Do not merge the PR or change its lifecycle state.

## Context

WT-P8 implementation, clean-code review, and artifact-based CI corrections are accepted. Final source/test CI is green.

- code_bearing_sha: 5e17deb8bb8080f8eebdc9bb3bed6c42ba3a2031
- workflow: Component CI
- workflow_run_id: 29127189662
- run_number: 1621
- conclusion: success

The next implementation phase is WT-P9 from crates/haze-sync-worktree/docs/implementation-plan.md.

This component-local pass owns Worktree doctor facts, non-destructive repair planning, and hostable interfaces. Do not edit Server files; concrete Server composition requires a dedicated later fan-in prompt.

## Read

Read the project process files, current Worktree control files, the exact WT-P9 plan section, current scanner/materialization/reconciliation/delete/runtime source and tests, accepted Core doctor and repair semantics, accepted Storage worktree-state boundaries, safe Server admin/doctor contracts, and PR diff. Do not read diagnostics artifacts unless a later fixer prompt explicitly requires them.

## Task

Implement WT-P9 Worktree-owned diagnostics and repair planning.

- produce safe doctor summaries for missing files, dirty files, content-hash mismatches, reserved-path violations, skipped symlinks/special files, and echo/drift state;
- derive diagnostics from accepted scan/reconciliation/runtime facts without treating watcher events as correctness evidence;
- model explicit repair plans without executing destructive changes by default;
- require confirmation or an accepted higher-level contract for any overwrite, move, trash, or delete action;
- expose path-safe, redacted, count/category-based status data suitable for later Server hosting;
- keep persisted state access behind injected accepted boundaries;
- add focused tests for healthy, dirty, missing, mismatch, reserved-path, skipped-file, stale-echo, partial-scan, and non-destructive repair-plan scenarios;
- document the future Server hosting boundary and remaining E2E fan-in work.

## Allowed files

- crates/haze-sync-worktree/src/**
- crates/haze-sync-worktree/docs/**
- crates/haze-sync-worktree/control/report.md

## Non-goals

No Server source changes, automatic destructive repair, provider sync, direct database ownership, public absolute paths, concrete CLI repair command, workflow/dependency changes, or sibling component changes.

Product/source/test/docs commits must run CI normally. A final report-only commit may skip CI.

Write only crates/haze-sync-worktree/control/report.md using REPORT_TYPE IMPLEMENTATION and phase_id WT-P9.
