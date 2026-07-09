# W1-CMM-P4 — Identifier and hash contract hardening

Component: common
Path: crates/haze-sync-common
Branch: component/common
PR: #46
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

CMM-P3 implementation and clean-code review are accepted. Component CI is green for the accepted code/docs state.

- workflow: Component CI
- workflow_run_id: 29009442790
- run_number: 492
- conclusion: success

The next implementation phase is CMM-P4 from crates/haze-sync-common/docs/implementation-plan.md.

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

Implement CMM-P4: Identifier and hash contract hardening.

Follow the implementation plan:

- verify ID max length and allowed character set across current shared identifiers;
- ensure typed IDs reject missing or wrong prefixes;
- ensure canonical hash output is always sha256 plus lowercase hex in the accepted wire form;
- decide whether additional IDs belong in common, such as BlobId, CursorId, or TombstoneId;
- keep ID generation outside common unless explicitly scoped by accepted contract.

## Allowed files

- crates/haze-sync-common/src/ids.rs
- crates/haze-sync-common/src/hash.rs
- crates/haze-sync-common/src/error.rs
- crates/haze-sync-common/docs/**
- crates/haze-sync-common/control/report.md

## Non-goals

- No storage behavior.
- No Core behavior.
- No runtime behavior.
- No provider-specific behavior.
- No sibling component changes.
- No workflow changes.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and implementation-worker-prompt.md. Product, test, dependency, contract, workflow, and implementation commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-common/control/report.md.

Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION.
