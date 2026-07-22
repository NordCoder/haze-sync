# Decisions: server

## Standing accepted decisions

The following earlier Server decisions remain accepted:

- Server is runtime composition and transaction orchestration, not sync policy.
- Runtime state is explicit; hidden globals are forbidden.
- Server owns live mutation transaction choreography over Core and Storage contracts.
- Dependency-free router behavior is a safe test surface, not production readiness.
- Provider runtimes and Worktree filesystem logic do not belong in generic route modules.
- Admin/status surfaces remain read-only until an explicit mutation contract exists.
- Partial route behavior must remain explicit and safe.
- Startup, config, listener and shutdown are explicit; migrations are governed by an explicit operational policy.
- SRV-P7A remains accepted: the Worktree snapshot is exact, lifecycle composition is active, Disabled is inert, enabled execution remains unavailable until a real executor is supplied.

## 2026-07-11 — SRV-P7B target architecture

Status: accepted cross-component architecture decision. Owner-component implementation phases are still required.

### Context

SRV-P7B1 proved that a Server-only implementation cannot correctly connect the current synchronous Worktree cycle executor to asynchronous SQLx/Tokio authority. Correct PUT/DELETE/conflict/idempotency behavior is also embedded in private async routes, while durable Worktree state/cursor and runtime policy are incomplete.

Forbidden workarounds remain forbidden: nested Tokio runtimes, `Handle::block_on`, ad hoc blocking, internal HTTP self-calls, detached tasks, fabricated synchronous summaries, duplicated route policy and in-memory production state.

## Decision A — Awaitable Worktree scheduler/cycle boundary

Decision:

Worktree retains ownership of scheduling semantics and synchronous filesystem primitives, but its authoritative cycle boundary becomes natively awaitable.

Contract shape:

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

`WorktreeRuntimeService::poll` becomes async and awaits at most one cycle. `start`, cancellation request, safe status and watcher shutdown may stay synchronous because they do not call asynchronous authority.

Ownership:

- Worktree owns the trait, request/summary/failure/cancellation vocabulary, scheduler validation, hint coalescing, periodic correctness and no-overlap state.
- Server owns the concrete executor and hosted lifecycle.
- Synchronous filesystem phases may run through an explicit awaited bounded blocking-pool call owned by Server composition. SQLx/application-service work is awaited normally.

Cancellation and overlap:

- one mutable runtime service/executor is owned by one host task;
- no new cycle starts after cancellation;
- cancellation is checked between bounded phases/items;
- a current atomic filesystem operation may finish, but later work does not start;
- the host waits for the bounded in-flight cycle, shuts down watcher resources and joins;
- no detached work and no overlapping manual/periodic/watcher cycles.

Correctness:

- watcher events remain path-free latency hints;
- startup, periodic and watcher-triggered local-observing cycles require a full scan;
- watcher failure never removes periodic correctness;
- failures/status remain safe categories and counts only.

Compatibility:

- existing request, policy, summary and status types remain where practical;
- existing synchronous fake executors migrate to immediately-ready async test executors;
- existing scheduler tests become async tests and retain deterministic fake clock/watcher behavior;
- scanner/planner/materializer public contracts remain unchanged until explicitly changed by a Worktree-owner phase.

Rejected alternatives:

1. Keep the scheduler/cycle fully synchronous.
   Rejected because correct authoritative operations are async and any bridge would block or create a second runtime.
2. Make only Server spawn hidden async work while returning synchronously.
   Rejected because summaries would be fabricated, errors/cancellation would be detached and no-overlap could not be proved.
3. Move scheduling entirely into Server.
   Rejected because full-scan/hint/budget semantics are Worktree-owned and would be duplicated.
4. Make every filesystem primitive async now.
   Rejected as unnecessary scope; the incompatibility is the cycle boundary, not the correctness of synchronous filesystem primitives.

## Decision B — Reusable Server application services

Decision:

Create Server-owned reusable async services used by both HTTP routes and Worktree execution.

Target modules/types:

```text
application/files.rs
application/deletes.rs
application/changes.rs
application/idempotency.rs
ServerApplicationServices
ApplicationActor
ApplyFileCommand / ApplyFileOutcome
ApplyDeleteCommand / ApplyDeleteOutcome
AuthoritativeChangesQuery / AuthoritativeChangeBatch
RevisionContentQuery / AuthoritativeRevisionContent
ApplicationError
```

Required operations:

- create/update from a normalized Worktree fact with known or explicit-null base;
- guarded local delete/tombstone submission;
- bounded ordered authoritative changes retrieval;
- revision metadata plus verified content retrieval;
- conflict-saved typed outcomes;
- deterministic non-HTTP Worktree idempotency;
- shared transaction/lock/object-store/operation-log/commit choreography.

Idempotency:

```text
put:    wt:v1:<adapter_id>:put:<path_hash>:<base_or_null>:<content_hash>
delete: wt:v1:<adapter_id>:delete:<path_hash>:<base_or_null>
```

Keys are deterministic, retry-stable and never logged/exposed. The same durable idempotency repository and fingerprint comparison are used by route and Worktree flows. HTTP client-supplied keys remain API-owned inputs.

Transaction ownership:

The service performs idempotency read, transaction begin, path lock, authoritative read, Core planning, object-store coordination, Storage persistence, conflict/tombstone/operation-log writes, safe replay persistence and commit/rollback. Storage helpers remain passive. Core remains the only policy arbiter.

Extraction:

- extract planning from `routes/v1/planning.rs`;
- extract persistence from `routes/v1/persistence.rs`;
- extract PUT/DELETE replay/fingerprint/storage choreography;
- extract DELETE guard/tombstone/operation-log transaction flow from `routes/delete.rs`;
- retain parsing, auth, HTTP status, DTO mapping and response construction in routes;
- remove obsolete route-private helpers only after parity tests pass.

Compatibility:

Public HTTP shapes, statuses, headers and dependency-free route behavior remain unchanged. The application layer returns internal typed outcomes, not API DTOs.

Rejected alternatives:

1. Worktree calls Server HTTP handlers or localhost routes.
   Rejected because it couples an in-process runtime to transport/auth and duplicates serialization/error behavior.
2. Duplicate route logic in the Worktree executor.
   Rejected because policy, idempotency and transaction behavior would drift.
3. Put full application transactions into Storage.
   Rejected because Storage persists facts and must not own Core policy/runtime choreography.
4. Put SQLx/runtime concerns into Core.
   Rejected because Core remains deterministic and persistence-neutral.

## Decision C — Durable Worktree state and cursor

Decision:

Storage owns durable Worktree instance/path state and export-cursor repository primitives. Server owns transaction timing; Worktree owns the semantic state model consumed by planners/materializers.

Existing schema assessment:

- current `worktree_state(path primary key, last_applied_revision_id, last_seen_sha256, last_seen_mtime, dirty, last_scanned_at, last_written_by_adapter)` is insufficient;
- it lacks adapter/root identity, explicit present/tombstoned kind, format version and accepted repository methods;
- `adapter_cursors.last_core_seq` is suitable for the authoritative export checkpoint after Storage adds locked contiguous advancement.

Minimum Storage change:

1. Add a versioned Worktree instance binding keyed by `adapter_id` with root fingerprint, state format version and safe timestamps.
2. Replace/migrate path state to a key of `(adapter_id, path)` and add explicit state kind plus last-applied revision/hash and accepted reconciliation fields.
3. Add typed repositories for bind/load/upsert/tombstone/snapshot operations.
4. Add cursor methods that lock/read and advance only monotonically to the exact contiguous processed sequence.

Identity:

- state is per Worktree adapter instance, not globally per path;
- V1 default adapter id is `worktree`;
- the instance is bound to a SHA-256 fingerprint of the normalized configured root;
- a binding mismatch is fail-closed and requires explicit operator migration/rebind policy;
- raw roots are not stored in public output.

Serialization/versioning:

- relational typed columns for identity, path state and checkpoint-critical fields;
- any reconciliation JSON extension must carry an explicit schema version and be bounded/validated;
- unknown future versions are rejected safely rather than silently interpreted.

Atomicity and advancement:

- accepted/same-content import then durable present state;
- accepted delete then durable tombstoned state;
- conflict/rejection/failure does not claim accepted local state;
- export applies one authoritative change, then path state and cursor advance together in one DB transaction;
- cursor never advances past an unprocessed sequence.

Crash semantics:

- before filesystem apply: cursor stays old, replay later;
- after filesystem apply but before state/cursor commit: replay yields `AlreadyCurrent`, then commits state/cursor;
- after accepted import but before state update: deterministic idempotency replays outcome, then state updates;
- DB rollback never leaves a falsely advanced cursor/state;
- object-store blobs may be orphaned by later DB rollback but never become current without metadata commit.

Durable vs process-local status:

- durable: instance binding/version, path state, cursor/checkpoint and last-success metadata;
- process-local: cycles completed/failed, pending hints, in-progress state and last-cycle summary, all labelled `since_start`.

Retention:

- Worktree path state is retained while the adapter instance exists and while tombstone/revision references remain operationally relevant;
- cleanup is a separate explicit Storage/Core/Deployment phase;
- no automatic destructive cleanup is introduced.

Required proof:

- no skipped export under crash/restart;
- replay of materialized-but-uncheckpointed changes;
- no unsafe duplicate import after accepted mutation;
- no false last-applied state on conflict/failure;
- cursor regression and instance-binding mismatch rejection;
- transaction rollback tests.

Rejected alternatives:

1. In-memory `WorktreeStateSnapshot` in production.
   Rejected because restart loses bases/echo/reconciliation safety.
2. Hidden files under the Worktree root as source-of-truth metadata.
   Rejected because Worktree files are a materialized view and can be edited/copied.
3. Use only `adapter_cursors.external_cursor_json` for all path state.
   Rejected because path-level querying/locking/versioning would be opaque and unsafe.
4. Advance cursor before materialization.
   Rejected because a crash would skip unapplied authoritative changes.

## Decision D — Runtime policy, config and invocation

Decision:

Server config owns explicit bounded policy values with documented deterministic defaults.

Defaults:

```text
adapter_id = worktree
max_import_actions = 100
max_delete_candidates = 100
max_export_actions = 100
periodic_correctness_interval = 60s
watcher_debounce = 500ms
max_watcher_hints_per_poll = 256
host_idle_wake_interval = 250ms
graceful_cycle_shutdown_budget = 30s
```

All values are non-zero, validated and bounded. Existing Worktree/Core delete count/ratio guards remain authoritative in addition to the cycle candidate budget.

Invocation:

- startup binds durable identity before the first cycle;
- one explicit joined Server host owns periodic/watcher/manual invocation;
- manual one-cycle requests use the same host queue and cannot overlap;
- a busy manual request is safely rejected or coalesced, never run concurrently;
- cancellation stops new work and joins the task;
- no hidden unbounded retries or detached background work.

Mode semantics:

- Disabled: no host task, watcher or state work.
- ReadOnly: authoritative Core-to-Worktree export only.
- ImportOnly: full-scan import and guarded local-delete submission only.
- ExportOnly: authoritative Core-to-Worktree export only.
- Bidirectional: both within budgets.
- DryRun: explicit manual cycle only; may load/scan/query/plan and return count-only summaries, but performs no Core mutation, cursor advancement, materialization, trash move, echo write or durable Worktree state mutation.

Readiness/status:

- `/health` remains process health and is not failed by a cycle error;
- Disabled does not make Server unready when required HTTP dependencies are ready;
- enabled mode readiness requires valid config, durable identity binding, DB/object-store readiness and a running/non-terminal host;
- host/cycle failure degrades Worktree readiness and safe admin status;
- public status contains mode/lifecycle/category/count/cursor-presence only, never raw root/cursor/error/payload/secret.

Rejected alternatives:

1. Hard-code all policy invisibly.
   Rejected because operator budgets/cadence must be explicit and auditable.
2. Make every value mandatory with no defaults.
   Rejected because deterministic conservative defaults improve deployability while remaining visible.
3. Start enabled runtime without durable binding/readiness.
   Rejected because it can import/export against the wrong root or lose cursor safety.
4. Treat DryRun as normal hosted mode.
   Rejected because continuous no-op work is misleading; V1 DryRun is explicit manual planning.

## Consequences and implementation order

Required order:

1. `worktree — W1 WT-P10 Async Runtime Contract`
2. `storage — W1 STOR-P10 Worktree Durable State`
3. `server — W1 SRV-P7B2 Application Services`
4. `server — W1 SRV-P7B3 Bounded Worktree Executor`
5. `server — W1 SRV-P7B4 Hosted Worktree Runtime`
6. `api — W1 API-P8 Worktree Runtime Status Contract`
7. `server — W1 SRV-P7B5 Worktree Status and Readiness`
8. `cli — W1 CLI-P6A Worktree Sync Once`
9. `deployment — W1 DEP-P5A Worktree Runtime Fan-In`

Worktree, Storage and Server application-service owner phases may be developed separately after this decision, but SRV-P7B3 cannot start until all three are clean-accepted and synchronized. Public status work waits for API ownership. CLI and Deployment remain blocked until Server runtime/status contracts are accepted.