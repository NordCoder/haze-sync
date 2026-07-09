# W1-SRV-P3C — Server clean-code review

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: clean-code-reviewer

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review.

## Context

The latest implementation report completed SRV-P3 production startup, config loading, listener, and graceful shutdown with status SELF_ACCEPT_PENDING_CI.

Component CI is green for the current PR head.

- workflow: Component CI
- workflow_run_id: 29003607526
- run_number: 406
- conclusion: success

## Read

Read all required sources before editing:

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- crates/haze-sync-server/docs/component-contract.md
- crates/haze-sync-server/docs/implementation-plan.md
- crates/haze-sync-server/docs/implementation-log.md
- crates/haze-sync-server/docs/dependency-map.md
- crates/haze-sync-server/control/prompt.md
- crates/haze-sync-server/control/report.md
- current component diff and relevant repository code

## Task

Review the SRV-P3 implementation and improve it where appropriate.

Focus areas:

- production startup correctness;
- config loading and object-store root preparation;
- PostgreSQL pool initialization path;
- Axum listener binding and graceful shutdown;
- sanitized startup errors;
- dependency scope expansion in Cargo.toml;
- preservation of route semantics and non-goals.

## Allowed files

- crates/haze-sync-server/**

## Forbidden changes

- Do not add adapter loops.
- Do not add provider calls.
- Do not add worktree runtime behavior.
- Do not change API/Core/Storage semantics.
- Do not add deployment scripts.
- Do not auto-run migrations unless the contract explicitly accepts it.
- Do not archive control files.

## Checks

Run applicable checks if possible.

If you cannot run shell commands, say so in the report. Do not claim checks passed unless you actually ran them or observed CI metadata.

## Report

Write only the report to crates/haze-sync-server/control/report.md.

Use report-template.md.

Set REPORT_TYPE to CLEAN_CODE_REVIEW.

Use one of:

- CLEAN_ACCEPT
- CLEAN_ACCEPT_PENDING_CI
- CLEAN_NEEDS_FIX
- CLEAN_BLOCKED_BY_CONTRACT
- CLEAN_BLOCKED_BY_SCOPE
- CLEAN_BLOCKED_BY_TOOLING
