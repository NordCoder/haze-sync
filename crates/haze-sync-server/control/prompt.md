# W1-SRV-P5 — Conflict, delete, and idempotency route fan-in hardening

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

SRV-P4 implementation and clean-code review are accepted. Component CI is green for the accepted product-code state.

- workflow: Component CI
- workflow_run_id: 29009520199
- run_number: 504
- conclusion: success

The next implementation phase is SRV-P5 from crates/haze-sync-server/docs/implementation-plan.md.

## Read

Read before editing:

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- implementation-worker-prompt.md from ChatGPT Project Sources
- chatgpt-gh-connector.md from ChatGPT Project Sources
- crates/haze-sync-server/docs/component-contract.md
- crates/haze-sync-server/docs/implementation-plan.md
- crates/haze-sync-server/docs/implementation-log.md
- crates/haze-sync-server/docs/dependency-map.md
- crates/haze-sync-server/control/prompt.md
- crates/haze-sync-server/control/report.md
- relevant current repository code and PR diff

Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Implement SRV-P5: Conflict, delete, and idempotency route fan-in hardening.

Follow the implementation plan:

- preserve conflict-saved outcomes without overwriting current revision;
- wire conflict list/status filtering through accepted Storage repositories and API DTOs where available;
- implement only accepted conflict resolution actions and return honest partial/not-implemented behavior where required;
- ensure DELETE requires base/null-base and idempotency metadata;
- use path locks for delete mutations where scoped;
- evaluate Core delete guard before tombstone persistence;
- persist tombstones, current-state clearing, operation-log entries, and idempotency responses atomically where accepted dependencies support it;
- test idempotent replay and same-key different-request conflicts where practical.

## Dependency guard

If Core/API/Storage surfaces are insufficient without cross-component edits, do not invent sibling behavior. Report BLOCKED_BY_DEPENDENCY or BLOCKED_BY_CONTRACT with the exact missing dependency.

## Allowed files

- crates/haze-sync-server/src/routes/conflicts/**
- crates/haze-sync-server/src/routes/delete/**
- crates/haze-sync-server/src/routes/v1/**
- crates/haze-sync-server/src/http/**
- crates/haze-sync-server/docs/**
- crates/haze-sync-server/control/report.md

## Non-goals

- No hard delete.
- No retention cleanup job.
- No provider/worktree trash side effects.
- No Web UI conflict center.
- No policy expansion such as latest-wins or incoming-wins.
- No sibling component changes.
- No workflow changes.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and implementation-worker-prompt.md. Product, test, dependency, contract, workflow, and implementation commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-server/control/report.md.

Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION.
