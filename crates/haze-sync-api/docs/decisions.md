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
