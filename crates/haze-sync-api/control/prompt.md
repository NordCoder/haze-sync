# W1-FIX-API-P3-CI — API API-P3 CI fix

Component: api
Path: crates/haze-sync-api
Branch: component/api
PR: #44
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

API-P3 implementation completed with SELF_ACCEPT_PENDING_CI. Product-code CI completed red.

- workflow: Component CI
- workflow_run_id: 29009458012
- run_number: 494
- run_attempt: 1
- artifact_id: 8194965506
- artifact_name: ci-diag__component-api__wf-component-ci__run-29009458012__attempt-1
- known failed check: rust-fmt

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

Use the GitHub connector to download and read the diagnostics artifact listed above.

Read summary.md, manifest.json, and every log listed in failed_checks.

If the artifact is missing, expired, malformed, or unreadable, report FIX_BLOCKED_BY_LOGS.

## Task

Fix the minimum cause of the API-P3 CI failure inside api scope.

Expected scope from triage: rustfmt formatting in crates/haze-sync-api/src/contracts/headers.rs.

## Allowed files

- crates/haze-sync-api/src/**
- crates/haze-sync-api/control/report.md

## Forbidden changes

- Do not change behavior.
- Do not change public API semantics.
- Do not change docs or contracts.
- Do not change workflow files.
- Do not change sibling components.
- Do not delete tests.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and fixer-worker-prompt.md. Product/source fixer commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-api/control/report.md.

Use report-template.md. Set REPORT_TYPE to FIX.
