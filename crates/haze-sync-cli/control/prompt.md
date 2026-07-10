# W1-FIX-CLI-P5-CI — CLI live-doctor CI fix

Component: cli
Path: crates/haze-sync-cli
Branch: component/cli
PR: #48
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

CLI-P5 implementation is complete, but its code-bearing Component CI run is red.

- code_bearing_sha: 8497ce6d80baa412893be46d5f8b96140367dcb1
- workflow: Component CI
- workflow_run_id: 29080204443
- run_number: 917
- run_attempt: 1
- artifact_id: 8222457216
- artifact_name: ci-diag__component-cli__wf-component-ci__run-29080204443__attempt-1
- artifact_expires_at: 2026-07-11T08:36:13Z

## Read

Read implementation-manifest.md, report-template.md, fixer-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, current source, and PR diff.

Download diagnostics artifact 8222457216. Read `summary.md`, `manifest.json`, and every failed-check log. If it is missing, expired, malformed, or unreadable, report `FIX_BLOCKED_BY_LOGS`.

## Task

Fix the minimum artifact-proven cause of the CLI-P5 CI failure. Preserve offline-by-default behavior, explicit `doctor --live`, accepted health/readiness/status aggregation, honest skipped/not-run checks, safe output, and all CLI-P5 non-goals.

## Allowed files

- crates/haze-sync-cli/src/**
- crates/haze-sync-cli/docs/** only if diagnostics prove a documentation-format failure
- crates/haze-sync-cli/control/report.md

## Forbidden changes

No repair behavior, direct DB/object-store/provider access, new Server/API routes or DTOs, concrete transport/config expansion beyond the artifact-proven fix, workflow changes, dependency changes, sibling component changes, test deletion, or assertion weakening.

## CI trigger policy

Product/source/docs fixer commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-cli/control/report.md. Use report-template.md. Set REPORT_TYPE to FIX and phase_id to FIX-CLI-P5-CI.
