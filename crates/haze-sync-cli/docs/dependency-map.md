# Dependency Map: cli

## Component role in dependency graph

`haze-sync-cli` is the operator command surface.

CLI owns command parsing, operator-facing output, exit-code behavior, offline/live command boundaries, and future safe administrative command UX.

CLI does not own Core sync policy, API DTO definitions, Server route execution, Storage persistence, provider behavior, Deployment automation, GitHub workflow policy, or Obsidian UI behavior.

## Independent development model

`haze-sync-cli` can be developed independently inside the `component/cli` branch.

The dependency map records operator-surface contracts and fan-in points. It does not impose a serial implementation order on Core, API, Server, Storage, Deployment, adapters, or CI.

Allowed independent work includes:

- parser/command model hardening;
- help/status/adapters command UX;
- offline doctor rendering where scoped;
- output formatting and exit-code contracts;
- config/source-loading boundaries;
- tests for parser/output safety.

If CLI needs live Server/API endpoints, Core doctor checks, Deployment config layout, or CI packaging behavior not currently contracted, it reports a contract-change request or fan-in need instead of implementing another component's responsibility.

## Upstream contracts consumed

CLI may consume:

- Core doctor/safety primitives when intentionally exposed;
- Server/API HTTP contracts for live commands when implemented;
- Common shared types where accepted;
- Deployment config guidance for operator defaults;
- CI packaging/build validation as workflow-owned validation.

CLI must not consume:

- Storage internals or direct DB access by default;
- Server private handler state;
- provider APIs directly;
- Obsidian plugin internals;
- deployment secrets;
- GitHub workflow internals.

## Downstream contracts exposed

Expected downstream users/consumers:

- human operators;
- scripts consuming stable output/exit codes once those formats are accepted;
- Deployment docs/runbooks;
- CI/package validation;
- future support/debug workflows.

Downstream consumers must not treat placeholder CLI output as proof of implemented runtime behavior.

## Forbidden dependency directions

CLI must not:

- decide final sync/conflict/delete policy outside Core;
- directly mutate database/provider state unless an explicit admin contract allows it;
- expose raw internal errors;
- print sensitive config values;
- assume production service layout without Deployment contract;
- implement Server/API behavior locally as a workaround.

## Cross-component contracts

Important CLI contracts:

- command output must be safe for humans and scripts;
- exit codes must be stable once documented;
- live commands call public Server/API contracts;
- offline diagnostics must clearly state they are offline;
- administrative mutation commands require explicit confirmation/audit semantics when implemented.

## Integration/fan-in ownership

Fan-in is required when:

- Server/API live endpoints are consumed by CLI;
- Core doctor checks are exposed through CLI;
- Deployment uses CLI in runbooks;
- CI/package validation changes CLI build expectations;
- output formats become scripting contracts.

These are integration gates. They do not block independent CLI work inside its component boundary.

## Dependency rules

- CLI owns operator UX, not runtime authority.
- CLI should call Server/API for live behavior rather than direct DB/provider access.
- CLI output must distinguish placeholders, offline diagnostics, and live results.
- CLI tests should verify parsing/output without requiring production services by default.
- Any destructive/admin command requires explicit upstream contracts.

## Contract-change notes

Current known contract questions:

1. Live command API surface
   - CLI may need Server/API endpoints for status, adapters, sync, bootstrap, and admin operations.
   - Missing endpoints are API/Server/Core fan-in points.

2. Doctor integration
   - CLI can render checks independently.
   - Core/Server own check semantics and live runtime access.

3. Scripting output
   - JSON/stable output formats should be treated as compatibility contracts once introduced.

No serial implementation dependency is implied by this map.
