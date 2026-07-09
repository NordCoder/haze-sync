# W1-STOR-P3 — Storage object-store hardening

Component: storage
Path: crates/haze-sync-storage
Branch: component/storage
PR: #47
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

STOR-P2 and its CI fixer loop are complete. Current Component CI is green for the current PR head.

- workflow: Component CI
- workflow_run_id: 29006885420
- run_number: 455
- conclusion: success

The next implementation phase is STOR-P3 from crates/haze-sync-storage/docs/implementation-plan.md.

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

Implement STOR-P3: Object-store hardening.

Follow the implementation plan:

- preserve hash-addressed object path behavior;
- harden write, read, and stat verification behavior;
- add practical tests for duplicate writes, missing blobs, hash mismatch handling, unexpected object-store entries, and path-free error formatting;
- review temporary-write and cleanup assumptions;
- document object-store root ownership and deployment expectations where useful.

## Allowed files

- crates/haze-sync-storage/src/object_store/**
- crates/haze-sync-storage/docs/**
- crates/haze-sync-storage/control/report.md

## Non-goals

- No object-store HTTP API.
- No garbage collection.
- No retention cleanup.
- No provider blob storage.
- No encryption layer.
- No sibling component changes.
- No workflow changes.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and the relevant Project Source worker prompt. Product, test, dependency, contract, workflow, and implementation commits must not skip CI.

## Report

Write only the report to crates/haze-sync-storage/control/report.md.

Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION.
