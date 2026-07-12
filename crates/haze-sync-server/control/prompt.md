# W1-SRV-P7B2-CLEAN — Reusable Server application-services clean-code review

Before starting, name this worker chat exactly:

`server — W1 SRV-P7B2 Clean-Code Review`

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: clean-code-reviewer
Phase: SRV-P7B2-CLEAN

Work through the GitHub connector. Do not merge PR #45 into main, change its draft state, rebase, reset, rewrite history, force-push, modify sibling branches, or read failure artifacts when the final authoritative CI is green.

## Accepted review baseline

SRV-P7B2 implementation, source cleanup, PostgreSQL provisioning, main synchronization, artifact-driven DB-test corrections and final CI environment isolation are complete.

Key evidence:

- accepted SRV-P7A baseline: `37706634fd8dd2d9b299a1c453718f2de63981d0`;
- initial SRV-P7B2 implementation SHA: `81e6f6ae69f4bda92d284991e4909457a6ef8e60`;
- post-source-fix SHA: `9304263f4be8234e513bf335dc886f96c53d0cce`;
- synchronized helper merge commit: `716bd357f52831d796c7e1ea57c83a2849e6ce03`;
- final code-bearing/tooling candidate SHA: `647dce7b624d67663632808906896cb6745ea7e7`;
- authoritative Component CI run: `29186058268`;
- run number: `1835`;
- attempt: `1`;
- conclusion: `success`.

The final CI passed:

- PostgreSQL Server service initialization;
- PostgreSQL Storage service initialization;
- cargo fmt;
- cargo check;
- isolated `haze-sync-server` tests with serial execution;
- isolated `haze-sync-storage` tests with a separate database;
- all remaining workspace package tests;
- cargo clippy with `-D warnings`;
- diagnostics finalizer.

Do not read failure artifacts: the final authoritative run is green.

## Important cross-component contract evidence

The final Storage acceptance review was blocked because the stale Server copy present on `component/storage` enabled `haze-sync-storage/test-support` in Server normal dependencies.

At the exact Server candidate SHA `647dce7b624d67663632808906896cb6745ea7e7`, `crates/haze-sync-server/Cargo.toml` is correctly gated:

- normal `[dependencies]` contains `haze-sync-storage = { path = "../haze-sync-storage" }` without `test-support`;
- `[dev-dependencies]` contains `haze-sync-storage = { path = "../haze-sync-storage", features = ["test-support"] }`.

Storage contract requires that production crates never enable `test-support` in normal dependencies. This clean review must explicitly verify and report whether the final Server candidate satisfies that contract. A `CLEAN_ACCEPT` report must provide the exact evidence needed to unblock the final Storage cross-branch confirmation.

## Review scope

Review the complete SRV-P7B2 range and its focused corrections, especially:

- reusable async `ServerApplicationServices` for file PUT, guarded DELETE, changes and revision-content retrieval;
- route delegation and transport/auth/API mapping boundaries;
- transaction, advisory-lock, idempotency, Core planning, object-store, conflict, tombstone and operation-log choreography;
- deterministic future Worktree idempotency derivation and secrecy;
- application, delete-route, v1-route and conflict-route PostgreSQL tests;
- test-only database lease, schema probe, cleanup and serial execution;
- current `Cargo.toml` normal/dev dependency gating;
- `.github/workflows/component-ci.yml` isolated Server/Storage database services and complete workspace coverage;
- current-main synchronization and absence of temporary write-enabled workflows.

Green CI is necessary but not sufficient. Review correctness, ownership, maintainability, transaction atomicity, error mapping, test quality, dependency gating and CI integrity.

## Mandatory review questions

1. Is there exactly one reusable async application-service authority for PUT, DELETE, changes and revision-content retrieval?
2. Are routes limited to transport, auth, DTO and status mapping rather than persistence/policy choreography?
3. Do Core and Storage ownership boundaries remain intact?
4. Are caller-owned transactions, advisory locks, idempotency records, object-store writes, revisions, tombstones, conflicts and operation-log entries composed atomically?
5. Are stale-base, replay, conflict and guarded-delete semantics preserved across application services and routes?
6. Are errors sanitized without raw SQLx details, database URLs, local paths, idempotency keys or Worktree derivation material?
7. Does deterministic Worktree idempotency derivation avoid exposing raw paths or keys?
8. Are test-only database lease and cleanup strictly test-scoped, with no production global mutex or cleanup behavior?
9. Does schema probing avoid unsafe repeated non-idempotent migrations while still failing when required test DB setup is absent or invalid?
10. Do persisted stale-revision fixtures test real foreign-key-valid stale behavior rather than invented identifiers?
11. Do isolated Server and Storage PostgreSQL services prevent cross-package migration races without skipping or weakening any tests?
12. Does the final CI still cover every workspace package exactly once or intentionally more, with no silent omission?
13. Is `haze-sync-storage/test-support` absent from Server normal dependencies and enabled only in dev/test scope?
14. Does the final candidate therefore satisfy Storage's production-gating contract?
15. Are synthetic credentials confined to ephemeral CI services and free of repository secrets?
16. Are there avoidable broad visibilities, duplicated choreography, weak types, dead code, hidden tasks, nested runtimes, internal HTTP self-calls or unbounded operations?
17. Are SRV-P7A startup/composition behavior and dependency-free router construction preserved?
18. Is the candidate clean and stable enough to become the exact accepted Server SHA for SRV-P7B3 fan-in?

## Corrections

You may make focused Server-local clean-code or correctness corrections only when a concrete material issue is found. Preserve the accepted architecture and all mandatory DB-backed tests.

Any source, test, Cargo, documentation or workflow correction requires a new DB-capable Component CI run with all final steps green. A report-only commit is not code-bearing evidence.

Allowed files:

- SRV-P7B2 application services, route delegation and Server tests;
- Server test-only DB harness;
- `crates/haze-sync-server/Cargo.toml` only for a proven dependency-gating defect;
- SRV-P7B2 Server-owned docs;
- `.github/workflows/component-ci.yml` only for a proven DB isolation or complete-coverage defect;
- `crates/haze-sync-server/control/report.md`.

Forbidden:

- Storage, Worktree, Core, API, Common, CLI, Deployment, provider, schema or migration changes;
- production runtime mutexes or database cleanup;
- test deletion, ignoring, silent skip or assertion weakening;
- public API/DTO redesign outside a proven defect;
- Worktree executor, scheduler, watcher, host or background-task implementation;
- broad unrelated refactoring;
- archiving control files.

## Report

Write `crates/haze-sync-server/control/report.md` using `report-template.md`.

Set:

- `REPORT_TYPE: CLEAN_CODE_REVIEW`
- `phase_id: SRV-P7B2-CLEAN`
- `chat_name: server — W1 SRV-P7B2 Clean-Code Review`

Use one honest status:

- `CLEAN_ACCEPT`
- `CLEAN_NEEDS_FIX`
- `CLEAN_BLOCKED_BY_CONTRACT`
- `CLEAN_BLOCKED_BY_SCOPE`
- `CLEAN_BLOCKED_BY_TOOLING`

The report must state the exact final reviewed code-bearing SHA, complete review range, application-service/route/transaction assessment, test-harness and CI-isolation assessment, explicit normal-versus-dev Storage dependency evidence, secrecy assessment, corrections and final CI evidence if any, and whether SRV-P7B2 is accepted for SRV-P7B3 fan-in and Storage contract unblocking.