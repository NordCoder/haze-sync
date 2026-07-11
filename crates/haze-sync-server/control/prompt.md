# W1-FIX-SRV-P7B2-DB-CI — PostgreSQL-capable Server application-services CI

Before starting, name this worker chat exactly:

`server — W1 SRV-P7B2 PostgreSQL CI Fix`

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: fixer-worker
Phase: FIX-SRV-P7B2-DB-CI

Work only through the GitHub connector. Do not merge the PR, change draft state, rebase, reset, rewrite history, force-push, modify `main`, or modify sibling branches.

## Trigger

The first SRV-P7B2 fixer corrected all source-level rustfmt/clippy findings proven by artifact `8252253613`, but post-fix Component CI remains red because mandatory DB-backed parity tests have no PostgreSQL service or test URL.

Current evidence:

- accepted SRV-P7B2 application-services implementation SHA: `81e6f6ae69f4bda92d284991e4909457a6ef8e60`
- post-source-fix code-bearing SHA: `9304263f4be8234e513bf335dc886f96c53d0cce`
- post-fix Component CI run: `29167206582`
- run number: `1798`
- conclusion: `failure`
- visible fmt/check/test/clippy wrappers: success
- finalizer: failure
- current Component CI workflow has no PostgreSQL service and no `HAZE_SYNC_TEST_DATABASE_URL`/`DATABASE_URL`

The prior authorized artifact proved mandatory DB failures in:

- `application::tests::application_services_preserve_atomic_file_delete_and_read_semantics`
- `routes::delete::tests::delegated_delete_route_preserves_tombstone_replay_and_stale_base_behavior`
- `routes::v1::tests::delegated_routes_preserve_put_get_and_changes_wire_behavior`

No test may be deleted, weakened, ignored, or made optional.

## Exact post-fix diagnostics artifact

Use only this new artifact for the current failure:

- artifact name: `ci-diag__component-server__wf-component-ci__run-29167206582__attempt-1`
- artifact id: `8252500221`
- artifact head SHA: `9304263f4be8234e513bf335dc886f96c53d0cce`
- artifact digest: `sha256:89fdf6a62c826744abf49c5b462325b9e42ba797d5367a5ef2ff60e10ac240c8`
- artifact size bytes: `8591`
- created at: `2026-07-11T20:33:42Z`
- expires at: `2026-07-12T20:33:41Z`
- status when assigned: available and unexpired

## Diagnostics protocol

1. Fetch artifact `8252500221` from run `29167206582`.
2. Verify exact head SHA and digest.
3. Read `summary.md`, `manifest.json`, every failure marker, and every log listed by `failed_checks`.
4. Confirm whether the remaining failure is PostgreSQL provisioning/test-environment only.
5. Apply the complete minimum artifact-proven tooling correction.

Raw GitHub job logs are not authorized as fallback. If the artifact is missing, expired, malformed, mismatched, or unreadable, report `FIX_BLOCKED_BY_LOGS`.

## Authorized PostgreSQL CI scope

If and only if the artifact confirms the mandatory DB tests fail because no PostgreSQL test database is provisioned, you are explicitly authorized to make the smallest branch-local CI tooling change necessary to:

- provision an ephemeral PostgreSQL service with test-only non-secret credentials;
- wait for readiness deterministically;
- expose a test-only database URL under the exact environment variable expected by Server test support;
- run the existing mandatory DB-backed SRV-P7B2 tests as part of normal `cargo test --workspace` or an explicit mandatory step;
- fail CI if connection/setup/tests fail;
- preserve diagnostics finalization and artifact upload;
- avoid repository secrets and external managed services.

Prefer a conservative shared workflow shape. Do not perform unrelated workflow refactoring or change release/deployment workflows.

## Preservation requirements

Preserve:

- one reusable async `ServerApplicationServices` authority for PUT, DELETE, changes, and revision-content retrieval;
- routes as transport/auth/API mapping adapters only;
- Core policy ownership and passive caller-transaction-owned Storage;
- transaction, advisory-lock, idempotency, object-store, conflict, tombstone, and operation-log semantics;
- deterministic path-hashed Worktree idempotency contracts without exposing keys or raw paths;
- existing public routes, DTOs, headers, statuses, and sanitized errors;
- dependency-free router construction;
- accepted SRV-P7A startup/composition behavior;
- no Worktree executor, scheduler, watcher, host, runtime-status, provider, hard-delete, or repair behavior.

Do not use nested runtimes, `block_on`, internal HTTP calls, fake repositories, in-memory production substitutes, silent DB skips, or fabricated test summaries.

## Allowed files

- `.github/workflows/component-ci.yml` only when artifact-proven for PostgreSQL provisioning;
- `.github/scripts/**` only if a small deterministic readiness helper is strictly necessary;
- SRV-P7B2 Server source/tests only if the new artifact or live DB execution proves an additional genuine defect;
- SRV-P7B2 docs/implementation log for factual tooling documentation;
- `crates/haze-sync-server/control/report.md`.

Forbidden:

- Worktree, Storage, Core, API, Common, CLI, Deployment, migration, schema, public DTO, or Server runtime-host changes;
- real credentials or secrets;
- weakening/ignoring mandatory tests;
- unrelated CI cleanup;
- archiving control files.

## Acceptance

`FIX_COMPLETE` requires:

- every artifact-listed failure addressed;
- mandatory DB-backed SRV-P7B2 parity tests actually executed against ephemeral PostgreSQL;
- a green post-fix code-bearing Component CI run with fmt/check/test/clippy/finalizer success;
- no semantic regression in application-service ownership or public route behavior.

## Report

Write `crates/haze-sync-server/control/report.md` using `report-template.md`.

Set:

- `REPORT_TYPE: FIX`
- `phase_id: FIX-SRV-P7B2-DB-CI`
- `chat_name: server — W1 SRV-P7B2 PostgreSQL CI Fix`

Use one honest status:

- `FIX_COMPLETE`
- `FIX_NEEDS_MORE`
- `FIX_BLOCKED_BY_LOGS`
- `FIX_BLOCKED_BY_SCOPE`
- `FIX_BLOCKED_BY_CONTRACT`
- `FIX_BLOCKED_BY_TOOLING`

The report must include exact artifact verification, failed checks, PostgreSQL service/configuration changes, exact DB tests that ran, changed files, final code-bearing SHA, authoritative CI run, behavior/parity assessment, secrecy assessment, and whether SRV-P7B2 is ready for mandatory clean-code review.