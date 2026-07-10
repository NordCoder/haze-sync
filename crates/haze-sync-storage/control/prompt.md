# W1-FIX-STOR-P8-CI — Storage STOR-P8 CI correction

Component: storage
Path: crates/haze-sync-storage
Branch: component/storage
PR: #47
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

STOR-P8 implementation completed, but its final source/docs Component CI run failed at diagnostics finalization.

- code_bearing_sha: 8fc722dcad3633ade252ba01a106a52cf08afdae
- workflow: Component CI
- workflow_run_id: 29107221057
- run_number: 1364
- run_attempt: 1
- artifact_id: 8233330099
- artifact_name: ci-diag__component-storage__wf-component-ci__run-29107221057__attempt-1
- artifact_expires_at: 2026-07-11T16:25:27Z

Use the diagnostics artifact as source of truth.

## Read

Read implementation-manifest.md, report-template.md, fixer-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, current STOR-P8 repository/tests/docs, and PR diff. Download artifact 8233330099 and read summary.md, manifest.json, and every failed-check log. If unavailable or unreadable, report FIX_BLOCKED_BY_LOGS.

## Task

Apply only the minimum artifact-proven correction for the STOR-P8 CI failure. Preserve caller-owned transaction boundaries, complete-fact upserts, safe persisted-row validation, path/provider/hash/revision/sequence/timestamp checks, path-free/provider-value-free public errors, feature-gated PostgreSQL tests, and component ownership.

## Allowed files

- crates/haze-sync-storage/src/repositories/gdrive_mapping/**
- crates/haze-sync-storage/src/repositories/worktree_state/**
- crates/haze-sync-storage/src/repositories/mod.rs
- crates/haze-sync-storage/src/models/** only if diagnostics directly require it
- crates/haze-sync-storage/docs/** only if diagnostics prove a documentation issue
- crates/haze-sync-storage/control/report.md

## Boundaries

No provider calls, credential loading, provider identity interpretation, adapter policy, filesystem behavior, public rendering, schema/migration expansion, workflow/dependency changes, sibling changes, test deletion, or assertion weakening.

## CI trigger policy

Source/test/docs fixer commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-sync-storage/control/report.md. Use report-template.md, REPORT_TYPE FIX, phase_id FIX-STOR-P8-CI.
