# Dependency Map: storage

## Component role in dependency graph

`haze-sync-storage` is the durable persistence implementation component.

Storage owns PostgreSQL schema access patterns, repository implementations, object-store persistence helpers, transactional boundaries, and durable metadata behavior needed by Core/Server runtime code.

Storage does not own Core sync policy, API DTOs, Server route execution, provider behavior, Deployment automation, CI workflow policy, or client UI behavior.

## Independent development model

`haze-sync-storage` can be developed independently inside the `component/storage` branch.

The dependency map records repository contracts, durability boundaries, and fan-in points. It does not impose a serial implementation order on Core, Server, API, adapters, Deployment, or CI.

Allowed independent work includes:

- repository interfaces and implementations;
- migration planning and schema access code;
- object-store path/content helpers;
- transaction helpers;
- persistence-focused tests;
- safe storage error classification.

If Storage needs Core policy, API DTOs, Server routes, provider semantics, deployment secrets, or CI workflow behavior, it reports a contract-change request instead of absorbing that responsibility.

## Upstream contracts consumed

Storage may consume:

- `haze-sync-common` shared IDs/value types;
- accepted persistence requirements from Core contracts;
- database/object-store libraries approved for the storage layer;
- migration/tooling contracts when explicitly scoped.

Storage must not consume:

- Core decision implementation as persistence logic;
- API DTOs as database schema authority;
- Server handler internals;
- provider/client code;
- deployment runtime paths as hard-coded assumptions;
- CI workflow behavior.

## Downstream contracts exposed

Expected downstream consumers:

- Core, for durable metadata, revision, tombstone, conflict, and object-store operations;
- Server, for runtime construction and health/doctor integration when scoped;
- CLI/doctor indirectly through Core/Server contracts;
- Deployment, through documented migration/object-store requirements;
- tests and future integration harnesses.

Downstream consumers must not bypass Storage contracts by embedding SQL/persistence assumptions in other components.

## Forbidden dependency directions

Storage must not:

- decide conflict/delete/base-revision policy;
- expose raw database errors publicly;
- call provider APIs;
- own HTTP API shapes;
- own deployment secret layout;
- assume local-only paths in product contracts;
- treat schema internals as public API.

## Cross-component contracts

Important Storage contracts:

- persistence errors are classified safely before public exposure;
- repository methods preserve Core invariants and transaction boundaries;
- schema/migration changes are explicit and auditable;
- object storage remains content-addressed and safe for durable use;
- hard-delete/retention behavior follows Core policy and accepted migration contracts.

## Integration/fan-in ownership

Fan-in is required when:

- Core needs a new durable operation;
- Server needs to wire Storage into runtime state;
- Deployment needs migration/backup/restore/runbook details;
- API/CLI/doctor needs public-safe storage health representation;
- migrations affect existing runtime assumptions.

These are integration gates. They do not block independent Storage work inside its component boundary.

## Dependency rules

- Storage owns persistence mechanics, not sync decisions.
- Storage should provide narrow repository contracts consumed by Core/Server.
- Storage must not leak raw database internals into public API responses.
- Migration changes must be documented and reversible/operationally understandable where possible.
- Storage tests should prove repository behavior without requiring live providers or clients.

## Contract-change notes

Current known contract questions:

1. Repository surface for future Core phases
   - Core may request new persistence methods.
   - Storage should add them as explicit contracts, not ad hoc SQL in Core.

2. Migration ownership
   - Storage owns schema/migration content.
   - Deployment owns operational execution/runbooks.

3. Object-store retention/cleanup
   - Storage may implement durable mechanics.
   - Core owns policy and safety semantics.

No serial implementation dependency is implied by this map.
