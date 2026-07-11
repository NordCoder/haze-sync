# W1-STOR-P10-BRANCH-SYNC — Synchronize with main and execute PostgreSQL verification

Before starting, name this worker chat exactly:

`storage — W1 STOR-P10 Branch Sync and PostgreSQL Run`

Component: storage
Path: crates/haze-sync-storage
Branch: component/storage
PR: #47
Role: implementation-worker
Phase: STOR-P10-BRANCH-SYNC-DB-CI

Work through the GitHub connector. Do not merge PR #47 into `main`, change its draft state, rebase, reset, rewrite history, force-push, modify sibling branches, or discard accepted Storage changes.

## Trigger

STOR-P10 implementation and artifact correction are complete. A Storage-only PostgreSQL verification job was added, but GitHub did not create a pull-request workflow run because PR #47 is currently non-mergeable.

Accepted evidence:

- accepted post-fix Storage code-bearing SHA: `abca69058390983894465cef9d66c38960fae4c7`;
- ordinary green Component CI: run `29167108216`, run number `1792`;
- verification tooling/docs SHA before its report-only commit: `a9b916d740439f8ceb4d7e2a4b9beebb962857fb`;
- verification report status: `BLOCKED_BY_TOOLING`;
- PR #47 was observed open, draft, unmerged, and `mergeable: false`;
- current observed `main` head: `c1e69a664388b0cba028170e8398b9088218957d`;
- current merge base before synchronization: `9ee3ced989bf60a71d0d7b37ff046118b0b2d1a2`.

The previous verification phase already added a Storage-only `storage-postgres` job that provisions `postgres:16-alpine`, binds a synthetic test-only `HAZE_SYNC_TEST_DATABASE_URL`, executes `cargo test -p haze-sync-storage --features test-support -- --ignored`, and verifies the four mandatory STOR-P10 test names. Do not redesign that job unless synchronization or live execution proves a defect.

## Goal

Create a real, non-force, two-parent merge of current `main` into `component/storage`, resolve the CI-file conflict without losing either side, restore PR mergeability, and obtain an authoritative pull-request Component CI run that executes both:

1. the ordinary Rust workspace job; and
2. the Storage PostgreSQL verification job.

A fabricated merge commit whose tree simply ignores `main` changes is forbidden.

## Required synchronization protocol

1. Re-read the current heads of `component/storage` and `main` immediately before constructing the merge.
2. Recompute the merge base and compare merge-base..main and merge-base..component/storage.
3. If `main` moved beyond the observed head, include every new main change and document it. Do not rely blindly on the file list below.
4. At the observed main head, main changed exactly these paths since the old merge base:
   - `.github/docs/ci-diagnostics-artifacts.md`
   - `.github/scripts/ci-finalize.sh`
   - `.github/scripts/ci-run.sh`
   - `.github/workflows/ci.yml`
   - `.github/workflows/component-ci.yml`
   - `.github/workflows/obsidian-plugin.yml`
   - `.github/workflows/rust.yml`
5. The final merge tree must contain exact current-main content for every main-changed path not intentionally extended by this Storage phase.
6. `.github/workflows/component-ci.yml` must be a semantic merge of current main plus the accepted Storage-only `storage-postgres` job. Preserve main's ordinary Rust job, diagnostics wrapper/finalizer, artifact upload, pull-request trigger, workflow dispatch, permissions, and concurrency behavior.
7. Preserve every Storage product/schema/test/docs/control change already present on the component branch.
8. Construct a true merge commit with:
   - first parent: the current component/storage head at execution time;
   - second parent: the current main head at execution time;
   - a merged tree containing both sides' accepted changes.
9. Move `component/storage` to the new merge commit using a fast-forward ref update only. Never force-update.
10. Verify PR #47 becomes mergeable and obtains a non-null merge commit SHA.

GitHub git-data operations such as tree creation, multi-parent commit creation, and non-force ref update are authorized for this exact synchronization. Do not create an ours-only merge or omit non-conflicting main changes.

## Required CI execution

After the merge commit updates the branch:

- observe the new pull-request Component CI run associated with the synchronized branch head;
- require the ordinary Rust workspace job to pass fmt, check, workspace tests, clippy, and diagnostics finalization;
- require the Storage PostgreSQL job to provision PostgreSQL successfully and execute exactly:
  `cargo test -p haze-sync-storage --features test-support -- --ignored`;
- require evidence checks for these four tests to pass:
  - `repositories::adapter_cursors::postgres_tests::exact_cursor_progression_is_locked_contiguous_and_rollback_safe`;
  - `repositories::worktree_state::postgres_tests::durable_instances_and_path_state_are_isolated_and_transactional`;
  - `test_support::postgres::tests::fresh_and_current_schema_preparation_is_idempotent`;
  - `test_support::postgres::tests::migrates_empty_pre_p10_schema_and_rejects_nonempty_legacy_state`;
- require both diagnostics finalizers to pass.

If CI is red, record the exact synchronized code-bearing/tooling SHA, run id, run number, failed job/check, and diagnostics artifact metadata. Do not read diagnostics artifacts in this implementation role and do not guess.

If GitHub still does not schedule a run after a genuine merge commit and PR mergeability is restored, report `BLOCKED_BY_TOOLING` with exact evidence. Do not create meaningless commits repeatedly.

## Preservation requirements

Preserve:

- migration `0010_worktree_durable_state.sql` fail-before-destructive behavior;
- versioned adapter/root-fingerprint binding without raw-root persistence;
- per-instance present/tombstoned state;
- bounded deterministic snapshots;
- caller-owned transaction semantics;
- exact contiguous cursor advancement and rollback safety;
- no hard delete, implicit cleanup, or production database credentials;
- Storage-passive ownership and all existing tests/assertions.

Do not modify Server, Worktree, Core, API, CLI, Deployment, provider behavior, production migration policy, or unrelated workflow logic.

## Allowed files and operations

- the merge commit and exact conflict resolutions required to synchronize current main;
- `.github/workflows/component-ci.yml` only for preserving the accepted Storage PostgreSQL job over current main;
- Storage verification docs only if synchronization changes a documented fact;
- `crates/haze-sync-storage/control/report.md`.

Do not archive control files.

## Report

Write `crates/haze-sync-storage/control/report.md` using `report-template.md`.

Set:

- `REPORT_TYPE: IMPLEMENTATION`
- `phase_id: STOR-P10-BRANCH-SYNC-DB-CI`
- `chat_name: storage — W1 STOR-P10 Branch Sync and PostgreSQL Run`

Use one honest status:

- `SELF_ACCEPT`
- `SELF_ACCEPT_PENDING_CI`
- `SELF_NEEDS_FIX`
- `BLOCKED_BY_TOOLING`
- `BLOCKED_BY_SCOPE`
- `BLOCKED_BY_CONTRACT`

The report must include old/new branch head, exact main head and merge base, both merge parents, all main-changed paths incorporated, conflict resolutions, final workflow SHA, PR mergeability after sync, exact CI run/jobs, strict PostgreSQL command and four-test evidence, final accepted code-bearing/tooling SHA, secrecy assessment, and whether STOR-P10 is ready for mandatory clean-code review.