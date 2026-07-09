# W1-STOR-P5 — Normal file flow repository support

Component: storage
Path: crates/haze-sync-storage
Branch: component/storage
PR: #47
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

STOR-P4 implementation and clean-code review are accepted. Component CI evidence for the accepted product-code state is green.

- workflow: Component CI
- workflow_run_id: 29023928350
- run_number: 577
- conclusion: success

The next implementation phase is STOR-P5 from crates/haze-sync-storage/docs/implementation-plan.md.

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

Implement STOR-P5: Normal file flow repository support.

Follow the implementation plan:

- verify content blob metadata insertion/read behavior;
- verify sync object current-path/current-revision behavior;
- verify immutable file revision insertion/read behavior;
- verify operation-log append and changes-page repository behavior;
- verify per-path advisory lock helper use in integration examples/tests;
- provide repository outputs that Server can map to Core/API models.

## Allowed files

- crates/haze-sync-storage/src/repositories/content_blobs/**
- crates/haze-sync-storage/src/repositories/objects/**
- crates/haze-sync-storage/src/repositories/revisions/**
- crates/haze-sync-storage/src/repositories/operation_log/**
- crates/haze-sync-storage/src/locks.rs
- crates/haze-sync-storage/docs/**
- crates/haze-sync-storage/control/report.md

## Non-goals

- No Core upsert decision implementation.
- No HTTP handler.
- No content upload streaming runtime.
- No adapter loop.
- No conflict/delete behavior beyond current flow dependencies.
- No sibling component changes.
- No workflow changes.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and implementation-worker-prompt.md. Product/source/docs commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-storage/control/report.md.

Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION.
