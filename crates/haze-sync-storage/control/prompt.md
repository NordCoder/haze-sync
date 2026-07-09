# W1-FIX-STOR-P5-CI-RERUN — Storage STOR-P5 CI fix

Component: storage
Path: crates/haze-sync-storage
Branch: component/storage
PR: #47
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Rerun guard

This is an explicit refreshed active prompt. The current report before this prompt was for STOR-P5, not FIX-STOR-P5-CI. Therefore FIX-STOR-P5-CI-RERUN is not already complete.

## Context

STOR-P5 implementation completed with SELF_ACCEPT_PENDING_CI. Component CI for the code/docs-bearing commit failed.

- workflow: Component CI
- workflow_run_id: 29035028384
- run_number: 701
- run_attempt: 1
- artifact_id: 8205530856
- artifact_name: ci-diag__component-storage__wf-component-ci__run-29035028384__attempt-1
- artifact_expires_at: 2026-07-10T16:54:29Z

## Read

Read implementation-manifest.md, report-template.md, fixer-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, relevant code and PR diff. Download and read diagnostics artifact 8205530856. Read summary.md, manifest.json, and every failed-check log. If missing/expired/malformed/unreadable, report FIX_BLOCKED_BY_LOGS.

## Task

Fix the minimum cause of the STOR-P5 CI failure inside storage scope. Expected recent changed area: normal file flow repository support, content blobs, objects, revisions, operation log, locks, and storage docs. Use the artifact as source of truth.

## Allowed files

- crates/haze-sync-storage/src/repositories/content_blobs/**
- crates/haze-sync-storage/src/repositories/objects/**
- crates/haze-sync-storage/src/repositories/revisions/**
- crates/haze-sync-storage/src/repositories/operation_log/**
- crates/haze-sync-storage/src/locks.rs
- crates/haze-sync-storage/docs/** only if the artifact proves a docs formatting failure
- crates/haze-sync-storage/control/report.md

## Forbidden changes

No Core upsert decisions, HTTP handlers, content streaming runtime, adapter loop, extra conflict/delete behavior, workflow changes, or sibling component changes.

## CI trigger policy

Product/source/docs fixer commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-storage/control/report.md. Use report-template.md. Set REPORT_TYPE to FIX and phase_id to FIX-STOR-P5-CI-RERUN.
