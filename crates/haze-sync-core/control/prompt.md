# W1-CORE-P6C — Core idempotency/cursor clean-code review

Component: core
Path: crates/haze-sync-core
Branch: component/core
PR: #43
Role: clean-code-reviewer

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

CORE-P6 implementation and artifact-based CI fixer are complete. Post-fix Component CI is green.

- code_bearing_sha: 97704236cdd13714a0d0c6e4ebac7a27d9a5b231
- workflow: Component CI
- workflow_run_id: 29088874164
- run_number: 1149
- conclusion: success

## Read

Read implementation-manifest.md, report-template.md, clean-code-reviewer-prompt.md, chatgpt-gh-connector.md, component docs/control files, current CORE-P6 source/tests/docs, fixer changes, accepted Storage/API boundaries, and PR diff. Do not read diagnostics artifacts unless a future fixer prompt explicitly instructs it.

## Task

Review CORE-P6 idempotency, operation-log, changes-page, and cursor primitive hardening plus the CI fixer.

Focus on redacted idempotency-key formatting, deterministic fingerprints, validated replay snapshots, safe persisted headers, operation sequence/limit/page invariants, cursor monotonicity, durable fan-in documentation, source compatibility, tests, and non-goal preservation.

## Allowed files

- crates/haze-sync-core/src/idempotency/**
- crates/haze-sync-core/src/operation_log/**
- crates/haze-sync-core/docs/**
- crates/haze-sync-core/control/report.md

## Boundaries

No durable repository implementation, database append logic, HTTP replay middleware, adapter polling loop, Storage/API/Server edits, workflow/dependency changes, sibling changes, test deletion, or assertion weakening.

## CI trigger policy

Source/test/docs clean-code commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-sync-core/control/report.md. Use report-template.md, REPORT_TYPE CLEAN_CODE_REVIEW, phase_id CORE-P6C.
