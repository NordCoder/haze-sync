# W1-FIX-WT-P6-CI — Worktree WT-P6 CI correction

Component: worktree
Path: crates/haze-sync-worktree
Branch: component/worktree
PR: #49
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

WT-P6 implementation is complete, but its final code-bearing Component CI run failed.

- code_bearing_sha: 1c8443795ab64cfb5fb2bfd016b3a4a23e3c9957
- workflow: Component CI
- workflow_run_id: 29086948389
- run_number: 1110
- run_attempt: 1
- artifact_id: 8225158699
- artifact_name: ci-diag__component-worktree__wf-component-ci__run-29086948389__attempt-1
- artifact_expires_at: 2026-07-11T10:39:40Z

Use the diagnostics artifact as source of truth.

## Read

Read implementation-manifest.md, report-template.md, fixer-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, current WT-P6 source/tests, and PR diff. Download artifact 8225158699 and read summary.md, manifest.json, and every failed-check log. If unavailable or unreadable, report FIX_BLOCKED_BY_LOGS.

## Task

Apply only the minimum artifact-proven correction for the WT-P6 CI failure. Preserve bounded exact echo suppression, durable marker safety, reconciliation classifications, persisted-state abstraction, tests, and component boundaries.

## Allowed files

- crates/haze-sync-worktree/src/**
- crates/haze-sync-worktree/docs/** only if diagnostics prove a documentation issue
- crates/haze-sync-worktree/control/report.md

## Boundaries

No direct DB ownership, Server runtime work, provider behavior, delete/trash phase work, watcher/runtime service work, Core policy changes, workflow/dependency changes, sibling changes, test deletion, or assertion weakening.

## CI trigger policy

Source/test/docs fixer commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-sync-worktree/control/report.md. Use report-template.md, REPORT_TYPE FIX, phase_id FIX-WT-P6-CI.
