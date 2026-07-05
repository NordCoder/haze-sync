# Implementation Plan: api

## Current state

`haze-sync-api` is an active contract crate with post-W3 DTO and passive route-helper coverage.

Implemented current-state surfaces:

- `auth`
  - pure adapter principal, role, bearer-token, token-hash, and SHA-256 verifier primitives.
- `contracts::headers`
  - public header names and validated header wrappers.
- `contracts::errors`
  - sanitized public error response shapes and public error codes.
- `dto`
  - JSON DTOs for server-info, changes, file metadata/upload/delete, conflicts, shared enums, and string primitives.
- `routes`
  - passive contract helpers for file PUT/GET, changes, conflicts, DELETE, and admin/status surfaces.

The crate is intentionally passive. It does not register Axum routes, start a server, query storage, call providers, create tombstones/conflicts/revisions, update cursors, or spawn background work.

## Target state

The target for the API component is a stable, safe Rust contract surface consumed by `haze-sync-server` and indirectly by adapters, plugin clients, CLI surfaces, and E2E tests.

Target properties:

- public DTO vocabulary is explicit, JSON-serializable, and deterministic;
- route helpers parse and validate request-shaped inputs without runtime side effects;
- public errors are sanitized and stable;
- auth helpers model principals, roles, and token verification primitives without owning runtime auth lookup;
- API docs clearly identify ownership boundaries with Core, Server, Storage, CLI, Worktree, GDrive, and Obsidian;
- tests verify serialization, validation, safe error output, and passive behavior expectations;
- DTO/route helper contracts are stable enough for cross-language client fixtures.

API is V1-ready when:

- all currently advertised route surfaces have passive DTO/helper coverage;
- write/delete contracts preserve `Idempotency-Key` and explicit base/null-base semantics;
- conflict/delete/admin/status outputs are safe and do not expose raw runtime/provider/storage details;
- Server can wire routes using API helpers without inventing public vocabulary;
- adapter/plugin clients can consume stable JSON examples or compatibility fixtures;
- no Axum, SQLx, provider SDK, filesystem watcher, background job, or runtime service dependency exists in API.

## Implementation phases

### API-P1 — Component contract and planning normalization

Status: completed by T0-P2 plus this Architect planning pass.

Goal:

```text
Keep API docs accurate enough that future workers can evolve public contract
surfaces without turning API into runtime/service logic.
```

Allowed scope:

```text
crates/haze-sync-api/docs/**
crates/haze-sync-api/control/state.md when explicitly assigned by Orchestrator
```

Completed deliverables:

- current component contract;
- dependency map;
- implementation log entry;
- passive-API decision;
- expanded phased implementation plan.

Non-goals:

- no product code changes;
- no route registration;
- no Core/Storage/Server runtime calls;
- no sibling component changes.

Acceptance:

- docs describe the current API module surface;
- docs distinguish public contract ownership from Server runtime wiring;
- future phases are implementable without adding runtime dependencies.

### API-P2 — Public DTO and serialization audit

Goal:

```text
Audit all public DTO modules for stable JSON vocabulary, safe output, and clear
ownership boundaries before adapters/plugin rely on them.
```

Allowed scope:

```text
crates/haze-sync-api/src/dto/**
crates/haze-sync-api/src/contracts/errors/**
crates/haze-sync-api/docs/**
```

Likely work:

- verify stable `snake_case` values for shared enums and status/action vocabularies;
- add serde roundtrip tests for server-info, changes, file metadata, upload outcomes, delete outcomes, conflict list/resolve, and admin/status DTOs;
- verify raw file bytes are absent from JSON DTOs;
- verify raw cursors, token hashes, bearer tokens, idempotency keys, database URLs, local paths, stack traces, provider payloads, and raw request bodies are absent from public DTOs/errors;
- document any DTO that is intentionally partial or placeholder.

Non-goals:

- no route wiring;
- no storage/core calls;
- no client TypeScript edits;
- no provider-specific DTOs unless a public contract requires them.

Contract-change triggers:

- changing public field names or status values;
- adding provider-specific payloads;
- exposing raw cursor/token/key/body/file content values;
- moving Core semantic decisions into DTO constructors.

Acceptance:

- DTO tests cover current public vocabulary;
- public JSON remains safe and deterministic;
- docs reflect any intentionally deferred DTO behavior.

### API-P3 — Header, auth, and safe error contract hardening

Goal:

```text
Make request metadata contracts explicit and safe before Server/CLI/adapters
expand authentication, idempotency, and content-hash usage.
```

Allowed scope:

```text
crates/haze-sync-api/src/auth/**
crates/haze-sync-api/src/contracts/headers/**
crates/haze-sync-api/src/contracts/errors/**
crates/haze-sync-api/docs/**
```

Likely work:

- test `Authorization: Bearer <token>` parsing/wrapping without leaking token text;
- test token hash formatting redaction and constant-time verification behavior;
- test `Idempotency-Key` validation and non-exposure in errors/debug output;
- test `X-Content-SHA256` parsing and canonical conversion to common hash types;
- test `X-Base-Revision-Id` including literal `null` semantics;
- test public error response codes/messages/details for safe field/header/query naming.

Non-goals:

- no token persistence;
- no token creation/rotation;
- no runtime auth lookup;
- no middleware;
- no SQLx or config loading.

Contract-change triggers:

- changing required headers;
- changing null-base representation;
- changing token hash representation;
- adding runtime auth lookup or token repository calls to API.

Acceptance:

- request metadata helpers are safe and tested;
- Server can use helpers without duplicating parsing rules;
- public errors never leak raw secret/key/header values.

### API-P4 — File and changes route-helper contract hardening

Goal:

```text
Stabilize passive helpers for normal file PUT/GET and changes feed surfaces so
Server can wire them to Core/Storage without inventing request/response shape.
```

Allowed scope:

```text
crates/haze-sync-api/src/routes/files/**
crates/haze-sync-api/src/routes/changes/**
crates/haze-sync-api/src/dto/files/**
crates/haze-sync-api/src/dto/changes/**
crates/haze-sync-api/docs/**
```

Likely work:

- test path parsing through `VaultPath`;
- test upload metadata extraction for path, base revision, content hash, idempotency key, adapter principal requirement, and body length metadata if represented;
- test mapping of accepted/same-content/conflict-saved/hash-mismatch/stale outcomes into public response metadata;
- test changes query `since`/`limit` bounds and response page metadata;
- ensure helpers remain passive and do not call Core/Storage.

Non-goals:

- no Axum handler implementation;
- no object-store reads/writes;
- no operation-log queries;
- no content streaming;
- no background cursor updates.

Contract-change triggers:

- changing upload response status vocabulary;
- hiding explicit null-base semantics;
- adding live storage/Core calls to helper functions;
- changing changes-feed pagination semantics.

Acceptance:

- Server has a stable passive contract for files/changes;
- route helpers parse/shape metadata only;
- tests prove safe handling of boundary values.

### API-P5 — Conflict and delete route-helper contract hardening

Goal:

```text
Stabilize public conflict/delete vocabulary before full conflict-resolution and
delete propagation fan-in.
```

Allowed scope:

```text
crates/haze-sync-api/src/routes/conflicts/**
crates/haze-sync-api/src/routes/delete/**
crates/haze-sync-api/src/dto/conflicts/**
crates/haze-sync-api/src/dto/files/** when delete DTOs live there
crates/haze-sync-api/docs/**
```

Likely work:

- test conflict list query parsing, especially status filters and bounds;
- test conflict resolve action parsing for `accept_current`, `accept_conflict`, `keep_both`, and `mark_resolved`;
- test conflict list/resolve DTO serde roundtrips and safe path/hash/revision fields;
- test DELETE metadata extraction for path, base revision/null-base, idempotency key, and adapter principal requirement;
- test delete response vocabulary for tombstoned/not_found/rejected/guard-blocked style outcomes;
- ensure API does not implement conflict resolution or tombstone creation itself.

Non-goals:

- no conflict row repository;
- no `accept_conflict` content replacement implementation;
- no tombstone persistence;
- no provider/worktree trash behavior;
- no mass-delete guard execution beyond public contract representation.

Contract-change triggers:

- changing resolution action vocabulary;
- adding latest-wins/incoming-wins public policy;
- exposing raw conflict bytes;
- implying hard delete in public output;
- adding Core/Storage calls to helpers.

Acceptance:

- conflict/delete surfaces expose safe vocabulary only;
- Server/Core/Storage can wire real behavior later without public contract ambiguity;
- adapters/plugin can reason about conflict/delete responses without policy duplication.

### API-P6 — Admin/status and doctor-facing contract hardening

Goal:

```text
Keep operational DTOs useful while preventing status/admin surfaces from leaking
runtime secrets, cursors, paths, provider payloads, or raw errors.
```

Allowed scope:

```text
crates/haze-sync-api/src/routes/admin/**
crates/haze-sync-api/src/dto/server/**
crates/haze-sync-api/src/dto/common/** if shared status types live there
crates/haze-sync-api/docs/**
```

Likely work:

- test server-info capabilities and version/protocol metadata;
- test admin status DTOs for readiness, adapter summaries, cursor presence, pause support, and sanitized runtime state;
- represent skipped/not-run/placeholder status honestly where Server does not perform live checks;
- verify admin/status output does not expose raw cursors, token hashes, database URLs, local absolute paths, provider payloads, or raw errors;
- keep mutation/admin actions out of API unless a public contract exists.

Non-goals:

- no live doctor checks;
- no server readiness implementation;
- no adapter pause/resume mutation;
- no repair execution;
- no provider calls.

Contract-change triggers:

- exposing cursor values instead of cursor-presence summaries;
- adding admin mutation routes;
- embedding runtime error payloads;
- changing advertised capabilities ahead of server behavior without caveats.

Acceptance:

- operator-visible DTOs are safe and honest;
- Server can map runtime summaries into API shapes;
- future CLI/status commands have stable public vocabulary.

### API-P7 — Cross-component compatibility fixtures

Goal:

```text
Provide stable examples for Rust Server/CLI and TypeScript Obsidian/plugin clients
without moving client behavior into API.
```

Allowed scope:

```text
crates/haze-sync-api/**
```

Possible deliverables:

- JSON fixtures for server-info, file PUT outcomes, file metadata, changes pages, conflict list/resolve, delete outcomes, public errors, and admin/status summaries;
- Rust tests asserting fixture stability;
- notes for TypeScript mirror types in Obsidian plugin planning.

Non-goals:

- no TypeScript code edits in this component phase;
- no generated client pipeline unless explicitly accepted;
- no server route tests;
- no provider-specific examples containing real payloads.

Contract-change triggers:

- needing to edit Obsidian plugin or Server to make fixtures meaningful;
- discovering DTO ownership ambiguity with Core;
- adding raw provider/runtime data to fixture payloads.

Acceptance:

- downstream clients can validate public JSON compatibility;
- fixtures remain secret-free and provider-free;
- API public contract drift becomes test-visible.

## Dependency gates

See `dependency-map.md`.

Important gates:

- changes to shared domain identifiers, path normalization, hash representation, adapter roles/modes, or validation must be coordinated with `haze-sync-common`;
- changes to Core operation/change/conflict/delete/idempotency semantics must be coordinated with `haze-sync-core`;
- runtime route behavior must be implemented in `haze-sync-server`, not hidden in API helpers;
- storage schema or repository changes must remain in `haze-sync-storage`;
- adapter/client-specific behavior must remain in Worktree/GDrive/Obsidian components;
- public JSON fixtures should be added before plugin/client work relies heavily on API DTOs.

## Known risks

- Public DTO changes can silently break adapters or plugin clients if tests only cover Rust compilation.
- API helpers can become de facto service logic if future changes add storage/Core calls directly.
- Admin/status output can accidentally expose sensitive runtime values.
- Header wrappers and auth primitives expose raw values through intentionally narrow methods; callers must not log or serialize those values.
- Cross-crate conversions from Core types must remain pure mapping, not runtime coupling.
- Public capability metadata can overclaim if Server advertises behavior before it is actually wired.
- TypeScript mirror types can drift without fixtures or explicit compatibility tests.

## Deferred work

Deferred outside this Architect documentation/planning pass:

- run shell checks or observe CI for this branch;
- execute API-P2 through API-P7 implementation/clean-code/CI phases;
- full HTTP integration and route registration behavior in Server;
- durable idempotency and storage behavior in Storage/Server;
- cross-language contract tests with the Obsidian plugin;
- provider-specific adapter behavior in adapter components;
- production E2E tests.

## Completion criteria for the component

`haze-sync-api` is V1-ready when:

- public DTOs and route-helper contracts cover accepted V1 surfaces;
- serialization and parsing tests cover the stable public vocabulary;
- header/auth/error helpers are safe and tested;
- no runtime/persistence/provider/server dependency enters the crate;
- Server can wire live handlers using API contracts without inventing public shapes;
- adapters/plugin/CLI can consume stable JSON vocabulary or fixtures;
- docs, dependency map, and tests are updated whenever public API vocabulary changes.
