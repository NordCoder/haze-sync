# W1-FIX-SRV-P5-CI — Server SRV-P5 CI fix

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

SRV-P5 implementation completed with SELF_ACCEPT_PENDING_CI. The code-bearing commit failed Component CI.

- workflow: Component CI
- workflow_run_id: 29023987460
- run_number: 581
- run_attempt: 1
- artifact_id: 8200704446
- artifact_name: ci-diag__component-server__wf-component-ci__run-29023987460__attempt-1
- artifact_expires_at: 2026-07-10T14:07:51Z

The workflow step summary shows the check wrapper steps completed and the diagnostics finalizer failed. Read the diagnostics artifact; do not infer the failed underlying check only from step conclusions.

## Read

Read before editing:

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- fixer-worker-prompt.md from ChatGPT Project Sources
- chatgpt-gh-connector.md from ChatGPT Project Sources
- crates/haze-sync-server/docs/component-contract.md
- crates/haze-sync-server/docs/implementation-plan.md
- crates/haze-sync-server/docs/implementation-log.md
- crates/haze-sync-server/docs/dependency-map.md
- crates/haze-sync-server/control/prompt.md
- crates/haze-sync-server/control/report.md
- relevant current repository code and PR diff

Use the GitHub connector to download and read the diagnostics artifact listed above.

Read summary.md, manifest.json, and every log listed in failed_checks.

If the artifact is missing, expired, malformed, or unreadable, report FIX_BLOCKED_BY_LOGS.

## Task

Fix the minimum cause of the SRV-P5 CI failure inside server scope.

Expected recent changed source area: conflict route/storage dependency behavior and tests. Use the diagnostics artifact as source of truth.

## Allowed files

- crates/haze-sync-server/src/routes/conflicts.rs
- crates/haze-sync-server/src/routes/conflicts/**
- crates/haze-sync-server/src/routes/conflicts/tests.rs
- crates/haze-sync-server/docs/implementation-log.md only if the artifact proves a formatting/doc failure there
- crates/haze-sync-server/control/report.md

## Forbidden changes

- Do not change API/Core/Storage contracts.
- Do not add provider/worktree behavior.
- Do not add hard delete or new conflict policy.
- Do not change workflow files.
- Do not change sibling components.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and fixer-worker-prompt.md. Product/source fixer commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-server/control/report.md.

Use report-template.md. Set REPORT_TYPE to FIX.
