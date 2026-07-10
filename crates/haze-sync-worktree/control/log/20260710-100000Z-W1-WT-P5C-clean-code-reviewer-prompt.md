# W1-WT-P5C — Worktree materializer clean-code review

Component: worktree
Path: crates/haze-sync-worktree
Branch: component/worktree
PR: #49
Role: clean-code-reviewer

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

WT-P5 implementation and CI fixer are complete. Post-fix Component CI is green.

- code_bearing_sha: fe568904f82d2ea772249fae31c4e9800f20798a
- workflow: Component CI
- workflow_run_id: 29079896125
- run_number: 896
- conclusion: success

## Read

Read implementation-manifest.md, report-template.md, clean-code-reviewer-prompt.md, chatgpt-gh-connector.md, component docs/control files, current source, and PR diff. Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Review WT-P5 materializer and atomic writer plus the formatting fixer.

Focus areas:

- authoritative materialization request boundaries;
- incoming hash verification before filesystem mutation;
- dirty, missing, untracked, already-current, and tombstoned local-state handling;
- deferred import plans instead of silent overwrite;
- reserved temp staging, sync, rename, cleanup, and destination revalidation;
- root/parent path and symlink safety;
- state advancement only after successful or already-current outcomes;
- echo-marker behavior and ownership;
- clear separation between filesystem mechanics and Core conflict policy;
- preservation of no-hard-delete and other WT-P5 non-goals.

## Allowed files

- crates/haze-sync-worktree/src/**
- crates/haze-sync-worktree/docs/**
- crates/haze-sync-worktree/control/report.md

## Forbidden changes

No Core conflict policy, API DTO redesign, provider behavior, watcher/runtime service work, hard delete, workflow changes, dependency changes, or sibling component changes.

## CI trigger policy

Source/doc clean-code commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-worktree/control/report.md. Use report-template.md. Set REPORT_TYPE to CLEAN_CODE_REVIEW and phase_id to WT-P5C.
