# W1-FIX-CLI-P4-CI — CLI CLI-P4 CI fix

Component: cli
Path: crates/haze-sync-cli
Branch: component/cli
PR: #48
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

CLI-P4 implementation completed with SELF_ACCEPT_PENDING_CI. Component CI for the code-bearing commit failed.

- workflow: Component CI
- workflow_run_id: 29028358662
- run_number: 634
- run_attempt: 1
- artifact_id: 8202662299
- artifact_name: ci-diag__component-cli__wf-component-ci__run-29028358662__attempt-1
- artifact_expires_at: 2026-07-10T15:11:27Z

Use the diagnostics artifact as source of truth.

## Read

Read before editing:

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- fixer-worker-prompt.md from ChatGPT Project Sources
- chatgpt-gh-connector.md from ChatGPT Project Sources
- crates/haze-sync-cli/docs/component-contract.md
- crates/haze-sync-cli/docs/implementation-plan.md
- crates/haze-sync-cli/docs/implementation-log.md
- crates/haze-sync-cli/docs/dependency-map.md
- crates/haze-sync-cli/control/prompt.md
- crates/haze-sync-cli/control/report.md
- relevant current repository code and PR diff

Download and read diagnostics artifact 8202662299. Read summary.md, manifest.json, and every failed-check log.

If the artifact is missing, expired, malformed, or unreadable, report FIX_BLOCKED_BY_LOGS.

## Task

Fix the minimum cause of the CLI-P4 CI failure inside cli scope.

Expected recent changed area: read-only Server/API client boundary, status/adapters commands, and safe output rendering. Use the artifact as source of truth.

## Allowed files

- crates/haze-sync-cli/src/**
- crates/haze-sync-cli/docs/** only if the artifact proves a docs formatting failure
- crates/haze-sync-cli/control/report.md

## Forbidden changes

- Do not add admin mutations.
- Do not add direct DB reads.
- Do not add provider calls.
- Do not change routes in sibling components.
- Do not add token rotation.
- Do not change workflow files.
- Do not change sibling components.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and fixer-worker-prompt.md. Product/source/docs fixer commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-cli/control/report.md.

Use report-template.md. Set REPORT_TYPE to FIX.
