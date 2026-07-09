# W1-FIX-GDA-P4-CI — GDrive adapter GDA-P4 CI fix

Component: gdrive-adapter
Path: crates/haze-gdrive-adapter
Branch: component/gdrive-adapter
PR: #50
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

GDA-P4 implementation completed with SELF_ACCEPT_PENDING_CI. Component CI for the code-bearing commit failed.

- workflow: Component CI
- workflow_run_id: 29034799276
- run_number: 687
- run_attempt: 1
- artifact_id: 8205445935
- artifact_name: ci-diag__component-gdrive-adapter__wf-component-ci__run-29034799276__attempt-1
- artifact_expires_at: 2026-07-10T16:51:05Z

Use the diagnostics artifact as source of truth.

## Read

Read before editing:

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- fixer-worker-prompt.md from ChatGPT Project Sources
- chatgpt-gh-connector.md from ChatGPT Project Sources
- crates/haze-gdrive-adapter/docs/component-contract.md
- crates/haze-gdrive-adapter/docs/implementation-plan.md
- crates/haze-gdrive-adapter/docs/implementation-log.md
- crates/haze-gdrive-adapter/docs/dependency-map.md
- crates/haze-gdrive-adapter/control/prompt.md
- crates/haze-gdrive-adapter/control/report.md
- relevant current repository code and PR diff

Download and read diagnostics artifact 8205445935. Read summary.md, manifest.json, and every failed-check log.

If the artifact is missing, expired, malformed, or unreadable, report FIX_BLOCKED_BY_LOGS.

## Task

Fix the minimum cause of the GDA-P4 CI failure inside gdrive-adapter scope.

Expected recent changed area: adapter-local mapping, cursor, echo-state boundary. Use the artifact as source of truth.

## Allowed files

- crates/haze-gdrive-adapter/src/**
- crates/haze-gdrive-adapter/docs/** only if the artifact proves a docs formatting failure
- crates/haze-gdrive-adapter/control/report.md

## Forbidden changes

- Do not add direct DB access.
- Do not add provider sync loop.
- Do not add Core policy.
- Do not add hard delete.
- Do not change workflow files.
- Do not change sibling components.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and fixer-worker-prompt.md. Product/source/docs fixer commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-gdrive-adapter/control/report.md.

Use report-template.md. Set REPORT_TYPE to FIX.
