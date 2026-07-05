# Implementation Plan: api

## Current state

`haze-sync-api` is an active contract crate with post-W3 DTO and passive route-helper coverage.

Implemented current-state surfaces:

- `auth` contains pure adapter principal, role, bearer-token, token-hash, and SHA-256 verifier primitives.
- `contracts::headers` contains public header names and validated header wrappers.
- `contracts::errors` contains sanitized public error response shapes and public error codes.
- `dto` contains JSON DTOs for server-info, changes, file metadata/upload/delete, conflicts, shared enums, and string primitives.
- `routes` contains passive contract helpers for file PUT/GET, changes, conflicts, DELETE, and admin/status surfaces.

The crate is intentionally passive. It does not register Axum routes, start a server, query storage, call providers, create tombstones/conflicts/revisions, update cursors, or spawn background work.

## Target state

The target for the API component is a stable, safe Rust contract surface consumed by `haze-sync-server` and indirectly by adapters/UI clients.

Target properties:

- public DTO vocabulary is explicit, JSON-serializable, and deterministic;
- route helpers parse and validate request-shaped inputs without runtime side effects;
- public errors are sanitized and stable;
- auth helpers model principals, roles, and token verification primitives without owning runtime auth lookup;
- API docs clearly identify ownership boundaries with Core, Server, Storage, CLI, and adapters;
- tests verify serialization, validation, safe error output, and passive behavior expectations.

## Waves

### T0-P2 — API component contract audit

Status: implemented in this branch.

Work:

- Replace generic scaffold docs with current-state API component documentation.
- Document passive API ownership boundary.
- Document DTOs, route-contract helpers, auth/header contracts, safe public errors, non-goals, dependencies, safety rules, test obligations, risks, and deferred work.
- Add a decision that API remains passive and Server owns runtime route wiring.
- Update control state and write the implementation report.

### Follow-up — Clean-code review

Expected after implementation accept, per lifecycle.

Review focus:

- documentation accuracy against current source;
- consistency with dependency map and decisions;
- whether any API source behavior contradicts the passive contract;
- test-honesty and CI status reporting.

### Future API implementation work

Future work should be prompt-scoped and contract-aligned:

- add or adjust DTOs only when a component contract or fan-in phase requires it;
- add passive helper coverage for newly accepted HTTP surfaces;
- add compatibility tests when adapters or plugin clients begin consuming stable JSON contracts;
- update docs when public API vocabulary changes.

## Dependency gates

See `dependency-map.md`.

Important gates:

- changes to shared domain identifiers or validation must be coordinated with `haze-sync-common`;
- changes to Core operation/change/conflict/delete semantics must be coordinated with `haze-sync-core`;
- runtime route behavior must be implemented in `haze-sync-server`, not hidden in API helpers;
- storage schema or repository changes must remain in `haze-sync-storage`;
- adapter/client-specific behavior must remain in adapter/plugin components.

## Known risks

- Public DTO changes can silently break adapters or plugin clients if tests only cover Rust compilation.
- API helpers can become de facto service logic if future changes add storage/Core calls directly.
- Admin/status output can accidentally expose sensitive runtime values.
- Header wrappers and auth primitives expose raw values through intentionally narrow methods; callers must not log or serialize those values.
- Cross-crate conversions from Core types must remain pure mapping, not runtime coupling.

## Deferred work

- CI and shell checks were not run by this connector-only worker; CI verification remains external.
- Full HTTP integration and route registration behavior remains server/fan-in work.
- Cross-language contract tests with the Obsidian plugin are deferred until plugin consumption is active.
- Provider-specific API behavior remains adapter-owned and out of scope for this component.
- Any future public API additions must update component docs, dependency map, and tests together.
