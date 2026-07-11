# W1-SRV-P7B2 — Reusable async Server application services

Before starting, name this worker chat exactly:

`server — W1 SRV-P7B2 Application Services`

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: implementation-worker

Work only through the GitHub connector. Do not merge the PR, change its draft state, rebase, reset, rewrite history, force-push, modify `main`, modify sibling branches, or implement Worktree runtime hosting.

## Accepted architecture authority

The completed architecture phase `ARCH-SRV-P7B-CONTRACTS` returned `ARCHITECT_CHANGED_CONTRACTS` and accepted one target architecture.

Authoritative evidence:

- documentation-bearing SHA: `b98f5079ed90ccf7eaec79e617ae591e0c308ff4`
- Component CI run: `29165123142`, run number `1719`, conclusion `success`
- final architecture report-only commit: `22b5e993431a1c0a858716ac835a3ece8ade6d5c`
- archived architecture report: `crates/haze-sync-server/control/log/20260711-191500Z-W1-ARCH-SRV-P7B-CONTRACTS-architect-report.md`

The chosen design places reusable asynchronous application transaction services in Server. HTTP routes and the future Worktree executor must consume the same services. Routes remain transport adapters; Core remains the policy owner; Storage remains passive and caller-transaction-owned.

Accepted Server baseline:

- SRV-P7A final code-bearing SHA: `37706634fd8dd2d9b299a1c453718f2de63981d0`
- Component CI run: `29161721748`, run number `1704`, conclusion `success`
- SRV-P7A clean status: `CLEAN_ACCEPT`
- existing Worktree composition remains honestly unavailable until later SRV-P7B3/P7B4 phases.

Parallel owner phases:

- `WT-P10` changes the Worktree-owned awaitable runtime contract on `component/worktree`;
- `STOR-P10` adds durable Worktree state/cursor repositories on `component/storage`;
- this phase must not depend on unaccepted intermediate commits from either branch.

## Required reads

Before editing, read:

- project implementation manifest, report template, implementation-worker prompt, and GitHub connector guidance;
- current Server control state and this prompt;
- archived architecture prompt/report and updated Server contract/plan/dependency-map/decisions/log at exact SHA `b98f5079ed90ccf7eaec79e617ae591e0c308ff4`;
- Server Cargo manifest, state, object-store integration, HTTP error mapping, auth/actor types, all file PUT/GET/changes routes, DELETE routes, conflict routes, planning/persistence helpers, transaction helpers, advisory-lock usage, idempotency handling, operation-log/revision/content repositories, and DB-backed tests;
- Core contracts for base revision, same-content, conflict preservation, delete guard, tombstone and idempotency decisions;
- API contracts for request parsing and public DTO/status/error ownership;
- Storage repository contracts currently consumed by Server.

Do not read CI diagnostics artifacts unless a later exact fixer prompt authorizes them.

## Goal

Extract and implement one reusable asynchronous internal application-service layer that owns correct transaction orchestration for file mutation, guarded delete, authoritative changes, and revision-content retrieval.

Both HTTP routes and the future Worktree executor must be able to call the same typed services without invoking HTTP handlers or duplicating Core/transaction policy.

This phase does not implement a Worktree executor, scheduler, host task, runtime status route, Storage migration, or public API change.

## Target module boundary

Prefer the accepted architecture names unless current code proves a narrower equivalent is cleaner:

- `src/application/mod.rs`
- `src/application/files.rs`
- `src/application/deletes.rs`
- `src/application/changes.rs`
- `src/application/idempotency.rs`
- `ServerApplicationServices`
- `ApplicationActor`
- `ApplyFileCommand` / `ApplyFileOutcome`
- `ApplyDeleteCommand` / `ApplyDeleteOutcome`
- `AuthoritativeChangesQuery` / `AuthoritativeChangeBatch`
- `RevisionContentQuery` / `AuthoritativeRevisionContent`
- `ApplicationError`

Do not create a generic framework. Keep the boundary explicit, typed, Server-internal, and aligned with current route behavior.

## Required operations

### 1. File create/update application service

Implement an async service operation equivalent to current correct PUT behavior.

Input must already be normalized/validated internal data and include:

- actor/source identity;
- vault path;
- explicit known or null base revision;
- content hash and bytes;
- durable idempotency metadata/fingerprint.

The service owns:

- durable idempotency lookup and same-key fingerprint comparison;
- SQLx transaction begin;
- normalized path advisory lock;
- authoritative current object/revision read;
- Core planning/decision;
- content-addressed object-store coordination;
- object/revision/current-state persistence;
- conflict preservation where Core requires it;
- operation-log append;
- safe replay outcome persistence;
- commit or rollback.

Typed outcomes must preserve at least:

- accepted authoritative revision/content metadata;
- same-content replay/no-op with authoritative metadata;
- conflict saved with safe conflict identity/path metadata already allowed internally;
- stale/invalid-base/unsafe/policy rejection;
- idempotency mismatch;
- safe not-found/internal categories where applicable.

Do not return public API DTOs from application services.

### 2. Guarded delete application service

Implement an async service operation equivalent to current correct DELETE behavior.

Input must include:

- actor/source identity;
- vault path;
- explicit known or null base revision as accepted by current contract;
- delete-guard metadata required by Core;
- durable idempotency metadata/fingerprint.

The service owns:

- idempotency lookup/fingerprint comparison;
- SQLx transaction and advisory path lock;
- current object/revision read;
- Core delete-guard evaluation;
- tombstone persistence and current-state clearing;
- operation-log append;
- idempotent replay outcome persistence;
- commit/rollback.

Typed outcomes must preserve tombstoned, not-found, rejected/guarded, stale/invalid base, replay, and safe failure semantics. No hard delete, filesystem trash, or provider side effect.

### 3. Bounded authoritative changes service

Implement an async read service for ordered authoritative changes:

- input: internal cursor/sequence position and explicit bounded limit;
- output: ordered change records plus `from`, `to`, and `has_more` semantics compatible with current changes behavior;
- use existing operation-log repository contracts;
- reject zero/unbounded/unsafe limits;
- do not expose raw external cursor JSON, database details, or provider payloads.

This service will later feed the Worktree executor but must also preserve current HTTP changes-route behavior.

### 4. Revision metadata and content retrieval service

Implement an async read service that returns verified authoritative revision metadata and bytes needed for materialization:

- resolve the requested authoritative path/revision through existing Storage repositories;
- load content-addressed bytes through the object store;
- verify expected metadata/hash consistency using existing accepted utilities;
- return a typed internal outcome;
- map missing/corrupt/unavailable conditions to stable safe categories without raw path, DB URL, SQLx/I/O error, or bytes in Debug/Display.

Do not perform Worktree filesystem materialization in this phase.

## Actor and idempotency ownership

Define `ApplicationActor` or an equivalent internal source identity that distinguishes HTTP principals from future Worktree execution without importing HTTP DTOs into service contracts.

Preserve HTTP client-supplied idempotency behavior exactly.

Add deterministic Worktree idempotency derivation helpers for future use, following the accepted architecture vocabulary:

- put: `wt:v1:<adapter_id>:put:<path_hash>:<base_or_null>:<content_hash>`
- delete: `wt:v1:<adapter_id>:delete:<path_hash>:<base_or_null>`

Requirements:

- use normalized vault-path hashing, never raw path text in the key where the architecture requires `path_hash`;
- output is deterministic and retry-stable;
- equivalent normalized inputs yield identical keys/fingerprints;
- materially different path/base/hash inputs cannot alias in tests;
- key values are never logged, displayed, exposed in status, or returned in public errors;
- do not invoke Worktree execution from these helpers.

If existing idempotency repository constraints require a typed adaptation, keep it Server-internal and document it. Do not change public API headers or DTOs.

## Route delegation and parity

Refactor existing routes to call the application services.

Routes must retain ownership only of:

- HTTP path/header/body parsing through API contracts;
- authentication and authorization mapping;
- construction of normalized internal commands;
- translation of typed application outcomes/errors into existing public API DTOs/status codes.

Extract or retire current private route helpers only after parity tests prove the new service path preserves behavior.

Do not leave two independent production implementations of PUT or DELETE transaction choreography. Temporary test-only comparison helpers may exist only within the same phase and must be removed before self-acceptance.

## Required compatibility

Preserve exactly:

- all existing public routes, methods, headers, request/response DTOs, status codes, and safe error bodies;
- Core ownership of overwrite/conflict/delete policy;
- Storage caller-transaction ownership;
- advisory-lock and atomic commit behavior;
- content-addressed object-store replay safety;
- conflict-saving and tombstone semantics;
- dependency-free router construction and passive-safe tests;
- SRV-P7A startup/worktree composition behavior;
- no provider calls, hard delete, repair execution, or hidden background work.

A DB rollback may leave an unreferenced content-addressed blob, but it must never make that blob authoritative without committed metadata. Preserve current cleanup/non-cleanup contract; do not add destructive orphan cleanup.

## Tests

Add focused unit and DB-backed integration/parity tests proving:

- file service accepted, same-content, stale/invalid-base, conflict-saved, idempotent replay, and same-key/different-fingerprint behavior;
- delete service tombstone, guard rejection, not-found, replay, stale/invalid-base, and rollback behavior;
- advisory locking and transaction ownership remain correct;
- operation-log and idempotency records commit atomically with authoritative mutation;
- failed transactions do not leave authoritative current-state/revision/conflict/tombstone/log/idempotency claims;
- route outputs for representative PUT/DELETE/changes/GET cases are byte/DTO/status compatible before and after extraction;
- changes service ordering, bounds, `from/to/has_more`, and empty results;
- revision-content service detects missing metadata/blob and hash inconsistency safely;
- deterministic Worktree idempotency derivation is stable, domain-separated, and secret-safe;
- application-service Debug/Display/error output contains no DB URL, tokens, idempotency keys, raw vault/local paths beyond already accepted safe vault-relative context, request bytes, SQLx/I/O error, or internal payload;
- dependency-free router tests remain unchanged and no database work occurs during router construction.

Mandatory DB-backed acceptance tests must use the established strict test-support path and run in Component CI. Do not silently treat required DB tests as optional/not-run.

## Allowed files

- `crates/haze-sync-server/src/application/**`
- `crates/haze-sync-server/src/routes/v1.rs`
- `crates/haze-sync-server/src/routes/v1/**`
- `crates/haze-sync-server/src/routes/delete.rs` and current delete submodules
- current GET/changes/conflict route modules only where needed to delegate to read/application services
- `crates/haze-sync-server/src/state.rs` only for explicit service construction/access
- existing Server-local transaction/error/idempotency helpers narrowly required by extraction
- Server-local unit and DB-backed tests for the above
- `crates/haze-sync-server/src/main.rs` only if module declaration/export is required; no startup/lifecycle behavior change
- `crates/haze-sync-server/docs/component-contract.md`
- `crates/haze-sync-server/docs/implementation-plan.md`
- `crates/haze-sync-server/docs/dependency-map.md`
- `crates/haze-sync-server/docs/decisions.md`
- `crates/haze-sync-server/docs/implementation-log.md`
- a focused application-services doc if useful
- `crates/haze-sync-server/control/report.md`

If another Server-local file is compile-proven necessary, document the exact need. Do not modify sibling components.

## Forbidden scope

- no changes under `crates/haze-sync-worktree/**` or `crates/haze-sync-storage/**`;
- no migration/schema changes;
- no Worktree concrete executor, scanner/reconciliation/materialization composition, runtime polling, host task, watcher, or scheduling;
- no Server config/runtime-policy/status/readiness changes for future hosted Worktree behavior;
- no new public route, DTO, header, status vocabulary, or API contract;
- no Core policy change;
- no provider behavior or internal HTTP self-calls;
- no nested runtime, `block_on`, detached task, or background loop;
- no hard delete, destructive cleanup, or automatic repair;
- no Cargo dependency/feature change unless compile-proven and explicitly reported as a blocker before expansion;
- no workflow changes;
- no unrelated cleanup;
- do not archive control files.

## CI policy

All source/test/docs changes must run normal Component CI.

Required gates:

- cargo fmt;
- cargo check;
- cargo test, including mandatory DB-backed parity tests;
- cargo clippy with warnings denied;
- Finalize CI diagnostics.

Only the final report-only commit may use `[skip ci]`. If CI fails, record the exact code-bearing SHA and run and stop; do not read raw logs or guess.

## Report

Write `crates/haze-sync-server/control/report.md` using `report-template.md`.

Set:

- `REPORT_TYPE: IMPLEMENTATION`
- `phase_id: SRV-P7B2`
- `chat_name: server — W1 SRV-P7B2 Application Services`

Use an honest status:

- `SELF_ACCEPT`
- `SELF_ACCEPT_PENDING_CI`
- `SELF_NEEDS_FIX`
- `BLOCKED_BY_CONTRACT`
- `BLOCKED_BY_SCOPE`
- `BLOCKED_BY_TOOLING`

The report must include exact service APIs/types, extracted route helpers, parity guarantees, transaction/idempotency ownership, DB tests actually run, changed files, final code-bearing SHA, authoritative CI evidence, secrecy/non-goal assessment, and readiness for mandatory clean-code review.
