# W1-SRV-P4C — Server file fan-in clean-code review

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: clean-code-reviewer

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

SRV-P4 implementation completed with SELF_ACCEPT_PENDING_CI. Product-code CI is green.

- workflow: Component CI
- workflow_run_id: 29009520199
- run_number: 504
- conclusion: success

## Read

Read before editing:

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- clean-code-reviewer-prompt.md from ChatGPT Project Sources
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

Review SRV-P4 file PUT/GET/changes fan-in hardening and improve it only where appropriate.

Focus areas:

- body-aware PUT idempotency fingerprint compatibility;
- safe request fingerprint construction;
- route transaction and idempotency ordering;
- GET and changes-feed behavior preservation;
- safe public error mapping;
- whether any legacy idempotency migration policy needs to be reported as deferred or blocked.

## Allowed files

- crates/haze-sync-server/src/routes/v1.rs
- crates/haze-sync-server/src/routes/v1/**
- crates/haze-sync-server/src/routes/**
- crates/haze-sync-server/docs/**
- crates/haze-sync-server/control/report.md

## Forbidden changes

- Do not change API DTO ownership.
- Do not change Storage schema.
- Do not add adapter loops.
- Do not add provider runtime.
- Do not add deployment scripts.
- Do not change workflow files.
- Do not change sibling components.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and clean-code-reviewer-prompt.md. Source clean-code commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-server/control/report.md.

Use report-template.md. Set REPORT_TYPE to CLEAN_CODE_REVIEW.
