# W1-STOR-P10 — Durable Worktree instance, path state, and export cursor

Before starting, name this worker chat exactly:

`storage — W1 STOR-P10 Worktree Durable State`

Component: storage
Path: crates/haze-sync-storage
Branch: component/storage
PR: #47
Role: implementation-worker

Work only through the GitHub connector. Do not merge the PR, change its draft state, rebase, reset, rewrite history, force-push, modify `main`, modify sibling branches, or implement Server/Worktree runtime behavior.

## Accepted architecture authority

The Server architecture phase `ARCH-SRV-P7B-CONTRACTS` returned `ARCHITECT_CHANGED_CONTRACTS` and assigned durable Worktree state/cursor ownership to Storage.

Authoritative evidence:

- Server architecture documentation SHA: `b98f5079ed90ccf7eaec79e617ae591e0c308ff4`
- architecture Component CI run: `29165123142`, run number `1719`, conclusion `success`
- final architecture report-only commit: `22b5e993431a1c0a858716ac835a3ece8ade6d5c`
- archived architecture report on `component/server`: `crates/haze-sync-server/control/log/20260711-191500Z-W1-ARCH-SRV-P7B-CONTRACTS-architect-report.md`

Accepted Storage baseline:

- code-bearing SHA: `aa59064d641f4850f7c70fa615e638b52613dd95`
- Component CI run: `29124956486`, run number `1580`, conclusion `success`
- prior Server production feature-isolation correction is accepted and must not be reopened.

The architecture determined that the existing path-only `worktree_state` shape is insufficient for per-instance identity, tombstones, versioning, reconciliation observations, and crash-safe cursor semantics. `adapter_cursors.last_core_seq` may remain the export checkpoint only if Storage adds locked, exact-contiguous, monotonic advancement operations.

## Required reads

Before editing, read:

- project implementation manifest, report template, implementation-worker prompt, and GitHub connector guidance;
- current Storage control state and this prompt;
- Storage component contract, implementation plan, dependency map, decisions, implementation log, Cargo manifest, all migrations, repository modules, models, transaction conventions, adapter cursor code, operation-log/revision/tombstone repositories, test support, and migration tests;
- existing `worktree_state` and `adapter_cursors` schema and every current consumer;
- the Server architecture report and updated Server docs at exact SHA `b98f5079ed90ccf7eaec79e617ae591e0c308ff4`;
- Worktree public state/reconciliation types on accepted baseline `component/worktree@4f7bc748d9b901d7d5c3e43c845ba407c0c36e59` only to understand semantic data, not to copy filesystem/runtime policy into Storage.

Do not read CI diagnostics artifacts unless a later exact fixer prompt authorizes them.

## Task

Implement the minimum Storage-owned schema, models, repositories, and tests for durable Worktree instance/path state and exact export cursor progression.

Storage remains passive: repositories operate over caller-owned SQLx transactions/connections and do not own Server scheduling, Core policy, Worktree filesystem behavior, or public API DTOs.

### 1. Versioned Worktree runtime-instance binding

Add a durable instance record keyed by `adapter_id`.

Required fields/semantics:

- adapter identity, with V1 default expected downstream as `worktree` but no Server defaulting logic in Storage;
- non-public SHA-256 fingerprint of the normalized configured root, never the raw root;
- explicit state/schema format version;
- safe created/updated/last-success timestamps only where useful;
- bind-or-verify behavior: first valid bind inserts; subsequent binds must match fingerprint and supported version;
- fingerprint/version mismatch fails closed through stable safe Storage errors;
- raw roots, database URLs, SQL text, row payloads, and internal errors are never exposed through Display/Debug/public error messages.

Do not invent root normalization policy inside Storage. Accept an already-computed fingerprint through a typed input.

### 2. Per-instance path state

Create or migrate durable path state keyed by `(adapter_id, vault_path)`.

Required representation:

- explicit `present` or `tombstoned` state kind;
- last-applied authoritative revision;
- content hash required for present state and absent for tombstone state;
- versioned, bounded reconciliation observation fields only when required by accepted Worktree contracts;
- relational typed columns for identity, revision, kind, hash, and checkpoint-critical fields;
- optional JSON only for bounded reconciliation metadata with an explicit schema version;
- unknown state/reconciliation versions fail safely;
- no production serialization of raw local absolute paths.

Repository operations must include at minimum:

- `load_path_state`;
- bounded/deterministic `load_snapshot` for one adapter instance;
- `upsert_present_state`;
- `upsert_tombstoned_state`;
- update of accepted reconciliation observation fields, if included;
- no hard-delete or implicit cleanup operation.

Use existing Common identifiers such as `AdapterId`, `VaultPath`, `RevisionId`, and `ContentHash` rather than unvalidated strings wherever the current Storage conventions support them.

### 3. Export cursor locking and exact-contiguous advancement

Use or extend `adapter_cursors.last_core_seq` as the authoritative Worktree export checkpoint.

Add caller-transaction-owned operations for:

- lock and read current cursor for an adapter instance;
- initialize a missing cursor safely;
- advance only from the exact expected current value to an exact next contiguous value;
- reject regression, gaps, stale expected values, and overflow;
- keep advancement monotonic and race-safe under concurrent transactions;
- represent cursor presence/value internally without exposing raw external cursor payloads publicly.

Do not advance a cursor merely because a batch was fetched. Future Server executor will advance only after one authoritative operation is safely materialized and path state is ready to commit in the same DB transaction.

### 4. Transaction and crash semantics

Repositories must support these future caller-owned atomic sequences:

- accepted/same-content import outcome, then present path state update;
- accepted tombstone, then tombstoned path state update;
- conflict/rejection/failure leaves path state unchanged;
- one export materialization, then path state plus exact cursor advancement in one transaction;
- rollback leaves no false applied state or cursor;
- replay after materialized-but-uncheckpointed export can safely observe already-current content and then commit state/cursor;
- deterministic idempotent import replay can repair missing state after crash.

Storage does not perform filesystem materialization and must not claim DB/filesystem atomicity. It supplies the transaction primitives required for the Server-owned crash/replay protocol.

### 5. Migration compatibility

Inspect the existing `worktree_state` table and choose the minimum safe migration path.

Requirements:

- do not silently reinterpret legacy rows as belonging to the new default instance unless the existing data contract proves that mapping safe;
- migration must be deterministic and fail safely on incompatible/ambiguous legacy state;
- update schema-completeness/test-support expectations for all new base tables/columns/indexes;
- preserve unrelated existing data and repository behavior;
- indexes and constraints must enforce identity uniqueness, state-kind/hash consistency, supported version bounds where practical, and cursor invariants without embedding Core policy.

Document any explicit legacy backfill, quarantine, or no-data precondition.

## Tests

Add focused unit, migration, and PostgreSQL-backed tests proving:

- first bind succeeds and identical rebind is idempotent;
- root fingerprint mismatch and unknown version fail closed;
- two adapter identities cannot read or overwrite each other’s path state;
- present/tombstoned invariants are enforced;
- snapshot ordering and bounds are deterministic;
- state upserts use caller-owned transactions and roll back cleanly;
- cursor initialization is safe;
- exact contiguous advance succeeds;
- cursor regression, gap, stale expected value, and concurrent race are rejected;
- transaction rollback does not advance cursor or leave false path state;
- migration from every supported pre-STOR-P10 schema is deterministic;
- partial/incompatible schema is rejected;
- Debug/Display/errors contain no DB URL, raw root, raw SQLx error, unsafe cursor payload, or row contents.

Use the strict Storage test-support path for mandatory DB tests. Optional silent skipping is not acceptable for tests that constitute acceptance evidence; report honestly if CI infrastructure cannot provide the required DB execution.

## Allowed files

- `crates/haze-sync-storage/migrations/**`
- `crates/haze-sync-storage/src/models/**` or current model locations
- `crates/haze-sync-storage/src/repositories/**` or current repository locations
- Storage transaction/error/export modules narrowly required by these repositories
- Storage-local tests and `test_support` schema expectations/fixtures narrowly required for STOR-P10
- `crates/haze-sync-storage/src/lib.rs` for exports
- `crates/haze-sync-storage/docs/component-contract.md`
- `crates/haze-sync-storage/docs/implementation-plan.md`
- `crates/haze-sync-storage/docs/dependency-map.md`
- `crates/haze-sync-storage/docs/decisions.md`
- `crates/haze-sync-storage/docs/implementation-log.md`
- a focused Storage doc for Worktree durable state if useful
- `crates/haze-sync-storage/control/report.md`

If another Storage-local file is compile- or migration-proven necessary, document it exactly. Do not modify sibling components.

## Forbidden scope

- no Server runtime/application service code;
- no Worktree scanner/planner/materializer/runtime code;
- no Core policy or API DTO changes;
- no provider/filesystem behavior;
- no background jobs, automatic cleanup, hard delete, or repair execution;
- no raw configured root storage;
- no production in-memory substitute;
- no migration execution policy in Server/Deployment;
- no workflow changes;
- no unrelated cleanup;
- do not archive control files.

## CI policy

All source/schema/test/docs changes must run normal Component CI.

Required gates:

- cargo fmt;
- cargo check;
- cargo test;
- cargo clippy with warnings denied;
- migration/schema validation and mandatory PostgreSQL tests used by the phase;
- Finalize CI diagnostics.

Only the final report-only commit may use `[skip ci]`. If CI fails, record the exact code-bearing SHA and run and stop; do not inspect raw logs or guess.

## Report

Write `crates/haze-sync-storage/control/report.md` using `report-template.md`.

Set:

- `REPORT_TYPE: IMPLEMENTATION`
- `phase_id: STOR-P10`
- `chat_name: storage — W1 STOR-P10 Worktree Durable State`

Use an honest status:

- `SELF_ACCEPT`
- `SELF_ACCEPT_PENDING_CI`
- `SELF_NEEDS_FIX`
- `BLOCKED_BY_CONTRACT`
- `BLOCKED_BY_SCOPE`
- `BLOCKED_BY_TOOLING`

The report must include migration strategy, exact schema/repository contracts, legacy handling, transaction/cursor invariants, DB tests actually executed, changed files, final code-bearing SHA, authoritative CI evidence, secrecy assessment, and readiness for mandatory clean-code review.
