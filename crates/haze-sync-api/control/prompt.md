# W1-FIX-API-P4-CI — API API-P4 CI fix

Component: api
Path: crates/haze-sync-api
Branch: component/api
PR: #44
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

API-P4 implementation completed, but Component CI for the code-bearing commit failed.

- workflow: Component CI
- workflow_run_id: 29028332903
- run_number: unknown
- run_attempt: 1
- artifact_id: 8202653799
- artifact_name: ci-diag__component-api__wf-component-ci__run-29028332903__attempt-1
- artifact_expires_at: 2026-07-10T15:11:11Z

Use the diagnostics artifact as source of truth. Do not infer the root cause only from workflow step summaries.

## Read

Read before editing:

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- fixer-worker-prompt.md from ChatGPT Project Sources
- chatgpt-gh-connector.md from ChatGPT Project Sources
- crates/haze-sync-api/docs/component-contract.md
- crates/haze-sync-api/docs/implementation-plan.md
- crates/haze-sync-api/docs/implementation-log.md
- crates/haze-sync-api/docs/dependency-map.md
- crates/haze-sync-api/control/prompt.md
- crates/haze-sync-api/control/report.md
- relevant current repository code and PR diff

Download and read diagnostics artifact 8202653799. Read summary.md, manifest.json, and every failed-check log.

If the artifact is missing, expired, malformed, or unreadable, report FIX_BLOCKED_BY_LOGS.

## Task

Fix the minimum cause of the API-P4 CI failure inside api scope.

Expected recent changed area: file and changes route-helper contract hardening. Use the artifact as source of truth.

## Allowed files

- crates/haze-sync-api/src/routes/files/**
- crates/haze-sync-api/src/routes/changes/**
- crates/haze-sync-api/src/dto/files/**
- crates/haze-sync-api/src/dto/changes/**
- crates/haze-sync-api/docs/** only if the artifact proves a docs formatting failure
- crates/haze-sync-api/control/report.md

## Forbidden changes

- Do not add Axum handler implementation.
- Do not add object-store reads or writes.
- Do not add operation-log queries.
- Do not add content streaming.
- Do not add background cursor updates.
- Do not change workflow files.
- Do not change sibling components.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and fixer-worker-prompt.md. Product/source/docs fixer commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-api/control/report.md.

Use report-template.md. Set REPORT_TYPE to FIX.
