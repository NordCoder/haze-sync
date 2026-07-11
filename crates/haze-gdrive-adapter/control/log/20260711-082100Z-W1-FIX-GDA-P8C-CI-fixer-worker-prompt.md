# W1-FIX-GDA-P8C-CI — GDrive diagnostics CI correction

Before starting, name this worker chat exactly:

`gdrive-adapter — W1 GDA-P8C CI Fix`

Component: gdrive-adapter
Path: crates/haze-gdrive-adapter
Branch: component/gdrive-adapter
PR: #50
Role: fixer-worker

Work only through the GitHub connector. Do not merge the PR, change its draft state, rebase, reset, rewrite history, or modify `main` or sibling branches.

## Context

GDA-P8C clean-code review completed inside GDrive Adapter scope. The final code-bearing source/test commit is:

- code_bearing_sha: `cfaf931bfa3ec58b19924b535475859300216272`
- workflow: `Component CI`
- workflow_run_id: `29143925456`
- run_number: `1653`
- workflow_run_attempt: `1`
- conclusion: `failure`

Visible Rust validation steps passed, but the workflow failed during diagnostics finalization. The clean-code review report is archived at:

- `crates/haze-gdrive-adapter/control/log/20260711-071400Z-W1-GDA-P8C-clean-code-reviewer-report.md`

Use this exact diagnostics artifact:

- artifact_name: `ci-diag__component-gdrive-adapter__wf-component-ci__run-29143925456__attempt-1`
- artifact_id: `8246115447`
- artifact_head_sha: `cfaf931bfa3ec58b19924b535475859300216272`
- artifact_status: available and unexpired when assigned

## Read

Before editing, read:

- project `implementation-manifest.md` and `report-template.md` sources;
- the Fixer Worker process prompt;
- GDrive Adapter component contract, implementation plan, implementation log, dependency map, and decisions;
- this active prompt;
- the archived GDA-P8C clean-code report referenced above;
- the GDA-P8/GDA-P8C source and tests relevant to the failure;
- the exact diagnostics artifact specified above.

Do not infer the failure from workflow metadata alone.

## Diagnostics protocol

Through the GitHub connector:

1. fetch artifact `8246115447` from workflow run `29143925456`;
2. read `ci-diagnostics/summary.md`;
3. read `ci-diagnostics/manifest.json`;
4. read every failure marker and log listed in `failed_checks`;
5. verify the artifact head SHA matches the code-bearing SHA above.

Raw GitHub job logs are not authorized as fallback in this prompt. If the artifact is missing, expired, malformed, mismatched, or unreadable, report `FIX_BLOCKED_BY_LOGS` without guessing.

## Task

Fix only the minimum artifact-proven cause of the failed Component CI run.

- Stay within GDrive Adapter component ownership.
- Preserve conservative delete-candidate semantics, current-state revalidation, Core arbitration, dry-run immutability, identity-aware recovery/retirement, and mass-delete safety.
- Do not weaken tests or assertions merely to obtain green CI.
- Do not add Drive trash/hard delete, live credentials/provider calls, direct Storage/DB ownership, concrete Server/API transport, unaudited manual unlock, background scheduling, or unrelated product behavior.
- If the artifact proves that the correction belongs to a workflow, shared CI infrastructure, Storage, Server, Core, API, Deployment, or another component, do not cross the boundary; report `FIX_BLOCKED_BY_CONTRACT` with the exact required owner and evidence.

## Allowed files

- `crates/haze-gdrive-adapter/src/**`
- `crates/haze-gdrive-adapter/docs/**` only when required to keep documentation accurate after an artifact-proven code correction
- `crates/haze-gdrive-adapter/control/report.md`

## Checks and CI

Re-run the artifact-proven failing check when possible. Any source/test/docs fix commit must run CI normally. CI skip is permitted only for a final report-only commit.

Do not claim success until a post-fix code-bearing `Component CI` run is observed green. If code is corrected but CI remains pending or red, report the corresponding honest fixer status.

## Report

Write only `crates/haze-gdrive-adapter/control/report.md` using `report-template.md`.

Set:

- `REPORT_TYPE: FIX`
- `phase_id: FIX-GDA-P8C-CI`

Use one of:

- `FIX_COMPLETE`
- `FIX_NEEDS_MORE`
- `FIX_BLOCKED_BY_LOGS`
- `FIX_BLOCKED_BY_CONTRACT`
- `FIX_BLOCKED_BY_TOOLING`

Fill `CI_DIAGNOSTICS` completely, including artifact name/id, run id/attempt, files read, and whether raw logs were used.
