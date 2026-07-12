# W1-FIX-SRV-P7B2-REMAINING-CI — Resolve the next exact Server CI failure

Before starting, name this worker chat exactly:

`server — W1 SRV-P7B2 Remaining CI Fix`

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: fixer-worker
Phase: FIX-SRV-P7B2-REMAINING-CI

Work through the GitHub connector. Do not merge PR #45 into main, change draft state, rebase, reset, rewrite history, force-push, modify sibling branches, or read raw GitHub job logs.

## Trigger and preserved correction baseline

The previous follow-up fixer read artifact `8253899359` and proved one remaining failure in:

`application::tests::application_services_preserve_atomic_file_delete_and_read_semantics`

The test claimed stale-base behavior but supplied an invented revision ID that had never been persisted, while conflict metadata is foreign-keyed to real revisions. The fixer made a test-only correction:

- persisted a first accepted revision;
- persisted a second current revision;
- used the first persisted revision as the genuine stale base;
- asserted that the second revision remains authoritative;
- used the first revision for stale-delete rejection and the second for current guarded/accepted delete behavior.

No production application-service, route, DTO, schema, migration, CI script, or sibling-component behavior changed.

Final correction code-bearing SHA:

`e2f85b18417aac630c8281ee4f9915c29474f514`

Authoritative Component CI:

- run id: `29185492952`;
- run number: `1834`;
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

Do not infer whether the remaining failure is the same test, a different test, formatting, clippy, or diagnostics state. Read only the new authorized artifact.

## Exact diagnostics artifact

Use only:

- artifact name: `ci-diag__component-server__wf-component-ci__run-29185492952__attempt-1`;
- artifact id: `8257869542`;
- artifact head SHA: `e2f85b18417aac630c8281ee4f9915c29474f514`;
- artifact digest: `sha256:d047efc0a3172c4c0b0a0511e79c65c8693814fdc72f251cf51b20c0fdc62e0b`;
- artifact size bytes: `9397`;
- created at: `2026-07-12T08:15:05Z`;
- expires at: `2026-07-13T08:15:05Z`;
- status when assigned: available and unexpired.

## Diagnostics protocol

1. Fetch artifact `8257869542` from run `29185492952`.
2. Verify exact artifact head SHA and digest.
3. Read `summary.md`, `manifest.json`, every failure marker, and every log named in `failed_checks`.
4. Identify the exact remaining failed command/test and root cause.
5. Apply the complete minimum artifact-proven correction.

Raw GitHub job logs are forbidden as fallback. If the artifact is missing, expired, malformed, mismatched, or unreadable, report `FIX_BLOCKED_BY_LOGS` without guessing.

## Preservation requirements

Preserve unless this artifact proves a concrete defect:

- the persisted stale-revision fixture correction at `e2f85b18417aac630c8281ee4f9915c29474f514`;
- all earlier test-only PostgreSQL lease, schema-probe, cleanup and workspace serialization corrections;
- one reusable async `ServerApplicationServices` authority for PUT, DELETE, changes and revision-content retrieval;
- routes as transport/auth/API mapping adapters only;
- transaction, advisory-lock, idempotency, Core planning, object-store, conflict, tombstone and operation-log semantics;
- deterministic path-hashed Worktree idempotency derivation without exposing raw paths or keys;
- exact public routes, DTOs, headers, status codes and sanitized errors;
- dependency-free router construction and accepted SRV-P7A lifecycle behavior;
- PostgreSQL 16 CI service and synthetic test-only database URL;
- strict execution of mandatory SRV-P7B2 DB-backed parity tests;
- current-main synchronization and absence of temporary write-enabled workflows;
- no product runtime mutex, global production DB cleanup, nested runtime, `block_on`, internal HTTP self-call, fake repository, hidden task, silent DB skip, assertion weakening, migration/schema change, or sibling-component change.

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
- no stale/false diagnostics marker remains;
- PostgreSQL container remains healthy;
- fmt, check, workspace tests, clippy and finalizer all pass in a new code-bearing Component CI run;
- mandatory SRV-P7B2 DB-backed parity tests execute successfully;
- all prior fixture/lease/serialization corrections remain semantically sound;
- no production or public behavior regression.

## Report

Write `crates/haze-sync-server/control/report.md` using `report-template.md`.

Set:

- `REPORT_TYPE: FIX`
- `phase_id: FIX-SRV-P7B2-REMAINING-CI`
- `chat_name: server — W1 SRV-P7B2 Remaining CI Fix`

Use one honest status:

- `FIX_COMPLETE`
- `FIX_NEEDS_MORE`
- `FIX_BLOCKED_BY_LOGS`
- `FIX_BLOCKED_BY_SCOPE`
- `FIX_BLOCKED_BY_CONTRACT`
- `FIX_BLOCKED_BY_TOOLING`

The report must include artifact verification, exact remaining failed checks and root cause, changed files, preservation assessment, final code-bearing/tooling SHA, new DB-capable Component CI evidence, secrecy assessment, and whether SRV-P7B2 is ready for mandatory clean-code review.