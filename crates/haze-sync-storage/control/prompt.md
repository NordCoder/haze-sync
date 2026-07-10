# W1-FIX-STOR-P6C-CI — Storage clean-code CI correction

Component: storage
Path: crates/haze-sync-storage
Branch: component/storage
PR: #47
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

STOR-P6C clean-code review changed source/tests/docs. Its final code-bearing Component CI run failed.

- code_bearing_sha: 4f2f27ee63ddaaf48dfecd0e6fbbb6deb40f086f
- workflow: Component CI
- workflow_run_id: 29084413726
- run_number: 1044
- run_attempt: 1
- artifact_id: 8224149261
- artifact_name: ci-diag__component-storage__wf-component-ci__run-29084413726__attempt-1
- artifact_expires_at: 2026-07-11T09:53:00Z

Use the diagnostics artifact as source of truth.

## Read

Read implementation-manifest.md, report-template.md, fixer-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, current STOR-P6C source/tests/docs, and PR diff. Download artifact 8224149261 and read summary.md, manifest.json, and every failed-check log. If unavailable or unreadable, report FIX_BLOCKED_BY_LOGS.

## Task

Apply only the minimum artifact-proven correction for the STOR-P6C CI failure. Preserve safe persisted operation-kind validation, conflict/tombstone repository semantics, caller-owned transactions, source compatibility, test coverage, and all component non-goals.

## Allowed files

- crates/haze-sync-storage/src/repositories/operation_log/**
- crates/haze-sync-storage/src/repositories/conflicts/** only if diagnostics directly require it
- crates/haze-sync-storage/src/repositories/tombstones/** only if diagnostics directly require it
- crates/haze-sync-storage/docs/** only if diagnostics prove a documentation issue
- crates/haze-sync-storage/control/report.md

## Boundaries

No Core policy, hard deletion, filesystem/provider behavior, API handlers, cleanup execution, workflow/dependency changes, sibling changes, test deletion, or assertion weakening.

## CI trigger policy

Source/test/docs fixer commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-sync-storage/control/report.md. Use report-template.md, REPORT_TYPE FIX, phase_id FIX-STOR-P6C-CI.
