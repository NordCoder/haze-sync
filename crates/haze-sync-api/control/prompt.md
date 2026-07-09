# W1-FIX-API-CI — API CI fix

Component: api
Path: crates/haze-sync-api
Branch: component/api
PR: #44
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review.

## Context

Component CI completed with failure for the current PR head.

- workflow: Component CI
- workflow_run_id: 29003592920
- run_number: 405
- run_attempt: 1
- artifact_id: 8192587540
- artifact_name: ci-diag__component-api__wf-component-ci__run-29003592920__attempt-1
- known failed check: rust-fmt

## Read

Read all required sources before editing:

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- crates/haze-sync-api/docs/component-contract.md
- crates/haze-sync-api/docs/implementation-plan.md
- crates/haze-sync-api/docs/dependency-map.md
- crates/haze-sync-api/control/prompt.md
- crates/haze-sync-api/control/report.md
- current PR diff and relevant current repository code

Use the GitHub connector to download and read the diagnostics artifact listed above.

Read inside the artifact:

- ci-diagnostics/summary.md
- ci-diagnostics/manifest.json
- every log listed in failed_checks

Do not ask Orchestrator to paste raw logs. If the artifact is missing, expired, or malformed, report FIX_BLOCKED_BY_LOGS.

## Task

Fix the minimum cause of the CI failure.

Expected scope is rustfmt-equivalent formatting only in api-owned Rust files.

## Allowed files

- crates/haze-sync-api/**/*.rs
- crates/haze-sync-api/control/report.md

## Forbidden changes

- Do not change behavior.
- Do not change public API semantics.
- Do not change component docs or contracts.
- Do not change workflow files.
- Do not change sibling components.
- Do not delete tests.

If the diagnostics reveal a non-formatting issue, fix it only if it is inside api scope and required for CI. Otherwise report the appropriate blocker.

## Checks

Run or observe the failing check again if possible through available tooling.

If you cannot run shell commands, say so in the report. Do not claim checks passed unless you actually ran them or observed CI metadata.

## Report

Write only the report to crates/haze-sync-api/control/report.md.

Use report-template.md.

Set REPORT_TYPE to FIX.

Use one of:

- FIX_COMPLETE
- FIX_NEEDS_MORE
- FIX_BLOCKED_BY_LOGS
- FIX_BLOCKED_BY_CONTRACT
- FIX_BLOCKED_BY_TOOLING
