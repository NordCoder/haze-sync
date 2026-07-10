# W1-FIX-GDA-P6-CI — GDrive GDA-P6 CI correction

Component: gdrive-adapter
Path: crates/haze-gdrive-adapter
Branch: component/gdrive-adapter
PR: #50
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

GDA-P6 implementation completed, but its final code-bearing Component CI run failed.

- code_bearing_sha: 6912e1165855672340980ae55e118f1469169de3
- workflow: Component CI
- workflow_run_id: 29090536525
- run_number: 1179
- run_attempt: 1
- artifact_id: 8226593826
- artifact_name: ci-diag__component-gdrive-adapter__wf-component-ci__run-29090536525__attempt-1
- artifact_expires_at: 2026-07-11T11:49:39Z

Use the diagnostics artifact as source of truth.

## Read

Read implementation-manifest.md, report-template.md, fixer-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, current GDA-P6 source/tests, and PR diff. Download artifact 8226593826 and read summary.md, manifest.json, and every failed-check log. If unavailable or unreadable, report FIX_BLOCKED_BY_LOGS.

## Task

Apply only the minimum artifact-proven correction for the GDA-P6 CI failure. Preserve cursor invalidation fallback, deterministic coalescing, success-only cursor advancement, retry/backoff classification, provider-token redaction, tests, and injected persistence boundaries.

## Allowed files

- crates/haze-gdrive-adapter/src/**
- crates/haze-gdrive-adapter/docs/** only if diagnostics prove a documentation issue
- crates/haze-gdrive-adapter/control/report.md

## Boundaries

No live provider client, export apply runner, provider mutation, concrete Storage/DB wiring, Core policy, background runtime, workflow/dependency changes, sibling changes, test deletion, or assertion weakening.

## CI trigger policy

Source/test/docs fixer commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-gdrive-adapter/control/report.md. Use report-template.md, REPORT_TYPE FIX, phase_id FIX-GDA-P6-CI.
