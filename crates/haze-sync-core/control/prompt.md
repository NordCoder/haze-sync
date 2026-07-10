# W1-CORE-P7 — Passive doctor and safety-report models

Component: core
Path: crates/haze-sync-core
Branch: component/core
PR: #43
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

CORE-P6 implementation, clean-code review, and artifact-based CI correction are accepted. Final source CI is green.

- code_bearing_sha: 90e2e03cfe41e36a072bdb87265eddc517f6fec6
- workflow: Component CI
- workflow_run_id: 29093462297
- run_number: 1218
- conclusion: success

The next implementation phase is CORE-P7 from crates/haze-sync-core/docs/implementation-plan.md.

## Read

Read implementation-manifest.md, report-template.md, implementation-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, current doctor models/tests, accepted Server/CLI diagnostic contracts, and PR diff. Do not read diagnostics artifacts unless a future fixer prompt explicitly instructs it.

## Task

Implement CORE-P7: Passive doctor and safety-report models.

- audit doctor statuses and aggregate summary rules;
- model missing-blob, DB, object-store, cursor, mapping, token-sanity, and worktree-drift summaries where accepted;
- represent passed, failed, skipped, not-run, and placeholder states honestly;
- require all public details to be pre-redacted and safe;
- add focused tests for aggregation, status precedence, skipped/not-run semantics, safe serialization, and redaction boundaries.

## Allowed files

- crates/haze-sync-core/src/doctor/**
- crates/haze-sync-core/docs/**
- crates/haze-sync-core/control/report.md

## Non-goals

No live DB connection, filesystem/object-store probing, provider/OAuth validation, CLI command implementation, repair behavior, mutation, workflow/dependency changes, or sibling component changes.

## CI trigger policy

Product/source/test/docs commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-sync-core/control/report.md. Use report-template.md, REPORT_TYPE IMPLEMENTATION, phase_id CORE-P7.
