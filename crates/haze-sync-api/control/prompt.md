# W1-API-P4C — API file/changes route-helper clean-code review

Component: api
Path: crates/haze-sync-api
Branch: component/api
PR: #44
Role: clean-code-reviewer

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

API-P4 implementation and CI fixer are complete. Post-fix Component CI is green.

- workflow: Component CI
- workflow_run_id: 29034803865
- run_number: 689
- conclusion: success

## Read

Read before editing:

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- clean-code-reviewer-prompt.md from ChatGPT Project Sources
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

Review API-P4 file and changes route-helper hardening plus the CI fixer.

Focus areas:

- VaultPath parsing through file route helpers;
- upload metadata extraction for path, idempotency key, content hash, base revision, body length, and body bytes;
- authenticated helper behavior for verified adapter principal without runtime auth lookup;
- public response mapping for accepted, same-content, conflict-saved, hash-mismatch, and stale outcomes;
- changes query since/limit bounds and response page metadata;
- source compatibility of public request-parts construction;
- preservation of passive API boundaries: no Axum handler implementation, no object-store access, no operation-log queries, no content streaming, no background cursor updates.

## Allowed files

- crates/haze-sync-api/src/routes/files/**
- crates/haze-sync-api/src/routes/changes/**
- crates/haze-sync-api/src/dto/files/**
- crates/haze-sync-api/src/dto/changes/**
- crates/haze-sync-api/docs/**
- crates/haze-sync-api/control/report.md

## Forbidden changes

- Do not add Axum handler implementation.
- Do not add object-store reads or writes.
- Do not add operation-log queries.
- Do not add content streaming.
- Do not add background cursor updates.
- Do not change workflow files.
- Do not change sibling components.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and clean-code-reviewer-prompt.md. Source/doc clean-code commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-api/control/report.md.

Use report-template.md. Set REPORT_TYPE to CLEAN_CODE_REVIEW.
