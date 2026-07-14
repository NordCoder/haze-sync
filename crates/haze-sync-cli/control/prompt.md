# W1-FIX-CLI-P6A-CI — Diagnose and fix exact CLI-P6A CI failure

Before starting, name this worker chat exactly:

`cli — W1 CLI-P6A CI Diagnostics Fix`

Component: cli
Path: crates/haze-sync-cli
Branch: component/cli
PR: #48
Role: fixer-worker
Phase: FIX-CLI-P6A-CI

Do not merge, change draft state, rewrite history, modify sibling branches, begin Deployment work, or perform unrelated cleanup.

## Candidate

- code-bearing SHA: `6c4c2ba4a66999e02542083512587d0d6ad8d437`;
- implementation report blob: `5df1ebcc3696b1d61d514e005b2dbaebe4213622`;
- Component CI run: `29331452103`, number `1946`, attempt `1`;
- diagnostics artifact ID: `8310157600`;
- artifact name: `ci-diag__component-cli__wf-component-ci__run-29331452103__attempt-1`;
- artifact digest: `sha256:49a5d411691f424fcd8aec8893086846674f4707e580f58f0302c10f1d4770d3`;
- artifact expires: `2026-07-15T12:11:04Z`.

## Required diagnosis

1. Download artifact `8310157600` first.
2. Read `summary.md`, `manifest.json`, and every log listed in `failed_checks`.
3. Treat artifact evidence as authoritative; do not guess from UI step summaries or the implementation report.
4. Use raw job logs only if the artifact is missing, corrupt, incomplete or inconsistent. If usable evidence is unavailable, report `FIX_BLOCKED_BY_LOGS`.
5. Identify the smallest exact cause and apply only the evidence-backed correction.

## Scope policy

CLI-P6A product files are protected unless the artifact directly identifies one of them:

- `crates/haze-sync-cli/Cargo.toml`;
- `crates/haze-sync-cli/src/commands.rs`;
- `crates/haze-sync-cli/src/main.rs`;
- `crates/haze-sync-cli/src/worktree_api.rs`;
- minimal focused CLI tests or `output.rs` if directly named by diagnostics.

Accepted API-P8 blobs are strictly protected and must remain byte-identical unless the artifact proves the copied source differs from the pinned accepted blob—which would be a fan-in correction, not a semantic edit:

- `crates/haze-sync-api/src/dto/worktree.rs`;
- `crates/haze-sync-api/src/dto/mod.rs`;
- `crates/haze-sync-api/src/dto/public_contract_tests.rs`;
- `crates/haze-sync-api/src/routes/worktree.rs`;
- `crates/haze-sync-api/src/routes/mod.rs`;
- accepted fixture/test/doc files.

Do not:

- weaken, skip or suppress diagnostics;
- force success or change CI semantics;
- add retries, polling, waits, tasks or fake-success transport;
- change Server/Worktree/Storage/Core/GDrive/Deployment product files;
- perform broad refactors or unrelated cleanup.

Formatting/style alone is non-blocking after the exact evidence-backed correction and green CI.

## Completion

Create a real code/tooling-bearing commit without CI skip and obtain authoritative Component CI success on the exact final SHA.

Write `crates/haze-sync-cli/control/report.md` with:

- `REPORT_TYPE: FIX`;
- `phase_id: FIX-CLI-P6A-CI`;
- `chat_name: cli — W1 CLI-P6A CI Diagnostics Fix`;
- status `FIX_COMPLETE`, `FIX_NEEDS_MORE_WORK`, `FIX_BLOCKED_BY_LOGS`, `FIX_BLOCKED_BY_SCOPE`, or `FIX_BLOCKED_BY_TOOLING`.

Record artifact files read, exact failed check/root cause, changed paths, whether accepted API blobs remained unchanged, final SHA, and exact CI evidence. Do not claim CLEAN_ACCEPT; a focused CLI functional review follows.
