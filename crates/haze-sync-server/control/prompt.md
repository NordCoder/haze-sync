# W1-SRV-P4 — File PUT/GET/changes transaction fan-in hardening

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

SRV-P3 implementation and clean-code review are complete. Current Component CI is green for the current PR head.

- workflow: Component CI
- workflow_run_id: 29006911851
- run_number: 458
- conclusion: success

The next implementation phase is SRV-P4 from crates/haze-sync-server/docs/implementation-plan.md.

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

Implement SRV-P4: File PUT/GET/changes transaction fan-in hardening.

Follow the implementation plan:

- ensure PUT uses API header/path parsers and Core revision outcomes;
- coordinate object-store write, storage metadata, current revision, file revision, operation log, and idempotency response where the existing component contracts allow it;
- ensure same-content and hash-mismatch outcomes map to safe API responses;
- implement or harden GET file metadata/content path using Storage/ObjectStore safely where scoped;
- implement or harden changes feed mapping from operation-log repository to API DTOs where scoped;
- preserve transaction boundaries and safe public error mapping.

## Dependency guard

If Storage/API/Core surfaces are insufficient for this phase without cross-component changes, do not invent sibling behavior. Report BLOCKED_BY_DEPENDENCY or BLOCKED_BY_CONTRACT with the exact missing dependency.

## Allowed files

- crates/haze-sync-server/src/routes/v1/**
- crates/haze-sync-server/src/routes/**
- crates/haze-sync-server/src/state.rs
- crates/haze-sync-server/src/http/**
- crates/haze-sync-server/docs/**
- crates/haze-sync-server/control/report.md

## Non-goals

- No conflict resolution route behavior beyond scoped preservation fan-in.
- No adapter loops.
- No provider runtime.
- No API DTO redesign.
- No Storage schema change unless explicitly blocked and reported.
- No sibling component changes.
- No workflow changes.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and the relevant Project Source worker prompt. Product, test, dependency, contract, workflow, and implementation commits must not skip CI.

## Report

Write only the report to crates/haze-sync-server/control/report.md.

Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION.
