# W1-FIX-WT-P9C-CI — Worktree clean-review CI correction

Before starting, name this worker chat exactly:

`worktree — W1 WT-P9C CI Fix`

Component: worktree
Path: crates/haze-sync-worktree
Branch: component/worktree
PR: #49
Role: fixer-worker

Work only through the GitHub connector. Do not merge the PR, change its draft state, rebase, reset, rewrite history, or modify `main` or sibling branches.

## Context

WT-P9C clean-code review made Worktree-owned source/test/docs corrections and then reported `CLEAN_BLOCKED_BY_TOOLING`. The final review code-bearing head is:

- code_bearing_sha: `b9520336c97ccf0e1b1905c7c9ad1d3a4a2d061e`
- workflow: `Component CI`
- workflow_run_id: `29148826285`
- run_number: `1664`
- workflow_run_attempt: `1`
- conclusion: `failure`

Visible `cargo fmt`, `cargo check`, `cargo test`, and `cargo clippy` steps passed; the workflow failed during diagnostics finalization. Do not infer the exact cause from visible metadata.

Archived WT-P9C evidence:

- clean-review prompt: `crates/haze-sync-worktree/control/log/20260711-101500Z-W1-WT-P9C-clean-code-reviewer-prompt.md`
- clean-review report: `crates/haze-sync-worktree/control/log/20260711-101500Z-W1-WT-P9C-clean-code-reviewer-report.md`

Use this exact diagnostics artifact:

- artifact_name: `ci-diag__component-worktree__wf-component-ci__run-29148826285__attempt-1`
- artifact_id: `8247562500`
- artifact_head_sha: `b9520336c97ccf0e1b1905c7c9ad1d3a4a2d061e`
- artifact_digest: `sha256:49ceff07739604143f67fcac37606d86181dcb60f2a19f531ec9d1e498177533`
- artifact_expires_at: `2026-07-12T10:08:37Z`
- artifact_status: available and unexpired when assigned

## Read

Before editing, read:

- project `implementation-manifest.md` and `report-template.md` sources;
- the Fixer Worker process prompt and GitHub connector guidance;
- Worktree component contract, implementation plan, implementation log, dependency map, and decisions;
- this active prompt;
- the archived WT-P9C prompt/report referenced above;
- current WT-P9C doctor/repair-planning source, tests, docs, and the review diff;
- the exact diagnostics artifact specified above.

## Diagnostics protocol

Through the GitHub connector:

1. fetch artifact `8247562500` from workflow run `29148826285`;
2. read `summary.md` and `manifest.json` at the archive location where they actually exist;
3. read every failure marker and log listed in `failed_checks`;
4. verify the artifact head SHA matches `b9520336c97ccf0e1b1905c7c9ad1d3a4a2d061e`;
5. apply only the artifact-proven correction.

Raw GitHub job logs are not authorized as fallback. If the artifact is missing, expired, malformed, mismatched, or unreadable, report `FIX_BLOCKED_BY_LOGS` without guessing.

## Task

Fix only the minimum artifact-proven cause of the WT-P9C CI failure.

Preserve the accepted clean-review corrections:

- repair plans are descriptive and non-executing;
- plans cannot carry or imply durable host authorization;
- risky execution must be confirmed only after fresh-fact revalidation inside a future execution boundary;
- aggregate expired-echo facts propose safe rescan rather than an unaddressable destructive action;
- authoritative facts, managed-runtime skip behavior, validated vault-relative paths, and injected fact-source boundaries remain unchanged.

## Allowed files

- `crates/haze-sync-worktree/src/**`
- `crates/haze-sync-worktree/docs/**` only if required by the artifact-proven correction
- `crates/haze-sync-worktree/control/report.md`

## Forbidden changes

No Server wiring, repair executor, automatic mutation, provider behavior, direct database access, public absolute paths, CLI command, workflow/dependency changes, sibling changes, test deletion, assertion weakening, reintroduction of authorization state, or unrelated cleanup.

## Checks and CI

Any source/test/docs fixer commit must run CI normally. CI skip is permitted only for a final report-only commit.

Do not claim success until a post-fix code-bearing `Component CI` run is observed green. If the artifact-proven fix is applied but CI remains pending or red, use the corresponding honest fixer status.

## Report

Write only `crates/haze-sync-worktree/control/report.md` using `report-template.md`.

Set:

- `REPORT_TYPE: FIX`
- `phase_id: FIX-WT-P9C-CI`

Use one of:

- `FIX_COMPLETE`
- `FIX_NEEDS_MORE`
- `FIX_BLOCKED_BY_LOGS`
- `FIX_BLOCKED_BY_CONTRACT`
- `FIX_BLOCKED_BY_TOOLING`

Fill `CI_DIAGNOSTICS` completely, including artifact name/id, run id/attempt, files read, head-SHA verification, and whether raw logs were used.
