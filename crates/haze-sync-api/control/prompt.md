# W1-API-P4 — File and changes route-helper contract hardening

Component: api
Path: crates/haze-sync-api
Branch: component/api
PR: #44
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

API-P3 implementation, CI fixer, and clean-code review are accepted. Component CI evidence for the accepted source state is green.

- workflow: Component CI
- workflow_run_id: 29011290632
- run_number: 532
- conclusion: success

The next implementation phase is API-P4 from crates/haze-sync-api/docs/implementation-plan.md.

## Read

Read before editing:

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- implementation-worker-prompt.md from ChatGPT Project Sources
- chatgpt-gh-connector.md from ChatGPT Project Sources
- crates/haze-sync-api/docs/component-contract.md
- crates/haze-sync-api/docs/implementation-plan.md
- crates/haze-sync-api/docs/implementation-log.md
- crates/haze-sync-api/docs/dependency-map.md
- crates/haze-sync-api/control/prompt.md
- crates/haze-sync-api/control/report.md
- relevant current repository code and PR diff

Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Implement API-P4: File and changes route-helper contract hardening.

Follow the implementation plan:

- test path parsing through VaultPath;
- test upload metadata extraction for path, base revision, content hash, idempotency key, adapter principal requirement, and body length metadata if represented;
- test mapping of accepted, same-content, conflict-saved, hash-mismatch, and stale outcomes into public response metadata;
- test changes query since/limit bounds and response page metadata;
- ensure helpers remain passive and do not call Core or Storage.

## Allowed files

- crates/haze-sync-api/src/routes/files/**
- crates/haze-sync-api/src/routes/changes/**
- crates/haze-sync-api/src/dto/files/**
- crates/haze-sync-api/src/dto/changes/**
- crates/haze-sync-api/docs/**
- crates/haze-sync-api/control/report.md

## Non-goals

- No Axum handler implementation.
- No object-store reads or writes.
- No operation-log queries.
- No content streaming.
- No background cursor updates.
- No sibling component changes.
- No workflow changes.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and implementation-worker-prompt.md. Product, test, dependency, contract, workflow, and implementation commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-api/control/report.md.

Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION.
