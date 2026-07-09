# W1-WT-P4C — Worktree import planner clean-code review

Component: worktree
Path: crates/haze-sync-worktree
Branch: component/worktree
PR: #49
Role: clean-code-reviewer

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

WT-P4 implementation and CI fixer are complete. Post-fix Component CI is green.

- workflow: Component CI
- workflow_run_id: 29034738311
- run_number: 679
- conclusion: success

## Read

Read before editing:

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- clean-code-reviewer-prompt.md from ChatGPT Project Sources
- chatgpt-gh-connector.md from ChatGPT Project Sources
- crates/haze-sync-worktree/docs/component-contract.md
- crates/haze-sync-worktree/docs/implementation-plan.md
- crates/haze-sync-worktree/docs/implementation-log.md
- crates/haze-sync-worktree/docs/dependency-map.md
- crates/haze-sync-worktree/control/prompt.md
- crates/haze-sync-worktree/control/report.md
- relevant current repository code and PR diff

Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Review WT-P4 import planner and Core/API submission boundary plus the CI fixer.

Focus areas:

- local state comparison against last applied Core revision;
- import plans for new, modified, and deleted local files;
- base revision and explicit null-base handling;
- stable-file-only content/hash inclusion;
- abstract API/Core client trait boundary;
- outcome handling for accepted, same-content, conflict-saved, rejected, tombstoned, and not-found;
- local state updates only after accepted outcomes;
- preservation of non-goals: no direct SQLx/Storage writes, no route handlers, no provider/GDrive behavior, no Worktree-owned conflict policy, no hard delete.

## Allowed files

- crates/haze-sync-worktree/src/**
- crates/haze-sync-worktree/docs/**
- crates/haze-sync-worktree/control/report.md

## Forbidden changes

- Do not add direct SQLx or Storage writes.
- Do not add route handlers.
- Do not add provider/GDrive behavior.
- Do not add conflict policy decisions inside Worktree.
- Do not add hard delete.
- Do not change workflow files.
- Do not change sibling components.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and clean-code-reviewer-prompt.md. Source/doc clean-code commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-worktree/control/report.md.

Use report-template.md. Set REPORT_TYPE to CLEAN_CODE_REVIEW.
