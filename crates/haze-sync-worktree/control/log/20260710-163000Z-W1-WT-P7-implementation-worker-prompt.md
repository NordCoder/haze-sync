# W1-WT-P7 — Delete, trash, and local retention behavior

Component: worktree
Path: crates/haze-sync-worktree
Branch: component/worktree
PR: #49
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

WT-P6 implementation, clean-code review, and artifact-based CI correction are accepted. Final source CI is green.

- code_bearing_sha: 3c4fbc394221ca319bf662ab831240255da1c973
- workflow: Component CI
- workflow_run_id: 29092763544
- run_number: 1209
- conclusion: success

The next implementation phase is WT-P7 from crates/haze-sync-worktree/docs/implementation-plan.md.

## Read

Read implementation-manifest.md, report-template.md, implementation-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, current scan/import/materialization/reconciliation source and tests, accepted Core/API delete semantics, and PR diff. Do not read diagnostics artifacts unless a future fixer prompt explicitly instructs it.

## Task

Implement WT-P7: Delete, trash, and local retention behavior.

- represent local delete candidates from scans;
- submit delete candidates through accepted Core/API abstractions with base/null-base semantics;
- materialize Core tombstones by moving local files to a configured Worktree-owned trash/backup area when scoped;
- preserve retention and restore-ready metadata where accepted;
- avoid hard delete by default;
- prevent mass local delete propagation without accepted Core/API guard behavior;
- add focused tests for candidate planning, path safety, retention, restore metadata, and guarded behavior.

## Allowed files

- crates/haze-sync-worktree/src/**
- crates/haze-sync-worktree/docs/**
- crates/haze-sync-worktree/control/report.md

## Non-goals

No Core tombstone policy, immediate hard-delete cleanup, provider delete calls, CLI repair command, direct DB mutation, watcher/runtime service work, Server hosting, workflow/dependency changes, or sibling component changes.

## CI trigger policy

Product/source/test/docs commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-sync-worktree/control/report.md. Use report-template.md, REPORT_TYPE IMPLEMENTATION, phase_id WT-P7.
