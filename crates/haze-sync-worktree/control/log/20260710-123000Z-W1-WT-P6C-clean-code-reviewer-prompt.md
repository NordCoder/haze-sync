# W1-WT-P6C — Worktree echo/reconciliation clean-code review

Component: worktree
Path: crates/haze-sync-worktree
Branch: component/worktree
PR: #49
Role: clean-code-reviewer

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

WT-P6 implementation and artifact-based CI fixer are complete. Post-fix Component CI is green.

- code_bearing_sha: 4d1fc6ea706fb61d700877189317279343502285
- workflow: Component CI
- workflow_run_id: 29088387939
- run_number: 1143
- conclusion: success

## Read

Read implementation-manifest.md, report-template.md, clean-code-reviewer-prompt.md, chatgpt-gh-connector.md, component docs/control files, current WT-P6 source/tests, fixer changes, and PR diff. Do not read diagnostics artifacts unless a future fixer prompt explicitly instructs it.

## Task

Review WT-P6 echo guard and worktree state reconciliation plus the CI fixer.

Focus on bounded one-shot marker handling, reserved runtime path safety, exact revision/hash matching, expiry/capacity cleanup, drift classification, skipped-prefix behavior, abstract state-store ownership, safe summaries, tests, and preservation of non-goals.

## Allowed files

- crates/haze-sync-worktree/src/**
- crates/haze-sync-worktree/docs/**
- crates/haze-sync-worktree/control/report.md

## Boundaries

No direct DB ownership, Server runtime work, provider behavior, delete/trash phase work, watcher/runtime service work, Core policy changes, workflow/dependency changes, sibling changes, test deletion, or assertion weakening.

## CI trigger policy

Source/test/docs clean-code commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-sync-worktree/control/report.md. Use report-template.md, REPORT_TYPE CLEAN_CODE_REVIEW, phase_id WT-P6C.
