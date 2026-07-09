# W1-SRV-P6 — Admin/status, readiness, doctor, and observability hardening

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

SRV-P5 implementation, CI fixer, and clean-code review are accepted. Component CI evidence for the accepted product-code state is green.

- workflow: Component CI
- workflow_run_id: 29028061038
- run_number: 619
- conclusion: success

The next implementation phase is SRV-P6 from crates/haze-sync-server/docs/implementation-plan.md.

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

Implement SRV-P6: Admin/status, readiness, doctor, and observability hardening.

Follow the implementation plan:

- harden `/ready` and DB/object-store readiness checks;
- map adapter summaries, cursor presence, pause support, and mode/status summaries safely;
- add doctor route or server-side doctor inputs only when Core/API contracts exist;
- ensure skipped/not-run checks are represented honestly;
- add structured logs/tracing with redaction guarantees if scoped;
- add metrics endpoint only if system scope accepts it.

## Allowed files

- crates/haze-sync-server/src/routes/admin/**
- crates/haze-sync-server/src/readiness/**
- crates/haze-sync-server/src/db/**
- crates/haze-sync-server/src/http/**
- crates/haze-sync-server/docs/**
- crates/haze-sync-server/control/report.md

## Non-goals

- No admin mutations by default.
- No repair execution.
- No token rotation.
- No provider calls unless a provider component contract supplies safe checks.
- No raw cursor/status payload exposure.
- No sibling component changes.
- No workflow changes.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and implementation-worker-prompt.md. Product/source/docs commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-server/control/report.md.

Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION.
