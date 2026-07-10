# W1-FIX-STOR-P6-CI — Storage STOR-P6 CI fix

Component: storage
Path: crates/haze-sync-storage
Branch: component/storage
PR: #47
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

STOR-P6 implementation is complete, but its code/docs-bearing Component CI run is red.

- code_bearing_sha: d7f66d03428df551ee4a49bc13b0d997faaab79a
- workflow: Component CI
- workflow_run_id: 29080167290
- run_number: 916
- run_attempt: 1
- artifact_id: 8222444064
- artifact_name: ci-diag__component-storage__wf-component-ci__run-29080167290__attempt-1
- artifact_expires_at: 2026-07-11T08:35:36Z

## Read

Read implementation-manifest.md, report-template.md, fixer-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, current source, and PR diff.

Download diagnostics artifact 8222444064. Read summary.md, manifest.json, and every failed-check log. If it is missing, expired, malformed, or unreadable, report FIX_BLOCKED_BY_LOGS.

## Task

Fix the minimum artifact-proven cause of the STOR-P6 CI failure. Preserve passive conflict/tombstone repository semantics, caller-owned transactions, safe operation-log mapping, no-hard-delete behavior, and all STOR-P6 non-goals.

## Allowed files

- crates/haze-sync-storage/src/repositories/conflicts/**
- crates/haze-sync-storage/src/repositories/tombstones/**
- crates/haze-sync-storage/src/repositories/operation_log/**
- crates/haze-sync-storage/src/models/** only when directly required by diagnostics
- crates/haze-sync-storage/docs/** only if diagnostics prove a docs-format issue
- crates/haze-sync-storage/control/report.md

## Forbidden changes

No Core conflict policy, hard delete, filesystem/provider trash behavior, API handlers, retention cleanup execution, workflow changes, dependency changes, sibling component changes, test deletion, or assertion weakening.

## CI trigger policy

Product/source/docs fixer commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-storage/control/report.md. Use report-template.md. Set REPORT_TYPE to FIX and phase_id to FIX-STOR-P6-CI.
