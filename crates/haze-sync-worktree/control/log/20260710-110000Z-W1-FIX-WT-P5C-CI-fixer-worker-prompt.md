# Archived active prompt

component: worktree
archived_at: 2026-07-10T11:00:00Z
wave: W1
phase: FIX-WT-P5C-CI
agent_role: fixer-worker
source_path: crates/haze-sync-worktree/control/prompt.md
source_sha: 4fa8893769d96ecc4b5c435c8b951f6879b4ce87

# W1-FIX-WT-P5C-CI — Worktree clean-code CI correction

Component: worktree
Path: crates/haze-sync-worktree
Branch: component/worktree
PR: #49
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

WT-P5C clean-code review made component source and test changes. Its final code-bearing Component CI run failed.

- code_bearing_sha: 6fbb047ded2082f37f2168ddd0e2b0f50ddc5d80
- workflow: Component CI
- workflow_run_id: 29082119591
- run_number: 982
- run_attempt: 1
- artifact_id: 8223221488
- artifact_name: ci-diag__component-worktree__wf-component-ci__run-29082119591__attempt-1
- artifact_expires_at: 2026-07-11T09:11:24Z

Use the diagnostics artifact as source of truth.

## Read

Read implementation-manifest.md, report-template.md, fixer-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, current source/tests, and PR diff. Download artifact 8223221488 and read summary.md, manifest.json, and every failed-check log. If unavailable or unreadable, report FIX_BLOCKED_BY_LOGS.

## Task

Apply only the minimum artifact-proven correction for the WT-P5C CI failure. Preserve the accepted path-boundary checks, materializer behavior, regression coverage, and component boundaries.

## Allowed files

- crates/haze-sync-worktree/src/**
- crates/haze-sync-worktree/docs/** only if diagnostics prove a documentation issue
- crates/haze-sync-worktree/control/report.md

## Boundaries

No Core policy changes, API contract redesign, provider behavior, watcher/runtime phase work, deletion policy changes, workflow/dependency changes, sibling changes, test deletion, or assertion weakening.

## CI trigger policy

Source/test/docs fixer commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-sync-worktree/control/report.md. Use report-template.md, REPORT_TYPE FIX, phase_id FIX-WT-P5C-CI.
