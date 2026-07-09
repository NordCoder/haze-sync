# W1-API-P4C-RERUN — API file/changes route-helper clean-code review

Component: api
Path: crates/haze-sync-api
Branch: component/api
PR: #44
Role: clean-code-reviewer

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Rerun guard

This is an explicit refreshed active prompt. The current report before this prompt was for FIX-API-P4-CI, not API-P4C. Therefore API-P4C-RERUN is not already complete.

## Context

API-P4 implementation and CI fixer are complete. Post-fix Component CI is green.

- workflow: Component CI
- workflow_run_id: 29034803865
- run_number: 689
- conclusion: success

## Read

Read implementation-manifest.md, report-template.md, clean-code-reviewer-prompt.md, chatgpt-gh-connector.md, component docs/control files, relevant code and PR diff. Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Review API-P4 file and changes route-helper hardening plus the CI fixer. Focus on VaultPath parsing, upload metadata extraction, verified-principal helper behavior, public response mapping, changes query/page metadata, source compatibility, and passive API boundary preservation.

## Allowed files

- crates/haze-sync-api/src/routes/files/**
- crates/haze-sync-api/src/routes/changes/**
- crates/haze-sync-api/src/dto/files/**
- crates/haze-sync-api/src/dto/changes/**
- crates/haze-sync-api/docs/**
- crates/haze-sync-api/control/report.md

## Forbidden changes

No handler runtime implementation, object-store access, operation-log queries, content streaming, background cursor updates, workflow changes, or sibling component changes.

## CI trigger policy

Source/doc clean-code commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-api/control/report.md. Use report-template.md. Set REPORT_TYPE to CLEAN_CODE_REVIEW and phase_id to API-P4C-RERUN.
