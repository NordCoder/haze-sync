# W1-FIX-WT-P8C-CI — Worktree WT-P8C validation correction

Component: worktree
Path: crates/haze-sync-worktree
Branch: component/worktree
PR: #49
Role: fixer-worker

Use only the GitHub connector. Do not merge the PR or change its lifecycle state.

## Evidence

The final WT-P8C source/test/docs head has a failed Component CI result.

- code_bearing_sha: e02ce213390d689f933d578f87729062f1c64a3c
- workflow_run_id: 29124728043
- run_attempt: 1
- artifact_id: 8239878384
- artifact_name: ci-diag__component-worktree__wf-component-ci__run-29124728043__attempt-1
- artifact_expires_at: 2026-07-11T21:28:37Z

Use the diagnostics artifact as the authoritative failure evidence.

## Required work

Read the project process files, current Worktree control files, WT-P8C source/tests/docs, PR diff, and every required file in artifact 8239878384. If the artifact is unavailable or incomplete, report FIX_BLOCKED_BY_LOGS.

Apply only the smallest correction demonstrated by the artifact. Preserve watcher resource release after close/poll failure, operation-correct safe error categories, host-driven lifecycle, path-free watcher hints, authoritative scans, mode/budget enforcement, cancellation/shutdown behavior, focused regressions, and component boundaries.

## Allowed files

- crates/haze-sync-worktree/src/**
- crates/haze-sync-worktree/docs/** only when directly required by diagnostics
- crates/haze-sync-worktree/control/report.md

Do not change Server composition, concrete watcher selection, provider behavior, persistence ownership, shared workflows/dependencies, sibling components, or unrelated test expectations.

Source/test/docs correction commits must run CI normally. A final report-only commit may skip CI.

Write only crates/haze-sync-worktree/control/report.md using REPORT_TYPE FIX and phase_id FIX-WT-P8C-CI.
