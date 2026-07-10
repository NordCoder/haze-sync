# Archived active prompt

component: worktree
wave: W1
phase: WT-P6
agent_role: implementation-worker
source_path: crates/haze-sync-worktree/control/prompt.md
source_sha: 086db2988f12cbbef30fdd5c837f4db025b327e8
archive_reason: WT-P6 report closed the implementation slot and CI requires fixer triage.

---

# W1-WT-P6 — Echo guard and worktree state reconciliation

Component: worktree
Path: crates/haze-sync-worktree
Branch: component/worktree
PR: #49
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

WT-P5 implementation, clean-code review, and CI fixer are accepted. Post-fix Component CI is green.

- code_bearing_sha: 1c762e0159525e11daa030358e08e3cb055627a5
- workflow: Component CI
- workflow_run_id: 29084291087
- run_number: 1033
- conclusion: success

The next implementation phase is WT-P6 from crates/haze-sync-worktree/docs/implementation-plan.md.

## Read

Read implementation-manifest.md, report-template.md, implementation-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, current source/tests, accepted API/Storage/Server boundaries, and PR diff. Do not read diagnostics artifacts unless a future fixer prompt explicitly instructs it.

## Task

Implement WT-P6: Echo guard and worktree state reconciliation.

- define bounded echo-guard state from the last adapter write;
- compare filesystem observations with the last applied revision, hash, and modification facts;
- classify clean, dirty, missing, extra, conflict-materialized, and skipped files;
- consume or expire echo markers without treating them as permanently authoritative;
- integrate persisted worktree state only through an already accepted abstraction; do not add direct DB ownership;
- expose safe reconciliation summaries without absolute local paths;
- add focused tests for echo suppression, expiry, drift classification, and reconciliation state transitions.

## Allowed files

- crates/haze-sync-worktree/src/**
- crates/haze-sync-worktree/docs/**
- crates/haze-sync-worktree/control/report.md

## Non-goals

No direct DB ownership, Server status route implementation, repair execution, provider behavior, delete/trash phase work, watcher/runtime service work, workflow/dependency changes, or sibling-component changes.

## CI trigger policy

Product/source/docs commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-sync-worktree/control/report.md. Use report-template.md, REPORT_TYPE IMPLEMENTATION, phase_id WT-P6.
