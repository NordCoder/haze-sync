# W1-FIX-CORE-P7C-CI — Core CORE-P7C CI correction

Component: core
Path: crates/haze-sync-core
Branch: component/core
PR: #43
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

CORE-P7C clean-code review completed with source/test/docs corrections, but its final code-bearing Component CI run failed at diagnostics finalization.

- code_bearing_sha: 08038795df8be46f3727b3b50f09b46d37ba122c
- workflow: Component CI
- workflow_run_id: 29110520533
- run_number: 1422
- run_attempt: 1
- artifact_id: 8234601738
- artifact_name: ci-diag__component-core__wf-component-ci__run-29110520533__attempt-1
- artifact_expires_at: 2026-07-11T17:20:23Z

Use the diagnostics artifact as source of truth.

## Read

Read implementation-manifest.md, report-template.md, fixer-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, current CORE-P7C doctor source/tests/docs, and PR diff. Download artifact 8234601738 and read summary.md, manifest.json, and every failed-check log. If unavailable or unreadable, report FIX_BLOCKED_BY_LOGS.

## Task

Apply only the minimum artifact-proven correction for the CORE-P7C CI failure. Preserve semantic result/report validation, fixed redacted messages, stable wire fields/order, accepted CLI field compatibility, cursor anomaly precedence, disabled-fact normalization, deterministic aggregation, regression tests, and passive Core ownership.

## Allowed files

- crates/haze-sync-core/src/doctor/**
- crates/haze-sync-core/docs/** only if diagnostics prove a documentation issue
- crates/haze-sync-core/control/report.md

## Boundaries

No live checks, CLI implementation changes, HTTP policy, persistence, provider behavior, repair execution, workflow/dependency changes, sibling changes, test deletion, assertion weakening, or arbitrary public diagnostic strings.

## CI trigger policy

Source/test/docs fixer commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-sync-core/control/report.md. Use report-template.md, REPORT_TYPE FIX, phase_id FIX-CORE-P7C-CI.
