# Dependency Map: server

## Component role

`haze-sync-server` is the runtime composition, application-service and HTTP execution boundary.

It owns startup, explicit dependency assembly, reusable async application services, transaction orchestration, hosted Worktree lifecycle, readiness/status mapping and safe transport translation.

It does not own Core policy, API public DTOs, Storage schema/repository mechanics, Worktree filesystem/scheduler semantics, provider behavior, CLI UX or Deployment automation.

## Upstream contracts consumed

### Common

Server consumes shared `AdapterId`, `VaultPath`, `RevisionId`, `ContentHash`, operation/conflict/tombstone identities and accepted adapter-mode vocabulary.

### Core

Core owns deterministic revision, base-revision, conflict preservation, delete guard, tombstone and idempotency decisions. Server application services invoke Core primitives and persist their typed outcomes; they must not reproduce policy in route or Worktree executor code.

### API

API owns HTTP parsing, headers, public DTOs and public error vocabulary. Existing route behavior remains compatible. New public Worktree status/manual-cycle fields require API-P8 before Server exposes them.

### Storage

Storage owns:

- migrations and table/row shape;
- caller-transaction-owned repositories;
- advisory locks;
- content-addressed object-store primitives;
- versioned Worktree instance/path-state repositories;
- monotonic contiguous adapter-cursor advancement.

Server owns transactions and application choreography over those primitives. Server must not embed replacement SQL for the new Worktree state contract.

### Worktree

Worktree owns:

- scanner and stable-file observation;
- import/delete planning;
- reconciliation;
- materialization, echo suppression and trash;
- scheduler policy validation, hint coalescing, full-scan correctness and count-only cycle summaries;
- async-compatible `WorktreeRuntimeCycle` and awaitable runtime `poll` contracts.

Server supplies the executor and host lifecycle. Server may use awaited bounded blocking-pool calls for synchronous filesystem phases, but never for SQLx/application-service work.

### Deployment

Deployment supplies operational configuration, directories, permissions, migration/backup/runbook behavior and service policy after product contracts are accepted. Product code must not hard-code deployment roots or secret files.

## Downstream consumers

- HTTP clients and Obsidian/GDrive adapters consume public Server/API routes.
- CLI consumes safe Server/API status and explicit manual-cycle contracts.
- Deployment consumes Server config, readiness, graceful shutdown and durable-state requirements.
- CI and integration tests consume deterministic application-service and runtime behavior.

## SRV-P7B ownership matrix

| Concern | Contract owner | Runtime consumer/host |
|---|---|---|
| Full-scan, planning, materialization, echo, trash | Worktree | Server executor |
| Awaitable cycle and scheduler/no-overlap validation | Worktree | Server host |
| Core conflict/delete/revision/idempotency policy | Core | Server application services |
| Public HTTP/operator DTOs | API | Server routes, CLI |
| Durable Worktree instance/path state | Storage | Server executor |
| Export checkpoint/cursor repository | Storage | Server executor |
| Application transaction/lock/object-store/operation-log choreography | Server | Routes and Worktree executor |
| Hosted task/startup/cancellation/shutdown | Server | Deployment |
| Operator commands | CLI | Server API |
| Paths, env, service and rollout runbooks | Deployment | Operator |

## Allowed dependency direction

```text
HTTP route -> API parser/auth -> ServerApplicationServices
Worktree host -> WorktreeRuntimeService -> ServerWorktreeCycleExecutor
ServerWorktreeCycleExecutor -> Worktree filesystem/planning interfaces
ServerWorktreeCycleExecutor -> ServerApplicationServices
ServerWorktreeCycleExecutor -> Storage Worktree state/cursor repositories
ServerApplicationServices -> Core policy + Storage repositories/object store
```

## Forbidden dependency direction

```text
Worktree -> Server route handlers
Worktree -> direct SQLx/Storage writes
Server executor -> internal HTTP self-call
Server routes -> duplicated Core policy
Storage -> scheduler/runtime/provider behavior
API -> runtime/DB/filesystem behavior
CLI -> direct DB/Worktree mutation
Deployment -> hidden product behavior
```

Also forbidden:

- nested Tokio runtimes, `Handle::block_on` or ad hoc async blocking;
- detached tasks or fabricated synchronous cycle summaries;
- production-only in-memory Worktree state/cursors;
- raw roots, cursors, idempotency keys, SQLx/I/O errors or secrets in public status;
- hard delete or automatic destructive repair.

## Fan-in order

1. Worktree async runtime contract (`WT-P10`).
2. Storage durable state/cursor contract (`STOR-P10`).
3. Server reusable application services (`SRV-P7B2`).
4. Server real executor after 1-3 (`SRV-P7B3`).
5. Server hosted lifecycle/config (`SRV-P7B4`).
6. API passive status contract (`API-P8`).
7. Server status/readiness mapping (`SRV-P7B5`).
8. CLI operator commands (`CLI-P6A`).
9. Deployment fan-in (`DEP-P5A`).

Owner-component clean acceptance and exact SHA synchronization are required before each consumer phase.

## Cross-component invariants

- Core remains authoritative and the only policy arbiter.
- Worktree is a materialized replica.
- Watchers are latency; full scans are correctness.
- Every imported mutation carries known or explicit-null base semantics.
- Durable state advances only after authoritative success.
- Export cursor advances only contiguously after successful/idempotently confirmed materialization.
- Existing HTTP behavior and dependency-free router tests remain compatible.
- Public errors/status remain secret-, path- and raw-error-safe.