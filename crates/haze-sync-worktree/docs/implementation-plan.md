# Implementation Plan: worktree

## Current state

`haze-sync-worktree` is currently a placeholder crate.

Implemented current-state surface:

- `CRATE_ROLE` smoke-check constant;
- `package_name()` smoke-check helper;
- one package-name unit test.

There is no scanner, writer, materializer, importer, echo guard, watcher, trash, repair, doctor, runtime service, or Server integration yet.

The component docs were scaffold-level before this planning pass.

## Target state

The target state for Worktree is a built-in VPS filesystem adapter that materializes Core state into a local worktree and imports safe local filesystem changes back through Core/API.

The Worktree component is V1-ready when:

- it treats Core metadata/object store as authoritative;
- it maps only safe paths under a configured worktree root;
- it scans the filesystem for correctness and may watch for latency;
- it detects stable local files before import;
- it computes/validates content hashes before submitting facts;
- it writes Core revisions atomically into the worktree;
- it avoids overwriting locally dirty files without Core conflict handling;
- it suppresses echo for files it just materialized;
- it represents delete/trash/retention behavior safely when scoped;
- it exposes safe doctor/repair facts;
- Server can host its runtime through explicit composition without owning Worktree logic.

## Implementation phases

### WT-P1 — Component contract and planning normalization

Status: completed by this Architect planning pass.

Goal:

```text
Replace scaffold worktree docs with a real contract, dependency map,
implementation plan, decisions, and baseline implementation log.
```

Allowed scope:

```text
crates/haze-sync-worktree/docs/**
```

Completed deliverables:

- complete `component-contract.md`;
- complete `dependency-map.md`;
- complete `implementation-plan.md`;
- add initial worktree decisions;
- update implementation log with planning baseline.

Non-goals:

- no product code changes;
- no Server runtime integration;
- no filesystem scanner/writer implementation;
- no Storage schema changes;
- no CI/workflow changes.

Acceptance:

- docs define Worktree as materialized filesystem replica, not source of truth;
- docs preserve Worktree/Server/Core/API/Storage/adapters boundaries;
- future Worktree phases are implementable without inventing sync policy locally.

### WT-P2 — Worktree config and path-mapping foundation

Goal:

```text
Implement safe configuration and path mapping between `VaultPath` and local
filesystem paths under one configured worktree root.
```

Allowed scope:

```text
crates/haze-sync-worktree/src/**
crates/haze-sync-worktree/docs/**
```

Likely work:

- introduce `WorktreeConfig` or equivalent;
- represent configured worktree root without exposing it in public errors;
- map `VaultPath` to local file paths under root;
- map local paths back to `VaultPath` only when they are inside root;
- reject traversal, absolute-path escape, backslash escape, symlink/special-file cases if scoped;
- define reserved runtime directories such as temp/trash/metadata/echo areas;
- add tests for path normalization, root containment, and reserved paths.

Non-goals:

- no scanner loop;
- no Server config loading;
- no API calls;
- no Core policy;
- no filesystem watcher.

Contract-change triggers:

- allowing paths outside root;
- following symlinks by default;
- changing Common `VaultPath` semantics;
- exposing local absolute paths in public errors;
- adding hidden runtime directories that conflict with Core conflict materialization.

Acceptance:

- all path mapping is deterministic and root-contained;
- unsafe paths are rejected safely;
- reserved path policy is documented.

### WT-P3 — Scanner, ignore rules, and stable-file detection

Goal:

```text
Implement correctness-oriented filesystem scanning that produces safe local file
facts only after files are stable enough to import.
```

Allowed scope:

```text
crates/haze-sync-worktree/src/**
crates/haze-sync-worktree/docs/**
```

Likely work:

- walk the configured worktree root without escaping it;
- ignore reserved runtime directories, temp files, trash, and optionally hidden implementation files;
- represent local file facts with `VaultPath`, size, mtime, content hash, and stability metadata;
- detect stable files by repeated metadata/hash observation or an accepted heuristic;
- skip files currently written by the adapter if echo state is available;
- classify unsafe filesystem entries without panicking;
- add tests using temporary directories and simulated partial writes.

Non-goals:

- no watcher reliance for correctness;
- no Core/API import calls yet unless scoped;
- no materialization writer;
- no delete propagation;
- no provider behavior.

Contract-change triggers:

- treating watcher events as sole correctness source;
- importing unstable/partial files;
- following symlinks/special files without explicit safety decision;
- exposing local absolute paths in scan output.

Acceptance:

- scanner outputs safe facts only;
- unstable/unsafe files are skipped with safe diagnostics;
- scan correctness does not depend on watcher availability.

### WT-P4 — Import planner and Core/API submission boundary

Goal:

```text
Convert safe local scan facts into Core/API write/delete candidate requests while
preserving base revision and no-silent-overwrite semantics.
```

Allowed scope:

```text
crates/haze-sync-worktree/src/**
crates/haze-sync-worktree/docs/**
```

Likely work:

- represent worktree state needed to identify local changes against last applied Core revision;
- produce import plans for new/modified/deleted local files;
- attach current known base revision or explicit null base;
- include content hash and bytes only for stable file imports;
- submit through an abstract API/Core client trait, not direct DB writes;
- handle Core/API outcomes: accepted, same-content, conflict-saved, rejected, tombstoned, not-found;
- update local state only after accepted outcomes.

Non-goals:

- no direct SQLx/Storage writes;
- no route handler implementation;
- no provider/GDrive behavior;
- no conflict policy decisions inside Worktree;
- no hard delete.

Contract-change triggers:

- Worktree writing directly to Storage DB;
- importing without base/null-base semantics;
- ignoring conflict-saved outcomes;
- treating local file state as authoritative over Core.

Acceptance:

- import planner submits facts and obeys Core/API outcomes;
- no local overwrite/delete policy is invented;
- state updates are outcome-driven.

### WT-P5 — Materializer and atomic writer

Goal:

```text
Materialize Core revisions into the local worktree safely and atomically while
protecting locally dirty files.
```

Allowed scope:

```text
crates/haze-sync-worktree/src/**
crates/haze-sync-worktree/docs/**
```

Likely work:

- represent materialization requests from Core/API changes feed or Server-hosted state;
- verify incoming bytes against expected content hash;
- write through temp files under reserved runtime area;
- fsync/rename or accepted platform-safe equivalent where practical;
- avoid overwriting dirty local files; create conflict/import plan instead where necessary;
- update worktree state after successful materialization;
- add echo-guard marker for adapter-written files;
- test crash/partial-write cleanup where practical.

Non-goals:

- no Core conflict policy;
- no API DTO redesign;
- no provider calls;
- no watcher requirement;
- no hard delete.

Contract-change triggers:

- overwriting dirty files without Core conflict handling;
- writing outside root;
- skipping hash verification;
- exposing temp/local paths publicly.

Acceptance:

- materialization is atomic enough for V1 filesystem expectations;
- dirty local files are protected;
- written files can be echo-suppressed.

### WT-P6 — Echo guard and worktree state reconciliation

Goal:

```text
Prevent adapter-written files from being re-imported as local edits and maintain
worktree state useful for reconciliation and doctor checks.
```

Allowed scope:

```text
crates/haze-sync-worktree/src/**
crates/haze-sync-worktree/docs/**
```

Likely work:

- define echo-guard state from last adapter write;
- compare current filesystem observations to last applied revision/hash/mtime;
- classify clean, dirty, missing, extra, conflict materialization, and skipped files;
- integrate with persisted `worktree_state` only through accepted API/Storage/Server boundary;
- expose safe reconciliation summaries.

Non-goals:

- no direct DB ownership by default;
- no server status route implementation;
- no repair execution unless scoped;
- no provider behavior.

Contract-change triggers:

- storing worktree state directly in DB from Worktree without accepted boundary;
- treating echo markers as authoritative forever;
- exposing local absolute paths in status output.

Acceptance:

- echo suppression reduces self-induced loops;
- state reconciliation is explicit and testable;
- drift facts are safe for doctor/status mapping.

### WT-P7 — Delete, trash, and local retention behavior

Goal:

```text
Implement safe local behavior for Core tombstones/deletes and local delete
candidates without immediate irreversible hard delete.
```

Allowed scope:

```text
crates/haze-sync-worktree/src/**
crates/haze-sync-worktree/docs/**
```

Likely work:

- represent local delete candidates from scans;
- submit delete candidates through Core/API with base/null-base semantics;
- materialize Core tombstones by moving local files to configured trash/backup area when scoped;
- respect retention metadata and avoid hard delete by default;
- prevent mass local delete propagation without Core/API delete guard behavior;
- test local trash path safety and restore-ready metadata if accepted.

Non-goals:

- no Core tombstone policy;
- no hard delete cleanup;
- no provider delete calls;
- no CLI repair command;
- no DB row mutation directly.

Contract-change triggers:

- immediate hard delete;
- local mass delete propagation without guard;
- delete decisions made without Core/API confirmation;
- trash paths outside root or configured safe area.

Acceptance:

- delete behavior is reversible/retained where possible;
- local deletes are submitted as facts, not policy;
- Core remains delete arbiter.

### WT-P8 — Watcher latency layer and runtime service

Goal:

```text
Add optional watcher/runtime service behavior for responsiveness while preserving
full scans as the correctness mechanism.
```

Allowed scope:

```text
crates/haze-sync-worktree/src/**
crates/haze-sync-worktree/docs/**
```

Likely work:

- introduce runtime service abstraction that can be hosted by Server;
- add watcher support as a debounce/schedule hint only;
- trigger scans/import planning after watcher events;
- support clean startup/shutdown cancellation;
- expose safe runtime status summaries;
- ensure disabled/read-only/import-only/export-only/bidirectional modes are respected where applicable.

Non-goals:

- no Server startup code unless fan-in scoped;
- no provider behavior;
- no watcher-only correctness;
- no hidden background jobs outside explicit lifecycle.

Contract-change triggers:

- claiming watcher reliability as correctness guarantee;
- starting background tasks without config/lifecycle ownership;
- ignoring adapter mode;
- moving runtime composition into generic Server routes without fan-in.

Acceptance:

- watcher improves latency but scans remain authoritative;
- runtime lifecycle is explicit and hostable;
- mode behavior is testable.

### WT-P9 — Doctor, repair planning, and Server-hosted fan-in

Goal:

```text
Expose safe worktree health/drift diagnostics and integrate with Server hosting
only through explicit component boundaries.
```

Allowed scope:

```text
crates/haze-sync-worktree/src/**
crates/haze-sync-worktree/docs/**
crates/haze-sync-server/** only in a dedicated cross-component fan-in prompt
```

Likely work:

- produce doctor summaries for missing files, dirty files, hash mismatches, reserved path violations, skipped symlinks/special files, and echo/drift state;
- plan repair actions without executing destructive changes by default;
- provide safe status data for Server admin/doctor routes;
- integrate Server-hosted lifecycle when Server fan-in is scheduled;
- add E2E tests for local worktree materialization/import loop.

Non-goals:

- no automatic destructive repair by default;
- no provider sync;
- no direct DB writes unless accepted by architecture;
- no public absolute local paths.

Contract-change triggers:

- repair actions that delete/overwrite without confirmation;
- status output leaking local absolute paths;
- Server integration requiring Worktree logic to move into Server.

Acceptance:

- operators can diagnose Worktree drift safely;
- repair plans are explicit and non-destructive by default;
- Server fan-in can host Worktree runtime without ownership collapse.

## Dependency gates

Worktree planning depends on Common/Core/API/Storage/Server contracts:

- Common owns path/hash/ID validation.
- Core owns conflict/delete/revision policy.
- API owns HTTP DTOs, headers, and public error vocabulary.
- Storage owns persisted `worktree_state` shape and repositories if used.
- Server owns hosting/composition/lifecycle if the Worktree runtime runs inside Server.

Worktree implementation should begin after path/base-revision/error contracts are stable enough to avoid duplicating Core/API behavior.

## Known risks

- Treating the filesystem as source of truth can bypass Core conflict safety.
- Path traversal, symlink following, or reserved-path mistakes can escape the configured root.
- Watcher events can be missed, duplicated, or reordered; scans must remain authoritative.
- Atomic write behavior is platform-sensitive and needs careful tests.
- Echo suppression can either miss self-writes or suppress real user edits if poorly modeled.
- Dirty-file detection must prevent silent overwrite during materialization.
- Local delete propagation is high-risk without tombstone/trash/retention guardrails.
- Server-hosted runtime can blur boundaries if Worktree logic migrates into Server.

## Deferred work

Deferred outside this Architect documentation/planning pass:

- run shell checks or observe CI;
- execute WT-P2 through WT-P9 implementation/clean-code/CI phases;
- implement scanner/writer/runtime code;
- add Worktree dependencies;
- integrate with Server startup;
- persist `worktree_state` through Storage/Server;
- add E2E tests using a real temporary worktree;
- implement local trash/repair/destructive behavior;
- support provider/GDrive/Obsidian behavior.

## Completion criteria for the component

`haze-sync-worktree` is V1-ready when:

- safe path mapping and scanning are implemented and tested;
- stable-file detection prevents partial import;
- import planner submits only Core/API facts with base/null-base semantics;
- materializer writes atomically and protects dirty local files;
- echo guard prevents self-induced sync loops;
- delete/trash behavior is safe and Core-confirmed;
- watcher is optional latency layer, not correctness foundation;
- doctor/repair facts are safe and useful;
- Server can host runtime through explicit fan-in without taking ownership of Worktree logic.
