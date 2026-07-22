# Worktree Runtime Service

## Ownership

`WorktreeRuntimeService` is a host-driven library abstraction owned by the Worktree component.

It does not:

- start or detach a Tokio task;
- create a nested runtime or call `Handle::block_on`;
- create a process or listener;
- wire itself into Server startup;
- own Storage, SQLx, Core/API clients, or HTTP behavior;
- treat watcher events as filesystem facts.

A future Server fan-in owns the concrete async executor and joined host loop. Its lifecycle is:

```text
start -> await poll* -> request_cancel -> shutdown
```

Each awaited `poll` runs at most one cycle. The mutable service borrow and internal `cycle_in_progress` invariant prevent overlapping startup, watcher, or periodic cycles.

## Awaitable cycle contract

`WorktreeRuntimeCycle` returns a boxed `Send` future using only `std`:

```rust
fn run_cycle<'a>(
    &'a mut self,
    request: WorktreeRuntimeCycleRequest,
    cancellation: WorktreeCancellationToken,
) -> WorktreeRuntimeCycleFuture<'a>;
```

The alias is a pinned boxed future whose output is the existing safe cycle summary/failure result. No `async-trait` or runtime dependency is required.

Synchronous filesystem primitives remain synchronous. The future boundary lets a Server-owned executor await Storage/Core authority without blocking or fabricating summaries.

## Correctness and watcher hints

Watcher events are path-free scheduling hints. Their sequence values are diagnostic only. The runtime remains correct when hints are missing, duplicated, coalesced, reordered, or unavailable.

Startup and periodic cycles are independent of watcher delivery. Any automatic mode that imports local state sets `full_scan_required`. The executor must run `WorktreeScanner` before deriving local import or guarded-delete facts.

Watcher failures degrade latency but do not stop periodic correctness cycles. Public watcher failures are normalized to `Start`, `Poll`, or `Shutdown`.

## Modes

| Mode | Automatic cycle | Watch local files | Submit local facts | Apply Core exports | Manual planning |
| --- | --- | --- | --- | --- | --- |
| `Disabled` | no | no | no | no | no |
| `ReadOnly` | yes | no | no | yes | no |
| `ImportOnly` | yes | yes | yes | no | no |
| `ExportOnly` | yes | no | no | yes | no |
| `Bidirectional` | yes | yes | yes | yes | no |
| `DryRun` | no | no | no | no | yes, future host only |

`DryRun` is explicit and inert in the automatic scheduler. It does not authorize Core mutation, cursor advancement, materialization, trash movement, echo writes, or durable Worktree-state mutation. `WorktreeRuntimeCycleRequest::manual_dry_run` creates a planning-only request with all mutation permissions false; WT-P10 does not add the future Server endpoint or executor.

## Bounded work and validation

`WorktreeRuntimePolicy` bounds watcher hints, planned imports, guarded delete candidates, and applied exports. The scheduler validates the real executor summary before accepting status. It rejects missing required scans, forbidden-direction work, budget overflow, and submitted-import counts above planned counts.

A failed or rejected cycle is not retried in a tight loop. The next automatic attempt is scheduled at the periodic deadline.

## Cooperative cancellation

`WorktreeCancellationToken` is a clonable `Arc<AtomicBool>` view with `cancel` and `is_cancelled`. It contains no paths, payloads, cursors, secrets, or runtime handles.

A Server executor can observe cancellation before and after phases and between bounded items. One already-running atomic filesystem operation may finish, but the executor can stop before later phases. The scheduler also rechecks cancellation after awaiting the executor and normalizes the result to `WorktreeRuntimeCycleFailure::Cancelled`.

After cancellation is requested, no later cycle begins. `request_cancel` and external token cancellation both lead to the explicit `Cancelling` lifecycle. Shutdown remains synchronous because watcher shutdown has no async authority in this contract.

## Lifecycle and status safety

Shutdown before start, double start, restart after shutdown, and double shutdown are rejected explicitly. Watcher resources are released after a successful watcher start, including after close or polling failure.

`WorktreeRuntimeStatus` and the custom `Debug` implementation expose only lifecycle/mode enums, watcher categories, count metrics, scheduling state, and safe summaries/failures. Generic executor, watcher, clock internals and local paths are not formatted.
