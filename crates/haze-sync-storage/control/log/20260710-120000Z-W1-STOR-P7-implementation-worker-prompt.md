# Archived active prompt

component: storage
archived_at: 2026-07-10T12:00:00Z
wave: W1
phase: STOR-P7
agent_role: implementation-worker
source_path: crates/haze-sync-storage/control/prompt.md
source_sha: 2c7f064c16a5eedad9d1f822ca4aa85b0aa1ab67

Reason: STOR-P7 implementation report closed the active phase, but Component CI run 29088574856 failed. Orchestrator is replacing the active slot with an artifact-based fixer prompt.

---

# W1-STOR-P7 — Idempotency and cursor repository support

Component: storage
Path: crates/haze-sync-storage
Branch: component/storage
PR: #47
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

STOR-P6 implementation, clean-code review, and CI fixer are complete. Post-fix Component CI is green.

- code_bearing_sha: 56ee302fa490c94f2c3ed3e405d3fc91d5a63bf9
- workflow: Component CI
- workflow_run_id: 29086476328
- run_number: 1097
- conclusion: success

The next implementation phase is STOR-P7 from crates/haze-sync-storage/docs/implementation-plan.md.

## Read

Read implementation-manifest.md, report-template.md, implementation-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, current idempotency/cursor models and repositories, accepted Core idempotency/operation-log primitives, and PR diff. Do not read diagnostics artifacts unless a future fixer prompt explicitly instructs it.

## Task

Implement STOR-P7: Idempotency and cursor repository support.

- test idempotency record insert/read/check-or-store behavior;
- verify same-key same-request and same-key different-request outcomes match accepted Core primitives;
- preserve safe stored response snapshot behavior;
- test adapter cursor read/update behavior and cursor regression rejection;
- ensure raw external cursor JSON is not exposed through public admin/status surfaces;
- keep persistence concerns inside Storage without adding HTTP middleware or adapter polling.

## Allowed files

- crates/haze-sync-storage/src/repositories/idempotency/**
- crates/haze-sync-storage/src/repositories/adapter_cursors/**
- crates/haze-sync-storage/src/models/** when directly scoped
- crates/haze-sync-storage/docs/**
- crates/haze-sync-storage/control/report.md

## Non-goals

No HTTP replay middleware, adapter polling loop, provider changes-feed calls, public admin/status rendering, workflow/dependency changes, or sibling component changes.

## CI trigger policy

Product/source/docs commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-sync-storage/control/report.md. Use report-template.md, REPORT_TYPE IMPLEMENTATION, phase_id STOR-P7.
