# Component Contract: worktree

## Responsibility

`haze-sync-worktree` owns the built-in VPS worktree adapter/runtime logic for Haze Sync.

The worktree is a materialized filesystem replica of Core state for local tools and agents. It is not the source of truth.

The component is responsible for future Worktree behavior such as:

- mapping between normalized `VaultPath` values and local filesystem paths under one configured worktree root;
- safe filesystem scanning;
- stable-file detection before importing local edits;
- ignore/reserved-path handling;
- atomic materialization of Core revisions into the worktree;
- echo suppression for files just written by the adapter;
- dirty-state tracking and reconciliation support;
- local trash/backup behavior for safe deletes when scoped;
- repair/doctor facts related to worktree drift;
- runtime service abstractions that Server may host in V1.

Current code state is placeholder-only. The component contract therefore defines the intended V1 ownership boundary and implementation plan before runtime behavior is added.

## Public interfaces

Current public Rust interface:

```text
CRATE_ROLE
package_name()
```

Target public interfaces should be introduced in phases and may include:

```text
WorktreeConfig
WorktreeMode
WorktreeScanner
WorktreeScanResult
WorktreeFileSnapshot
StableFileDetector
WorktreeMaterializer
AtomicWorktreeWriter
WorktreeImporter
WorktreeApplyPlan
WorktreeEchoGuard
WorktreeTrash
WorktreeRepairPlan
WorktreeDoctorSummary
```

Names are indicative. Implementation workers should choose final names consistent with crate style and current upstream contracts.

The public interface must separate:

- filesystem observation;
- conversion to Core/API facts;
- materialization of Core revisions;
- local safety/echo/trash state;
- runtime lifecycle composition owned by Server.

## Input contracts

Worktree inputs come from two directions.

### Filesystem inputs

Filesystem scans and watchers observe untrusted local state under a configured worktree root.

Required behavior:

- never follow paths outside the configured worktree root;
- normalize candidate paths into `VaultPath` before sending facts to Core/API;
- reject traversal, absolute paths, symlinks or special files unless explicitly accepted by a future contract;
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

Worktree outputs should be facts, plans, or local side effects within the configured worktree root.

Expected outputs:

- scan facts representing local files or delete candidates;
- import requests to Core/API with explicit base revision or explicit null base;
- materialized local files for accepted Core revisions;
- dirty-state summaries;
- local trash/backup records when delete behavior is scoped;
- doctor/repair facts indicating drift, missing files, unexpected hashes, reserved path violations, or skipped unsafe entries.

Public or operator-facing outputs must be sanitized:

- avoid local absolute paths where possible;
- represent paths as vault-relative `VaultPath` values;
- do not expose bearer tokens, OAuth tokens, token hashes, idempotency keys, provider payloads, database URLs, stack traces, or raw filesystem errors.

## Error contracts

Worktree errors must be safe across component boundaries.

Errors must not expose:

- local absolute paths outside explicit safe operator-only diagnostics;
- bearer tokens;
- OAuth tokens;
- token hashes;
- Idempotency-Key values;
- database URLs;
- raw provider payloads;
- raw API response bodies containing secrets;
- stack traces;
- raw filesystem errors with sensitive paths.

When filesystem failures need operator action, errors should provide safe categories and vault-relative path context where possible.

## Persistence/runtime ownership

Worktree owns local filesystem adapter logic, but not the authoritative sync database.

Worktree owns:

- path mapping under the configured worktree root;
- local scanner/watcher abstractions when implemented;
- stable-file detection;
- atomic writer/materializer behavior;
- echo guard behavior;
- local trash/backup behavior when scoped;
- worktree-specific doctor/repair facts;
- local runtime service code if the crate defines it.

Worktree does not own:

- Core conflict/delete/revision policy;
- API DTO/header/public error vocabulary;
- Storage schema or repository ownership, except through accepted integration boundaries;
- Server runtime startup or hosting lifecycle;
- Google Drive API calls;
- Obsidian plugin behavior;
- production deployment files;
- hard delete of Core blobs/database rows/provider files.

In V1, Server may host the Worktree runtime, but Server owns composition/lifecycle while Worktree owns scanner/materializer/importer semantics.

## Security and secrecy rules

- Do not commit secrets.
- Do not expose tokens or token hashes in public outputs.
- Do not expose database URLs, provider payloads, or local absolute paths in public API/debug output.
- Do not follow filesystem paths outside the configured worktree root.
- Do not let user-controlled paths choose temp/runtime paths outside the root.
- Do not silently overwrite locally dirty files.
- Do not hard-delete local files without trash/backup/retention behavior explicitly scoped.
- Do not import files from reserved runtime directories.
- Do not trust filesystem watcher events as sufficient for correctness; scans are required for correctness.

## Non-goals

Worktree must not implement:

- Core sync policy decisions;
- API route handlers or DTO ownership;
- PostgreSQL schema/repository logic;
- Server startup/listener behavior;
- Google Drive provider integration;
- Obsidian plugin local state/UI;
- global background runtime without Server composition contract;
- semantic markdown merge;
- CRDT/block-level merge;
- rename tracking beyond V1 delete+create semantics unless a future contract changes this;
- hard delete of Core/object-store/provider data.

## Dependencies

See `dependency-map.md`.

## Dependents

See `dependency-map.md`.

## Invariants

- Core metadata and object store are authoritative.
- Worktree is a materialized filesystem replica, not source of truth.
- Local filesystem changes are facts submitted to Core/API; they are not overwrite decisions.
- Watchers are for latency; scans are for correctness.
- Every imported write must carry base revision semantics or explicit null base.
- Dirty local files must not be silently overwritten by materialization.
- Adapter-written files must not be re-imported as new local edits without echo suppression.
- Delete behavior must be tombstone/trash/retention-oriented and must not become immediate hard delete.
- All local paths must stay under the configured worktree root.

## Test obligations

Worktree tests should eventually cover:

- path mapping from `VaultPath` to local paths and back;
- rejection of traversal, absolute paths, symlinks/special files, and reserved runtime paths;
- ignore rules and temp-file exclusion;
- stable-file detection under concurrent write simulation;
- content hash computation and mismatch handling;
- atomic writer behavior and crash-safe temp cleanup where practical;
- materialization avoiding dirty-file overwrite;
- echo guard suppression for adapter-written files;
- scan correctness independent of watcher events;
- local delete/trash behavior when scoped;
- doctor/repair summaries with safe path-redacted output;
- Server-hosted lifecycle behavior in fan-in/E2E tests when Worktree runtime is integrated.

Checks expected for Worktree changes when shell or CI is available:

```bash
cargo fmt --check
cargo check -p haze-sync-worktree
cargo test -p haze-sync-worktree
```

## Contract change protocol

Request a contract change instead of silently broadening scope when implementation requires:

- treating Worktree as source of truth;
- bypassing Core/API write semantics;
- adding direct DB writes from Worktree;
- adding provider/GDrive behavior;
- adding Server startup/lifecycle code inside Worktree beyond library service abstractions;
- following symlinks or paths outside root;
- hard-deleting local files without trash/retention contract;
- changing V1 rename semantics;
- exposing local absolute paths or secrets in public outputs.
