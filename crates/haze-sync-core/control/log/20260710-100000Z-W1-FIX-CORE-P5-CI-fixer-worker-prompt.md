# W1-FIX-CORE-P5-CI — Core CORE-P5 CI fix

Component: core
Path: crates/haze-sync-core
Branch: component/core
PR: #43
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

CORE-P5 implementation is complete, but its code/docs-bearing Component CI run is red.

- code_bearing_sha: 5bbb15d056f305ecb95b353ea8f70162bd0677d9
- workflow: Component CI
- workflow_run_id: 29080048723
- run_number: 911
- run_attempt: 1
- artifact_id: 8222399369
- artifact_name: ci-diag__component-core__wf-component-ci__run-29080048723__attempt-1
- artifact_expires_at: 2026-07-11T08:33:27Z

## Read

Read implementation-manifest.md, report-template.md, fixer-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, current source, and PR diff.

Download diagnostics artifact 8222399369. Read summary.md, manifest.json, and every failed-check log. If it is missing, expired, malformed, or unreadable, report FIX_BLOCKED_BY_LOGS.

## Task

Fix the minimum artifact-proven cause of the CORE-P5 CI failure. Preserve tombstone validation, restore/retention classifiers, delete-guard thresholds and scoped unlock semantics, storage/adapter neutrality, and all CORE-P5 non-goals.

## Allowed files

- crates/haze-sync-core/src/tombstone_service/**
- crates/haze-sync-core/src/delete_guard/**
- crates/haze-sync-core/docs/** only if diagnostics prove a docs-format issue
- crates/haze-sync-core/control/report.md

## Forbidden changes

No hard-delete cleanup, filesystem trash behavior, provider calls, tombstone repository, CLI parsing, API/Server route wiring, workflow changes, dependency changes, sibling component changes, test deletion, or assertion weakening.

## CI trigger policy

Product/source/docs fixer commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-core/control/report.md. Use report-template.md. Set REPORT_TYPE to FIX and phase_id to FIX-CORE-P5-CI.
