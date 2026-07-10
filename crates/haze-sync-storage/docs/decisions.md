# Decisions: storage

## 2026-07-05 — Storage persists facts, not Core policy

Decision:

`haze-sync-storage` owns schema, rows, repositories, object-store operations, locks, and storage-safe errors. It does not decide whether a write is accepted, conflicted, tombstoned, replayed, rejected, exported, or resolved.

Rationale:

Core is the safety arbiter. If Storage starts deciding sync policy, the system gains duplicate and potentially divergent behavior. Storage should make persistence reliable and transactional while preserving decisions made by Core and orchestrated by Server.

Alternatives:

- Implement accepted/conflict/delete decision logic in repository helpers.
- Let Storage rows become public API DTOs.
- Move Core service logic into Storage for convenience.

Consequences:

- Server/integration phases must map Core decisions into Storage writes.
- Storage repository names and docs should describe persisted facts, not policy ownership.
- Storage tests should check persistence behavior, not Core semantic correctness except through fixture-like integration tests.

Affected contracts:

- component contract;
- dependency map;
- Core/Storage boundary;
- Server transaction fan-in;
- repository implementation phases.

## 2026-07-05 — Repository helpers use caller-owned executors and transactions

Decision:

Storage repository helpers execute SQL against caller-owned SQLx executors or transactions. Storage does not create pools, own runtime state, or silently open independent transactions for higher-level flows.

Rationale:

Server must compose multi-step flows such as PUT, conflict preservation, delete, idempotency, and cursor updates atomically. Hidden transactions inside repositories would make cross-table consistency harder to reason about.

Alternatives:

- Give each repository helper its own pool or transaction.
- Let Storage own complete service-layer workflows.
- Move transaction boundaries into Core.

Consequences:

- Server/fan-in code must explicitly manage transaction boundaries.
- Repository helpers stay testable and reusable.
- Documentation must identify transaction-sensitive helpers.

Affected contracts:

- repository helpers;
- Server runtime composition;
- Storage test support;
- Core trait adaptation.

## 2026-07-05 — Object-store paths derive only from content hashes

Decision:

The local object store stores blobs under a canonical content-addressed layout derived from validated SHA-256 values. Vault paths, provider paths, adapter IDs, and user filenames must not influence blob paths.

Rationale:

Object-store paths must be stable, deduplicated, provider-neutral, and safe from path traversal or user-controlled path injection. Vault path identity belongs in metadata tables, not blob storage layout.

Alternatives:

- Store blobs under vault paths.
- Store blobs under provider file IDs.
- Include adapter IDs in object-store paths.

Consequences:

- Blob deduplication by hash is natural.
- Object-store cleanup/retention must consult metadata, not path layout.
- Object-store errors must remain path-redacted.

Affected contracts:

- object_store;
- content_blobs repository;
- deployment directory layout;
- backup/restore planning.

## 2026-07-05 — Storage row models are not public API output

Decision:

Storage row models may represent persisted database fields directly, including values that are unsafe for public output. API/Server/CLI must map and sanitize before exposing data.

Rationale:

Database rows may contain token hashes, raw external cursor JSON, internal object-store paths, provider metadata, or operational details. Public DTOs need stricter secrecy and stability boundaries.

Alternatives:

- Return row models directly from API routes.
- Remove sensitive fields from storage rows even if persistence needs them.
- Duplicate every row model in API manually without mapping discipline.

Consequences:

- Server/API mapping code is required.
- Admin/status DTOs should expose cursor presence or summaries, not raw values.
- Tests should check redaction at public boundaries, not by weakening storage persistence.

Affected contracts:

- models;
- repositories;
- API DTOs;
- Server route mappings;
- CLI status/doctor output.

## 2026-07-05 — Advisory locks are transaction-scoped PostgreSQL discipline

Decision:

Storage provides deterministic path lock keys and `pg_advisory_xact_lock` helper, but correctness depends on callers acquiring locks within the transaction that owns the critical section.

Rationale:

Per-path serialization is necessary for safe multi-step file updates. PostgreSQL transaction-scoped advisory locks are simple and durable enough for V1, but they are not process-global magic and do not protect code that skips them.

Alternatives:

- Use in-memory process locks.
- Use table-row locking only.
- Let Core own locking.

Consequences:

- Server fan-in must consistently acquire locks for mutation paths.
- Tests should prove deterministic key derivation and, where DB is available, transaction lock behavior.
- Deployment must use one authoritative PostgreSQL database for lock discipline to work.

Affected contracts:

- locks;
- Server transaction fan-in;
- Storage repository phases;
- E2E concurrency tests.

## 2026-07-05 — GDrive mapping persistence does not grant adapter DB ownership

Decision:

Storage may own persisted `gdrive_mapping` table and repository helpers, but that does not automatically mean the GDrive adapter owns direct database access. Runtime access boundary must be decided by Server/API/adapter integration design.

Rationale:

The mapping table is a persistence concern, while provider semantics, OAuth, echo guard, and import/export behavior belong to the GDrive adapter. Direct DB access from the adapter would couple adapter runtime to Storage and may bypass Server/Core/API safety boundaries.

Alternatives:

- Give the GDrive adapter direct unrestricted DB access.
- Move mapping persistence entirely into GDrive adapter local state.
- Put provider semantics into Storage repositories.

Consequences:

- A future integration decision is required before wiring GDrive mapping runtime access.
- Storage can still provide repository primitives if chosen.
- GDrive adapter docs must not assume direct DB ownership by default.

Affected contracts:

- gdrive_mapping row/repository planning;
- GDrive adapter component contract;
- Server/API adapter integration;
- deployment topology.

## 2026-07-09 — Object-store root is caller-owned durable storage

Decision:

`LocalObjectStore` treats its root as a caller-provided durable storage directory. Storage owns the hash-addressed layout below that root, including the internal `tmp` directory and `sha256/<prefix>/<full-hex>` blob layout, but it does not choose deployment paths, create backups, run garbage collection, or expose local paths in public errors.

Rationale:

The object store must be reusable by Server, deployment, backup/restore, and test harnesses without hard-coding environment-specific paths. Keeping temporary files under the same root keeps commit operations on one filesystem for the current hard-link commit strategy. Path-free errors preserve the component secrecy contract even when filesystem sources include local paths.

Alternatives:

- Hard-code a production object-store root in Storage.
- Put temporary blobs outside the object-store root.
- Include local filesystem paths in public error formatting for easier debugging.
- Add cleanup or garbage-collection jobs directly to Storage.

Consequences:

- Server/deployment must configure and provision the root directory.
- Backup/restore and doctor phases must treat the root as durable data.
- Storage tests cover missing blobs, corrupted blobs, unexpected entries, duplicate writes, temporary cleanup after failed commits, and path-free error formatting.
- Retention cleanup and garbage collection remain future scoped work outside STOR-P3.

Affected contracts:

- object_store;
- component contract;
- deployment/runbook planning;
- backup/restore planning;
- future doctor checks.

## 2026-07-09 — Repository helpers stay passive and transaction-sensitive

Decision:

Repository helpers remain caller-owned-executor primitives. Helpers that mutate more than one logical part of a future Server/Core flow are transaction-sensitive: callers must compose them inside the transaction that owns the corresponding Core decision, idempotency check, operation-log append, path advisory lock, or cursor advancement. Storage validates only narrow storage constraints such as sequence bounds, list limits, representable byte sizes, operation-kind vocabulary, conflict-status vocabulary, and cursor monotonicity.

Rationale:

Server and Core fan-in need to combine content blobs, file revisions, sync objects, operation-log entries, idempotency records, tombstones, conflicts, and adapter cursors atomically. If repositories open hidden transactions or make policy decisions, callers lose control over cross-table consistency. If repositories leak SQLx errors, public layers could accidentally expose database URLs, SQL, credentials, or local paths.

Alternatives:

- Let each repository helper create its own pool or transaction.
- Let Storage infer Core outcomes such as accepted write, conflict, tombstone, replay, or cursor policy.
- Return raw SQLx errors and rely on API/Server to redact them.
- Allow unbounded repository list pages.

Consequences:

- Server/Core integration must explicitly manage transaction boundaries and path locks.
- Repository errors use stable safe codes/messages instead of raw SQLx formatting.
- Repository list helpers must share bounded page validation.
- Repository row outputs remain internal values that public surfaces must sanitize.

Transaction-sensitive helper groups:

- content blob + file revision + sync object current-revision updates;
- operation-log append and changes-feed reads around Server fan-in;
- idempotency check/store helpers around request replay handling;
- conflict and tombstone metadata updates around Core conflict/delete decisions;
- adapter cursor initialization/update around adapter changes-feed progress.

Affected contracts:

- repositories;
- component contract;
- Server transaction fan-in;
- Core persistence boundary;
- API/CLI error sanitization.

## 2026-07-09 — Normal file flow is caller-composed repository work

Decision:

Normal file PUT/GET/changes support is represented as caller-composed repository primitives rather than a Storage-owned service workflow. A caller-owned transaction should acquire the path advisory lock, create or read content blob metadata, create or find the sync object, insert the immutable file revision, set the current revision according to a Core decision, and append operation-log metadata. Storage returns row/value outputs and change-feed pages that Server can map into Core/API models.

Rationale:

Storage needs to prove durable repository mechanics for the normal file path without absorbing Core upsert policy or Server route orchestration. Keeping the flow as repository composition preserves atomicity under Server-owned transactions while allowing Core to decide base-revision, conflict, idempotency, and delete semantics.

Alternatives:

- Add a Storage `upsert_file` service that decides accepted/conflict outcomes.
- Add HTTP handlers or upload streaming logic in Storage.
- Let Server bypass Storage repositories and write ad hoc SQL.
- Append operation-log entries outside the transaction that updates file metadata.

Consequences:

- Server fan-in must compose the normal file flow explicitly inside one transaction.
- Core remains responsible for whether a revision becomes current.
- Operation-log changes pages remain storage-safe internal rows, not public API DTOs.
- Repository tests cover the normal flow against PostgreSQL when test-support database configuration is available.

Affected contracts:

- content_blobs repository;
- sync_objects repository;
- file_revisions repository;
- operation_log repository;
- locks;
- Server transaction fan-in;
- Core write-decision boundary.

## 2026-07-10 — Conflict acceptance and restore remain fan-in workflows

Decision:

Storage exposes passive primitives for conflict insertion, bounded listing, lookup, guarded lifecycle status updates, tombstone insertion, lookup, one-shot `restored_at` updates, and related operation-log rows. Full `accept_conflict` content replacement and file restoration remain Core/API/Server fan-in workflows composed inside caller-owned transactions.

Rationale:

Accepting a conflict can require reading preserved revisions, selecting replacement content, updating the current revision, materializing or removing backup paths, resolving metadata, and appending operation-log entries. Restoring a tombstone can similarly require Core delete guards, object metadata updates, and adapter-visible operations. Storage can persist each decided fact but must not choose the accepted version, bypass base-revision policy, move files, or infer whether a restore is safe.

Alternatives:

- Add a Storage `accept_conflict` service that chooses and installs replacement content.
- Treat setting `conflicts.status = resolved` as sufficient to replace file content.
- Let tombstone `restored_at` updates implicitly clear object deletion state.
- Perform conflict or restore operation-log appends outside the caller transaction.

Consequences:

- Core owns conflict acceptance and restore/delete policy.
- Server/API fan-in must coordinate content, object/revision metadata, conflict/tombstone metadata, and operation-log writes atomically.
- Storage lifecycle updates are guarded metadata transitions only.
- Physical trash movement, retention cleanup, and hard delete remain out of scope.

Affected contracts:

- conflicts repository;
- tombstones repository;
- operation_log repository;
- Core conflict/delete policy;
- API conflict actions;
- Server transaction fan-in.
