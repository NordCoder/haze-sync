# W1-FIX-WT-P7C-CI — Worktree WT-P7C CI correction

Component: worktree
Path: crates/haze-sync-worktree
Branch: component/worktree
PR: #49
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

WT-P7C clean-code review completed with source/test corrections, but its final code-bearing Component CI run failed at diagnostics finalization.

- code_bearing_sha: bec979eb6bebb94fa94920aa8b2af73d03c71669
- workflow: Component CI
- workflow_run_id: 29110274762
- run_number: unknown
- run_attempt: 1
- artifact_id: 8234518397
- artifact_name: ci-diag__component-worktree__wf-component-ci__run-29110274762__attempt-1
- artifact_expires_at: 2026-07-11T17:16:36Z

Use the diagnostics artifact as source of truth.

## Read

Read implementation-manifest.md, report-template.md, fixer-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, current WT-P7C source/tests, and PR diff. Download artifact 8234518397 and read summary.md, manifest.json, and every failed-check log. If unavailable or unreadable, report FIX_BLOCKED_BY_LOGS.

## Task

Apply only the minimum artifact-proven correction for the WT-P7C CI failure. Preserve removal of the obsolete delete-inferencing planner, the public put-only import boundary, guarded delete submission, safe retained-trash metadata and byte verification, retention integrity, path/symlink safety, rollback durability, focused regressions, and component ownership.

## Allowed files

- crates/haze-sync-worktree/src/**
- crates/haze-sync-worktree/docs/** only if diagnostics prove a documentation issue
- crates/haze-sync-worktree/control/report.md

## Boundaries

No Core policy changes, hard-delete cleanup, provider behavior, CLI repair work, direct DB mutation, watcher/runtime service work, Server hosting, workflow/dependency changes, sibling changes, test deletion, or assertion weakening.

## CI trigger policy

Source/test/docs fixer commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-sync-worktree/control/report.md. Use report-template.md, REPORT_TYPE FIX, phase_id FIX-WT-P7C-CI.
