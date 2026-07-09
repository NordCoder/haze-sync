# W1-STOR-P4 — Repository validation and safe error boundary hardening

Component: storage
Path: crates/haze-sync-storage
Branch: component/storage
PR: #47
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

STOR-P3 implementation and clean-code review are accepted. Component CI is green for the accepted product-code state.

- workflow: Component CI
- workflow_run_id: 29009545498
- run_number: 506
- conclusion: success

The next implementation phase is STOR-P4 from crates/haze-sync-storage/docs/implementation-plan.md.

## Read

Read before editing:

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- implementation-worker-prompt.md from ChatGPT Project Sources
- chatgpt-gh-connector.md from ChatGPT Project Sources
- crates/haze-sync-storage/docs/component-contract.md
- crates/haze-sync-storage/docs/implementation-plan.md
- crates/haze-sync-storage/docs/implementation-log.md
- crates/haze-sync-storage/docs/dependency-map.md
- crates/haze-sync-storage/control/prompt.md
- crates/haze-sync-storage/control/report.md
- relevant current repository code and PR diff

Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Implement STOR-P4: Repository validation and safe error boundary hardening.

Follow the implementation plan:

- audit repository modules for caller-owned executor/transaction usage;
- test validation helpers for sequences, limits, size conversions, status parsing, operation kind parsing, and cursor regression where applicable;
- verify mapped repository errors do not expose raw SQLx/database internals;
- keep repository errors stable and safe;
- document transaction-sensitive repository functions where useful.

## Allowed files

- crates/haze-sync-storage/src/repositories/**
- crates/haze-sync-storage/docs/**
- crates/haze-sync-storage/control/report.md

## Non-goals

- No DB pool creation.
- No server route wiring.
- No Core policy decisions.
- No public HTTP status mapping.
- No provider behavior.
- No sibling component changes.
- No workflow changes.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and implementation-worker-prompt.md. Product, test, dependency, contract, workflow, and implementation commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-storage/control/report.md.

Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION.
