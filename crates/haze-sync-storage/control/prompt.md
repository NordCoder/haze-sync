# W1-STOR-P6C — Storage conflict/tombstone clean-code review

Component: storage
Path: crates/haze-sync-storage
Branch: component/storage
PR: #47
Role: clean-code-reviewer

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

STOR-P6 implementation and CI fixer are complete. Final code-bearing Component CI is green.

- code_bearing_sha: b47bf13f652e0287c6153e6b0c969140920b796e
- workflow_run_id: 29082024073
- run_number: 972
- conclusion: success

## Read

Read process sources, component docs/control files, current source/tests/docs, and PR diff. Do not read diagnostics artifacts unless a future fixer prompt instructs it.

## Task

Review STOR-P6 conflict, tombstone, restore-metadata, operation-log, and compatibility corrections.

Focus on passive repository ownership, safe persisted-value validation, conflict lifecycle updates, bounded and source-compatible listing APIs, tombstone restore metadata, caller-owned transactions, operation-feed mappings, integration test clarity, and non-goal preservation.

## Allowed files

- crates/haze-sync-storage/src/repositories/conflicts/**
- crates/haze-sync-storage/src/repositories/tombstones/**
- crates/haze-sync-storage/src/repositories/operation_log/**
- crates/haze-sync-storage/src/models/** when directly relevant
- crates/haze-sync-storage/docs/**
- crates/haze-sync-storage/control/report.md

## Boundaries

No Core policy, hard deletion, filesystem/provider behavior, API handlers, cleanup execution, workflow/dependency changes, or sibling changes.

## CI trigger policy

Source/docs clean-code commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-sync-storage/control/report.md. Use REPORT_TYPE CLEAN_CODE_REVIEW and phase_id STOR-P6C.
