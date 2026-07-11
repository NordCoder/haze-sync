# W1-SRV-P7B2-LOCAL-SYNC — Local git merge and DB-capable CI

Before starting, name this worker chat exactly:

`server — W1 SRV-P7B2 Local Git Sync and PostgreSQL Run`

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: implementation-worker
Phase: SRV-P7B2-LOCAL-SYNC-DB-CI

## Explicit tooling authorization

Two connector-only phases proved that the available GitHub connector cannot safely create the required merge because it exposes neither a commit root-tree read nor a native branch-merge operation.

For this phase only, you are explicitly authorized to use a normal local git checkout and authenticated non-force push to synchronize `component/server`. Continue using the GitHub connector for PR, CI and report evidence.

Authorized local operations are limited to clone/fetch, checkout `component/server`, `git merge --no-ff origin/main`, genuine conflict resolution, merge-tree/parent inspection, and normal non-force push.

Forbidden: rebase, reset, cherry-picking main commits instead of merging, history rewrite, force-push, merging PR #45 into main, temporary write-enabled Actions bootstrap, or modifying sibling branches.

## Current evidence

- observed component/server head before this prompt rotation: `229baaf9cd00ca70ea294b6123b7e678ce50f272`;
- observed main head: `c1e69a664388b0cba028170e8398b9088218957d`;
- old merge base: `9ee3ced989bf60a71d0d7b37ff046118b0b2d1a2`;
- PR #45: open, draft, unmerged, `mergeable: false`, `merge_commit_sha: null`;
- accepted implementation SHA: `81e6f6ae69f4bda92d284991e4909457a6ef8e60`;
- post-source-fix SHA: `9304263f4be8234e513bf335dc886f96c53d0cce`;
- accepted PostgreSQL workflow tooling SHA: `d48bbd847c8b79511a7ac32cfcb14c671f3880c1`;
- exact prior artifact `8252500221` proved that the remaining failures were the three mandatory DB-backed parity tests with no database URL;
- prior connector-only branch-sync report: `BLOCKED_BY_TOOLING`.

Re-read both remote refs immediately before merging and use the fresh SHAs if either moved.

## Required merge

1. Begin from a clean local checkout.
2. Fetch current `origin/main` and `origin/component/server`.
3. Checkout the exact current component/server head.
4. Merge exact current `origin/main` using `git merge --no-ff`.
5. The merge commit must have the pre-merge component/server head as first parent and current main head as second parent.
6. Resolve `.github/workflows/component-ci.yml` semantically:
   - retain all current-main triggers, permissions, concurrency, component context, diagnostics wrapper/finalizer and artifact upload;
   - retain the accepted PostgreSQL 16 service, deterministic health check and synthetic test-only `HAZE_SYNC_TEST_DATABASE_URL` for `haze_sync_test`;
   - preserve the ordinary Rust workspace job and mandatory DB-backed Server tests.
7. Every other main-changed path since the old merge base must match current main exactly unless a genuine conflict proves otherwise:
   - `.github/docs/ci-diagnostics-artifacts.md`;
   - `.github/scripts/ci-finalize.sh`;
   - `.github/scripts/ci-run.sh`;
   - `.github/workflows/ci.yml`;
   - `.github/workflows/obsidian-plugin.yml`;
   - `.github/workflows/rust.yml`.
8. Preserve every accepted Server application-service, route, test, docs and control change.
9. Verify before push: clean tree, two parents in required order, no conflict markers, no temporary write permission/bootstrap workflow, final workflow `contents: read`, no product/test weakening.
10. Push normally to `origin/component/server` without force.

Do not create an ours-only or partial merge.

## Required GitHub and CI verification

After push, use the connector to verify:

- PR #45 remains open, draft and unmerged;
- PR becomes mergeable with a non-null merge commit SHA;
- branch head equals the local merge commit;
- Component CI runs for that exact head.

Acceptance requires PostgreSQL health plus green fmt, check, workspace tests, clippy and diagnostics finalizer. The green test run must include successful execution of:

- `application::tests::application_services_preserve_atomic_file_delete_and_read_semantics`;
- `routes::delete::tests::delegated_delete_route_preserves_tombstone_replay_and_stale_base_behavior`;
- `routes::v1::tests::delegated_routes_preserve_put_get_and_changes_wire_behavior`.

If CI is red, do not read diagnostics artifacts in this implementation role. Record the run, failed check and artifact metadata for an Orchestrator-assigned fixer.

If local git or authenticated non-force push is unavailable, return `BLOCKED_BY_TOOLING` immediately. Do not repeat connector-only tree construction or create temporary workflows.

## Preservation requirements

Preserve one reusable async `ServerApplicationServices` authority, routes as transport/auth/API adapters, Core policy ownership, passive caller-transaction-owned Storage, transaction/locking/idempotency/object-store/conflict/tombstone/operation-log semantics, public API parity, dependency-free router construction and accepted SRV-P7A lifecycle behavior.

No Worktree executor/host, provider behavior, schema/migration, public DTO, hard delete, fake repository, silent DB skip or test weakening is allowed.

## Report

Write `crates/haze-sync-server/control/report.md` using `report-template.md`.

Set:

- `REPORT_TYPE: IMPLEMENTATION`
- `phase_id: SRV-P7B2-LOCAL-SYNC-DB-CI`
- `chat_name: server — W1 SRV-P7B2 Local Git Sync and PostgreSQL Run`

Use one honest status:

- `SELF_ACCEPT`
- `SELF_ACCEPT_PENDING_CI`
- `SELF_NEEDS_FIX`
- `BLOCKED_BY_TOOLING`
- `BLOCKED_BY_SCOPE`
- `BLOCKED_BY_CONTRACT`

The report must include local tooling, fresh refs, merge base, merge commit and ordered parents, conflict resolution, proof current-main files were incorporated, final workflow assessment, push evidence, PR mergeability, exact CI and DB-test evidence, secrecy assessment, and readiness for mandatory SRV-P7B2 clean-code review.