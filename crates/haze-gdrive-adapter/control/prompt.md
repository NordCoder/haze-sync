# W1-FIX-GDA-P3C-CI — GDrive adapter clean-code CI fix

Component: gdrive-adapter
Path: crates/haze-gdrive-adapter
Branch: component/gdrive-adapter
PR: #50
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

GDA-P3C clean-code review found and fixed a fake-provider correctness issue, then triggered Component CI. That code-bearing clean-code commit failed CI.

- workflow: Component CI
- workflow_run_id: 29023789542
- run_number: 560
- run_attempt: 1
- artifact_id: 8200613102
- artifact_name: ci-diag__component-gdrive-adapter__wf-component-ci__run-29023789542__attempt-1
- artifact_expires_at: 2026-07-10T14:04:52Z

The workflow step summary shows the check wrapper steps completed and the diagnostics finalizer failed. Read the diagnostics artifact; do not infer the failed underlying check only from step conclusions.

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

Use the GitHub connector to download and read the diagnostics artifact listed above.

Read summary.md, manifest.json, and every log listed in failed_checks.

If the artifact is missing, expired, malformed, or unreadable, report FIX_BLOCKED_BY_LOGS.

## Task

Fix the minimum cause of the GDA-P3C CI failure inside gdrive-adapter scope.

Expected recent changed source area: crates/haze-gdrive-adapter/src/drive.rs around fake-provider upload/list behavior and related tests. Use the artifact as source of truth.

## Allowed files

- crates/haze-gdrive-adapter/src/**
- crates/haze-gdrive-adapter/control/report.md

## Forbidden changes

- Do not add real Google SDK wiring.
- Do not add live provider calls or credential behavior.
- Do not add Core/API writes.
- Do not change docs or contracts unless the artifact proves a doc-only formatting failure.
- Do not change workflow files.
- Do not change sibling components.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and fixer-worker-prompt.md. Product/source fixer commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-gdrive-adapter/control/report.md.

Use report-template.md. Set REPORT_TYPE to FIX.
