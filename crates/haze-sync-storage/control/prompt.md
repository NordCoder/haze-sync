# W1-STOR-P10-LOCAL-SYNC — Local git merge and PostgreSQL verification

Before starting, name this worker chat exactly:

`storage — W1 STOR-P10 Local Git Sync and PostgreSQL Run`

Component: storage
Path: crates/haze-sync-storage
Branch: component/storage
PR: #47
Role: implementation-worker
Phase: STOR-P10-LOCAL-SYNC-DB-CI

## Explicit tooling authorization

Two connector-only synchronization passes proved that the available GitHub connector cannot safely create the required merge: it exposes create-tree/create-commit/update-ref but neither the root tree SHA nor a native branch-merge operation. The bounded temporary Actions bootstrap also failed and was removed.

For this phase only, you are explicitly authorized to use a normal local git checkout and authenticated non-force push for branch synchronization. Use the GitHub connector for repository evidence, PR metadata, CI metadata, and the final report.

Authorized local operations are limited to:

- clone/fetch the repository;
- checkout `component/storage`;
- fetch current `origin/main` and `origin/component/storage`;
- perform `git merge --no-ff origin/main`;
- resolve only genuine merge conflicts;
- inspect the resulting merge tree and parents;
- push `component/storage` without force.

Do not use rebase, reset, cherry-pick of main commits, history rewriting, force-push, merge PR #47 into main, or modify another component branch.

## Current evidence

- current observed component/storage head before this prompt rotation: `1c477695e4d411eb5b6b2866bff7061770d8366c`;
- current observed main head: `c1e69a664388b0cba028170e8398b9088218957d`;
- old merge base: `9ee3ced989bf60a71d0d7b37ff046118b0b2d1a2`;
- PR #47: open, draft, unmerged, `mergeable: false`, `merge_commit_sha: null`;
- accepted Storage code-bearing SHA before DB tooling: `abca69058390983894465cef9d66c38960fae4c7`;
- ordinary green Component CI: run `29167108216`, run number `1792`;
- canonical safe Storage workflow restoration SHA: `f48a666d1d77377ebfef3329e5a015bd90800533`;
- canonical Storage Component CI workflow blob: `01db4cfa0af1ced8cd9070d83a987c5aa4259222`;
- prior connector-only report: `BLOCKED_BY_TOOLING`.

Re-read all refs immediately before the merge. If main or component/storage moved, use the fresh refs and report the exact values.

## Required merge

1. Start from a clean local checkout with no untracked/generated files.
2. Fetch both refs from origin.
3. Checkout the exact current `origin/component/storage` head as local branch `component/storage`.
4. Merge the exact current `origin/main` with `git merge --no-ff`.
5. The merge commit must have:
   - first parent: the pre-merge component/storage head;
   - second parent: the merged current main head.
6. Resolve `.github/workflows/component-ci.yml` semantically:
   - retain all current-main ordinary CI, diagnostics scripts/finalization, triggers, permissions and concurrency;
   - retain the accepted Storage-only `storage-postgres` job;
   - retain `postgres:16-alpine`, synthetic test-only credentials, deterministic readiness, `HAZE_SYNC_TEST_DATABASE_URL`, `RUST_TEST_THREADS=1`, exact strict command, four-test evidence verification and distinct diagnostics slug.
7. Every other path changed by main since the merge base must match current main exactly unless a genuine non-workflow conflict proves otherwise:
   - `.github/docs/ci-diagnostics-artifacts.md`;
   - `.github/scripts/ci-finalize.sh`;
   - `.github/scripts/ci-run.sh`;
   - `.github/workflows/ci.yml`;
   - `.github/workflows/obsidian-plugin.yml`;
   - `.github/workflows/rust.yml`.
8. Preserve every existing Storage product, migration, repository, test, documentation and control file.
9. Verify with local git before push:
   - clean index/worktree;
   - exactly two merge parents in the required order;
   - no conflict markers;
   - no deleted/rewritten Storage implementation;
   - no temporary contents-write/bootstrap workflow;
   - final component-ci workflow remains `contents: read`.
10. Push normally with no force and verify origin points to the merge commit.

Do not create a fake/ours-only merge. Do not produce a merge commit until the full merged tree is correct.

## Required GitHub and CI verification

After push, use the GitHub connector to verify:

- PR #47 remains open, draft and unmerged;
- PR becomes mergeable and has a non-null merge commit SHA;
- the branch head equals the pushed two-parent merge commit;
- Component CI is scheduled for that exact branch head.

Acceptance requires both jobs green:

1. ordinary Rust workspace: fmt, check, workspace tests, clippy and diagnostics finalizer;
2. Storage PostgreSQL verification:
   - PostgreSQL readiness succeeds;
   - exact command executes: `cargo test -p haze-sync-storage --features test-support -- --ignored`;
   - these tests are visibly successful:
     - `repositories::adapter_cursors::postgres_tests::exact_cursor_progression_is_locked_contiguous_and_rollback_safe`;
     - `repositories::worktree_state::postgres_tests::durable_instances_and_path_state_are_isolated_and_transactional`;
     - `test_support::postgres::tests::fresh_and_current_schema_preparation_is_idempotent`;
     - `test_support::postgres::tests::migrates_empty_pre_p10_schema_and_rejects_nonempty_legacy_state`;
   - diagnostics finalizer succeeds.

If CI is red, do not read its diagnostics artifact in this implementation role. Record exact run/job/check and artifact metadata for an Orchestrator-assigned fixer.

If local git or authenticated non-force push is unavailable, report `BLOCKED_BY_TOOLING` immediately. Do not retry connector-only tree construction or create another temporary workflow.

## Preservation requirements

Preserve all accepted STOR-P10 semantics, including fail-before-destructive migration behavior, root-fingerprint secrecy, per-instance present/tombstoned state, transaction ownership, deterministic snapshots, exact contiguous cursor advancement and rollback safety. No hard delete, production credential, test weakening or sibling-component change is allowed.

## Report

Write `crates/haze-sync-storage/control/report.md` using `report-template.md`.

Set:

- `REPORT_TYPE: IMPLEMENTATION`
- `phase_id: STOR-P10-LOCAL-SYNC-DB-CI`
- `chat_name: storage — W1 STOR-P10 Local Git Sync and PostgreSQL Run`

Use one honest status:

- `SELF_ACCEPT`
- `SELF_ACCEPT_PENDING_CI`
- `SELF_NEEDS_FIX`
- `BLOCKED_BY_TOOLING`
- `BLOCKED_BY_SCOPE`
- `BLOCKED_BY_CONTRACT`

The report must include local tooling used, exact fetched refs, old head, main head, merge base, merge commit and ordered parents, conflict resolution, proof main files were incorporated, final workflow blob/content assessment, push evidence, PR mergeability, exact CI jobs/runs, strict DB test evidence, secrecy assessment, and readiness for mandatory STOR-P10 clean-code review.