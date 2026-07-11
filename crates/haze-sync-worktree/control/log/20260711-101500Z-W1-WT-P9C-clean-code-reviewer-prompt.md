# W1-WT-P9C — Worktree doctor and repair-planning clean-code review

Before starting, name this worker chat exactly:

`worktree — W1 WT-P9C Clean-Code Review`

Component: worktree
Path: crates/haze-sync-worktree
Branch: component/worktree
PR: #49
Role: clean-code-reviewer

Work only through the GitHub connector. Do not merge the PR, change its draft state, rebase, reset, rewrite history, or modify `main` or sibling branches.

## Context

WT-P9 implementation and its artifact-based CI correction are complete. The final code-bearing Worktree source/test/docs head is green.

- code_bearing_sha: `ea24e15f45613886f9dfad0f331543daf45d93bc`
- workflow: `Component CI`
- workflow_run_id: `29145333765`
- run_number: `1660`
- workflow_run_attempt: `1`
- conclusion: `success`

Archived lifecycle evidence:

- implementation report: `crates/haze-sync-worktree/control/log/20260711-071300Z-W1-WT-P9-implementation-worker-report.md`
- fixer prompt: `crates/haze-sync-worktree/control/log/20260711-082000Z-W1-FIX-WT-P9-CI-fixer-worker-prompt.md`
- fixer report: `crates/haze-sync-worktree/control/log/20260711-082000Z-W1-FIX-WT-P9-CI-fixer-worker-report.md`

The fixer changed formatting only. Review the complete WT-P9 implementation, not merely the formatting diff.

## Read

Read the project process files, current Worktree control files, Worktree component contract, implementation plan, implementation log, dependency map, decisions, the archived WT-P9 implementation and fixer reports referenced above, current doctor/repair-planning source and tests, relevant scanner/reconciliation/runtime boundaries, accepted Core doctor/repair semantics, and the PR/phase diff.

Do not read CI diagnostics artifacts. The final code-bearing run is green and diagnostics inspection is not part of this role.

## Task

Review WT-P9 Worktree-owned diagnostics and non-destructive repair planning.

Focus on:

- doctor facts being derived from authoritative scan/reconciliation/runtime state rather than watcher hints;
- correct classification of missing, dirty, content-hash mismatch, reserved-path, skipped symlink/special/unsafe/filesystem, stale/expired echo, partial-scan, and degraded-runtime conditions;
- the managed unscoped `_haze_runtime` reserved skip not producing a false operator-visible violation;
- safe count/category summaries and only validated vault-relative optional paths;
- injected persistence/fact-source boundaries without direct Storage/DB ownership;
- repair plans remaining non-executing and requiring explicit host confirmation for overwrite, move, trash, or delete risk;
- no durable execution authority being inferred from a previously generated plan;
- deterministic APIs, clear module shape, focused tests, and suitability for later Server hosting;
- preservation of Worktree/Core/Server ownership boundaries and all WT-P9 non-goals.

## Allowed files

- `crates/haze-sync-worktree/src/**`
- `crates/haze-sync-worktree/docs/**`
- `crates/haze-sync-worktree/control/report.md`

## Boundaries

No concrete Server composition or routes, repair executor, automatic destructive mutation, provider sync, direct database access, public absolute paths, CLI repair command, workflow/dependency changes, sibling changes, test deletion, or assertion weakening.

Source/test/docs review commits must run CI normally. A final report-only commit may skip CI.

## Report

Write only `crates/haze-sync-worktree/control/report.md` using `report-template.md`.

Set:

- `REPORT_TYPE: CLEAN_CODE_REVIEW`
- `phase_id: WT-P9C`

Use an honest clean-review status and include the exact code-bearing SHA and authoritative CI evidence.
