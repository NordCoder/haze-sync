# W1-FIX-STOR-P5-CI — Storage STOR-P5 CI fix

Component: storage
Path: crates/haze-sync-storage
Branch: component/storage
PR: #47
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

STOR-P5 implementation completed with SELF_ACCEPT_PENDING_CI. Component CI for the code/docs-bearing commit failed.

- workflow: Component CI
- workflow_run_id: 29035028384
- run_number: 701
- run_attempt: 1
- artifact_id: 8205530856
- artifact_name: ci-diag__component-storage__wf-component-ci__run-29035028384__attempt-1
- artifact_expires_at: 2026-07-10T16:54:29Z

Use the diagnostics artifact as source of truth.

## Read

Read before editing:

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- fixer-worker-prompt.md from ChatGPT Project Sources
- chatgpt-gh-connector.md from ChatGPT Project Sources
- crates/haze-sync-storage/docs/component-contract.md
- crates/haze-sync-storage/docs/implementation-plan.md
- crates/haze-sync-storage/docs/implementation-log.md
- crates/haze-sync-storage/docs/dependency-map.md
- crates/haze-sync-storage/control/prompt.md
- crates/haze-sync-storage/control/report.md
- relevant current repository code and PR diff

Download and read diagnostics artifact 8205530856. Read summary.md, manifest.json, and every failed-check log.

If the artifact is missing, expired, malformed, or unreadable, report FIX_BLOCKED_BY_LOGS.

## Task

Fix the minimum cause of the STOR-P5 CI failure inside storage scope.

Expected recent changed area: normal file flow repository support, content blobs, objects, revisions, operation log, locks, and storage docs. Use the artifact as source of truth.

## Allowed files

- crates/haze-sync-storage/src/repositories/content_blobs/**
- crates/haze-sync-storage/src/repositories/objects/**
- crates/haze-sync-storage/src/repositories/revisions/**
- crates/haze-sync-storage/src/repositories/operation_log/**
- crates/haze-sync-storage/src/locks.rs
- crates/haze-sync-storage/docs/** only if the artifact proves a docs formatting failure
- crates/haze-sync-storage/control/report.md

## Forbidden changes

- Do not add Core upsert decision implementation.
- Do not add HTTP handlers.
- Do not add content upload streaming runtime.
- Do not add adapter loop.
- Do not add conflict/delete behavior beyond current flow dependencies.
- Do not change workflow files.
- Do not change sibling components.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and fixer-worker-prompt.md. Product/source/docs fixer commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-storage/control/report.md.

Use report-template.md. Set REPORT_TYPE to FIX.
