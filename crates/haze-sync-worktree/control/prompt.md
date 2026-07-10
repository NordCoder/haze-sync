# W1-FIX-WT-P8-CI — Worktree WT-P8 validation correction

Component: worktree
Path: crates/haze-sync-worktree
Branch: component/worktree
PR: #49
Role: fixer-worker

Use only the GitHub connector. Do not merge the PR or change its lifecycle state.

## Evidence

The WT-P8 implementation head has a failed Component CI result.

- code_bearing_sha: 6d342f9d89fff8123a4bb5458390a24a587eb812
- workflow_run_id: 29116232313
- run_attempt: 1
- artifact_id: 8236750926
- artifact_name: ci-diag__component-worktree__wf-component-ci__run-29116232313__attempt-1
- artifact_expires_at: 2026-07-11T18:56:49Z

The diagnostics artifact is the authoritative failure evidence.

## Required work

Read the project process files, current Worktree control files, WT-P8 implementation/tests/docs, PR diff, and every required file in artifact 8236750926. If the artifact is unavailable or incomplete, report FIX_BLOCKED_BY_LOGS.

Apply only the smallest correction demonstrated by the artifact. Keep the existing WT-P8 runtime lifecycle, watcher hint behavior, scan scheduling, mode checks, bounded work accounting, safe status output, tests, and ownership boundaries unchanged except where the artifact directly requires a correction.

## Allowed files

- crates/haze-sync-worktree/src/**
- crates/haze-sync-worktree/docs/** only when directly required by the diagnostics
- crates/haze-sync-worktree/control/report.md

Do not change Server composition, provider behavior, persistence ownership, shared workflows, dependencies, sibling components, or test expectations unrelated to the recorded failure.

Source/test/docs correction commits must run CI normally. A final report-only commit may skip CI.

Write the result only to crates/haze-sync-worktree/control/report.md using REPORT_TYPE FIX and phase_id FIX-WT-P8-CI.
