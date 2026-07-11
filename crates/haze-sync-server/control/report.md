REPORT_TYPE: FIX
STATUS: FIX_BLOCKED_BY_TOOLING
phase_id: FIX-SRV-P7B2-CI
chat_name: server — W1 SRV-P7B2 CI Fix
component: server
branch: component/server

## Trigger

SRV-P7B2 completed the reusable asynchronous Server application-service extraction at code-bearing SHA `81e6f6ae69f4bda92d284991e4909457a6ef8e60`, but Component CI run `29166317284` / `#1766` failed during authoritative diagnostics finalization.

## Diagnostics artifact verification

Only the artifact authorized by the active fixer prompt was downloaded and read:

- artifact: `ci-diag__component-server__wf-component-ci__run-29166317284__attempt-1`
- artifact id: `8252253613`
- expected and verified head SHA: `81e6f6ae69f4bda92d284991e4909457a6ef8e60`
- expected and verified digest: `sha256:f5e0a2f8dba439b43ca6329b9bd4c3e08292a2e6d50fa2686f59b5ba8d2e87c1`
- expected size: `14020` bytes

Read completely:

- `summary.md`
- `manifest.json`
- `failures/rust-fmt.txt`
- `failures/cargo-clippy.txt`
- `failures/cargo-test.txt`
- every log named by the artifact's `failed_checks`

Raw GitHub job logs were not read. The later post-fix diagnostics artifact was not downloaded or read because the active prompt authorizes only artifact `8252253613`.

## Artifact-proven causes

The original artifact proved three categories:

1. `rust-fmt` reported formatting differences in the extracted application and route-delegation files.
2. `cargo-clippy` reported narrow visibility, dead-code, unused-reexport and explicit-auto-deref findings.
3. Three mandatory DB-backed parity tests failed because `connect_test_database_from_env()` returned no configured PostgreSQL test database:
   - `application::tests::application_services_preserve_atomic_file_delete_and_read_semantics`
   - `routes::delete::tests::delegated_delete_route_preserves_tombstone_replay_and_stale_base_behavior`
   - `routes::v1::tests::delegated_routes_preserve_put_get_and_changes_wire_behavior`

The current `.github/workflows/component-ci.yml` defines neither a PostgreSQL service nor `HAZE_SYNC_TEST_DATABASE_URL` / `DATABASE_URL`. The SRV-P7B2 prompt requires these DB-backed tests to be mandatory and explicitly forbids silently treating them as optional. Workflow changes are outside this fixer prompt's allowed scope.

## Corrections applied

Artifact-proven source corrections:

- removed unused public-in-crate reexports of future Worktree idempotency helpers;
- aligned `ServerAppState::application_services` visibility with its `pub(crate)` return type;
- preserved future SRV-P7B3 Worktree idempotency helpers with narrow documented `dead_code` allowances instead of deleting their accepted contract;
- removed the explicit transaction auto-deref reported by Clippy;
- applied the complete rustfmt changes across application services and route delegation/tests.

Changed files:

- `crates/haze-sync-server/src/application/mod.rs`
- `crates/haze-sync-server/src/application/changes.rs`
- `crates/haze-sync-server/src/application/deletes.rs`
- `crates/haze-sync-server/src/application/files.rs`
- `crates/haze-sync-server/src/application/idempotency.rs`
- `crates/haze-sync-server/src/routes/delete.rs`
- `crates/haze-sync-server/src/routes/delete/tests.rs`
- `crates/haze-sync-server/src/routes/v1.rs`
- `crates/haze-sync-server/src/routes/v1/tests.rs`
- `crates/haze-sync-server/src/state.rs`

Final code-bearing SHA:

- `9304263f4be8234e513bf335dc886f96c53d0cce`

No test was deleted, weakened or converted into a silent skip. No workflow, Cargo manifest, migration, schema, public API DTO, config, Core, Storage, Worktree or sibling-component file was modified.

## Post-fix CI

- workflow: `Component CI`
- run id: `29167206582`
- run number: `1798`
- attempt: `1`
- head SHA: `9304263f4be8234e513bf335dc886f96c53d0cce`
- conclusion: `failure`

Visible workflow-step results:

- cargo fmt wrapper: success
- cargo check wrapper: success
- cargo test wrapper: success
- cargo clippy wrapper: success
- Finalize CI diagnostics: failure
- diagnostics upload: success

Because this repository's `ci-run.sh` records command failures for the finalizer, wrapper-step success is not claimed as proof that the mandatory DB-backed test command passed. The original artifact and workflow definition prove the unresolved tooling mismatch: mandatory PostgreSQL tests require a test database that Component CI does not provision.

Post-fix artifact metadata, not contents:

- artifact id: `8252500221`
- artifact name: `ci-diag__component-server__wf-component-ci__run-29167206582__attempt-1`
- digest: `sha256:89fdf6a62c826744abf49c5b462325b9e42ba797d5367a5ef2ff60e10ac240c8`
- head SHA: `9304263f4be8234e513bf335dc886f96c53d0cce`

## Preservation assessment

Preserved:

- one reusable async Server application-service authority for PUT, DELETE, changes and revision-content retrieval;
- HTTP routes as parsing/auth/API mapping adapters only;
- Core policy ownership;
- Storage passive and caller-transaction-owned behavior;
- transaction, advisory-lock, idempotency, object-store, conflict, tombstone and operation-log choreography;
- deterministic path-hashed future Worktree idempotency contracts without exposing keys or paths;
- existing public routes, DTOs, headers, statuses and sanitized errors;
- dependency-free router behavior and accepted SRV-P7A lifecycle composition;
- no Worktree executor, host, watcher, scheduler, polling loop, hard delete, provider call or hidden task.

## Blocker and next gate

The remaining blocker is CI tooling, not an accepted source-code workaround:

- mandatory DB-backed parity tests must run;
- current Component CI provides no PostgreSQL test database or validated test URL;
- silently skipping those tests would violate the active contract;
- modifying the workflow is forbidden in this fixer scope.

SRV-P7B2 is therefore not ready for mandatory clean-code review yet. The Orchestrator must route an explicit CI-tooling phase or otherwise provide an authorized DB-capable verification path, then obtain a green code-bearing Component CI run.

NEXT_RECOMMENDED_AGENT: orchestrator
FINAL_VERDICT: FIX_BLOCKED_BY_TOOLING
PUSHED: yes
