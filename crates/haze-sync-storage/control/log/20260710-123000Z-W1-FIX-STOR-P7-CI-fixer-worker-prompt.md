# W1-FIX-STOR-P7-CI — Storage STOR-P7 CI correction

Component: storage
Path: crates/haze-sync-storage
Branch: component/storage
PR: #47
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

STOR-P7 implementation is complete, but its final code/docs-bearing Component CI run failed.

- code_bearing_sha: 83b0d2e620cbb420f147ece97aeeee4484bc6d7c
- workflow: Component CI
- workflow_run_id: 29088574856
- run_number: 1148
- run_attempt: 1
- artifact_id: 8225807379
- artifact_name: ci-diag__component-storage__wf-component-ci__run-29088574856__attempt-1
- artifact_expires_at: 2026-07-11T11:11:32Z

Use the diagnostics artifact as source of truth.

## Read

Read implementation-manifest.md, report-template.md, fixer-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, current STOR-P7 source/tests/docs, and PR diff. Download artifact 8225807379 and read summary.md, manifest.json, and every failed-check log. If unavailable or unreadable, report FIX_BLOCKED_BY_LOGS.

## Task

Apply only the minimum correction proven by the STOR-P7 diagnostics. Preserve idempotency repository outcomes, stored response handling, monotonic cursor updates, lower-sequence rejection, cursor-presence summaries, existing tests, and component boundaries.

## Allowed files

- crates/haze-sync-storage/src/repositories/idempotency/**
- crates/haze-sync-storage/src/repositories/adapter_cursors/**
- crates/haze-sync-storage/src/models/** only if diagnostics directly require it
- crates/haze-sync-storage/docs/** only if diagnostics prove a documentation issue
- crates/haze-sync-storage/control/report.md

## Boundaries

No HTTP middleware, adapter polling loop, provider calls, public admin rendering, Core policy changes, workflow/dependency changes, sibling changes, test deletion, or assertion weakening.

## CI trigger policy

Source/test/docs fixer commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-sync-storage/control/report.md. Use report-template.md, REPORT_TYPE FIX, phase_id FIX-STOR-P7-CI.
