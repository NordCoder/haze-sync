# W1-FIX-CLI-P4-CI-2-RERUN — CLI CLI-P4 follow-up CI fix

Component: cli
Path: crates/haze-sync-cli
Branch: component/cli
PR: #48
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Rerun guard

This is an explicit refreshed active prompt. The current report before this prompt was for FIX-CLI-P4-CI, not FIX-CLI-P4-CI-2. Therefore FIX-CLI-P4-CI-2-RERUN is not already complete.

## Context

The first CLI-P4 fixer completed, but post-fix Component CI failed.

- workflow: Component CI
- workflow_run_id: 29034837249
- run_number: 693
- run_attempt: 1
- artifact_id: 8205459786
- artifact_name: ci-diag__component-cli__wf-component-ci__run-29034837249__attempt-1
- artifact_expires_at: 2026-07-10T16:51:38Z

## Read

Read implementation-manifest.md, report-template.md, fixer-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, relevant code and PR diff. Download and read diagnostics artifact 8205459786. Read summary.md, manifest.json, and every failed-check log. If missing/expired/malformed/unreadable, report FIX_BLOCKED_BY_LOGS.

## Task

Fix the minimum cause of the latest CLI-P4 CI failure inside cli scope. Expected recent changed area: CLI status/adapters output compatibility and safe rendering. Use the artifact as source of truth.

## Allowed files

- crates/haze-sync-cli/src/**
- crates/haze-sync-cli/docs/** only if the artifact proves a docs formatting failure
- crates/haze-sync-cli/control/report.md

## Forbidden changes

No admin mutations, direct DB reads, provider calls, sibling route changes, token rotation, workflow changes, sibling component changes, or test deletion.

## CI trigger policy

Product/source/docs fixer commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-cli/control/report.md. Use report-template.md. Set REPORT_TYPE to FIX and phase_id to FIX-CLI-P4-CI-2-RERUN.
