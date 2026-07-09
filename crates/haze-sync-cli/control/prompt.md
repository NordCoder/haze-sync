# W1-FIX-CLI-P3-CI — CLI CLI-P3 CI fix

Component: cli
Path: crates/haze-sync-cli
Branch: component/cli
PR: #48
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

CLI-P3 implementation completed with SELF_ACCEPT_PENDING_CI. Product-code CI completed red.

- workflow: Component CI
- workflow_run_id: 29009634703
- run_number: 511
- run_attempt: 1
- artifact_id: 8195027380
- artifact_name: ci-diag__component-cli__wf-component-ci__run-29009634703__attempt-1
- known failed checks: rust-fmt, cargo-clippy

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

Use the GitHub connector to download and read the diagnostics artifact listed above.

Read summary.md, manifest.json, and every log listed in failed_checks.

If the artifact is missing, expired, malformed, or unreadable, report FIX_BLOCKED_BY_LOGS.

## Task

Fix the minimum cause of the CLI-P3 CI failure inside cli scope.

Expected scope from triage:

- rustfmt formatting in crates/haze-sync-cli/src/config.rs;
- clippy derivable_impls for TokenSource and CliConfig defaults.

## Allowed files

- crates/haze-sync-cli/src/**
- crates/haze-sync-cli/control/report.md

## Forbidden changes

- Do not change runtime behavior except behavior-preserving Default derivation if required by clippy.
- Do not add config IO.
- Do not add server calls.
- Do not change docs or contracts.
- Do not change workflow files.
- Do not change sibling components.
- Do not delete tests.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and fixer-worker-prompt.md. Product/source fixer commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-cli/control/report.md.

Use report-template.md. Set REPORT_TYPE to FIX.
