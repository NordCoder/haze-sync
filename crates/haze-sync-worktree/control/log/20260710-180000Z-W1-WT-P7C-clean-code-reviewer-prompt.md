# W1-WT-P7C — Worktree delete and retention clean-code review

Component: worktree
Path: crates/haze-sync-worktree
Branch: component/worktree
PR: #49
Role: clean-code-reviewer

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

WT-P7 implementation and artifact-based CI correction are complete. Final source CI is green.

- code_bearing_sha: e9e7120916ff26ad7e6f4443bad3db2752f4acb1
- workflow: Component CI
- workflow_run_id: 29107152621
- run_number: 1362
- conclusion: success

## Read

Read implementation-manifest.md, report-template.md, clean-code-reviewer-prompt.md, chatgpt-gh-connector.md, component docs/control files, current WT-P7 source/tests, fixer changes, accepted Core/API delete semantics, and PR diff. Do not read diagnostics artifacts unless a future fixer prompt explicitly instructs it.

## Task

Review WT-P7 guarded delete submission, retained trash, restore metadata, and the CI corrections.

Focus on:

- safe candidate derivation from complete scans;
- known and explicit null base semantics;
- count/ratio guards and manual-unlock boundaries;
- public put-only import behavior;
- retained move and rollback ordering;
- path/root/symlink safety;
- restore metadata integrity, including retained byte length;
- state advancement only after accepted outcomes and durable local completion;
- private compatibility planner cleanup without widening the public delete surface;
- tests, redaction, and non-goal preservation.

## Allowed files

- crates/haze-sync-worktree/src/**
- crates/haze-sync-worktree/docs/**
- crates/haze-sync-worktree/control/report.md

## Boundaries

No Core tombstone policy, hard-delete cleanup, provider calls, CLI repair work, direct DB mutation, watcher/runtime service work, Server hosting, workflow/dependency changes, sibling changes, test deletion, or assertion weakening.

## CI trigger policy

Source/test/docs clean-code commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-sync-worktree/control/report.md. Use report-template.md, REPORT_TYPE CLEAN_CODE_REVIEW, phase_id WT-P7C.
