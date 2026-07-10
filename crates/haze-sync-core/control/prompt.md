# W1-FIX-CORE-P7-CI — Core CORE-P7 CI correction

Component: core
Path: crates/haze-sync-core
Branch: component/core
PR: #43
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

CORE-P7 implementation completed, but its final code/docs Component CI run failed at diagnostics finalization.

- code_bearing_sha: 6b6803d934a38883c0e0032a5f5a3917d4a99d43
- workflow: Component CI
- workflow_run_id: 29103013245
- run_number: 1278
- run_attempt: 1
- artifact_id: 8231629761
- artifact_name: ci-diag__component-core__wf-component-ci__run-29103013245__attempt-1
- artifact_expires_at: 2026-07-11T15:17:39Z

Use the diagnostics artifact as source of truth.

## Read

Read implementation-manifest.md, report-template.md, fixer-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, current CORE-P7 source/tests/docs, and PR diff. Download artifact 8231629761 and read summary.md, manifest.json, and every failed-check log. If unavailable or unreadable, report FIX_BLOCKED_BY_LOGS.

## Task

Apply only the minimum artifact-proven correction for the CORE-P7 CI failure. Preserve passive doctor ownership, fixed safe messages, explicit skipped/not-run/placeholder states, deterministic aggregation, validating serialization, downstream contract documentation, all regression tests, and component boundaries.

## Allowed files

- crates/haze-sync-core/src/doctor/**
- crates/haze-sync-core/docs/** only if diagnostics prove a documentation issue
- crates/haze-sync-core/control/report.md

## Boundaries

No live checks, CLI behavior, HTTP policy, persistence, provider behavior, repair execution, workflow/dependency changes, sibling changes, test deletion, or assertion weakening.

## CI trigger policy

Source/test/docs fixer commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-sync-core/control/report.md. Use report-template.md, REPORT_TYPE FIX, phase_id FIX-CORE-P7-CI.
