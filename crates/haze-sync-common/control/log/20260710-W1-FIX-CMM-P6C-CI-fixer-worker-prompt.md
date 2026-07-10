# Archived active prompt

component: common
wave: W1
phase: FIX-CMM-P6C-CI
agent_role: fixer-worker
source_path: crates/haze-sync-common/control/prompt.md
source_sha: d428ff39e7237e23040ec0978fe002cede752676
archive_reason: fixer completed, post-fix CI is green, and CMM-P6 is the final Common component-plan phase.

---

# W1-FIX-CMM-P6C-CI — Common fixture-review CI correction

Component: common
Path: crates/haze-sync-common
Branch: component/common
PR: #46
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

CMM-P6C clean-code review changed tests/docs. Its final code-bearing Component CI run failed.

- code_bearing_sha: 4dd1896932c2d4114b2f9b16d86ad1bf5b8e3b81
- workflow: Component CI
- workflow_run_id: 29084340110
- run_number: 1036
- run_attempt: 1
- artifact_id: 8224118461
- artifact_name: ci-diag__component-common__wf-component-ci__run-29084340110__attempt-1
- artifact_expires_at: 2026-07-11T09:51:35Z

Use the diagnostics artifact as source of truth.

## Read

Read implementation-manifest.md, report-template.md, fixer-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, current compatibility fixture tests/docs, and PR diff. Download artifact 8224118461 and read summary.md, manifest.json, and every failed-check log. If unavailable or unreadable, report FIX_BLOCKED_BY_LOGS.

## Task

Apply only the minimum artifact-proven correction for the CMM-P6C CI failure. Preserve strict fixture schema validation, complete unique unordered vocabulary checks, JSON-safe roundtrip assertions, fixture values, production Common behavior, downstream guidance, and all component non-goals.

## Allowed files

- crates/haze-sync-common/tests/**
- crates/haze-sync-common/docs/** only if diagnostics prove a documentation issue
- crates/haze-sync-common/fixtures/** only if diagnostics directly prove a fixture defect
- crates/haze-sync-common/src/** only if diagnostics directly prove a production defect
- crates/haze-sync-common/control/report.md

## Boundaries

No TypeScript edits, generated client pipeline, API DTO ownership, Core policy, runtime/provider behavior, workflow/dependency changes, sibling changes, fixture coverage removal, test deletion, or assertion weakening.

## CI trigger policy

Source/tests/docs/fixture fixer commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-sync-common/control/report.md. Use report-template.md, REPORT_TYPE FIX, phase_id FIX-CMM-P6C-CI.
