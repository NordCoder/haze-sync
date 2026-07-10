# W1-FIX-STOR-P7C-CI — Storage STOR-P7C CI correction

Component: storage
Path: crates/haze-sync-storage
Branch: component/storage
PR: #47
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

STOR-P7C clean-code review completed with source/test corrections, but its final code-bearing Component CI run failed.

- code_bearing_sha: 7334c568c95823350fdb1f8bbc0ad8743877d4d4
- workflow: Component CI
- workflow_run_id: 29092943655
- run_number: 1212
- run_attempt: 1
- artifact_id: 8227556619
- artifact_name: ci-diag__component-storage__wf-component-ci__run-29092943655__attempt-1
- artifact_expires_at: 2026-07-11T12:35:14Z

Use the diagnostics artifact as source of truth.

## Read

Read implementation-manifest.md, report-template.md, fixer-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, current STOR-P7C source/tests, and PR diff. Download artifact 8227556619 and read summary.md, manifest.json, and every failed-check log. If unavailable or unreadable, report FIX_BLOCKED_BY_LOGS.

## Task

Apply only the minimum artifact-proven correction for the STOR-P7C CI failure. Preserve validated idempotency input construction, first-write/replay/conflict behavior, stored response safety, monotonic cursor behavior, public cursor-presence summaries, transaction boundaries, tests, and component ownership.

## Allowed files

- crates/haze-sync-storage/src/repositories/idempotency/**
- crates/haze-sync-storage/src/repositories/adapter_cursors/**
- crates/haze-sync-storage/src/models/** only if diagnostics directly require it
- crates/haze-sync-storage/docs/** only if diagnostics prove a documentation issue
- crates/haze-sync-storage/control/report.md

## Boundaries

No HTTP middleware, adapter polling, provider calls, public admin rendering, Core policy implementation, schema/migration expansion, workflow/dependency changes, sibling changes, test deletion, or assertion weakening.

## CI trigger policy

Source/test/docs fixer commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-sync-storage/control/report.md. Use report-template.md, REPORT_TYPE FIX, phase_id FIX-STOR-P7C-CI.
