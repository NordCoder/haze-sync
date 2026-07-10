# Component Contract: core

## Responsibility

`haze-sync-core` owns deterministic Core service-layer primitives for Haze Sync.

The component is responsible for pure policy and decision logic that keeps Core safe as the source of truth:

- normal file revision upsert semantics;
- stale, unknown, and explicit-null base revision safety outcomes;
- conflict policy selection and conflict-copy path generation;
- metadata-only `conflict_saved` preservation planning;
- storage/API-neutral conflict resolution action planning;
- tombstone metadata creation, validation, and pure restore/retention-cleanup eligibility classification;
- mass-delete guard threshold evaluation;
- idempotency key, fingerprint, and replay decision primitives;
- operation-log, changes-feed, and adapter-cursor value models;
- passive doctor-check result models and redacted summaries.

Core does not own runtime wiring. It must remain usable from tests and fan-in layers without starting a server, opening a database pool, calling an adapter, or touching provider APIs.

## Public interfaces

The public crate surface is exported from `src/lib.rs`.

Current public modules:

- `revision_service` — storage-agnostic upsert algorithm over caller-owned repository, content-store, and operation-log traits.
- `conflict_service` — conflict policy names, conflict record metadata, conflict-copy path requests, conflict path generation, conflict validation errors, and storage/API-neutral conflict resolution action plans.
- `policy_engine` — pure conflict policy application that keeps the current revision authoritative and plans incoming-content backup/conflict metadata.
- `conflict_saved_planner` — fan-in planner that converts `revision_service` `conflict_saved` outcomes into storage/API-neutral conflict preservation plans.
- `tombstone_service` — tombstone identifiers, validated retention and restore metadata, restore eligibility, and retention-cleanup eligibility without restore or cleanup side effects.
- `delete_guard` — deterministic mass-delete threshold policy, per-run scope model, category-scoped manual unlock model, and guard decisions.
- `idempotency` — idempotency key validation, safe request fingerprints, stored response snapshots, stored record models, and replay/conflict classification.
- `operation_log` — operation sequences, bounded changes queries, operation kinds, changes pages, tombstone identifiers for operation rows, and cursor update classification.
- `doctor` — passive doctor check identifiers, JSON-safe check results, redacted check details, and aggregate report summaries.

The crate also exposes `CRATE_ROLE` and `package_name()` for scaffold smoke checks and documentation.

## Input contracts

Core inputs must already be authenticated and scoped by the caller.

Required input rules:

- Paths, IDs, adapter IDs, revision IDs, operation IDs, conflict IDs, and content hashes use `haze-sync-common` value types at Core boundaries wherever practical.
- Raw boundary strings may be accepted only by explicit parser helpers that validate and return safe Core errors.
- Every write decision that can overwrite existing file content must include an explicit `base_revision_id`, where `None` means explicit `base_revision_id = null`.
- Upsert content must be accompanied by the expected SHA-256 content hash.
- Caller-owned repository/content-store/operation-log implementations must enforce their own transaction boundaries, locks, durability, and persistence behavior.
- Tombstone cleanup classification must receive an explicit UTC evaluation timestamp; Core must not read the system clock.
- Restored tombstone metadata is valid only when restore timestamp, adapter ID, and restore revision ID are all present together.
- Delete-guard evaluations must include an adapter/run scope and the caller's proposed delete count plus pre-run total file count.
- Manual delete unlocks must be scoped to the exact adapter/run and threshold category they cover.
- Conflict resolution planning must receive a validated open conflict record and a Core-owned action name.
- Doctor inputs must be pre-redacted. Core doctor helpers must receive booleans, counts, hashes, and other safe summaries rather than secrets, database URLs, provider payloads, or local absolute paths.

## Output contracts

Core outputs are safe, deterministic Rust values suitable for API/storage mapping by downstream components.

Required output rules:

- Accepted upserts return revision metadata and operation-log metadata, not HTTP responses.
- Duplicate same-content upserts return a same-content outcome without creating a new revision in Core's pure decision model.
- Hash mismatch outcomes include expected and actual content hashes only.
- Stale, unknown, or explicit-null base writes against different existing content must not silently overwrite current content. They return a `conflict_saved` semantic outcome when current content exists.
- Conflict preservation outputs are metadata-only plans. They do not write conflict-copy bytes, persist conflict rows, or append operation-log entries.
- Conflict resolution outputs are storage/API-neutral plans. `accept_current`, `keep_both`, and `mark_resolved` are metadata-only from Core's perspective; `accept_conflict` requires downstream storage to create a new current revision from preserved incoming conflict content.
- Tombstone outputs record delete intent and retention metadata only. They do not physically remove revisions, blobs, worktree files, or provider files.
- Restore eligibility classifies a tombstone as restore-ready or already restored; it does not create a restore revision or mutate metadata.
- Retention-cleanup eligibility classifies a tombstone as retained, eligible at/after the exact retention boundary, or ineligible because it was restored; it does not delete data.
- Delete-guard outputs classify whether a proposed delete run is allowed, blocked by count, blocked by ratio, or blocked pending manual unlock.
- Operation-log and changes-feed outputs validate sequence ordering and limit bounds but do not query storage.
- Doctor outputs contain JSON-safe statuses, messages, counts, booleans, and sampled content hashes only.
- Public serialization must not expose raw file bytes. In particular, in-memory incoming conflict bytes are skipped during serialization.

## Error contracts

Public errors must be safe and must not expose secrets, raw provider payloads, database URLs, local absolute paths, stack traces, bearer tokens, OAuth tokens, token hashes, Idempotency-Key values, raw file content, or raw SQLx errors.

Core errors should provide stable machine-readable codes and stable human-readable messages where the module already exposes that pattern.

## Persistence/runtime ownership

Core owns no concrete persistence or runtime resources.

Core does not own:

- Axum routing or HTTP extraction;
- SQLx repositories, migrations, pools, transactions, or advisory locks;
- content-addressed object-store filesystem writes;
- operation-log database append implementation;
- provider or adapter calls;
- Google Drive API calls;
- Obsidian plugin runtime behavior;
- worktree filesystem scanning, trash, materialization, or watcher loops;
- background jobs, async runtimes, queues, or schedulers;
- restore execution, hard-delete cleanup, or retention execution.

Persistence and runtime components may use Core outputs to decide what to write, but they must implement storage, transactional integrity, idempotent persistence, and external side effects outside this crate.

## Security and secrecy rules

- Do not commit secrets.
- Do not expose tokens or token hashes in public outputs.
- Do not expose local absolute paths or database URLs.
- Do not serialize raw provider payloads unless an explicit non-Core component contract allows it.
- Do not include bearer tokens, OAuth tokens, token hashes, Idempotency-Key values, raw SQLx errors, stack traces, or raw file bytes in public Core errors or reports.
- Keep doctor outputs redacted and summary-oriented.
- Keep idempotency stored responses limited to safe replay headers and public JSON bodies.
- Preserve the invariant that adapters submit facts to Core and do not decide overwrite/conflict policy themselves.

## Non-goals

Core does not implement:

- Axum route handlers or route registration;
- API DTO ownership beyond Core-facing value models;
- SQLx repositories, migrations, or database schema changes;
- server configuration, auth/token verification, or middleware;
- object-store implementation details such as temp write, fsync, rename, or path layout;
- provider SDK integration;
- Google Drive, Obsidian, or worktree adapter loops;
- conflict-resolution API wiring;
- restore revision creation, physical trash moves, hard delete, or retention cleanup jobs;
- deployment, CLI command parsing, or server lifecycle management;
- broad runtime side effects.

## Dependencies

See `dependency-map.md`.

## Dependents

See `dependency-map.md`.

## Invariants

Core component invariants:

- Core is the only conflict arbiter.
- Adapters never silently overwrite.
- Every write has `base_revision_id` or explicit `base_revision_id = null`.
- Unknown base revision never overwrites different existing content.
- Delete is tombstone + trash + retention at the product level, never immediate hard delete.
- Conflict never blocks sync; preserve-both is the default safety posture.
- Bootstrap starts from exactly one trusted source.
- Watchers are for latency; scans are for correctness.
- Core helpers remain deterministic and side-effect free unless a future component contract explicitly expands this crate.
- Core decisions are storage/API-neutral; downstream layers map them to persistence rows, HTTP responses, CLI output, or adapter behavior.

## Test obligations

Core tests should cover the public behavior of each pure module without requiring secrets, network access, provider accounts, production databases, or persistent local state.

Required test categories:

- normal upsert outcomes: new file, current-base revision, duplicate same content, hash mismatch;
- stale, unknown, and explicit-null base behavior with no silent overwrite;
- conflict path generation, recursive conflict-area rejection, and path mismatch validation;
- conflict policy outcomes and conflict-saved planning;
- conflict resolution action planning, including metadata-only actions and new-current-revision effects;
- tombstone ID and metadata validation, restore-ready/already-restored classification, and retention cleanup before/at/after the exact boundary;
- delete guard zero-total behavior, exact count/ratio boundaries, over-threshold decisions, overflow-safe ratio comparisons, and adapter/run/category-scoped unlock mismatch;
- idempotency key validation, request fingerprint determinism, replay of same request, and conflict for different request;
- changes query bounds, monotonic changes pages, operation kind parsing, and cursor regression rejection;
- passive doctor check status derivation and redaction-oriented detail payloads;
- serde roundtrips for public DTO/value models where serialization is part of the contract.

Checks expected for Core changes when the environment supports shell execution:

- `cargo fmt --check`
- `cargo check -p haze-sync-core`
- `cargo test -p haze-sync-core`
- `cargo clippy -p haze-sync-core --all-targets -- -D warnings`

## Contract change protocol

If implementation would require serious hacks, unsafe behavior, or cross-component changes, the worker must report `CONTRACT_CHANGE_REQUESTED` instead of silently broadening scope.
