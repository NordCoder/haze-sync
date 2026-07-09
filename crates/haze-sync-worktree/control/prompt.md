# W1-WT-P3 — Worktree scanner, ignore rules, and stable-file detection

Component: worktree
Path: crates/haze-sync-worktree
Branch: component/worktree
PR: #49
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

WT-P2 and its CI fixer loop are complete. Current Component CI is green for the current PR head.

- workflow: Component CI
- workflow_run_id: 29006835773
- run_number: 450
- conclusion: success

The next implementation phase is WT-P3 from crates/haze-sync-worktree/docs/implementation-plan.md.

## Read

Read before editing:

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- implementation-worker-prompt.md from ChatGPT Project Sources
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

Implement WT-P3: Scanner, ignore rules, and stable-file detection.

Follow the implementation plan:

- walk the configured worktree root without escaping it;
- ignore reserved runtime directories, temp files, trash, and unsafe entries;
- represent local file facts with VaultPath, size, mtime or equivalent metadata, content hash, and stability metadata;
- detect stable files using a deterministic accepted heuristic;
- classify unsafe filesystem entries without panicking;
- add tests using temp directories or pure helper seams where practical.

## Allowed files

- crates/haze-sync-worktree/src/**
- crates/haze-sync-worktree/docs/**
- crates/haze-sync-worktree/control/report.md

## Non-goals

- No watcher reliance for correctness.
- No Core/API import calls.
- No materialization writer.
- No delete propagation.
- No provider behavior.
- No sibling component changes.
- No workflow changes.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and the relevant Project Source worker prompt. Product, test, dependency, contract, workflow, and implementation commits must not skip CI.

## Report

Write only the report to crates/haze-sync-worktree/control/report.md.

Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION.
