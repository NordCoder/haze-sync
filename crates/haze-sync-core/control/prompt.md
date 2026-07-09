# W1-FIX-CORE-CI — Core CI fix

Component: core
Path: crates/haze-sync-core
Branch: component/core
PR: #43
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review.

## Context

Component CI completed with failure for the current PR head.

- workflow: Component CI
- workflow_run_id: 29003582305
- run_number: 404
- run_attempt: 1
- artifact_id: 8192586969
- artifact_name: ci-diag__component-core__wf-component-ci__run-29003582305__attempt-1
- known failed check: rust-fmt

## Read

Read all required sources before editing:

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- crates/haze-sync-core/docs/component-contract.md
- crates/haze-sync-core/docs/implementation-plan.md
- crates/haze-sync-core/docs/dependency-map.md
- crates/haze-sync-core/control/prompt.md
- crates/haze-sync-core/control/report.md
- current PR diff and relevant current repository code

Use the GitHub connector to download and read the diagnostics artifact listed above.

Read inside the artifact:

- ci-diagnostics/summary.md
- ci-diagnostics/manifest.json
- every log listed in failed_checks

Do not ask Orchestrator to paste raw logs. If the artifact is missing, expired, or malformed, report FIX_BLOCKED_BY_LOGS.

## Task

Fix the minimum cause of the CI failure.

Expected scope is rustfmt-equivalent formatting only in core-owned Rust files.

## Allowed files

- crates/haze-sync-core/**/*.rs
- crates/haze-sync-core/control/report.md

## Forbidden changes

- Do not change behavior.
- Do not change public API semantics.
- Do not change component docs or contracts.
- Do not change workflow files.
- Do not change sibling components.
- Do not delete tests.

If the diagnostics reveal a non-formatting issue, fix it only if it is inside core scope and required for CI. Otherwise report the appropriate blocker.

## Checks

Run or observe the failing check again if possible through available tooling.

If you cannot run shell commands, say so in the report. Do not claim checks passed unless you actually ran them or observed CI metadata.

## Report

Write only the report to crates/haze-sync-core/control/report.md.

Use report-template.md.

Set REPORT_TYPE to FIX.

Use one of:

- FIX_COMPLETE
- FIX_NEEDS_MORE
- FIX_BLOCKED_BY_LOGS
- FIX_BLOCKED_BY_CONTRACT
- FIX_BLOCKED_BY_TOOLING
