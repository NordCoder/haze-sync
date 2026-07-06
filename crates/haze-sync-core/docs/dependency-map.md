# Dependency Map: core

## Component role in dependency graph

`haze-sync-core` is the safety and decision layer of Haze Sync.

Core owns file-level sync policy, revision semantics, conflict/tombstone decision rules, incoming-change evaluation, apply-result semantics, and safety invariants that must be shared by all adapters and runtime surfaces.

Core is the source of sync truth. It is not an HTTP server, provider adapter, filesystem watcher, deployment layer, or UI/client component.

## Independent development model

`haze-sync-core` can be developed independently inside the `component/core` branch.

The dependency map records contracts consumed/exposed by Core and the fan-in points needed for integration. It does not impose a serial implementation order on API, Storage, Server, adapters, CLI, Deployment, or CI.

Allowed independent work includes:

- Core domain types and service contracts;
- revision/base-revision validation;
- conflict policy evaluation;
- tombstone/delete policy evaluation;
- doctor/safety primitives owned by Core;
- unit tests for Core invariants;
- public-safe Core results consumed by Server/API wiring.

If Core needs persistence, route, provider, plugin, deployment, or CI behavior that is not covered by a current contract, it reports a contract-change request instead of implementing another component's responsibilities.

## Upstream contracts consumed

Core may consume:

- `haze-sync-common` shared IDs/value types where accepted;
- `haze-sync-storage` repository contracts for durable metadata/object operations;
- standard Rust libraries and approved domain-only dependencies.

Core must not consume:

- Server route handlers;
- API DTO serialization logic as decision authority;
- Google Drive provider clients;
- Obsidian plugin internals;
- CLI parsing/output code;
- deployment scripts;
- GitHub workflow logic.

## Downstream contracts exposed

Expected downstream consumers:

- Server, for runtime request handling and route execution;
- API, for mapping Core concepts into public-safe DTO contracts;
- Storage, for persistence needs discovered through Core contracts;
- Worktree, GDrive adapter, Obsidian plugin, and CLI indirectly through Server/API contracts;
- tests and future integration harnesses.

Downstream consumers must treat Core decisions as authoritative and must not silently bypass conflict/tombstone/base-revision rules.

## Forbidden dependency directions

Core must not depend on:

- adapters for policy decisions;
- provider-specific metadata as authoritative identity;
- HTTP request state;
- filesystem paths as hidden source-of-truth state;
- deployment environment layout;
- CI workflow assumptions.

Adapters must not bypass Core by directly choosing overwrite/delete/conflict outcomes.

## Cross-component contracts

Important Core contracts:

- every write has a known base revision or explicit null base;
- stale or unknown base cannot silently overwrite newer different content;
- conflicts preserve both sides unless an accepted policy says otherwise;
- deletes produce tombstones/trash/retention behavior rather than immediate hard-delete;
- adapters normalize external changes into Core input rather than deciding final state;
- public outputs are safe and do not expose internal/raw persistence/provider details.

## Integration/fan-in ownership

Fan-in is required when:

- Server exposes new Core decisions through HTTP routes;
- API adds/changes DTOs that represent Core results;
- Storage changes repository behavior Core depends on;
- adapters need a Core contract not currently available;
- Deployment/CLI/doctor flows surface Core health/safety checks.

These are integration gates. They do not block independent Core work inside its component boundary.

## Dependency rules

- Core owns sync decisions, not transport or provider execution.
- Core consumes Storage through contracts, not by embedding database-specific behavior.
- Core exposes stable service-level results for Server/API mapping.
- Core must keep safety invariants independent of adapter implementation details.
- Core tests should prove policy invariants without requiring live provider or deployment state.

## Contract-change notes

Current known contract questions:

1. Storage repository surface
   - Core may require additional durable operations as phases progress.
   - Missing operations should be reported as Storage contract-change requests.

2. API/Server representation
   - Core may define decision outputs before API/Server expose them.
   - HTTP/DTO mapping remains an API/Server fan-in concern.

3. Adapter integration
   - Adapters may need additional Core apply/result semantics.
   - They should request Core contracts rather than duplicate decision logic.

No serial implementation dependency is implied by this map.
