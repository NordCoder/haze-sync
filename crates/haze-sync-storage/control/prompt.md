# W1-FIX-STOR-P10-CI — Durable Worktree state CI correction

Before starting, name this worker chat exactly:

`storage — W1 STOR-P10 CI Fix`

Component: storage
Path: crates/haze-sync-storage
Branch: component/storage
PR: #47
Role: fixer-worker

Work only through the GitHub connector. Do not merge the PR, change its draft state, rebase, reset, rewrite history, force-push, modify `main`, or modify sibling branches.

## Trigger and implementation evidence

STOR-P10 completed its scoped migration, model, repository, cursor and test-support implementation, but its authoritative Component CI is red at diagnostics finalization.

Evidence:

- implementation phase: `STOR-P10`
- implementation status: `BLOCKED_BY_TOOLING`
- final source/schema/test/docs SHA: `63d80764933cba5f23fb43bad44201a75e1dc16a`
- workflow: `Component CI`
- workflow run id: `29166287661`
- run number: `1765`
- attempt: `1`
- conclusion: `failure`
- visible passing steps: cargo fmt, cargo check, cargo test, cargo clippy
- visible failing stage: Finalize CI diagnostics

Archived phase evidence:

- prompt snapshot: `crates/haze-sync-storage/control/log/20260711-201500Z-W1-STOR-P10-implementation-worker-prompt.md`
- report snapshot: `crates/haze-sync-storage/control/log/20260711-201500Z-W1-STOR-P10-implementation-worker-report.md`
- exact implementation report blob: `d51b6039fced1a0c7b245d1d5d863eeb2d215724`

The accepted implementation includes:

- root migration `migrations/0010_worktree_durable_state.sql` with fail-before-destructive handling for non-empty legacy path-only state;
- versioned Worktree instance binding keyed by adapter id and root fingerprint;
- per-instance present/tombstoned path state;
- bounded deterministic snapshots and guarded observations;
- caller-transaction-owned cursor lock/initialize/exact contiguous advance;
- safe redacted repository errors;
- migration-aware test support and explicit strict ignored PostgreSQL acceptance tests.

Do not redesign this contract or silently reinterpret legacy rows.

## Exact diagnostics artifact

Use only:

- artifact name: `ci-diag__component-storage__wf-component-ci__run-29166287661__attempt-1`
- artifact id: `8252246409`
- artifact head SHA: `63d80764933cba5f23fb43bad44201a75e1dc16a`
- artifact digest: `sha256:c2e86e52d1bd342fd577503042c4e4640591179a08fae678de1a27d22d4e0f2c`
- artifact size bytes: `13317`
- created at: `2026-07-11T20:03:15Z`
- expires at: `2026-07-12T20:03:15Z`
- status when assigned: available and unexpired

## Required reads and diagnostics protocol

Read the implementation manifest, report template, fixer-worker prompt, GitHub connector guidance, current Storage state/prompt, archived STOR-P10 evidence, changed Storage source/schema/tests/docs, and the exact artifact.

Then:

1. fetch artifact `8252246409` from run `29166287661`;
2. verify the artifact head SHA exactly matches `63d80764933cba5f23fb43bad44201a75e1dc16a`;
3. read `summary.md`, `manifest.json`, every failure marker and every log listed by `failed_checks`;
4. apply only the complete minimum artifact-proven correction.

Raw GitHub job logs are not authorized as fallback. If the artifact is missing, expired, malformed, mismatched or unreadable, report `FIX_BLOCKED_BY_LOGS` without guessing.

## Separate PostgreSQL evidence blocker

STOR-P10 also requires strict live-PostgreSQL acceptance evidence. The implementation report states that the mandatory ignored tests were not run because the current Component CI had no dedicated `HAZE_SYNC_TEST_DATABASE_URL`.

This fixer phase is primarily artifact-driven. Do not falsely treat ordinary `cargo test` success with ignored PostgreSQL tests as phase acceptance.

- If the artifact-proven failure is independent of the missing DB evidence, fix it minimally and obtain a green post-fix code-bearing Component CI run; the next gate remains a dedicated DB-capable STOR-P10 verification phase before clean-code review.
- If the artifact itself proves that the failing workflow is specifically caused by the missing required DB execution, report the exact tooling requirement and only make workflow/test-harness changes if they are both artifact-proven and explicitly within Storage ownership. Do not invent credentials, commit secrets or silently downgrade mandatory tests to optional.
- Do not claim `CLEAN_ACCEPT` or full STOR-P10 acceptance in this fixer report.

## Preservation requirements

Preserve unless the exact artifact proves a correction is required:

- fail-closed instance fingerprint/version binding;
- no raw root persistence or rendering;
- per-adapter path isolation;
- present/tombstoned revision/hash invariants;
- bounded deterministic snapshot ordering;
- caller-owned SQLx transaction semantics;
- no hard delete or implicit cleanup;
- exact expected N to N+1 cursor advancement;
- rejection of cursor regression, gap, stale expected value and overflow;
- rollback without false state/cursor claims;
- deterministic fail-safe legacy migration behavior;
- safe redacted errors and Debug output;
- Storage-passive ownership with no Server runtime, Worktree filesystem, Core policy or API DTO behavior.

## Allowed files

Only artifact-proven changes within:

- `migrations/0010_worktree_durable_state.sql` or directly related Storage migration validation;
- STOR-P10 Storage models/repositories/schema exports and tests;
- Storage test-support schema/strict PostgreSQL harness used by STOR-P10;
- STOR-P10 Storage docs if a corrected fact changes;
- `crates/haze-sync-storage/control/report.md`.

Do not modify Server, Worktree, Core, API, CLI, Deployment, provider behavior, production migration execution policy, unrelated migrations, or sibling control files. Do not archive control files. Do not commit secrets or a real database URL.

## CI and report

All source/schema/test/docs changes must run normal Component CI. CI skip is allowed only for the final report-only commit.

Write `crates/haze-sync-storage/control/report.md` using `report-template.md` with:

- `REPORT_TYPE: FIX`
- `phase_id: FIX-STOR-P10-CI`
- `chat_name: storage — W1 STOR-P10 CI Fix`

Use an honest status:

- `FIX_COMPLETE`
- `FIX_NEEDS_MORE`
- `FIX_BLOCKED_BY_LOGS`
- `FIX_BLOCKED_BY_SCOPE`
- `FIX_BLOCKED_BY_CONTRACT`
- `FIX_BLOCKED_BY_TOOLING`

The report must include artifact verification, exact failure cause, changed files, behavior/migration impact, post-fix code-bearing SHA and CI run, whether strict PostgreSQL tests actually ran, and the exact remaining gate. Even after `FIX_COMPLETE`, STOR-P10 must not advance to clean-code review until a dedicated DB-capable verification has produced the required migration/repository/cursor evidence.
