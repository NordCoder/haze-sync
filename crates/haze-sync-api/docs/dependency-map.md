# Dependency Map: api

## Component role in dependency graph

`haze-sync-api` is the public contract layer for Haze Sync HTTP surfaces.

API owns public request/response shapes, DTO naming, API-level error envelopes, route contract descriptions, and safe serialization boundaries used by Server and clients.

API does not own Core decision policy, Storage persistence, Server runtime execution, provider behavior, deployment automation, or client UI behavior.

## Independent development model

`haze-sync-api` can be developed independently inside the `component/api` branch.

The dependency map records public contracts, consumers, and fan-in points. It does not impose a serial implementation order on Core, Server, adapters, CLI, Deployment, or CI.

Allowed independent work includes:

- DTO and error-envelope definitions;
- route contract helpers and API tests;
- public-safe representations of Core concepts;
- compatibility notes and examples;
- serialization/deserialization boundaries.

If API needs Core behavior, Server routing, Storage fields, provider metadata, or client behavior not currently contracted, it reports a contract-change request rather than implementing another component's responsibilities.

## Upstream contracts consumed

API may consume:

- `haze-sync-common` shared identifiers/value types when stable;
- Core concepts as public-safe representations, not as policy execution;
- accepted route/status/error semantics from component contracts.

API must not consume:

- Storage internals;
- Server handler implementation details;
- provider-specific clients;
- Obsidian plugin internals;
- deployment configuration;
- CI workflow logic.

## Downstream contracts exposed

Expected downstream consumers:

- Server, for route request/response contracts;
- Obsidian plugin, through HTTP responses served by Server;
- GDrive adapter, through Server/API contracts when the adapter talks to Core via HTTP;
- CLI, for future live commands;
- tests, examples, and docs.

Downstream consumers should not invent response shapes that conflict with API contracts.

## Forbidden dependency directions

API must not:

- decide Core conflict/delete policy;
- read/write the database;
- call providers;
- perform route runtime side effects;
- expose raw internal errors;
- encode deployment-specific file paths or host assumptions.

Server must implement API contracts without turning handler internals into public API.

## Cross-component contracts

Important API contracts:

- public errors are stable and safe;
- request/response fields have explicit semantics;
- conflict/tombstone/base-revision fields map to Core concepts but do not override Core;
- route contracts are versioned or changed intentionally;
- provider/internal state is not leaked through public DTOs.

## Integration/fan-in ownership

Fan-in is required when:

- Core introduces a new decision/result that needs public representation;
- Server exposes or changes a route using API DTOs;
- clients/adapters need fields not currently exposed;
- error/status vocabulary changes affect multiple consumers;
- examples/docs must align with implemented routes.

These are integration gates. They do not block independent API work inside its component boundary.

## Dependency rules

- API defines public shape; Core defines policy; Server executes routes.
- API should remain transport-contract code, not runtime orchestration.
- API changes that affect clients require fan-in coordination.
- API should avoid exposing fields before their semantics are clear.
- API docs/examples must distinguish planned contracts from implemented runtime behavior.

## Contract-change notes

Current known contract questions:

1. Public representation of Core results
   - API may need to add DTOs as Core phases define richer outcomes.
   - Missing Core semantics should be requested from Core, not guessed.

2. Adapter/client needs
   - GDrive adapter, CLI, and Obsidian plugin may request fields or routes.
   - Those needs should be fan-in points with Server/API/Core as appropriate.

3. Error taxonomy
   - Public error shape should remain stable and safe.
   - Raw component errors stay inside owning components.

No serial implementation dependency is implied by this map.
