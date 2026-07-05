# System architecture

## Architectural style

Haze Sync V1 uses centralized server-side file-level synchronization.

The server-side system owns the authoritative metadata, object storage, operation log, conflict records, tombstones, adapter cursors, and idempotency records. External replicas import and export changes through explicit Core/API contracts.

A file is the atomic synchronization unit. V1 deliberately does not implement CRDT, block-level sync, semantic markdown merge, full three-way merge, or real-time collaborative editing.

## Product goal

Haze Sync keeps one vault coherent across:

- Core server state;
- a VPS worktree used by Haze agents;
- Google Drive for PC/human/connector visibility;
- a local Obsidian vault, including mobile use.

The goal is near-realtime practical sync while preserving data under conflicting edits, stale bases, unknown bases, provider glitches, crashes, and unsafe delete bursts.

## Runtime mental model

```text
Core metadata + object store
  authoritative sync state

VPS worktree
  materialized filesystem view for agents

Google Drive
  external cloud replica

Obsidian vault
  external local replica
```

Core is the only component allowed to decide whether a write is accepted, ignored, saved as a conflict, or rejected as unsafe. Adapters report facts and obey Core outcomes.

## Main runtime shape

```text
VPS
├── reverse proxy
├── haze-sync-server
│   ├── HTTP API
│   ├── Core services
│   ├── Storage repositories
│   └── built-in worktree runtime wiring, when enabled
├── haze-gdrive-adapter
├── PostgreSQL
├── content-addressed object store
└── /srv/haze-vault/worktree

Client devices
└── Obsidian plugin
```

The worktree adapter is conceptually an adapter, but V1 may run its runtime loop inside `haze-sync-server` because it uses local filesystem access on the same VPS. The implementation must still keep Worktree logic in the Worktree component and keep Server responsible for runtime composition.

## Repository components

```text
crates/haze-sync-common
  shared domain, value, path, hash, adapter, and security primitives

crates/haze-sync-core
  deterministic sync decisions, revision semantics, conflict/delete policy,
  idempotency semantics, doctor summaries, and operation outcomes

crates/haze-sync-api
  public HTTP DTOs, headers, route contracts, auth helper types, and safe errors

crates/haze-sync-storage
  PostgreSQL schema, migrations, repository implementations, object-store
  implementation, and storage test support

crates/haze-sync-server
  Axum runtime, router composition, readiness, auth execution, runtime state,
  error mapping, and service wiring

crates/haze-sync-worktree
  scanner, ignore rules, stable-file detection, atomic writer, echo guard,
  materializer/importer primitives, trash, and repair reports

crates/haze-gdrive-adapter
  Google Drive provider client, scan/change/import/export planners,
  mapping semantics, echo guard, delete guard, and adapter runtime

crates/haze-sync-cli
  operator commands, status, doctor, adapter visibility, bootstrap support,
  dry-run commands, and safe operational wrappers

apps/haze-obsidian-plugin
  Obsidian settings, API client, local state, scanner, pending queue,
  sync planner, apply engine, event watcher, and conflict UI

deploy
  local and production deployment scaffolding without secrets

.github/workflows
  CI checks and quality gates
```

## Data model summary

Core state is represented by:

- normalized vault paths;
- immutable file revisions;
- SHA-256 content hashes;
- content-addressed blobs;
- current object state per path;
- append-only operation log entries;
- adapter cursors;
- tombstones and retention metadata;
- conflict records;
- idempotency records;
- adapter-specific mapping tables such as Google Drive file mapping and worktree state.

The object store and metadata database together define authoritative sync state. Neither the filesystem worktree nor Google Drive is sufficient as source state because they do not carry base revisions, operation order, tombstones, conflict records, adapter cursors, or idempotency records.

## Normal write flow

```text
external replica change
  -> adapter normalizes path/content/base
  -> adapter submits PUT/DELETE with idempotency key and base revision
  -> API validates public request shape
  -> Server authenticates and maps request into service call
  -> Core decides accepted/ignored/conflict/rejected
  -> Storage persists metadata and blobs under Server-owned transaction boundaries
  -> operation log advances
  -> adapters consume changes after their cursors
```

Adapters must advance cursors only after they have successfully processed all changes up to the acknowledged sequence.

## Adapter boundary

Each adapter follows this conceptual contract:

```text
external state/change
  -> normalize
  -> submit to Core/API
  -> receive Core outcome
  -> update adapter-local state
  -> apply export or skip according to outcome
```

Adapters may detect local changes, provider changes, missing files, duplicate provider objects, unsupported file kinds, local drift, and potential deletes. They must not implement overwrite policy, conflict resolution policy, or tombstone authority.

## Current implementation status

Implemented server-side foundation:

- Rust workspace and component crate boundaries;
- shared domain/value/path/hash/auth primitives;
- storage schema, migrations, repositories, and local object-store primitives;
- normal Core upsert semantics;
- conflict-saved planning primitives;
- tombstone/delete guard/idempotency primitives;
- API DTOs and passive route helpers;
- server routes for files, changes, server-info, conflicts, delete, admin/status, and doctor scaffolding;
- read-only CLI/status/doctor foundation;
- CI/dev tooling;
- Obsidian plugin scaffold.

Not production-complete yet:

- Google Drive runtime sync;
- Obsidian plugin runtime sync;
- worktree scanner/watcher/materialization runtime;
- bootstrap/import flow;
- background retention/cleanup;
- restore workflow;
- full `accept_conflict` content replacement behavior;
- production E2E sync scenario;
- real vault rollout tooling.

## V1 non-goals

Do not implement in V1 unless the system contract is explicitly changed:

- CRDT;
- block-level sync;
- semantic markdown merge;
- full three-way merge;
- real-time collaborative editing;
- Google Docs/Sheets/Slides content support;
- Google Drive shared drives;
- Google Drive shortcuts;
- multi-user SaaS mode;
- end-to-end encryption;
- iOS background sync guarantee;
- multiple simultaneous vaults;
- object-level permissions beyond simple adapter roles;
- Web UI dashboard.

## Architecture risks to keep visible

The most important risks are cross-component leakage:

- adapters implementing Core policy locally;
- API helpers becoming runtime services;
- Storage repositories encoding conflict/delete semantics;
- Server owning adapter internals instead of composition;
- Worktree being treated as source state;
- Google Drive mapping forcing the adapter to become a DB-owning service;
- CLI exposing destructive actions before backup, doctor, and delete guard support exist.

Component plans and reviews should explicitly check these risks.
