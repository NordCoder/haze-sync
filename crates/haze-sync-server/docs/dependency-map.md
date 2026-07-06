# Dependency Map: server

## Component role in dependency graph

`haze-sync-server` is the runtime composition and HTTP execution boundary.

Server owns application startup, runtime state assembly, route wiring, HTTP handler execution, health/status surfaces, and safe translation between API contracts and Core/Storage/Worktree services.

Server does not own Core sync policy, API DTO definitions, Storage persistence mechanics, provider adapter behavior, Deployment automation, GitHub CI workflow policy, or client UI behavior.

## Independent development model

`haze-sync-server` can be developed independently inside the `component/server` branch.

The dependency map records runtime composition contracts and fan-in points. It does not impose a serial implementation order on Core, API, Storage, Worktree, adapters, CLI, Deployment, or CI.

Allowed independent work includes:

- route wiring against current API/Core contracts;
- runtime state construction;
- health/status/doctor handler integration;
- safe error mapping;
- server-local tests;
- small server-internal cleanup that does not change sibling contracts.

If Server needs a DTO, Core service, Storage repository method, Worktree runtime feature, adapter behavior, deployment config, or CI policy not currently contracted, it reports a contract-change request or fan-in need rather than implementing another component's responsibility.

## Upstream contracts consumed

Server may consume:

- API request/response/error contracts;
- Core service contracts and decision results;
- Storage repository/object-store contracts through runtime state;
- Worktree adapter/service contracts when built into server runtime;
- Common shared types;
- configuration contracts accepted by Deployment.

Server must not consume:

- provider APIs directly for GDrive/Obsidian behavior;
- Obsidian plugin internals;
- CLI parsing/output internals;
- GitHub workflow logic;
- raw deployment secret files as product contracts.

## Downstream contracts exposed

Expected downstream consumers:

- Obsidian plugin, through HTTP API;
- GDrive adapter, through HTTP/Core API where applicable;
- CLI, through future live commands;
- Deployment, through service configuration and health surfaces;
- GitHub CI, through testable server behavior;
- human operators through safe status/doctor output.

Downstream consumers must treat Server as a runtime surface, not as a place to bypass Core/API/Storage contracts.

## Forbidden dependency directions

Server must not:

- silently decide conflict/delete policy outside Core;
- define public DTOs outside API ownership;
- embed raw SQL/persistence behavior that belongs to Storage;
- directly call Google Drive or Obsidian provider internals;
- expose raw internal errors publicly;
- hard-code deployment-only secrets/paths as product behavior.

## Cross-component contracts

Important Server contracts:

- HTTP behavior follows API DTO/error contracts;
- route execution delegates sync decisions to Core;
- persistence behavior goes through Storage contracts;
- public output is safe and does not expose raw internal state;
- runtime health/doctor surfaces distinguish implemented behavior from placeholders;
- built-in Worktree integration remains within accepted Worktree/Server contracts.

## Integration/fan-in ownership

Fan-in is required when:

- API DTOs change and Server routes must be aligned;
- Core service behavior changes and route execution must be updated;
- Storage repository behavior changes runtime construction;
- Worktree service behavior becomes runtime-mounted;
- Deployment needs service config/runbook changes;
- adapters/clients require new HTTP route semantics.

These are integration gates. They do not block independent Server work inside its component boundary.

## Dependency rules

- Server composes runtime behavior; it does not own domain policy.
- Server uses API for public shapes and Core for decisions.
- Server uses Storage through explicit contracts.
- Server should keep handler errors public-safe.
- Server route tests should verify wiring without assuming live provider behavior.

## Contract-change notes

Current known contract questions:

1. Runtime state shape
   - Server may need new Core/Storage/Worktree dependencies as phases progress.
   - Missing upstream contracts should be requested explicitly.

2. HTTP route coverage
   - Server can wire current route contracts independently.
   - New route shapes require API/Core fan-in where applicable.

3. Deployment integration
   - Server may expose config/health surfaces.
   - Deployment owns service placement and operational runbooks.

No serial implementation dependency is implied by this map.
