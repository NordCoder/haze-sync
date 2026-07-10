# W1-FIX-CMM-P6-CI — Common compatibility fixture CI fix

Component: common
Path: crates/haze-sync-common
Branch: component/common
PR: #46
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

CMM-P6 implementation is complete, but its fixture/test/docs-bearing Component CI run is red.

- code_bearing_sha: ad64fbfacc160d491c8e4e961b18ca4b7efcf0f4
- workflow: Component CI
- workflow_run_id: 29079946397
- run_number: 903
- run_attempt: 1
- artifact_id: 8222362767
- artifact_name: ci-diag__component-common__wf-component-ci__run-29079946397__attempt-1
- artifact_expires_at: 2026-07-11T08:31:49Z

## Read

Read implementation-manifest.md, report-template.md, fixer-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, current fixture/tests/docs, and PR diff.

Download diagnostics artifact 8222362767. Read `summary.md`, `manifest.json`, and every failed-check log. If it is missing, expired, malformed, or unreadable, report `FIX_BLOCKED_BY_LOGS`.

## Task

Fix the minimum artifact-proven cause of the CMM-P6 CI failure. Preserve stable Common-owned primitive fixtures, complete vocabulary checks, safe deterministic examples, downstream mirroring documentation, and all CMM-P6 non-goals.

## Allowed files

- crates/haze-sync-common/fixtures/**
- crates/haze-sync-common/tests/**
- crates/haze-sync-common/docs/** only if diagnostics prove a docs issue
- crates/haze-sync-common/src/** only if diagnostics directly prove an implementation defect
- crates/haze-sync-common/control/report.md

## Forbidden changes

No TypeScript edits, generated-code pipeline, API DTO ownership, Core policy, runtime/provider behavior, workflow changes, dependency changes, sibling component changes, removal of fixture coverage, or weakening of safety assertions.

## CI trigger policy

Product/source/docs/fixture fixer commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-common/control/report.md. Use report-template.md. Set REPORT_TYPE to FIX and phase_id to FIX-CMM-P6-CI.
