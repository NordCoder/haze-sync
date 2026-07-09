# W1-STOR-P4C — Storage repository boundary clean-code review

Component: storage
Path: crates/haze-sync-storage
Branch: component/storage
PR: #47
Role: clean-code-reviewer

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

STOR-P4 implementation completed with SELF_ACCEPT_PENDING_CI. Component CI for the STOR-P4 code/docs commit is now green.

- workflow: Component CI
- workflow_run_id: 29023928350
- run_number: 577
- conclusion: success

## Read

Read before editing:

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- clean-code-reviewer-prompt.md from ChatGPT Project Sources
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

Review STOR-P4 repository validation and safe error boundary hardening.

Focus areas:

- caller-owned executor/transaction boundaries;
- revision list limit validation and shared helper consistency;
- sequence, limit, size conversion, status parsing, operation kind parsing, and cursor regression tests;
- safe RepositoryError code/message/Display behavior;
- raw SQLx/database/internal text redaction through map_sqlx_error;
- transaction-sensitive helper documentation.

## Allowed files

- crates/haze-sync-storage/src/repositories/**
- crates/haze-sync-storage/docs/**
- crates/haze-sync-storage/control/report.md

## Forbidden changes

- Do not add DB pool creation.
- Do not add server route wiring.
- Do not add Core policy decisions.
- Do not add public HTTP status mapping.
- Do not add provider behavior.
- Do not change workflow files.
- Do not change sibling components.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and clean-code-reviewer-prompt.md. Source/doc clean-code commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-storage/control/report.md.

Use report-template.md. Set REPORT_TYPE to CLEAN_CODE_REVIEW.
