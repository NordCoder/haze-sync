# W1-FIX-API-P5-CI — API API-P5 CI correction

Component: api
Path: crates/haze-sync-api
Branch: component/api
PR: #44
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

API-P5 implementation is complete, but the Component CI run for its source commit did not complete successfully.

- source_commit: b31c4b89482e184d42091a0718c73f63a38b9cae
- workflow: Component CI
- workflow_run_id: 29067723610
- run_number: 848
- run_attempt: 1
- artifact_id: 8217771726
- artifact_name: ci-diag__component-api__wf-component-ci__run-29067723610__attempt-1
- artifact_expires_at: 2026-07-11T03:52:54Z

Use the diagnostics artifact as the authoritative description of the failed check.

## Read

Read implementation-manifest.md, report-template.md, fixer-worker-prompt.md, chatgpt-gh-connector.md, component docs and control files, relevant source files, and the current PR diff.

Download artifact 8217771726. Read `summary.md`, `manifest.json`, and each log named by `failed_checks`. If the artifact is unavailable or cannot be interpreted, report `FIX_BLOCKED_BY_LOGS`.

## Task

Apply the smallest API-local correction needed for the failed API-P5 check. Preserve the existing conflict and delete request/response contracts, compatibility, safe error vocabulary, and phase boundaries.

## Allowed files

- crates/haze-sync-api/src/routes/conflicts/**
- crates/haze-sync-api/src/routes/delete/**
- crates/haze-sync-api/src/dto/conflicts/**
- crates/haze-sync-api/src/dto/files/** when directly required
- crates/haze-sync-api/docs/** only when the diagnostics identify a docs-format issue
- crates/haze-sync-api/control/report.md

## Boundaries

Do not add new route execution behavior, persistence behavior, provider behavior, deletion execution, workflow changes, dependency changes, or sibling component changes.

## CI trigger policy

Source or documentation correction commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-api/control/report.md. Use report-template.md. Set REPORT_TYPE to FIX and phase_id to FIX-API-P5-CI.
