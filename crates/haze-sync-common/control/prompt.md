# W1-CMM-P5-RERUN — Adapter mode, role, and security primitive hardening

Component: common
Path: crates/haze-sync-common
Branch: component/common
PR: #46
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Rerun guard

This is an explicit refreshed active prompt. The current report before this prompt was for CMM-P4C, not CMM-P5. Therefore CMM-P5-RERUN is not already complete.

## Context

CMM-P4 implementation, CI fixer, and clean-code review are accepted. Component CI evidence for the accepted source state is green.

- workflow: Component CI
- workflow_run_id: 29028045014
- conclusion: success

The next implementation phase is CMM-P5 from crates/haze-sync-common/docs/implementation-plan.md.

## Read

Read implementation-manifest.md, report-template.md, implementation-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, relevant code and PR diff. Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Implement CMM-P5: Adapter mode, role, and security primitive hardening. Verify adapter modes, policy-free declarative helpers, role/mode serde wire values, ReadonlyAgent V1 decision, SecretString non-leak behavior, and whether common needs additional redaction wrappers.

## Allowed files

- crates/haze-sync-common/src/adapter.rs
- crates/haze-sync-common/src/security/**
- crates/haze-sync-common/src/error.rs
- crates/haze-sync-common/docs/**
- crates/haze-sync-common/control/report.md

## Non-goals

No storage behavior, Core behavior, runtime/provider behavior, sibling component changes, workflow changes, permission enforcement in common, or token verification/hash/load/persist/generation behavior.

## CI trigger policy

Product/source/docs commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-common/control/report.md. Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION and phase_id to CMM-P5-RERUN.
