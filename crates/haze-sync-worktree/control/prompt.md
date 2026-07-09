# W1-FIX-WORKTREE-WT-P3-CI — Worktree WT-P3 CI fix

Component: worktree
Path: crates/haze-sync-worktree
Branch: component/worktree
PR: #49
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

WT-P3 implementation completed with SELF_ACCEPT_PENDING_CI. Product-code CI completed red.

- workflow: Component CI
- workflow_run_id: 29009639256
- run_number: 512
- run_attempt: 1
- artifact_id: 8195028185
- artifact_name: ci-diag__component-worktree__wf-component-ci__run-29009639256__attempt-1
- known failed checks: rust-fmt, cargo-clippy

## Read

Read before editing:

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- fixer-worker-prompt.md from ChatGPT Project Sources
- chatgpt-gh-connector.md from ChatGPT Project Sources
- crates/haze-sync-worktree/docs/component-contract.md
- crates/haze-sync-worktree/docs/implementation-plan.md
- crates/haze-sync-worktree/docs/implementation-log.md
- crates/haze-sync-worktree/docs/dependency-map.md
- crates/haze-sync-worktree/control/prompt.md
- crates/haze-sync-worktree/control/report.md
- relevant current repository code and PR diff

Use the GitHub connector to download and read the diagnostics artifact listed above.

Read summary.md, manifest.json, and every log listed in failed_checks.

If the artifact is missing, expired, malformed, or unreadable, report FIX_BLOCKED_BY_LOGS.

## Task

Fix the minimum cause of the WT-P3 CI failure inside worktree scope.

Expected scope from triage:

- rustfmt formatting in crates/haze-sync-worktree/src/scanner.rs;
- clippy needless range loop in scanner SHA-256 schedule code.

## Allowed files

- crates/haze-sync-worktree/src/**
- crates/haze-sync-worktree/control/report.md

## Forbidden changes

- Do not change Worktree behavior except as required by formatting or clippy-equivalent cleanup.
- Do not add dependencies.
- Do not change docs or contracts.
- Do not change workflow files.
- Do not change sibling components.
- Do not delete tests.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and fixer-worker-prompt.md. Product/source fixer commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-worktree/control/report.md.

Use report-template.md. Set REPORT_TYPE to FIX.
