# W1-FIX-SRV-P7A-CI — Server Worktree composition CI correction

Before starting, name this worker chat exactly:

`server — W1 SRV-P7A CI Fix`

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: fixer-worker

Work only through the GitHub connector. Do not merge the PR, change its draft state, rebase, reset, rewrite history, force-push, modify `main`, modify `component/worktree`, or change sibling branches.

## Context

SRV-P7A-FIX activated the Server-owned Worktree composition boundary and reached implementation self-acceptance, but the final code-bearing Component CI run is red.

Authoritative implementation evidence:

- code_bearing_sha: `fe9101871462fc271a320726f4ad18668d1a9a5b`
- workflow: `Component CI`
- workflow_run_id: `29160824666`
- run_number: `1703`
- workflow_run_attempt: `1`
- conclusion: `failure`
- failed check: `Finalize CI diagnostics`

The implementation report states that cargo fmt, cargo check, cargo test, and cargo clippy passed, including the now-compiled Worktree composition tests. Do not infer the exact failure from that visible summary.

Archived phase evidence:

- implementation prompt: `crates/haze-sync-server/control/log/20260711-171000Z-W1-SRV-P7A-FIX-implementation-worker-prompt.md`
- implementation report: `crates/haze-sync-server/control/log/20260711-171000Z-W1-SRV-P7A-FIX-implementation-worker-report.md`
- earlier recovery report: `crates/haze-sync-server/control/log/20260711-163000Z-W1-SRV-P7A-REPORT-RECOVERY-implementation-worker-report.md`

Use this exact diagnostics artifact:

- artifact_name: `ci-diag__component-server__wf-component-ci__run-29160824666__attempt-1`
- artifact_id: `8250773210`
- artifact_head_sha: `fe9101871462fc271a320726f4ad18668d1a9a5b`
- artifact_digest: `sha256:a02d9916569faa4457f8b3509b6b11908b8805f113b82c9aa96e58d10e1b7d6d`
- artifact_size_bytes: `1993`
- artifact_created_at: `2026-07-11T17:02:32Z`
- artifact_expires_at: `2026-07-12T17:02:32Z`
- artifact_status: available and unexpired when assigned

## Required reads

Before editing, read:

- project implementation manifest, report template, fixer-worker prompt, and GitHub connector guidance;
- current Server control state and this prompt;
- archived SRV-P7A-FIX prompt/report and earlier recovery report;
- changed Server source/tests involved in SRV-P7A-FIX;
- the exact diagnostics artifact specified above.

## Diagnostics protocol

Through the GitHub connector:

1. fetch artifact `8250773210` from workflow run `29160824666`;
2. read `summary.md` and `manifest.json` at the archive location where they actually exist;
3. read every failure marker and log named by `failed_checks`;
4. verify artifact head SHA exactly matches `fe9101871462fc271a320726f4ad18668d1a9a5b`;
5. apply only the minimum artifact-proven correction.

Raw GitHub job logs are not authorized as fallback. If the artifact is missing, expired, malformed, mismatched, or unreadable, report `FIX_BLOCKED_BY_LOGS` without guessing.

## Task

Fix only the artifact-proven cause of the SRV-P7A-FIX CI failure.

Preserve the accepted implementation:

- `worktree_runtime.rs` remains part of the active Server crate graph;
- production construction uses existing `ServerConfig.worktree.mode` and `.root`;
- lifecycle order remains explicit: construct, start once, retain for serve lifetime, shutdown once after both successful and failed serve completion;
- Disabled remains inert;
- enabled modes remain honestly unavailable with `CycleExecutorNotWired`;
- DryRun remains unsupported;
- no fake watcher, executor, polling loop, scan, import/export mutation, repair execution, provider call, or background task;
- no absolute worktree root, database URL, token, raw filesystem error, or internal debug payload is exposed;
- dependency-free routes and existing HTTP behavior remain unchanged;
- accepted Worktree product files remain unchanged.

## Allowed files

- `crates/haze-sync-server/src/**` only when required by the exact artifact-proven correction
- `crates/haze-sync-server/docs/implementation-log.md` only if the artifact-proven correction changes a documented fact
- `crates/haze-sync-server/control/report.md`

## Forbidden changes

- no changes under `crates/haze-sync-worktree/**`;
- no Cargo dependency/feature changes;
- no workflow changes;
- no Core/API/Storage/Common/GDrive/Obsidian/CLI/Deployment changes;
- no public route or DTO changes;
- no SRV-P7B executor/runtime expansion;
- no provider behavior, hard delete, destructive repair, hidden global, background task, test deletion, assertion weakening, or unrelated cleanup.

If the artifact proves a required correction outside the allowed scope, report `FIX_BLOCKED_BY_CONTRACT` rather than expanding scope.

## Checks and CI

Any source/test/docs fixer commit must run CI normally. CI skip is permitted only for a final report-only commit.

Do not claim success until a post-fix code-bearing Component CI run is observed green. If the exact correction is applied but CI remains pending or red, use the corresponding honest fixer status.

## Report

Write only `crates/haze-sync-server/control/report.md` using `report-template.md`.

Set:

- `REPORT_TYPE: FIX`
- `phase_id: FIX-SRV-P7A-CI`
- `chat_name: server — W1 SRV-P7A CI Fix`

Use one of:

- `FIX_COMPLETE`
- `FIX_NEEDS_MORE`
- `FIX_BLOCKED_BY_LOGS`
- `FIX_BLOCKED_BY_CONTRACT`
- `FIX_BLOCKED_BY_TOOLING`

Fill `CI_DIAGNOSTICS` completely: exact artifact name/id/digest, run id/attempt, files read, head-SHA verification, raw-log usage, applied correction, final code-bearing SHA, and post-fix workflow evidence.
