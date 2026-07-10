# W1-FIX-GDA-P7-CI — GDrive GDA-P7 CI correction

Component: gdrive-adapter
Path: crates/haze-gdrive-adapter
Branch: component/gdrive-adapter
PR: #50
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

GDA-P7 implementation completed, but its final code-bearing Component CI run failed at diagnostics finalization.

- code_bearing_sha: 79a02e1f42084b8396148aba668388b5e7f441cb
- workflow: Component CI
- workflow_run_id: 29110469626
- run_number: 1420
- run_attempt: 1
- artifact_id: 8234581891
- artifact_name: ci-diag__component-gdrive-adapter__wf-component-ci__run-29110469626__attempt-1
- artifact_expires_at: 2026-07-11T17:19:29Z

Use the diagnostics artifact as source of truth.

## Read

Read implementation-manifest.md, report-template.md, fixer-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, current GDA-P7 source/tests, and PR diff. Download artifact 8234581891 and read summary.md, manifest.json, and every failed-check log. If unavailable or unreadable, report FIX_BLOCKED_BY_LOGS.

## Task

Apply only the minimum artifact-proven correction for the GDA-P7 CI failure. Preserve verified Core source bytes, create/update/trash planning, mode enforcement, stable operation replay, provider confirmation ordering, mapping/echo/cursor persistence ordering, safe retry classifications, redacted content handling, injected boundaries, focused tests, and component ownership.

## Allowed files

- crates/haze-gdrive-adapter/src/**
- crates/haze-gdrive-adapter/docs/** only if diagnostics prove a documentation issue
- crates/haze-gdrive-adapter/control/report.md

## Boundaries

No Core policy changes, Drive hard delete, live credentials/provider wiring, direct Storage/DB ownership, concrete Server/API transport, background scheduler, workflow/dependency changes, sibling changes, test deletion, or assertion weakening.

## CI trigger policy

Source/test/docs fixer commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-gdrive-adapter/control/report.md. Use report-template.md, REPORT_TYPE FIX, phase_id FIX-GDA-P7-CI.
