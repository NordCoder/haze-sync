# Decisions: api

## 2026-07-05 — API remains passive; server owns runtime route wiring

Decision:
The API component remains a passive contract crate. It owns DTOs, header/auth value contracts, safe public error shapes, and pure route-contract helpers. `haze-sync-server` owns Axum route registration, middleware, runtime authentication lookup, app state, storage/Core dependency wiring, and final HTTP request handling.

Rationale:
Keeping API passive prevents contract types from accumulating runtime side effects or duplicating Core/server responsibilities. This preserves clear component boundaries: API describes public surfaces, Server wires those surfaces into HTTP behavior, Core decides sync safety outcomes, and Storage persists state.

Alternatives:
- Let API own Axum route registration. Rejected because it would make API depend on server runtime state and blur component ownership.
- Let API call Core/storage directly from route helpers. Rejected because helpers would stop being passive contracts and become service implementations.
- Duplicate runtime authorization and policy decisions in API. Rejected because it would increase drift risk and could bypass Core safety invariants.

Consequences:
- Server/fan-in phases must perform route wiring and handler integration.
- API tests should remain focused on serialization, validation, safe errors, and pure mapping.
- API helpers may carry role requirements and parsed metadata, but may not authenticate, query, mutate, or call providers.
- Future runtime behavior additions must be scoped to server, core, storage, CLI, or adapter components as appropriate.

Affected contracts:
- `crates/haze-sync-api/docs/component-contract.md`
- `crates/haze-sync-api/docs/dependency-map.md`
- `crates/haze-sync-server` route wiring contract

## 2026-07-05 — API owns public vocabulary, not Core policy

Decision:

API may define DTOs and public status/action strings for Core outcomes, but Core remains the owner of whether those outcomes occur.

Rationale:

Adapters, plugin clients, CLI, and Server need stable public JSON vocabulary. But if API starts deciding accepted/conflict/tombstone/idempotency outcomes, Core safety invariants can be bypassed or duplicated.

Alternatives:

- Let API evaluate conflict/delete/idempotency semantics.
- Let Core own all HTTP DTOs directly.
- Let Server invent public strings per route.

Consequences:

- API DTOs and helpers must remain pure mapping/contract surfaces.
- Server must preserve Core semantics when mapping into API responses.
- Public vocabulary changes require API docs/tests and likely Core/Server coordination.

Affected contracts:

- DTO modules;
- route helpers;
- Core outcome mapping;
- Server route implementation;
- adapter/plugin client expectations.

## 2026-07-05 — Header contracts preserve safety-critical request metadata

Decision:

API owns the public header contract for authorization, idempotency, content hash, and base revision metadata. Runtime enforcement belongs to Server/Core/Storage, but parsing/validation and public semantics belong in API.

Rationale:

`Idempotency-Key`, `X-Content-SHA256`, and `X-Base-Revision-Id` are not incidental transport details. They carry safety-critical semantics for replay, hash verification, and no-silent-overwrite behavior.

Alternatives:

- Parse these headers ad hoc in Server handlers.
- Hide base/null-base semantics inside Core only.
- Let adapters use inconsistent header names.

Consequences:

- Server should use API helpers or equivalent API-owned contracts when extracting request metadata.
- API must test malformed and sensitive header handling.
- API errors must never expose raw bearer tokens or idempotency keys.

Affected contracts:

- header helpers;
- Server file/delete route handlers;
- adapter clients;
- Obsidian plugin client;
- Core idempotency/revision semantics.

## 2026-07-05 — Admin/status output is summary-only

Decision:

API admin/status DTOs may expose public summaries, readiness state, adapter identity/mode/status, cursor presence, and capability flags. They must not expose raw cursors, token hashes, database URLs, local absolute paths, provider payloads, or raw runtime errors.

Rationale:

Operator surfaces are useful but leak-prone. API needs stable shapes for Server/CLI, while Server/Storage/adapters own actual checks and runtime state.

Alternatives:

- Return raw runtime/provider diagnostic objects.
- Put live doctor checks inside API.
- Omit admin/status contracts until late integration.

Consequences:

- Server must sanitize runtime details before constructing API DTOs.
- CLI can render status safely.
- Missing or skipped live checks must be represented honestly.

Affected contracts:

- admin route helpers;
- server-info/status DTOs;
- CLI status/doctor planning;
- Server admin/status routes;
- adapter status reporting.

## 2026-07-05 — Cross-language JSON compatibility needs fixtures

Decision:

API should eventually provide fixture examples for stable public JSON contracts instead of relying only on Rust type checking.

Rationale:

The Obsidian plugin is TypeScript and adapters/clients may not share Rust types directly. Rust compile-time checks cannot catch TypeScript mirror drift or accidental JSON vocabulary changes.

Alternatives:

- Generate TypeScript types from Rust immediately.
- Let the plugin duplicate JSON shapes manually without fixtures.
- Treat API docs as the only compatibility reference.

Consequences:

- A future API phase should add JSON fixtures and fixture stability tests.
- Obsidian plugin planning should consume those fixtures rather than inventing shapes.
- Generated clients remain optional and out of scope until explicitly accepted.

Affected contracts:

- API fixtures phase;
- Obsidian plugin client plan;
- adapter/client compatibility tests;
- E2E testing strategy.
