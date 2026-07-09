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
