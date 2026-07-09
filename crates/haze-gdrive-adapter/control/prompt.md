# W1-FIX-GDA-P4-CI-RERUN — GDrive adapter GDA-P4 CI fix

Component: gdrive-adapter
Path: crates/haze-gdrive-adapter
Branch: component/gdrive-adapter
PR: #50
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Rerun guard

This is an explicit refreshed active prompt. The current report before this prompt was for GDA-P4, not FIX-GDA-P4-CI. Therefore FIX-GDA-P4-CI-RERUN is not already complete.

## Context

GDA-P4 implementation completed with SELF_ACCEPT_PENDING_CI. Component CI for the code-bearing commit failed.

- workflow: Component CI
- workflow_run_id: 29034799276
- run_number: 687
- run_attempt: 1
- artifact_id: 8205445935
- artifact_name: ci-diag__component-gdrive-adapter__wf-component-ci__run-29034799276__attempt-1
- artifact_expires_at: 2026-07-10T16:51:05Z

## Read

Read implementation-manifest.md, report-template.md, fixer-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, relevant code and PR diff. Download and read diagnostics artifact 8205445935. Read summary.md, manifest.json, and every failed-check log. If missing/expired/malformed/unreadable, report FIX_BLOCKED_BY_LOGS.

## Task

Fix the minimum cause of the GDA-P4 CI failure inside gdrive-adapter scope. Expected recent changed area: adapter-local mapping, cursor, echo-state boundary. Use the artifact as source of truth.

## Allowed files

- crates/haze-gdrive-adapter/src/**
- crates/haze-gdrive-adapter/docs/** only if the artifact proves a docs formatting failure
- crates/haze-gdrive-adapter/control/report.md

## Forbidden changes

No direct DB access, provider sync loop, Core policy, hard delete, workflow changes, or sibling component changes.

## CI trigger policy

Product/source/docs fixer commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-gdrive-adapter/control/report.md. Use report-template.md. Set REPORT_TYPE to FIX and phase_id to FIX-GDA-P4-CI-RERUN.
