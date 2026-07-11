# Component Contract: server

## Responsibility

`haze-sync-server` is the runtime composition and application-service boundary for Haze Sync.

Server owns:

- Axum routing, listener startup, graceful shutdown, auth execution, readiness and safe status mapping;
- explicit runtime dependency assembly;
- reusable asynchronous application services that coordinate Core decisions with Storage repositories and the object store;
- SQLx transaction and advisory-lock choreography for accepted mutations;
- hosting the built-in Worktree runtime without taking ownership of Worktree filesystem semantics.

Server is not the source of sync policy. Core owns revision/conflict/delete/idempotency decisions. API owns public HTTP DTOs, parsers and public errors. Storage owns migrations, durable repository primitives and object-store primitives. Worktree owns scanning, planning, reconciliation, materialization, echo suppression, trash and runtime scheduling semantics.

## Public HTTP compatibility

Existing HTTP behavior remains backward-compatible:

- `GET /health`, `GET /ready`, `GET /v1/server-info`;
- `GET /v1/changes`, `GET|PUT|DELETE /v1/files/{path}`;
- conflict list/resolve routes;
- read-only admin/status routes.

SRV-P7B changes internal composition only. Any new public Worktree status/manual-cycle DTO requires an API-owner phase before Server route implementation.

## Reusable asynchronous application services

HTTP routes and Worktree execution must share Server-owned services. Routes become transport adapters and must not remain the only owners of correct mutation behavior.

Target modules and types:

```text
src/application/mod.rs
src/application/files.rs
src/application/deletes.rs
src/application/changes.rs
src/application/idempotency.rs

ServerApplicationServices
ApplicationActor
ApplyFileCommand / ApplyFileOutcome
ApplyDeleteCommand / ApplyDeleteOutcome
AuthoritativeChangesQuery / AuthoritativeChangeBatch
RevisionContentQuery / AuthoritativeRevisionContent
ApplicationError
```

Contract-level operations:

```rust
async fn apply_file(
    &self,
    actor: ApplicationActor,
    command: ApplyFileCommand,
) -> Result<ApplyFileOutcome, ApplicationError>;

async fn apply_delete(
    &self,
    actor: ApplicationActor,
    command: ApplyDeleteCommand,
) -> Result<ApplyDeleteOutcome, ApplicationError>;

async fn list_authoritative_changes(
    &self,
    query: AuthoritativeChangesQuery,
) -> Result<AuthoritativeChangeBatch, ApplicationError>;

async fn load_revision_content(
    &self,
    query: RevisionContentQuery,
) -> Result<AuthoritativeRevisionContent, ApplicationError>;
```

`ApplicationActor` contains validated adapter identity/role, never a bearer token. Typed outcomes must cover accepted revision, same-content replay/no-op, conflict saved, rejected request, tombstoned, not found, bounded ordered changes, and verified revision bytes.

For mutations, the application service owns: durable idempotency lookup; SQLx transaction; path advisory lock; current-state read; Core planning; object-store coordination; Storage persistence; operation-log append; safe replay record; commit/rollback. Core policy is not duplicated. Storage remains passive and caller-transaction-owned.

Later extraction rules:

- extract Core planning from `routes/v1/planning.rs`;
- extract persistence from `routes/v1/persistence.rs`;
- extract PUT/DELETE idempotency choreography from route modules;
- extract DELETE guard/tombstone/operation-log transaction flow from `routes/delete.rs`;
- retain API parsing, auth, HTTP status selection, DTO mapping and response construction in routes;
- delete obsolete private helpers only after both routes and Worktree executor use the services and compatibility tests pass.

## Async Worktree runtime boundary

The accepted target is an awaitable Worktree scheduler/cycle contract while synchronous filesystem primitives remain Worktree-owned.

Worktree must own an async-compatible cycle interface equivalent to:

```rust
pub trait WorktreeRuntimeCancellation: Send + Sync {
    fn is_cancelled(&self) -> bool;
}

pub trait WorktreeRuntimeCycle: Send {
    fn run_cycle<'a>(
        &'a mut self,
        request: WorktreeRuntimeCycleRequest,
        cancellation: &'a dyn WorktreeRuntimeCancellation,
    ) -> Pin<Box<dyn Future<Output = Result<
        WorktreeRuntimeCycleSummary,
        WorktreeRuntimeCycleFailure,
    >> + Send + 'a>>;
}
```

`WorktreeRuntimeService::poll` becomes async and awaits at most one cycle. `start`, cancellation request, status and watcher shutdown may remain synchronous because they do not perform authoritative async mutation. Scheduler validation, watcher-hint coalescing, periodic correctness, budgets, count-only summaries and no-overlap remain Worktree-owned.

Server must not use nested Tokio runtimes, `Handle::block_on`, internal HTTP self-calls, detached tasks, fabricated synchronous summaries or duplicated route policy.

Synchronous scanner/materializer/trash/echo work may execute through an explicit awaited bounded blocking-pool call. SQLx and application-service operations are awaited normally and never bridged through blocking.

## Hosted runtime ownership

Server owns one `ServerWorktreeRuntimeHost` per configured Worktree adapter instance.

The host must:

- own and join one named task; never detach it;
- serialize watcher hints, periodic ticks, manual requests and shutdown;
- guarantee at most one cycle in progress;
- reject or coalesce manual invocation while busy;
- request cooperative cancellation, stop new scheduling, wait for the bounded in-flight cycle, shut down watcher resources and join;
- expose only safe lifecycle/mode/count/category status.

Cancellation is checked before/after each phase and between bounded items. A current atomic filesystem operation may finish, but no later phase starts. Open SQLx transactions commit only on complete success and otherwise roll back.

Watcher events remain latency hints. Every startup, periodic or watcher-triggered cycle that observes local files performs a full scan before import/delete planning.

## Durable Worktree state

Storage owns durable Worktree state and cursor repositories. Production-only in-memory state is forbidden.

Durable identity is a stable Worktree `AdapterId`, bound to a non-public SHA-256 fingerprint of the normalized configured root. Raw roots are never rendered publicly.

Required Storage contract:

- versioned runtime-instance binding keyed by `adapter_id`;
- versioned path state keyed by `(adapter_id, VaultPath)`;
- explicit present/tombstoned kind;
- last-applied revision and present-file content hash;
- reconciliation observation fields required by accepted Worktree contracts;
- monotonic authoritative export checkpoint through the adapter cursor contract;
- typed load/upsert/bind/checkpoint methods over caller-owned executors/transactions.

The existing path-only `worktree_state` table is insufficient because it has no adapter/root binding, explicit state kind, format version or repository interface. Existing `adapter_cursors.last_core_seq` remains the export checkpoint after Storage adds locked contiguous advancement support.

State advancement rules:

- accepted/same-content import: persist authoritative present state;
- accepted delete: persist tombstoned state;
- rejection/conflict/failure: do not claim accepted local state;
- export: after one authoritative filesystem apply, update path state and advance the cursor to that exact contiguous sequence in one DB transaction;
- never advance beyond an unprocessed sequence.

Crash recovery is replay-based:

- crash before materialization: cursor stays old and the change replays;
- crash after materialization but before DB state/cursor commit: replay returns `AlreadyCurrent`, then persists state/checkpoint;
- crash after accepted import but before path-state update: deterministic idempotency replays the authoritative outcome, then state is updated.

Process counters are `since_start` only. Durable state includes instance binding/version, path state, cursor and last-success metadata.

## Worktree idempotency

For non-HTTP Worktree operations, Server derives deterministic keys from stable non-secret facts:

```text
put:    wt:v1:<adapter_id>:put:<path_hash>:<base_or_null>:<content_hash>
delete: wt:v1:<adapter_id>:delete:<path_hash>:<base_or_null>
```

The raw key is never logged or exposed. The application service uses the same durable idempotency repository and request-fingerprint comparison as routes. Route-generated client keys remain API-owned inputs and are not replaced.

## Runtime policy and modes

Server config owns explicit runtime values with deterministic documented defaults:

- adapter id: `worktree`;
- max imports per cycle: `100`;
- max delete candidates per cycle: `100` plus existing Worktree/Core ratio/count guards;
- max exports per cycle: `100`;
- periodic correctness interval: `60s`;
- watcher debounce: `500ms`;
- watcher hints consumed per poll: `256`;
- host idle wake interval: `250ms`;
- graceful cycle shutdown budget: `30s`.

All values are validated as non-zero and bounded. No unbounded hidden work is allowed.

Mode semantics:

- `disabled`: no hosted task or work;
- `read_only`: Core-to-Worktree export only;
- `import_only`: full-scan import and guarded local-delete submission only;
- `export_only`: Core-to-Worktree export only;
- `bidirectional`: both directions within budgets;
- `dry_run`: explicit manual cycle only; may scan/load/query/plan and return count-only summaries, but performs no Core mutation, cursor advance, materialization, trash move, echo write or durable Worktree state change.

Hosted startup binds durable identity before first cycle. Binding mismatch is fail-closed and not ready. Disabled remains ready if HTTP dependencies are ready. Enabled runtime readiness requires valid config, durable binding, a running/non-failed host and DB/object-store readiness. A cycle failure degrades Worktree readiness/status without exposing raw errors; it does not make `/health` fail.

## Security and invariants

- Core remains the only conflict/delete/revision arbiter.
- No hard delete or automatic destructive repair.
- No provider behavior in generic Server routes.
- Public/debug/status output contains no secrets, DB URLs, token values/hashes, absolute roots, raw SQLx/I/O errors or internal payloads.
- Dependency-free router tests and existing route behavior remain supported.
- Accepted SRV-P7A behavior remains unchanged until explicit owner phases implement this contract.

## Contract change protocol

Any implementation that requires blocking async bridges, hidden task spawning, direct Worktree DB writes, route-policy duplication, public DTO changes outside API ownership, schema changes outside Storage ownership, or Worktree scheduler changes outside Worktree ownership must stop and request the corresponding owner-component phase.