# W1-API-P3C — API metadata contract clean-code review

Component: api
Path: crates/haze-sync-api
Branch: component/api
PR: #44
Role: clean-code-reviewer

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

API-P3 implementation and CI fixer are complete. Post-fix Component CI is green.

- workflow: Component CI
- workflow_run_id: 29011290632
- run_number: 532
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

Review API-P3 header, auth, and safe error contract hardening plus the CI fix.

Focus areas:

- Bearer parsing/wrapping and token non-leak behavior;
- Idempotency-Key validation and Debug redaction;
- X-Content-SHA256 parsing and common ContentHash conversion;
- X-Base-Revision-Id explicit null semantics;
- safe public error field/header/query names;
- preservation of passive API boundaries.

## Allowed files

- crates/haze-sync-api/src/auth/**
- crates/haze-sync-api/src/contracts/headers/**
- crates/haze-sync-api/src/contracts/errors/**
- crates/haze-sync-api/docs/**
- crates/haze-sync-api/control/report.md

## Forbidden changes

- Do not add token persistence or creation.
- Do not add runtime auth lookup.
- Do not add middleware.
- Do not add SQLx or config loading.
- Do not change workflow files.
- Do not change sibling components.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and clean-code-reviewer-prompt.md. Source clean-code commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-api/control/report.md.

Use report-template.md. Set REPORT_TYPE to CLEAN_CODE_REVIEW.
