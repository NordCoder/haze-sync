# W1-WT-P8C — Worktree runtime-service clean-code review

Component: worktree
Path: crates/haze-sync-worktree
Branch: component/worktree
PR: #49
Role: clean-code-reviewer

Work only through the GitHub connector. Do not merge the PR or change its lifecycle state.

## Context

WT-P8 implementation and artifact-based CI corrections are complete. Final source/test CI is green.

- code_bearing_sha: a300fc1e179ba2358f27ffec91778b6a52138720
- workflow: Component CI
- workflow_run_id: 29120582304
- run_number: 1538
- conclusion: success

## Read

Read the project process files, current Worktree control files, WT-P8 runtime source/tests/docs, fixer changes, accepted adapter-mode and future Server-hosting boundaries, and PR diff. Do not read diagnostics artifacts unless a later fixer prompt explicitly requires them.

## Task

Review WT-P8 hostable runtime-service design and its CI corrections.

Focus on:

- explicit lifecycle and no hidden task ownership;
- watcher events remaining path-free latency hints only;
- startup, debounce, periodic recovery, cancellation, shutdown, and no-overlap behavior;
- full-scan correctness requirements in importing modes;
- mode capability interpretation and bounded cycle budgets;
- executor-result validation and state advancement;
- safe count-only status/error categories;
- watcher degradation and retry scheduling;
- module decomposition, test clarity, and readiness for later Worktree/Server fan-in.

## Allowed files

- crates/haze-sync-worktree/src/**
- crates/haze-sync-worktree/docs/**
- crates/haze-sync-worktree/control/report.md

## Boundaries

No Server composition, concrete OS watcher selection, provider behavior, persistence ownership, Core/API policy changes, workflow/dependency changes, sibling changes, test deletion, or assertion weakening.

Source/test/docs review commits must run CI normally. A final report-only commit may skip CI.

Write only crates/haze-sync-worktree/control/report.md using REPORT_TYPE CLEAN_CODE_REVIEW and phase_id WT-P8C.
