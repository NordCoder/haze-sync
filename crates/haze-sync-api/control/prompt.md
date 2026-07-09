# W1-API-P3 — Header, auth, and safe error contract hardening

Component: api
Path: crates/haze-sync-api
Branch: component/api
PR: #44
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

API-P2 and its CI fixer loop are complete. Current Component CI is green for the current PR head.

- workflow: Component CI
- workflow_run_id: 29006904242
- run_number: 457
- conclusion: success

The next implementation phase is API-P3 from crates/haze-sync-api/docs/implementation-plan.md.

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

Implement API-P3: Header, auth, and safe error contract hardening.

Follow the implementation plan:

- test Bearer authorization parsing/wrapping without leaking token text;
- test token hash formatting redaction and verification behavior where applicable;
- test Idempotency-Key validation and non-exposure in errors/debug output;
- test X-Content-SHA256 parsing and canonical conversion to common hash types;
- test X-Base-Revision-Id including explicit null semantics;
- test public error response codes/messages/details for safe field/header/query naming.

## Allowed files

- crates/haze-sync-api/src/auth/**
- crates/haze-sync-api/src/contracts/headers/**
- crates/haze-sync-api/src/contracts/errors/**
- crates/haze-sync-api/docs/**
- crates/haze-sync-api/control/report.md

## Non-goals

- No token persistence.
- No token creation or rotation.
- No runtime auth lookup.
- No middleware.
- No SQLx or config loading.
- No sibling component changes.
- No workflow changes.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and the relevant Project Source worker prompt. Product, test, dependency, contract, workflow, and implementation commits must not skip CI.

## Report

Write only the report to crates/haze-sync-api/control/report.md.

Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION.
