# W1-CORE-P6 — Idempotency, operation-log, and cursor primitive hardening

Component: core
Path: crates/haze-sync-core
Branch: component/core
PR: #43
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

CORE-P5 implementation, CI fixer, and clean-code review are accepted. Final code/docs CI is green.

- code_bearing_sha: 657380f82dcae2e929431dc081793b7249a3bf90
- workflow: Component CI
- workflow_run_id: 29084444310
- run_number: 1049
- conclusion: success

The next implementation phase is CORE-P6 from crates/haze-sync-core/docs/implementation-plan.md.

## Read

Read implementation-manifest.md, report-template.md, implementation-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, current idempotency/operation-log source and tests, accepted Storage/API boundaries, and PR diff. Do not read diagnostics artifacts unless a future fixer prompt explicitly instructs it.

## Task

Implement CORE-P6: Idempotency, operation-log, and cursor primitive hardening.

- harden idempotency-key validation and redaction boundaries;
- test deterministic request fingerprinting and same/different request classification;
- document and validate the safe replay snapshot shape and its limitations;
- harden operation-kind parsing and serialization;
- validate changes-query limit boundaries;
- define and test cursor monotonicity, advancement, and regression rejection;
- document what durable Storage/Server components must persist without adding persistence to Core.

## Allowed files

- crates/haze-sync-core/src/idempotency/**
- crates/haze-sync-core/src/operation_log/**
- crates/haze-sync-core/docs/**
- crates/haze-sync-core/control/report.md

## Non-goals

No durable idempotency repository, database append implementation, HTTP replay middleware, adapter polling loop, Storage/API/Server edits, workflow/dependency changes, or sibling-component changes.

## CI trigger policy

Product/source/docs commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-sync-core/control/report.md. Use report-template.md, REPORT_TYPE IMPLEMENTATION, phase_id CORE-P6.
