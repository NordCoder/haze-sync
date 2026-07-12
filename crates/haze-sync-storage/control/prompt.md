# W1-FIX-STOR-P10-CLEAN-CI — Resolve clean-review correction CI failure

Before starting, name this worker chat exactly:

`storage — W1 STOR-P10 Clean Correction CI Fix`

Component: storage
Path: crates/haze-sync-storage
Branch: component/storage
PR: #47
Role: fixer-worker
Phase: FIX-STOR-P10-CLEAN-CI

Work through the GitHub connector. Do not merge PR #47 into main, change its draft state, rebase, reset, rewrite history, force-push, modify sibling branches, or read raw GitHub job logs.

## Trigger and accepted baseline

The mandatory STOR-P10 clean-code review found one genuine evidence gap and corrected it without changing production behavior:

- added `crates/haze-sync-storage/tests/stor_p10_migration_guard.rs`;
- directly executes migration 0010 against non-empty legacy state inside a savepoint;
- proves the explicit guard error, intact legacy row/table shape, and absence of `worktree_instances`;
- extended the Storage PostgreSQL CI evidence check from four to five mandatory tests;
- updated Storage test-support documentation.

Clean-review correction code/tooling SHA:

`a6f1edf48d23c767f0f9b34ab28aacd8bd586000`

Authoritative Component CI:

- run id: `29172303439`;
- run number: `1825`;
- attempt: `1`;
- conclusion: `failure`.

Visible job evidence:

- `Storage PostgreSQL verification`: success;
- PostgreSQL readiness: success;
- exact strict command succeeded: `cargo test -p haze-sync-storage --features test-support -- --ignored`;
- all five mandatory evidence checks succeeded;
- Storage PostgreSQL diagnostics finalizer: success;
- ordinary Rust `cargo fmt`: success wrapper;
- ordinary Rust `cargo check`: success wrapper;
- ordinary Rust `cargo test`: success wrapper;
- ordinary Rust `cargo clippy`: success wrapper;
- ordinary Rust `Finalize CI diagnostics`: failure;
- diagnostics upload: success.

Do not infer the underlying failed marker from wrapper summaries. Read only the exact authorized artifact.

## Exact diagnostics artifact

Use only:

- artifact name: `ci-diag__component-storage__wf-component-ci__run-29172303439__attempt-1`;
- artifact id: `8253874582`;
- artifact head SHA: `a6f1edf48d23c767f0f9b34ab28aacd8bd586000`;
- artifact digest: `sha256:41c87f30baa40ab384db02ff7884895d2727a94dcb50ea4e5b9b410fd467bf34`;
- artifact size bytes: `1824`;
- created at: `2026-07-11T23:35:00Z`;
- expires at: `2026-07-12T23:34:59Z`;
- status when assigned: available and unexpired.

## Diagnostics protocol

1. Fetch artifact `8253874582` from run `29172303439`.
2. Verify exact artifact head SHA and digest.
3. Read `summary.md`, `manifest.json`, every failure marker, and every log named in `failed_checks`.
4. Identify the exact failed underlying command or stale marker.
5. Apply the complete minimum artifact-proven correction.

Raw GitHub job logs are forbidden as fallback. If the artifact is missing, expired, malformed, mismatched, or unreadable, report `FIX_BLOCKED_BY_LOGS` without guessing.

## Preservation requirements

Preserve unless the artifact proves a defect:

- migration `0010_worktree_durable_state.sql` and its fail-before-destructive behavior;
- direct savepoint-backed non-empty legacy SQL-guard test;
- all five mandatory strict PostgreSQL evidence checks;
- versioned adapter/root-fingerprint binding without raw-root persistence or rendering;
- per-instance present/tombstoned path-state invariants;
- deterministic bounded snapshots;
- passive caller-transaction-owned repository behavior;
- exact-contiguous cursor locking, checked progression, rollback and race semantics;
- dedicated test-only database URL validation and redaction;
- current-main synchronization and `contents: read` workflow permissions;
- synthetic ephemeral PostgreSQL credentials only;
- no production behavior, schema, repository API, hard-delete, implicit cleanup, repair, or sibling-component changes;
- no test deletion, ignoring, conditional skip, assertion weakening, or removal of the fifth evidence test.

## Allowed files

Only artifact-proven changes within:

- STOR-P10 test files and test-support code if the artifact proves a test defect;
- `.github/scripts/ci-run.sh` or `.github/scripts/ci-finalize.sh` if the artifact proves diagnostics-state behavior;
- `.github/workflows/component-ci.yml` if the artifact proves a narrowly scoped wrapper/finalizer integration defect;
- Storage test-support documentation if a corrected fact changes;
- `crates/haze-sync-storage/control/report.md`.

Do not modify Server, Worktree, Core, API, Common, provider, CLI, Deployment, migration SQL unless the artifact proves a migration defect, public contracts, or unrelated workflows. Do not archive control files.

## Acceptance

`FIX_COMPLETE` requires:

- every artifact-listed failed check addressed;
- no stale/false diagnostics marker remains;
- the ordinary Rust workspace job is fully green;
- the Storage PostgreSQL job remains fully green;
- the exact strict ignored-test command executes;
- all five mandatory test evidence checks remain successful;
- both diagnostics finalizers succeed;
- no regression in STOR-P10 semantics or clean-review correction.

## Report

Write `crates/haze-sync-storage/control/report.md` using `report-template.md`.

Set:

- `REPORT_TYPE: FIX`
- `phase_id: FIX-STOR-P10-CLEAN-CI`
- `chat_name: storage — W1 STOR-P10 Clean Correction CI Fix`

Use one honest status:

- `FIX_COMPLETE`
- `FIX_NEEDS_MORE`
- `FIX_BLOCKED_BY_LOGS`
- `FIX_BLOCKED_BY_SCOPE`
- `FIX_BLOCKED_BY_CONTRACT`
- `FIX_BLOCKED_BY_TOOLING`

The report must include artifact verification, exact failed checks and root cause, changed files, preservation assessment, final code-bearing/tooling SHA, post-fix Component CI run and both job conclusions, five-test evidence, secrecy assessment, and whether STOR-P10 can return for final clean acceptance.