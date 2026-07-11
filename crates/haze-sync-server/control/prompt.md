# W1-FIX-SRV-P7B2-CI — Server application-services CI correction

Before starting, name this worker chat exactly:

`server — W1 SRV-P7B2 CI Fix`

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: fixer-worker

Work only through the GitHub connector. Do not merge the PR, change its draft state, rebase, reset, rewrite history, force-push, modify `main`, or modify sibling branches.

## Trigger and implementation evidence

SRV-P7B2 completed the reusable async Server application-services extraction and reported `SELF_ACCEPT_PENDING_CI`, but the authoritative code-bearing Component CI is red.

Evidence:

- implementation phase: `SRV-P7B2`
- implementation status: `SELF_ACCEPT_PENDING_CI`
- final code-bearing SHA: `81e6f6ae69f4bda92d284991e4909457a6ef8e60`
- workflow: `Component CI`
- workflow run id: `29166317284`
- run number: `1766`
- attempt: `1`
- conclusion: `failure`
- visible passing steps: cargo fmt, cargo check, cargo test, cargo clippy
- visible failing stage: Finalize CI diagnostics

Archived phase evidence:

- prompt snapshot: `crates/haze-sync-server/control/log/20260711-201500Z-W1-SRV-P7B2-implementation-worker-prompt.md`
- report snapshot: `crates/haze-sync-server/control/log/20260711-201500Z-W1-SRV-P7B2-implementation-worker-report.md`
- exact implementation report blob: `f0fd49086d32a8b8fe7e2e1548383ea3bb38f7e2`

Accepted SRV-P7B2 behavior includes:

- reusable async `ServerApplicationServices` shared by routes and the future Worktree executor;
- typed file PUT, guarded DELETE, bounded changes and revision-content operations;
- transaction, advisory-lock, idempotency, Core planning, object-store and repository choreography outside HTTP handlers;
- routes retained as transport/auth/API DTO/status adapters;
- deterministic path-hashed future Worktree idempotency derivation;
- strict DB-backed service and route parity tests;
- removal of obsolete route-private planning/persistence implementations;
- no Worktree executor, host, schema, public DTO, config, provider or sibling change.

Do not return transaction authority to routes or introduce a second production implementation.

## Exact diagnostics artifact

Use only:

- artifact name: `ci-diag__component-server__wf-component-ci__run-29166317284__attempt-1`
- artifact id: `8252253613`
- artifact head SHA: `81e6f6ae69f4bda92d284991e4909457a6ef8e60`
- artifact digest: `sha256:f5e0a2f8dba439b43ca6329b9bd4c3e08292a2e6d50fa2686f59b5ba8d2e87c1`
- artifact size bytes: `14020`
- created at: `2026-07-11T20:04:09Z`
- expires at: `2026-07-12T20:04:08Z`
- status when assigned: available and unexpired

## Required reads and diagnostics protocol

Read the implementation manifest, report template, fixer-worker prompt, GitHub connector guidance, current Server state/prompt, archived SRV-P7B2 evidence, changed application/route/state/tests/docs, and the exact artifact.

Then:

1. fetch artifact `8252253613` from run `29166317284`;
2. verify its head SHA exactly equals `81e6f6ae69f4bda92d284991e4909457a6ef8e60`;
3. read `summary.md`, `manifest.json`, every failure marker and every log listed by `failed_checks`;
4. apply only the complete minimum artifact-proven correction.

Raw GitHub job logs are not authorized as fallback. If the artifact is missing, expired, malformed, mismatched or unreadable, report `FIX_BLOCKED_BY_LOGS` without guessing.

## Preservation requirements

Preserve unless the exact artifact proves otherwise:

- one reusable async application-service authority for PUT, DELETE, changes and revision-content retrieval;
- routes remain transport parsing/auth/API mapping only;
- Core remains policy owner and Storage remains passive/caller-transaction-owned;
- durable idempotency lookup/fingerprint/replay behavior;
- advisory path locking and transaction commit/rollback semantics;
- content-addressed object-store replay safety;
- conflict preservation, tombstones and operation-log atomicity;
- deterministic Worktree idempotency derivation without exposing raw keys or paths;
- exact existing public routes, DTOs, headers, status codes and sanitized errors;
- dependency-free router construction;
- accepted SRV-P7A startup and Worktree composition behavior;
- no Worktree executor, scheduler, watcher, host task, polling loop or runtime status work;
- no migration/schema, public API, config, Cargo, workflow, provider or sibling changes unless the artifact proves a narrowly scoped Server-local correction is required;
- no nested runtime, `block_on`, internal HTTP self-call, fake repository or hidden task;
- no hard delete, destructive cleanup or automatic repair.

## Allowed files

Only artifact-proven changes within:

- `crates/haze-sync-server/src/application/**`;
- SRV-P7B2 route delegation modules and tests;
- `crates/haze-sync-server/src/state.rs` or `src/main.rs` only if directly artifact-proven;
- SRV-P7B2 Server docs only if a corrected fact changes;
- `crates/haze-sync-server/control/report.md`.

Do not modify Worktree, Storage, Core, API, Common, CLI, Deployment, migrations, workflows, public DTO contracts or sibling control files. Do not archive control files.

## CI and report

All source/test/docs changes must run normal Component CI, including strict DB-backed parity tests. CI skip is allowed only for the final report-only commit.

Write `crates/haze-sync-server/control/report.md` using `report-template.md` with:

- `REPORT_TYPE: FIX`
- `phase_id: FIX-SRV-P7B2-CI`
- `chat_name: server — W1 SRV-P7B2 CI Fix`

Use an honest status:

- `FIX_COMPLETE`
- `FIX_NEEDS_MORE`
- `FIX_BLOCKED_BY_LOGS`
- `FIX_BLOCKED_BY_SCOPE`
- `FIX_BLOCKED_BY_CONTRACT`
- `FIX_BLOCKED_BY_TOOLING`

The report must include artifact verification, exact failure cause, changed files, behavior/parity assessment, final code-bearing SHA, post-fix CI run and conclusion, preservation of application-service ownership, and whether SRV-P7B2 is ready for mandatory clean-code review.
