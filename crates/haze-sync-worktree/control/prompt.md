# W1-FIX-WT-P4-CI — Worktree WT-P4 CI fix

Component: worktree
Path: crates/haze-sync-worktree
Branch: component/worktree
PR: #49
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

WT-P4 implementation completed, but Component CI for the code-bearing commit failed.

- workflow: Component CI
- workflow_run_id: 29028324784
- run_number: unknown
- run_attempt: 1
- artifact_id: 8202650036
- artifact_name: ci-diag__component-worktree__wf-component-ci__run-29028324784__attempt-1
- artifact_expires_at: 2026-07-10T15:11:04Z

The implementation report observed cargo fmt/check/test/clippy step summaries as success but the diagnostics finalizer failed. The diagnostics artifact is the source of truth. Do not infer the root cause from step summaries alone.

## Read

Read before editing:

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- fixer-worker-prompt.md from ChatGPT Project Sources
- chatgpt-gh-connector.md from ChatGPT Project Sources
- crates/haze-sync-worktree/docs/component-contract.md
- crates/haze-sync-worktree/docs/implementation-plan.md
- crates/haze-sync-worktree/docs/implementation-log.md
- crates/haze-sync-worktree/docs/dependency-map.md
- crates/haze-sync-worktree/control/prompt.md
- crates/haze-sync-worktree/control/report.md
- relevant current repository code and PR diff

Use the GitHub connector to download and read the diagnostics artifact listed above.

Read summary.md, manifest.json, and every log listed in failed_checks.

If the artifact is missing, expired, malformed, or unreadable, report FIX_BLOCKED_BY_LOGS.

## Task

Fix the minimum cause of the WT-P4 CI failure inside worktree scope. Use the diagnostics artifact as source of truth.

Expected recent changed area: WT-P4 import planner and abstract Core/API submission boundary.

## Allowed files

- crates/haze-sync-worktree/src/**
- crates/haze-sync-worktree/docs/** only if the artifact proves a docs formatting failure
- crates/haze-sync-worktree/control/report.md

## Forbidden changes

- Do not add direct SQLx or Storage writes.
- Do not add route handlers.
- Do not add provider/GDrive behavior.
- Do not add conflict policy decisions inside Worktree.
- Do not add hard delete.
- Do not change workflow files.
- Do not change sibling components.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and fixer-worker-prompt.md. Product/source/docs fixer commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-worktree/control/report.md.

Use report-template.md. Set REPORT_TYPE to FIX.
