# W1-CMM-P3 — VaultPath contract hardening

Component: common
Path: crates/haze-sync-common
Branch: component/common
PR: #46
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

CMM-P2 and its CI fixer loop are complete. Current Component CI is green for the current PR head.

- workflow: Component CI
- workflow_run_id: 29006921208
- run_number: 459
- conclusion: success

The next implementation phase is CMM-P3 from crates/haze-sync-common/docs/implementation-plan.md.

## Read

Read before editing:

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- implementation-worker-prompt.md from ChatGPT Project Sources
- chatgpt-gh-connector.md from ChatGPT Project Sources
- crates/haze-sync-common/docs/component-contract.md
- crates/haze-sync-common/docs/implementation-plan.md
- crates/haze-sync-common/docs/implementation-log.md
- crates/haze-sync-common/docs/dependency-map.md
- crates/haze-sync-common/control/prompt.md
- crates/haze-sync-common/control/report.md
- relevant current repository code and PR diff

Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Implement CMM-P3: VaultPath contract hardening.

Follow the implementation plan:

- expand reserved path and invalid path test matrix;
- verify encoded/traversal/null-byte-like inputs cannot bypass validation;
- verify Windows and Unix escape forms are rejected;
- decide whether reserved conflict materialization paths are syncable or reserved;
- document the decision if clarified;
- avoid filesystem-specific behavior beyond string/path validation.

## Allowed files

- crates/haze-sync-common/src/path.rs
- crates/haze-sync-common/src/error.rs
- crates/haze-sync-common/docs/**
- crates/haze-sync-common/control/report.md

## Non-goals

- No runtime behavior.
- No sibling crate edits.
- No provider-specific path exceptions.
- No Core/API/Storage/Server behavior.
- No workflow changes.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and the relevant Project Source worker prompt. Product, test, dependency, contract, workflow, and implementation commits must not skip CI.

## Report

Write only the report to crates/haze-sync-common/control/report.md.

Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION.
