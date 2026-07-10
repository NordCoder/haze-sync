# W1-FIX-API-P6-CI — API status-contract CI correction

Component: api
Path: crates/haze-sync-api
Branch: component/api
PR: #44
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

API-P6 implementation is complete, but its final code/docs-bearing Component CI run failed.

- code_bearing_sha: 613b58cbc9be6c20329b1dfb36d889c879e8f8d1
- workflow: Component CI
- workflow_run_id: 29084450470
- run_number: 1050
- run_attempt: 1
- artifact_id: 8224162592
- artifact_name: ci-diag__component-api__wf-component-ci__run-29084450470__attempt-1
- artifact_expires_at: 2026-07-11T09:53:37Z

Use the diagnostics artifact as source of truth.

## Read

Read implementation-manifest.md, report-template.md, fixer-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, current API-P6 source/tests/docs, and PR diff. Download artifact 8224162592 and read summary.md, manifest.json, and every failed-check log. If unavailable or unreadable, report FIX_BLOCKED_BY_LOGS.

## Task

Apply only the minimum artifact-proven correction for the API-P6 CI failure. Preserve source compatibility, passive API ownership, safe operational DTO vocabulary, sanitized server/adapter status output, explicit skipped/not-run/placeholder semantics, tests, and all API-P6 non-goals.

## Allowed files

- crates/haze-sync-api/src/routes/admin/**
- crates/haze-sync-api/src/dto/server/**
- crates/haze-sync-api/src/dto/common/** only if diagnostics directly require it
- crates/haze-sync-api/docs/** only if diagnostics prove a documentation issue
- crates/haze-sync-api/control/report.md

## Boundaries

No live check execution, Server readiness behavior, pause/resume mutation, repair execution, provider calls, persistence, runtime route wiring, workflow/dependency changes, sibling changes, test deletion, or assertion weakening.

## CI trigger policy

Source/test/docs fixer commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-sync-api/control/report.md. Use report-template.md, REPORT_TYPE FIX, phase_id FIX-API-P6-CI.
