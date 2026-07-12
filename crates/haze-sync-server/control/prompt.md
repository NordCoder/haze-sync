# W1-FIX-SRV-P7B2-FOLLOWUP-CI — Resolve remaining Server CI failure

Before starting, name this worker chat exactly:

`server — W1 SRV-P7B2 Follow-Up CI Fix`

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: fixer-worker
Phase: FIX-SRV-P7B2-FOLLOWUP-CI

Work through the GitHub connector. Do not merge PR #45 into main, change its draft state, rebase, reset, rewrite history, force-push, modify sibling branches, or read raw GitHub job logs.

## Trigger and preserved correction baseline

The previous artifact-based fixer read artifact `8253726096` and proved that the first synchronized Server CI failure came from `cargo test --workspace` exit 101, not a stale finalizer. It made focused test/tooling corrections without changing production behavior:

- serialized ordinary workspace tests with `--test-threads=1`;
- replaced a two-command conflict fixture with one parameterized `UPDATE ... CASE`;
- added a test-only process-local async database lease;
- probes/applies schema once and cleans shared tables before each DB-backed test;
- applied the lease to SRV-P7B2 application, delete-route, v1-route and conflict-route DB tests;
- retained required DB behavior for the three SRV-P7B2 parity tests;
- retained optional local behavior for legacy conflict integration tests.

Final correction code-bearing SHA:

`4ecd69cd7e65b40b71374032718b5d52ab0fc585`

Authoritative post-fix Component CI:

- run id: `29172398405`;
- run number: `1832`;
- attempt: `1`;
- conclusion: `failure`.

Visible job evidence:

- PostgreSQL service initialization: success;
- cargo fmt wrapper: success;
- cargo check wrapper: success;
- cargo test wrapper: success;
- cargo clippy wrapper: success;
- Finalize CI diagnostics: failure;
- diagnostics upload: success.

Do not assume the previous failures are fully or partially repeated. Read only the new exact artifact before changing anything.

## Exact diagnostics artifact

Use only:

- artifact name: `ci-diag__component-server__wf-component-ci__run-29172398405__attempt-1`;
- artifact id: `8253899359`;
- artifact head SHA: `4ecd69cd7e65b40b71374032718b5d52ab0fc585`;
- artifact digest: `sha256:a5500f9acca3e1da247943e3b9a8aaab2843891dc7ba83a301a7e928a3b5c4bf`;
- artifact size bytes: `8545`;
- created at: `2026-07-11T23:38:53Z`;
- expires at: `2026-07-12T23:38:53Z`;
- status when assigned: available and unexpired.

## Diagnostics protocol

1. Fetch artifact `8253899359` from run `29172398405`.
2. Verify exact artifact head SHA and digest.
3. Read `summary.md`, `manifest.json`, every failure marker, and every log named in `failed_checks`.
4. Identify the exact remaining failed command and root cause.
5. Apply the complete minimum artifact-proven correction.

Raw GitHub job logs are forbidden as fallback. If the artifact is missing, expired, malformed, mismatched, or unreadable, report `FIX_BLOCKED_BY_LOGS` without guessing.

## Preservation requirements

Preserve unless the new artifact proves a defect:

- all previous DB-test serialization and lease corrections;
- one reusable async `ServerApplicationServices` authority for PUT, DELETE, changes and revision-content retrieval;
- routes as transport/auth/API mapping adapters only;
- transaction, advisory-lock, idempotency, Core planning, object-store, conflict, tombstone and operation-log semantics;
- deterministic path-hashed Worktree idempotency derivation without exposing raw paths or keys;
- exact public routes, DTOs, headers, statuses and sanitized errors;
- dependency-free router construction and accepted SRV-P7A startup/composition;
- PostgreSQL 16 CI service and synthetic test-only database URL;
- strict execution of the three mandatory SRV-P7B2 DB parity tests;
- current-main synchronization and absence of temporary `contents: write` workflows;
- no product runtime mutex, global DB cleanup, nested runtime, `block_on`, internal HTTP self-call, fake repository, hidden background task, silent DB skip, assertion weakening, migration/schema change, or sibling-component change.

## Allowed files

Only artifact-proven changes within:

- Server test-only DB harness and affected Server tests;
- `.github/workflows/component-ci.yml` if the artifact proves a workspace-test invocation or environment defect;
- `.github/scripts/ci-run.sh` or `.github/scripts/ci-finalize.sh` if the artifact proves diagnostics-state behavior;
- SRV-P7B2 tooling documentation if a corrected fact changes;
- `crates/haze-sync-server/control/report.md`.

Do not modify production application-service behavior, public routes/DTOs, Storage, Worktree, Core, API, Common, CLI, Deployment, schemas, migrations, provider behavior, release workflows, or unrelated CI. Do not archive control files.

## Acceptance

`FIX_COMPLETE` requires:

- every artifact-listed failed check addressed;
- no stale or false diagnostics marker remains;
- PostgreSQL container remains healthy;
- fmt, check, workspace tests, clippy and finalizer all pass in a new code-bearing Component CI run;
- all mandatory SRV-P7B2 DB-backed parity tests execute successfully;
- previous test lease/serialization fixes remain semantically sound;
- no public/product behavior regression.

## Report

Write `crates/haze-sync-server/control/report.md` using `report-template.md`.

Set:

- `REPORT_TYPE: FIX`
- `phase_id: FIX-SRV-P7B2-FOLLOWUP-CI`
- `chat_name: server — W1 SRV-P7B2 Follow-Up CI Fix`

Use one honest status:

- `FIX_COMPLETE`
- `FIX_NEEDS_MORE`
- `FIX_BLOCKED_BY_LOGS`
- `FIX_BLOCKED_BY_SCOPE`
- `FIX_BLOCKED_BY_CONTRACT`
- `FIX_BLOCKED_BY_TOOLING`

The report must include artifact verification, exact remaining failed checks and root cause, changed files, preservation assessment, final code-bearing/tooling SHA, new DB-capable Component CI evidence, secrecy assessment, and whether SRV-P7B2 is ready for mandatory clean-code review.