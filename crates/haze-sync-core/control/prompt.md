# W1-CORE-P4C — Core conflict primitives clean-code review

Component: core
Path: crates/haze-sync-core
Branch: component/core
PR: #43
Role: clean-code-reviewer

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

CORE-P4 implementation and CI fixer are complete. Follow-up Component CI is green.

- workflow: Component CI
- workflow_run_id: 29038598984
- run_number: unknown
- conclusion: success

## Read

Read implementation-manifest.md, report-template.md, clean-code-reviewer-prompt.md, chatgpt-gh-connector.md, component docs/control files, relevant current source, and PR diff. Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Review CORE-P4 conflict preservation and resolution primitives plus the CI fixer.

Focus areas:

- conflict-copy path generation and conflict-area recursion handling;
- preservation of current revision while saving conflict copies;
- pure decision primitives for accept_current, accept_conflict, keep_both, and mark_resolved;
- storage/API-neutral plan outputs;
- metadata-only versus current-revision-creating action clarity;
- test and docs clarity;
- preservation of non-goals.

## Allowed files

- crates/haze-sync-core/src/conflict_service/**
- crates/haze-sync-core/src/policy_engine/**
- crates/haze-sync-core/src/conflict_saved_planner/**
- crates/haze-sync-core/docs/**
- crates/haze-sync-core/control/report.md

## Forbidden changes

No route wiring, repository implementation, object-store writes, API DTO ownership changes, Obsidian UI behavior, workflow changes, or sibling component changes.

## CI trigger policy

Source/doc clean-code commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-core/control/report.md. Use report-template.md. Set REPORT_TYPE to CLEAN_CODE_REVIEW and phase_id to CORE-P4C.
