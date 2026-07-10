# Archived active prompt

component: worktree
archived_at: 2026-07-10T10:00:00Z
wave: W1
phase: FIX-WT-P5-CI
agent_role: fixer-worker
source_path: crates/haze-sync-worktree/control/prompt.md
source_sha: 5e9dca06abdd981aa7cf2ea93b0865bbefa4916b

# W1-FIX-WT-P5-CI — Worktree WT-P5 CI fix

Component: worktree
Path: crates/haze-sync-worktree
Branch: component/worktree
PR: #49
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

WT-P5 source implementation is complete, but its code-bearing Component CI run is red.

- code_bearing_sha: 1aef2177a2d6f5543c343136fa5aa01862c79afb
- workflow: Component CI
- workflow_run_id: 29067712825
- run_number: 847
- run_attempt: 1
- artifact_id: 8217769735
- artifact_name: ci-diag__component-worktree__wf-component-ci__run-29067712825__attempt-1
- artifact_expires_at: 2026-07-11T03:52:45Z

Visible wrapper-step conclusions are not the source of truth. Read the diagnostics artifact.

## Read

Read implementation-manifest.md, report-template.md, fixer-worker-prompt.md, chatgpt-gh-connector.md, the component contract/plan/log/dependency map, current control files, relevant source, and PR diff.

Download diagnostics artifact 8217769735. Read `summary.md`, `manifest.json`, and every log named in `failed_checks`. If the artifact is missing, expired, malformed, or unreadable, report `FIX_BLOCKED_BY_LOGS`.

## Task

Fix the minimum cause of the WT-P5 CI failure inside Worktree scope. Preserve the implemented materializer/atomic-writer semantics and all WT-P5 non-goals. Use the artifact as source of truth.

## Allowed files

- crates/haze-sync-worktree/src/**
- crates/haze-sync-worktree/docs/** only if diagnostics prove a documentation-format failure
- crates/haze-sync-worktree/control/report.md

## Forbidden changes

No Core conflict policy, API DTO redesign, provider behavior, watcher requirement, hard delete, workflow changes, dependency changes, or sibling component changes.

## CI trigger policy

Product/source/docs fixer commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-worktree/control/report.md. Use report-template.md. Set REPORT_TYPE to FIX and phase_id to FIX-WT-P5-CI.
