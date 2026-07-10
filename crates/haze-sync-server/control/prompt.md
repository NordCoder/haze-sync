# W1-FIX-SRV-P6C-CI — Server clean-code CI correction

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

SRV-P6C clean-code review made source/test changes, but the corresponding Component CI run failed.

- source_commit: 3ab500d1b4764c0b2775eb14d0c7be3ff6e7f57b
- workflow: Component CI
- workflow_run_id: 29067613093
- run_number: 826
- run_attempt: 1
- artifact_id: 8217733996
- artifact_name: ci-diag__component-server__wf-component-ci__run-29067613093__attempt-1
- artifact_expires_at: 2026-07-11T03:49:44Z

## Read

Read implementation-manifest.md, report-template.md, fixer-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, relevant source/tests, and PR diff.

Download artifact 8217733996. Read `summary.md`, `manifest.json`, and each failed-check log. If the artifact is unavailable or cannot be interpreted, report `FIX_BLOCKED_BY_LOGS`.

## Task

Apply the smallest Server-local correction required by the SRV-P6C diagnostics. Preserve readiness-first admin status behavior, best-effort optional metadata, safe public summaries, read-only operational semantics, and all SRV-P6 non-goals.

## Allowed files

- crates/haze-sync-server/src/routes/admin.rs
- crates/haze-sync-server/src/routes/admin/**
- crates/haze-sync-server/src/readiness/**
- crates/haze-sync-server/src/db/**
- crates/haze-sync-server/src/http/**
- crates/haze-sync-server/docs/** only when diagnostics identify a docs-format issue
- crates/haze-sync-server/control/report.md

## Boundaries

Do not add admin state changes, repair execution, token lifecycle changes, provider calls, new doctor/metrics surfaces, workflow changes, dependency changes, or sibling component changes.

## CI trigger policy

Source/test/docs correction commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-server/control/report.md. Use report-template.md. Set REPORT_TYPE to FIX and phase_id to FIX-SRV-P6C-CI.
