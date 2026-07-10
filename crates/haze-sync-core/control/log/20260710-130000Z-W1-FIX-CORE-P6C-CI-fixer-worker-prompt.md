# W1-FIX-CORE-P6C-CI — Core CORE-P6C CI correction

Component: core
Path: crates/haze-sync-core
Branch: component/core
PR: #43
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

CORE-P6C clean-code review completed with source/docs corrections, but its final code-bearing Component CI run failed.

- code_bearing_sha: 1714117a4254384fb286291b5a5d29314a65b749
- workflow: Component CI
- workflow_run_id: 29090558606
- run_number: 1181
- run_attempt: 1
- artifact_id: 8226606694
- artifact_name: ci-diag__component-core__wf-component-ci__run-29090558606__attempt-1
- artifact_expires_at: 2026-07-11T11:50:18Z

Use the diagnostics artifact as source of truth.

## Read

Read implementation-manifest.md, report-template.md, fixer-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, current CORE-P6C source/tests/docs, and PR diff. Download artifact 8226606694 and read summary.md, manifest.json, and every failed-check log. If unavailable or unreadable, report FIX_BLOCKED_BY_LOGS.

## Task

Apply only the minimum artifact-proven correction for the CORE-P6C CI failure. Preserve final-status replay validation, safe header restrictions, bounded and progressing changes-page semantics, deterministic fingerprints, cursor invariants, all new regression tests, and component boundaries.

## Allowed files

- crates/haze-sync-core/src/idempotency/**
- crates/haze-sync-core/src/operation_log/**
- crates/haze-sync-core/docs/** only if diagnostics prove a documentation issue
- crates/haze-sync-core/control/report.md

## Boundaries

No durable repository implementation, database append logic, HTTP replay middleware, adapter polling loop, Storage/API/Server edits, workflow/dependency changes, sibling changes, test deletion, or assertion weakening.

## CI trigger policy

Source/test/docs fixer commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-sync-core/control/report.md. Use report-template.md, REPORT_TYPE FIX, phase_id FIX-CORE-P6C-CI.
