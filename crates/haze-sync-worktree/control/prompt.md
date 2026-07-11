# W1-FIX-WT-P9-CI — Worktree diagnostics CI correction

Before starting, name this worker chat exactly:

`worktree — W1 WT-P9 CI Fix`

Component: worktree
Path: crates/haze-sync-worktree
Branch: component/worktree
PR: #49
Role: fixer-worker

Work only through the GitHub connector. Do not merge the PR, change its draft state, rebase, reset, rewrite history, or modify `main` or sibling branches.

## Context

WT-P9 implementation completed inside Worktree scope. The final code-bearing source/docs commit is:

- code_bearing_sha: `a14c4e953523e14d73431ae54ac03ab42101362c`
- workflow: `Component CI`
- workflow_run_id: `29143968689`
- run_number: `1655`
- workflow_run_attempt: `1`
- conclusion: `failure`

Visible Rust validation steps passed, but the workflow failed during diagnostics finalization. The implementation report is archived at:

- `crates/haze-sync-worktree/control/log/20260711-071300Z-W1-WT-P9-implementation-worker-report.md`

Use this exact diagnostics artifact:

- artifact_name: `ci-diag__component-worktree__wf-component-ci__run-29143968689__attempt-1`
- artifact_id: `8246127652`
- artifact_head_sha: `a14c4e953523e14d73431ae54ac03ab42101362c`
- artifact_status: available and unexpired when assigned

## Read

Before editing, read:

- project `implementation-manifest.md` and `report-template.md` sources;
- the Fixer Worker process prompt;
- Worktree component contract, implementation plan, implementation log, dependency map, and decisions;
- this active prompt;
- the archived WT-P9 implementation report referenced above;
- the WT-P9 source/tests/docs diff;
- the exact diagnostics artifact specified above.

Do not infer the failure from workflow metadata alone.

## Diagnostics protocol

Through the GitHub connector:

1. fetch artifact `8246127652` from workflow run `29143968689`;
2. read `ci-diagnostics/summary.md`;
3. read `ci-diagnostics/manifest.json`;
4. read every failure marker and log listed in `failed_checks`;
5. verify the artifact head SHA matches the code-bearing SHA above.

Raw GitHub job logs are not authorized as fallback in this prompt. If the artifact is missing, expired, malformed, mismatched, or unreadable, report `FIX_BLOCKED_BY_LOGS` without guessing.

## Task

Fix only the minimum artifact-proven cause of the failed Component CI run.

- Stay within Worktree component ownership.
- Preserve WT-P9 doctor and non-destructive repair-planning semantics.
- Do not weaken tests or assertions merely to obtain green CI.
- Do not introduce repair execution, Server wiring, provider behavior, direct database access, background jobs, public absolute paths, or unrelated product changes.
- If the artifact proves that the correction belongs to a workflow, shared CI infrastructure, Server, Storage, Core, API, CLI, or another component, do not cross the boundary; report `FIX_BLOCKED_BY_CONTRACT` with the exact required owner and evidence.

## Allowed files

- `crates/haze-sync-worktree/src/**`
- `crates/haze-sync-worktree/docs/**` only when required to keep documentation accurate after an artifact-proven code correction
- `crates/haze-sync-worktree/control/report.md`

## Checks and CI

Re-run the artifact-proven failing check when possible. Any source/test/docs fix commit must run CI normally. CI skip is permitted only for a final report-only commit.

Do not claim success until a post-fix code-bearing `Component CI` run is observed green. If code is corrected but CI remains pending or red, report the corresponding honest fixer status.

## Report

Write only `crates/haze-sync-worktree/control/report.md` using `report-template.md`.

Set:

- `REPORT_TYPE: FIX`
- `phase_id: FIX-WT-P9-CI`

Use one of:

- `FIX_COMPLETE`
- `FIX_NEEDS_MORE`
- `FIX_BLOCKED_BY_LOGS`
- `FIX_BLOCKED_BY_CONTRACT`
- `FIX_BLOCKED_BY_TOOLING`

Fill `CI_DIAGNOSTICS` completely, including artifact name/id, run id/attempt, files read, and whether raw logs were used.
