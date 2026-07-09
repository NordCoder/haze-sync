# W1-FIX-CMM-P4-CI — Common CMM-P4 CI fix

Component: common
Path: crates/haze-sync-common
Branch: component/common
PR: #46
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

CMM-P4 implementation completed with SELF_ACCEPT_PENDING_CI. The code/docs commit failed Component CI.

- workflow: Component CI
- workflow_run_id: 29024044709
- run_number: 586
- run_attempt: 1
- artifact_id: 8200732256
- artifact_name: ci-diag__component-common__wf-component-ci__run-29024044709__attempt-1
- artifact_expires_at: 2026-07-10T14:08:44Z

The workflow step summary shows the check wrapper steps completed and the diagnostics finalizer failed. Read the diagnostics artifact; do not infer the failed underlying check only from step conclusions.

## Read

Read before editing:

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- fixer-worker-prompt.md from ChatGPT Project Sources
- chatgpt-gh-connector.md from ChatGPT Project Sources
- crates/haze-sync-common/docs/component-contract.md
- crates/haze-sync-common/docs/implementation-plan.md
- crates/haze-sync-common/docs/implementation-log.md
- crates/haze-sync-common/docs/dependency-map.md
- crates/haze-sync-common/control/prompt.md
- crates/haze-sync-common/control/report.md
- relevant current repository code and PR diff

Use the GitHub connector to download and read the diagnostics artifact listed above.

Read summary.md, manifest.json, and every log listed in failed_checks.

If the artifact is missing, expired, malformed, or unreadable, report FIX_BLOCKED_BY_LOGS.

## Task

Fix the minimum cause of the CMM-P4 CI failure inside common scope.

Expected recent changed areas: identifier/hash tests and docs. Use the diagnostics artifact as source of truth.

## Allowed files

- crates/haze-sync-common/src/ids.rs
- crates/haze-sync-common/src/hash.rs
- crates/haze-sync-common/docs/**
- crates/haze-sync-common/control/report.md

## Forbidden changes

- Do not change storage behavior.
- Do not change Core behavior.
- Do not add runtime/provider behavior.
- Do not add sibling component changes.
- Do not change workflow files.
- Do not remove test coverage unless the artifact proves the test is invalid and replacement coverage is added.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and fixer-worker-prompt.md. Product/source/docs fixer commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-common/control/report.md.

Use report-template.md. Set REPORT_TYPE to FIX.
