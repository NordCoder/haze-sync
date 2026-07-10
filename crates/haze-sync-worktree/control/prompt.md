# W1-FIX-WT-P7-CI — Worktree WT-P7 CI correction

Component: worktree
Path: crates/haze-sync-worktree
Branch: component/worktree
PR: #49
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

WT-P7 implementation completed, but its final code-bearing Component CI run failed at diagnostics finalization.

- code_bearing_sha: 3156b1723b942b94568c846bcbf11915bbfe881e
- workflow: Component CI
- workflow_run_id: 29103228383
- run_number: unknown
- run_attempt: 1
- artifact_id: 8231727075
- artifact_name: ci-diag__component-worktree__wf-component-ci__run-29103228383__attempt-1
- artifact_expires_at: 2026-07-11T15:21:19Z

Use the diagnostics artifact as source of truth.

## Read

Read implementation-manifest.md, report-template.md, fixer-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, current WT-P7 source/tests, and PR diff. Download artifact 8231727075 and read summary.md, manifest.json, and every failed-check log. If unavailable or unreadable, report FIX_BLOCKED_BY_LOGS.

## Task

Apply only the minimum artifact-proven correction for the WT-P7 CI failure. Preserve guarded delete candidate planning, base/null-base semantics, put-only import behavior, reversible retained trash, restore-ready metadata, path safety, retention facts, all focused tests, and component boundaries.

## Allowed files

- crates/haze-sync-worktree/src/**
- crates/haze-sync-worktree/docs/** only if diagnostics prove a documentation issue
- crates/haze-sync-worktree/control/report.md

## Boundaries

No Core policy changes, hard-delete cleanup, provider behavior, CLI repair work, direct DB mutation, watcher/runtime service work, Server hosting, workflow/dependency changes, sibling changes, test deletion, or assertion weakening.

## CI trigger policy

Source/test/docs fixer commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-sync-worktree/control/report.md. Use report-template.md, REPORT_TYPE FIX, phase_id FIX-WT-P7-CI.
