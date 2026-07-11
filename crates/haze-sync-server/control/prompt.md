# W1-SRV-P7B2-BRANCH-SYNC — Synchronize with main and execute DB-capable CI

Before starting, name this worker chat exactly:

`server — W1 SRV-P7B2 Branch Sync and PostgreSQL Run`

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: implementation-worker
Phase: SRV-P7B2-BRANCH-SYNC-DB-CI

Work through the GitHub connector. Do not merge PR #45 into `main`, change its draft state, rebase, reset, rewrite history, force-push, modify sibling branches, or discard accepted Server changes.

## Trigger

SRV-P7B2 application services and source-level artifact corrections are complete. PostgreSQL provisioning was added to Component CI, but GitHub did not create a pull-request run because PR #45 is currently non-mergeable.

Accepted evidence:

- accepted SRV-P7B2 implementation SHA: `81e6f6ae69f4bda92d284991e4909457a6ef8e60`;
- post-source-fix code-bearing SHA: `9304263f4be8234e513bf335dc886f96c53d0cce`;
- PostgreSQL workflow tooling SHA before report-only commit: `d48bbd847c8b79511a7ac32cfcb14c671f3880c1`;
- previous fixer status: `FIX_BLOCKED_BY_TOOLING`;
- exact prior diagnostics artifact `8252500221` proved the remaining failures were the three mandatory DB-backed parity tests with no configured PostgreSQL URL;
- PR #45 was observed open, draft, unmerged, and `mergeable: false`;
- current observed `main` head: `c1e69a664388b0cba028170e8398b9088218957d`;
- current merge base before synchronization: `9ee3ced989bf60a71d0d7b37ff046118b0b2d1a2`.

The previous tooling phase already added `postgres:16-alpine` and a synthetic test-only `HAZE_SYNC_TEST_DATABASE_URL` to the ordinary Rust workspace job. Do not redesign application services or weaken tests.

## Goal

Create a real, non-force, two-parent merge of current `main` into `component/server`, resolve the CI-file conflict without losing either side, restore PR mergeability, and obtain an authoritative pull-request Component CI run that executes the mandatory SRV-P7B2 database-backed parity tests.

A fabricated merge commit whose tree simply ignores `main` changes is forbidden.

## Required synchronization protocol

1. Re-read the current heads of `component/server` and `main` immediately before constructing the merge.
2. Recompute the merge base and compare merge-base..main and merge-base..component/server.
3. If `main` moved beyond the observed head, include every new main change and document it. Do not rely blindly on the observed file list.
4. At the observed main head, main changed exactly these paths since the old merge base:
   - `.github/docs/ci-diagnostics-artifacts.md`
   - `.github/scripts/ci-finalize.sh`
   - `.github/scripts/ci-run.sh`
   - `.github/workflows/ci.yml`
   - `.github/workflows/component-ci.yml`
   - `.github/workflows/obsidian-plugin.yml`
   - `.github/workflows/rust.yml`
5. The final merge tree must contain exact current-main content for every main-changed path not intentionally extended by this Server phase.
6. `.github/workflows/component-ci.yml` must be a semantic merge of current main plus the accepted ephemeral PostgreSQL service and `HAZE_SYNC_TEST_DATABASE_URL` required by the Server DB-backed tests. Preserve main's pull-request trigger, workflow dispatch, permissions, concurrency, component context resolution, diagnostics wrapper/finalizer, and artifact upload.
7. Preserve every accepted Server application-service, route-delegation, test, documentation, and control change already present on the component branch.
8. Construct a true merge commit with:
   - first parent: the current component/server head at execution time;
   - second parent: the current main head at execution time;
   - a merged tree containing both sides' accepted changes.
9. Move `component/server` to the new merge commit using a fast-forward ref update only. Never force-update.
10. Verify PR #45 becomes mergeable and obtains a non-null merge commit SHA.

GitHub git-data operations such as tree creation, multi-parent commit creation, and non-force ref update are authorized for this exact synchronization. Do not create an ours-only merge or omit non-conflicting main changes.

## Required CI execution

After the merge commit updates the branch:

- observe the new pull-request Component CI run associated with the synchronized branch head;
- require PostgreSQL service health to succeed;
- require `cargo fmt`, `cargo check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and diagnostics finalization to pass;
- because the three tests are mandatory and fail immediately when no test database URL exists, a green workspace test step must include successful execution of:
  - `application::tests::application_services_preserve_atomic_file_delete_and_read_semantics`;
  - `routes::delete::tests::delegated_delete_route_preserves_tombstone_replay_and_stale_base_behavior`;
  - `routes::v1::tests::delegated_routes_preserve_put_get_and_changes_wire_behavior`.

If CI is red, record the exact synchronized code-bearing/tooling SHA, run id, run number, failed check, and diagnostics artifact metadata. Do not read diagnostics artifacts in this implementation role and do not guess.

If GitHub still does not schedule a run after a genuine merge commit and PR mergeability is restored, report `BLOCKED_BY_TOOLING` with exact evidence. Do not create meaningless commits repeatedly.

## Preservation requirements

Preserve:

- one reusable async `ServerApplicationServices` authority for PUT, DELETE, changes, and revision-content retrieval;
- routes as transport/auth/API mapping adapters only;
- Core policy ownership and passive caller-transaction-owned Storage;
- transaction, advisory-lock, idempotency, object-store, conflict, tombstone, and operation-log semantics;
- deterministic path-hashed Worktree idempotency contracts without exposing keys or raw paths;
- existing public routes, DTOs, headers, statuses, sanitized errors, and dependency-free router construction;
- accepted SRV-P7A startup/composition behavior;
- no Worktree executor, scheduler, host, runtime-status, provider, hard delete, or repair behavior;
- no test deletion, ignore, conditional skip, assertion weakening, fake repository, or in-memory production substitute.

Do not modify Storage, Worktree, Core, API, Common, CLI, Deployment, migrations, schemas, public DTO contracts, or unrelated workflows.

## Allowed files and operations

- the merge commit and exact conflict resolutions required to synchronize current main;
- `.github/workflows/component-ci.yml` only for preserving accepted Server PostgreSQL provisioning over current main;
- SRV-P7B2 tooling documentation only if synchronization changes a documented fact;
- `crates/haze-sync-server/control/report.md`.

Do not archive control files.

## Report

Write `crates/haze-sync-server/control/report.md` using `report-template.md`.

Set:

- `REPORT_TYPE: IMPLEMENTATION`
- `phase_id: SRV-P7B2-BRANCH-SYNC-DB-CI`
- `chat_name: server — W1 SRV-P7B2 Branch Sync and PostgreSQL Run`

Use one honest status:

- `SELF_ACCEPT`
- `SELF_ACCEPT_PENDING_CI`
- `SELF_NEEDS_FIX`
- `BLOCKED_BY_TOOLING`
- `BLOCKED_BY_SCOPE`
- `BLOCKED_BY_CONTRACT`

The report must include old/new branch head, exact main head and merge base, both merge parents, all main-changed paths incorporated, conflict resolutions, final workflow SHA, PR mergeability after sync, exact CI run, PostgreSQL service evidence, mandatory DB parity test evidence, final accepted code-bearing/tooling SHA, secrecy assessment, and whether SRV-P7B2 is ready for mandatory clean-code review.