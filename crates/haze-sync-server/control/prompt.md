# W1-FIX-SRV-P7B2-POST-SYNC-CI — Resolve synchronized Server CI finalizer failure

Before starting, name this worker chat exactly:

`server — W1 SRV-P7B2 Post-Sync CI Fix`

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: fixer-worker
Phase: FIX-SRV-P7B2-POST-SYNC-CI

Work through the GitHub connector. Do not merge PR #45 into main, change draft state, rebase, reset, rewrite history, force-push, modify sibling branches, or read raw GitHub job logs.

## Trigger and accepted baseline

The Server branch synchronization conflict is resolved. Current main was incorporated through helper PR #65, PR #45 is now open, draft, unmerged and mergeable, and the one-time write-enabled sync workflow was removed.

Accepted evidence:

- accepted SRV-P7B2 implementation SHA: `81e6f6ae69f4bda92d284991e4909457a6ef8e60`;
- post-source-fix SHA: `9304263f4be8234e513bf335dc886f96c53d0cce`;
- PostgreSQL provisioning tooling SHA: `d48bbd847c8b79511a7ac32cfcb14c671f3880c1`;
- helper synchronization PR: `#65`;
- helper merge commit into component/server: `716bd357f52831d796c7e1ea57c83a2849e6ce03`;
- final post-sync code/tooling SHA under CI: `56a0c8835e6d5ba33696b98814c0ac7b475c9b9e`;
- Component CI run: `29171731875`;
- run number: `1819`;
- attempt: `1`;
- conclusion: `failure`.

Visible job evidence:

- PostgreSQL service/container initialization: success;
- cargo fmt: success;
- cargo check: success;
- cargo test: success;
- cargo clippy: success;
- Finalize CI diagnostics: failure;
- Upload CI diagnostics: success.

Therefore all product and DB-backed tests passed. Do not redesign application services, routes or PostgreSQL provisioning unless the exact diagnostics artifact proves a narrowly scoped defect.

## Exact diagnostics artifact

Use only this artifact:

- artifact name: `ci-diag__component-server__wf-component-ci__run-29171731875__attempt-1`;
- artifact id: `8253726096`;
- artifact head SHA: `56a0c8835e6d5ba33696b98814c0ac7b475c9b9e`;
- artifact digest: `sha256:6d12f4a08e3fcc69a42252dbc9cd8b7c135aab68cbed5bb64d6cb76550058c9b`;
- artifact size bytes: `8872`;
- created at: `2026-07-11T23:13:44Z`;
- expires at: `2026-07-12T23:13:44Z`;
- status when assigned: available and unexpired.

## Diagnostics protocol

1. Fetch artifact `8253726096` from workflow run `29171731875`.
2. Verify exact artifact head SHA and digest.
3. Read `summary.md`, `manifest.json`, every failure marker and every log named in `failed_checks`.
4. Determine why the finalizer remained red after fmt/check/test/clippy all succeeded.
5. Apply the complete minimum artifact-proven correction.

Raw GitHub job logs are not authorized as fallback. If the artifact is missing, expired, malformed, mismatched or unreadable, report `FIX_BLOCKED_BY_LOGS` without guessing.

## Preservation requirements

Preserve unless the exact artifact proves otherwise:

- one reusable async `ServerApplicationServices` authority for PUT, DELETE, changes and revision-content retrieval;
- routes as transport/auth/API DTO/status adapters only;
- transaction, advisory-lock, idempotency, Core planning, object-store, conflict, tombstone and operation-log semantics;
- deterministic path-hashed Worktree idempotency derivation without exposing raw paths or keys;
- exact public routes, DTOs, headers, status codes and sanitized errors;
- dependency-free router construction;
- accepted SRV-P7A startup/composition behavior;
- ephemeral PostgreSQL 16 service and synthetic test-only `HAZE_SYNC_TEST_DATABASE_URL`;
- mandatory DB-backed tests executing inside ordinary workspace tests;
- current-main CI scripts/workflows incorporated through synchronization;
- no temporary `contents: write` workflow;
- no Worktree executor, scheduler, watcher, host, runtime-status or provider implementation;
- no schema/migration, public API, sibling component, hard-delete or repair changes;
- no nested runtime, `block_on`, internal HTTP self-call, fake repository, hidden task, silent DB skip or test weakening.

## Allowed files

Only artifact-proven changes within:

- `.github/scripts/ci-run.sh` and `.github/scripts/ci-finalize.sh` if the artifact proves a diagnostics-state defect;
- `.github/workflows/component-ci.yml` if the artifact proves a narrowly scoped workflow/finalizer integration defect;
- SRV-P7B2 Server source/tests only if the artifact reveals a genuine remaining product issue despite visible test success;
- SRV-P7B2 tooling documentation if a corrected fact changes;
- `crates/haze-sync-server/control/report.md`.

Do not modify Storage, Worktree, Core, API, Common, CLI, Deployment, migrations, public DTO contracts, release workflows or unrelated CI behavior. Do not archive control files.

## Acceptance

`FIX_COMPLETE` requires:

- every artifact-listed failed check addressed;
- no stale failure marker or false diagnostics state remains;
- PostgreSQL-backed workspace tests still execute and pass;
- a new code-bearing/tooling Component CI run finishes green for fmt, check, test, clippy and diagnostics finalizer;
- no semantic regression in application-service authority or public route behavior.

## Report

Write `crates/haze-sync-server/control/report.md` using `report-template.md`.

Set:

- `REPORT_TYPE: FIX`
- `phase_id: FIX-SRV-P7B2-POST-SYNC-CI`
- `chat_name: server — W1 SRV-P7B2 Post-Sync CI Fix`

Use one honest status:

- `FIX_COMPLETE`
- `FIX_NEEDS_MORE`
- `FIX_BLOCKED_BY_LOGS`
- `FIX_BLOCKED_BY_SCOPE`
- `FIX_BLOCKED_BY_CONTRACT`
- `FIX_BLOCKED_BY_TOOLING`

The report must include artifact verification, exact failed checks and cause, changed files, behavior/parity assessment, final code-bearing/tooling SHA, post-fix DB-capable CI run and conclusion, secrecy assessment, preserved synchronization/application-service invariants, and whether SRV-P7B2 is ready for mandatory clean-code review.