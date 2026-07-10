# W1-STOR-P9 — Storage test-support and integration harness hardening

Component: storage
Path: crates/haze-sync-storage
Branch: component/storage
PR: #47
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

STOR-P8 implementation, formatter correction, and clean-code review are accepted. Final source/test CI is green.

- code_bearing_sha: 1a53e555a933ee2c81ce4843be4506b2ad713f17
- workflow: Component CI
- workflow_run_id: 29113692274
- run_number: 1453
- conclusion: success

The next implementation phase is STOR-P9 from crates/haze-sync-storage/docs/implementation-plan.md.

## Read

Read implementation-manifest.md, report-template.md, implementation-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, current `test-support` feature gates and helpers, storage repository tests, CI documentation, and PR diff. Do not read diagnostics artifacts unless a future fixer prompt explicitly instructs it.

## Task

Implement STOR-P9: Storage test-support and integration harness hardening.

- audit the `test-support` feature and ensure production builds do not depend on test-only helpers;
- provide focused helpers for safe test database setup, isolation, cleanup, and deterministic fixture state where accepted;
- require an explicitly supplied test-only database URL and reject production-looking or missing configuration safely;
- make unavailable-database behavior explicit: skip only when the contract says not-run, never convert setup failure into a pass;
- avoid provider credentials, external vaults, production secrets, or live services;
- document exact commands and environment requirements for feature-gated Storage integration tests;
- add unit tests for configuration validation and helper behavior that do not require a live database.

## Allowed files

- crates/haze-sync-storage/src/test_support/**
- crates/haze-sync-storage/docs/**
- crates/haze-sync-storage/control/report.md

Only if directly required to keep the test-support feature correctly isolated:

- crates/haze-sync-storage/src/lib.rs
- crates/haze-sync-storage/Cargo.toml

## Non-goals

No production migration deployment, provider credentials, external vault access, product repository semantics, schema expansion, CI workflow edits, Server fan-in, workflow/dependency changes, or sibling component changes.

## CI trigger policy

Product/source/test/docs commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-sync-storage/control/report.md. Use report-template.md, REPORT_TYPE IMPLEMENTATION, phase_id STOR-P9.
