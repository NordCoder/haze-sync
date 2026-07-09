# W1-FIX-GDRIVE-ADAPTER-CI — GDrive adapter CI fix

Component: gdrive-adapter
Path: crates/haze-gdrive-adapter
Branch: component/gdrive-adapter
PR: #50
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review.

## Context

Component CI completed with failure for the current PR head.

- workflow: Component CI
- workflow_run_id: 29003656606
- run_number: 411
- run_attempt: 1
- artifact_id: 8192606873
- artifact_name: ci-diag__component-gdrive-adapter__wf-component-ci__run-29003656606__attempt-1
- known failed checks: rust-fmt, cargo-clippy

Known clippy category from diagnostics triage:

- clippy::derivable_impls for AdapterMode Default implementation

## Read

Read all required sources before editing:

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- crates/haze-gdrive-adapter/docs/component-contract.md
- crates/haze-gdrive-adapter/docs/implementation-plan.md
- crates/haze-gdrive-adapter/docs/dependency-map.md
- crates/haze-gdrive-adapter/control/prompt.md
- crates/haze-gdrive-adapter/control/report.md
- current PR diff and relevant current repository code

Use the GitHub connector to download and read the diagnostics artifact listed above.

Read inside the artifact:

- ci-diagnostics/summary.md
- ci-diagnostics/manifest.json
- every log listed in failed_checks

Do not ask Orchestrator to paste raw logs. If the artifact is missing, expired, or malformed, report FIX_BLOCKED_BY_LOGS.

## Task

Fix the minimum cause of the CI failure.

Expected scope:

- rustfmt-equivalent formatting in gdrive-adapter-owned Rust files;
- minimal behavior-preserving clippy fix for the AdapterMode Default implementation.

## Allowed files

- crates/haze-gdrive-adapter/**/*.rs
- crates/haze-gdrive-adapter/control/report.md

## Forbidden changes

- Do not change runtime behavior except where required by the clippy-equivalent Default derivation.
- Do not change component docs or contracts.
- Do not change workflow files.
- Do not change sibling components.
- Do not delete tests.
- Do not add live Google Drive calls or credential behavior.

If diagnostics reveal a broader issue, fix it only if it is inside gdrive-adapter scope and required for CI. Otherwise report the appropriate blocker.

## Checks

Run or observe the failing checks again if possible through available tooling.

If you cannot run shell commands, say so in the report. Do not claim checks passed unless you actually ran them or observed CI metadata.

## Report

Write only the report to crates/haze-gdrive-adapter/control/report.md.

Use report-template.md.

Set REPORT_TYPE to FIX.

Use one of:

- FIX_COMPLETE
- FIX_NEEDS_MORE
- FIX_BLOCKED_BY_LOGS
- FIX_BLOCKED_BY_CONTRACT
- FIX_BLOCKED_BY_TOOLING
