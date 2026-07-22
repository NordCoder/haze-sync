# Component Contract: worktree

## Responsibility

`haze-sync-worktree` owns the built-in VPS worktree adapter/runtime logic for Haze Sync.

The worktree is a materialized filesystem replica of Core state for local tools and agents. It is not the source of truth.

The component owns:

- mapping between normalized `VaultPath` values and local filesystem paths under one configured worktree root;
- safe filesystem scanning and stable-file detection;
- ignore/reserved-path handling;
- import/delete fact planning with explicit base semantics;
- atomic materialization of authoritative Core revisions;
- echo suppression, reconciliation, retained trash, and doctor/repair facts;
- the hostable scheduler state machine, awaitable cycle contract, cancellation vocabulary, mode permissions, bounded-work validation, and count-only runtime status.

Server may host the runtime and implement the concrete async executor, but Worktree remains the owner of scheduler and filesystem semantics.

## Public runtime boundary

The authoritative runtime cycle boundary is awaitable and dependency-free:

```rust
fn run_cycle<'a>(
    &'a mut self,
    request: WorktreeRuntimeCycleRequest,
    cancellation: WorktreeCancellationToken,
) -> WorktreeRuntimeCycleFuture<'a>;
```

`WorktreeRuntimeCycleFuture<'a>` is a pinned boxed `Send` future returning the existing safe summary/failure result. Worktree does not create a Tokio runtime, call `block_on`, spawn or detach tasks, or implement Server/Storage authority.

`WorktreeRuntimeService::poll` awaits at most one cycle. The mutable service borrow and `cycle_in_progress` invariant prohibit overlap. Watcher hints remain latency-only; authoritative scans remain the correctness source.

## Mode contract

- `Disabled` is completely inert.
- `ReadOnly` and `ExportOnly` may apply authoritative Core exports but may not submit local facts.
- `ImportOnly` may submit local facts but may not apply exports.
- `Bidirectional` permits both directions.
- `DryRun` is explicit planning-only vocabulary. It does not start automatic cycles and does not grant Core mutation, cursor advancement, materialization, trash movement, echo writes, or durable Worktree-state mutation.

A future Server-owned manual dry-run command may construct a planning request, but WT-P10 does not add that endpoint or executor.

## Cooperative cancellation

`WorktreeCancellationToken` contains only a shared atomic cancellation bit and exposes `cancel` plus `is_cancelled`.

- A future executor may observe cancellation before and after phases and between bounded items.
- One already-running atomic filesystem operation may complete.
- No new cycle begins after cancellation.
- The scheduler rechecks cancellation after awaiting the executor and normalizes the result to the safe `Cancelled` category.
- Shutdown and restart prevention remain explicit.

## Input contracts

### Filesystem inputs

Filesystem scans and watchers observe untrusted local state under a configured worktree root.

Required behavior:

- never follow paths outside the configured worktree root;
- normalize candidate paths into `VaultPath` before sending facts to Core/API;
- reject traversal, absolute paths, symlinks, and special files unless explicitly accepted by a future contract;
- ignore reserved runtime directories and temporary files;
- detect stable files before importing local edits;
- compute content hashes from observed bytes before submitting changes;
- avoid importing partial writes, temp files, or files currently written by the adapter itself.

### Core/API inputs

Materialization receives already-authoritative Core/API facts such as path, revision id, content hash, file bytes, tombstone/delete state, conflict state, and operation sequence.

Required behavior:

- treat Core/API state as authoritative;
- verify content hash before or during materialization;
- write files atomically through temp paths and rename/replace discipline;
- update echo/last-written metadata after successful adapter writes;
- never silently overwrite a locally dirty file without Core/API conflict handling;
- never treat local filesystem state as authoritative over Core.

## Output contracts

Worktree outputs are facts, plans, safe summaries, or local side effects within the configured worktree root.

Public/operator output must:

- use vault-relative `VaultPath` values where path context is necessary;
- remain count/category based for runtime status;
- avoid bearer/OAuth tokens, token hashes, idempotency keys, provider payloads, database URLs, raw internal errors, stack traces, local absolute paths, bytes, cursors, or runtime handles.

## Persistence/runtime ownership

Worktree owns local filesystem adapter logic and the runtime contract, but not the authoritative sync database or hosted async authority.

Worktree owns:

- path mapping, scanner and watcher abstractions;
- stable-file detection;
- import/delete planning;
- atomic writer/materializer behavior;
- echo, reconciliation, trash, and doctor behavior;
- runtime scheduling, watcher-hint coalescing, periodic full-scan rules, budgets, summary validation, cancellation vocabulary, DryRun semantics, and safe status.

Worktree does not own:

- Core conflict/delete/revision policy;
- Storage schema/repository or SQLx operations;
- Server listener, startup, hosted loop, concrete executor, joined task lifecycle, HTTP routes, or DTO ownership;
- provider/GDrive behavior;
- deployment files;
- hard delete of Core/object-store/provider data.

## Security and secrecy rules

- Do not commit secrets.
- Do not expose tokens, token hashes, database URLs, provider payloads, local absolute paths, generic executor/watcher internals, or raw I/O errors.
- Do not follow filesystem paths outside the configured root.
- Do not let user-controlled paths choose temp/runtime paths outside the root.
- Do not silently overwrite locally dirty files.
- Do not hard-delete local files without the accepted trash/retention contract.
- Do not trust watcher events as correctness evidence.
- Do not use nested runtimes, blocking async bridges, hidden tasks, or fabricated summaries.

## Invariants

- Core metadata and object storage are authoritative.
- Worktree is a materialized replica, not source of truth.
- Local changes are facts submitted to Core/API, not overwrite decisions.
- Watchers are for latency; scans are for correctness.
- Every imported write carries a known base revision or explicit null base.
- Dirty local files are not silently overwritten.
- Adapter-written files are protected by echo suppression.
- Delete behavior remains tombstone/trash/retention-oriented.
- All local paths remain under the configured worktree root.
- Runtime cycles are awaitable, `Send`, bounded, validated, non-overlapping, and cancellation-aware.
- DryRun is explicit and mutation-free.

## Test obligations

Worktree tests cover path safety, scanning, stable files, hashes, atomic materialization, echo suppression, reconciliation, guarded deletes, retained trash, doctor/repair summaries, and runtime scheduling.

Runtime contract tests must prove:

- real async executor summaries/failures are awaited;
- no overlapping cycles;
- cancellation before and during a cycle is safe;
- startup/watcher/periodic causes and full-scan rules remain correct;
- watcher degradation preserves periodic correctness;
- budgets and summary validation remain exact;
- DryRun is explicit and non-mutating;
- lifecycle misuse is rejected;
- status and Debug output remain path/secret/payload-free.

## Contract change protocol

Request another contract change instead of silently adding direct DB writes, provider behavior, Server hosting, nested runtimes, hidden tasks, blocking bridges, paths outside the root, hard delete, or public secret/absolute-path output.
