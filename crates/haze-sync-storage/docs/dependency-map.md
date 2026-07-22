# Dependency Map: storage

## Component role

`haze-sync-storage` is the durable persistence implementation component. It owns
schema/migrations, SQLx repositories, object-store primitives, storage-safe
errors, advisory locks, versioned Worktree durable state and adapter checkpoints.
It does not own Core policy, API DTOs, Server execution, adapter/filesystem
semantics, Deployment automation or CI policy.

## Independent development

Storage may independently implement:

- migrations and passive row models;
- caller-executor repository APIs;
- object-store and advisory-lock primitives;
- persistence validation and redaction;
- storage-focused unit and PostgreSQL tests.

If implementation requires Core decisions, Server orchestration, Worktree
filesystem behavior, public DTOs, provider calls, deployment secrets or workflow
changes, Storage reports the owner-component gate instead of absorbing it.

## Upstream contracts consumed

Storage consumes:

- Common validated `AdapterId`, `VaultPath`, revision IDs and SHA-256 values;
- accepted Core persistence outcomes, not Core implementation;
- accepted Server architecture requirements for caller-owned transaction
  composition;
- accepted Worktree state vocabulary only as persisted facts:
  present/tombstoned, authoritative revision/hash and bounded observation fields;
- PostgreSQL/SQLx contracts approved for the storage layer.

Storage does not consume API DTOs, route-private behavior, provider clients,
filesystem implementations or deployment paths as schema authority.

## Downstream contracts exposed

### General consumers

- Core/Server: revisions, objects, conflicts, tombstones, idempotency, operation
  log and object-store primitives;
- adapters through Server integration: durable progress and mapping facts;
- Deployment: migration, backup and object-store requirements;
- test harnesses: gated safe database/filesystem helpers.

### STOR-P10 consumers

Future Server Worktree executor/host phases consume:

- create-or-verify Worktree instance binding keyed by stable adapter identity;
- per-instance present/tombstoned path-state load/upsert and bounded snapshots;
- revision/hash-guarded reconciliation observation updates;
- transaction-only cursor lock/initialize/exact-contiguous advance;
- safe mismatch/version/gap/stale/missing/overflow errors.

Downstream code must not issue ad hoc SQL for these tables or expose internal rows
directly.

## Forbidden dependency directions

Storage must not:

- decide import/export, conflict/delete/base-revision or repair policy;
- normalize or persist raw Worktree roots;
- scan/watch/materialize/trash filesystem content;
- host runtime tasks or choose checkpoint timing;
- call providers;
- own HTTP/public status shapes;
- expose raw DB, cursor, fingerprint, token or path details;
- run production migrations implicitly.

## Cross-component invariants

- Server controls the transaction that combines filesystem outcome, path-state
  persistence and exact cursor advancement.
- Worktree controls filesystem and reconciliation semantics.
- Core remains the authoritative revision/conflict/delete arbiter.
- Storage validates persisted shape and exact checkpoint mechanics only.
- API/CLI/status layers receive sanitized summaries, never raw cursor JSON or root
  fingerprints.
- Migration 0010 does not invent an adapter/root binding for legacy rows.

## Fan-in gates

1. **Server bounded Worktree executor**
   - requires STOR-P10 clean acceptance and synchronization;
   - must use one transaction for path state plus exact cursor checkpoint;
   - must not bypass repositories with direct SQL.

2. **Deployment migration runbook**
   - requires accepted migration 0010;
   - must stop on non-empty legacy path-only state and use an explicit operator
     migration/clearance decision;
   - owns backup, invocation and rollback procedure.

3. **Public status/API**
   - requires API-owned DTOs;
   - may expose only safe lifecycle/count/cursor-presence summaries.

4. **PostgreSQL acceptance infrastructure**
   - ordinary Component CI proves compile/pure checks only;
   - STOR-P10 acceptance additionally requires the explicit ignored DB test
     command against a dedicated database.

## Dependency rules

- Storage persists facts, not semantics.
- Repository helpers remain narrow and caller-transaction-owned.
- Migration changes are explicit and operationally documented.
- No live provider/client dependency is required for Storage tests.
- Consumer phases start only from an accepted synchronized Storage SHA.

## Current contract notes

- Storage owns migration content; Server/Deployment own production execution.
- Storage owns durable Worktree state; Worktree does not gain direct DB ownership.
- The broad monotonic cursor API remains for compatibility; Worktree export uses
  the exact-contiguous transaction API.
- Physical retention cleanup and object-store garbage collection remain deferred
  owner-scoped work.
