# W1-FIX-WT-P6C-CI — Worktree WT-P6C CI correction

Component: worktree
Path: crates/haze-sync-worktree
Branch: component/worktree
PR: #49
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

WT-P6C clean-code review completed with source/test corrections, but its final code-bearing Component CI run failed.

- code_bearing_sha: 3c593f544dca3fdfd31d4613314a8e75e33dccb4
- workflow: Component CI
- workflow_run_id: 29090759385
- run_number: 1184
- run_attempt: 1
- artifact_id: 8226685262
- artifact_name: ci-diag__component-worktree__wf-component-ci__run-29090759385__attempt-1
- artifact_expires_at: 2026-07-11T11:54:10Z

Use the diagnostics artifact as source of truth.

## Read

Read implementation-manifest.md, report-template.md, fixer-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, current WT-P6C source/tests, and PR diff. Download artifact 8226685262 and read summary.md, manifest.json, and every failed-check log. If unavailable or unreadable, report FIX_BLOCKED_BY_LOGS.

## Task

Apply only the minimum artifact-proven correction for the WT-P6C CI failure. Preserve conservative incomplete-scan handling, idempotent durable-marker reload behavior, bounded exact echo suppression, reconciliation classifications, all regression tests, and component boundaries.

## Allowed files

- crates/haze-sync-worktree/src/**
- crates/haze-sync-worktree/docs/** only if diagnostics prove a documentation issue
- crates/haze-sync-worktree/control/report.md

## Boundaries

No direct DB ownership, Server runtime work, provider behavior, delete/trash work, watcher/runtime service work, Core policy changes, workflow/dependency changes, sibling changes, test deletion, or assertion weakening.

## CI trigger policy

Source/test/docs fixer commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-sync-worktree/control/report.md. Use report-template.md, REPORT_TYPE FIX, phase_id FIX-WT-P6C-CI.
