# W1-CMM-P5 — Adapter mode, role, and security primitive hardening

Component: common
Path: crates/haze-sync-common
Branch: component/common
PR: #46
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

CMM-P4 implementation, CI fixer, and clean-code review are accepted. Component CI evidence for the accepted source state is green.

- workflow: Component CI
- workflow_run_id: 29028045014
- conclusion: success

The next implementation phase is CMM-P5 from crates/haze-sync-common/docs/implementation-plan.md.

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

Implement CMM-P5: Adapter mode, role, and security primitive hardening.

Follow the implementation plan:

- verify adapter modes match system rollout semantics;
- add declarative helpers only when they remain policy-free;
- ensure role/mode serde wire values are stable;
- audit whether ReadonlyAgent should remain a V1 role or be renamed/removed by contract change;
- verify SecretString does not serialize or leak through formatting;
- decide whether common needs additional redaction wrappers for token hashes or public-safe labels.

## Allowed files

- crates/haze-sync-common/src/adapter.rs
- crates/haze-sync-common/src/security/**
- crates/haze-sync-common/src/error.rs
- crates/haze-sync-common/docs/**
- crates/haze-sync-common/control/report.md

## Non-goals

- No storage behavior.
- No Core behavior.
- No runtime/provider behavior.
- No sibling component changes.
- No workflow changes.
- No permission enforcement in common.
- No token verification, hashing, loading, persistence, or generation.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and implementation-worker-prompt.md. Product/source/docs commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-common/control/report.md.

Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION.
