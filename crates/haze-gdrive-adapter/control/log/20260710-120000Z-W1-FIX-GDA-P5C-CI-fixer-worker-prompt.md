# Archived active prompt

component: gdrive-adapter
archived_at: 2026-07-10T12:00:00Z
wave: W1
phase: FIX-GDA-P5C-CI
agent_role: fixer-worker
source_path: crates/haze-gdrive-adapter/control/prompt.md
source_sha: dd6012f1de9df2ec3817bb6404c632e2302895ef

Reason: fixer report closed FIX-GDA-P5C-CI and post-fix Component CI run 29088281762 completed successfully. Orchestrator is replacing the active slot with GDA-P6 implementation.

---

# W1-FIX-GDA-P5C-CI — GDrive clean-code CI correction

Component: gdrive-adapter
Path: crates/haze-gdrive-adapter
Branch: component/gdrive-adapter
PR: #50
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

GDA-P5C clean-code review changed source/tests. Its final code-bearing Component CI run failed.

- code_bearing_sha: 21954425a2a78b0fa5ce437d0882d205974311fe
- workflow: Component CI
- workflow_run_id: 29086780265
- run_number: 1106
- run_attempt: 1
- artifact_id: 8225096800
- artifact_name: ci-diag__component-gdrive-adapter__wf-component-ci__run-29086780265__attempt-1
- artifact_expires_at: 2026-07-11T10:36:42Z

Use the diagnostics artifact as source of truth.

## Read

Read implementation-manifest.md, report-template.md, fixer-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, current GDA-P5C source/tests, and PR diff. Download artifact 8225096800 and read summary.md, manifest.json, and every failed-check log. If unavailable or unreadable, report FIX_BLOCKED_BY_LOGS.

## Task

Apply only the minimum artifact-proven correction for the GDA-P5C CI failure. Preserve request-content redaction, provider-segment validation, mapping identity/path conflict handling, conservative delete-candidate suppression, tests, and component boundaries.

## Allowed files

- crates/haze-gdrive-adapter/src/**
- crates/haze-gdrive-adapter/docs/** only if diagnostics prove a documentation issue
- crates/haze-gdrive-adapter/control/report.md

## Boundaries

No export phase work, live provider integration, Core policy changes, immediate deletion behavior, direct DB ownership, workflow/dependency changes, sibling changes, test deletion, or assertion weakening.

## CI trigger policy

Source/test/docs fixer commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-gdrive-adapter/control/report.md. Use report-template.md, REPORT_TYPE FIX, phase_id FIX-GDA-P5C-CI.
