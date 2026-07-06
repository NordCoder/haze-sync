# Dependency Map: gdrive-adapter

## Component role in dependency graph

`haze-gdrive-adapter` is the Google Drive external replica adapter.

GDrive adapter owns Google Drive API interaction, OAuth/token loading boundary, Drive scan/change-feed/export planning, Drive-to-Core normalization, provider echo suppression, and Drive-specific safety guardrails.

GDrive adapter does not own Core sync policy, API DTO definitions, Server runtime internals, Storage schema, Worktree behavior, Obsidian plugin behavior, Deployment automation, or CI workflow policy.

## Independent development model

`haze-gdrive-adapter` can be developed independently inside the `component/gdrive-adapter` branch.

The dependency map records provider adapter contracts and fan-in points. It does not impose a serial implementation order on Core, API, Server, Storage, Deployment, or CI.

Allowed independent work includes:

- adapter config/runtime skeleton;
- OAuth token loading boundary without committing credentials;
- Google Drive client abstraction;
- provider DTO normalization;
- mapping/cursor/echo-state design;
- full scan/change-feed/export planners;
- delete-candidate guardrails;
- dry-run/status/doctor foundations;
- provider-safe tests using mocks/fixtures.

If GDrive adapter needs Core/API/Server endpoints, Storage-backed mapping persistence, Deployment secret layout, or CI provider-test support not currently contracted, it reports a contract-change request or fan-in need instead of implementing another component's responsibility.

## Upstream contracts consumed

GDrive adapter may consume:

- Server/API HTTP contracts for submitting normalized changes and reading Core state;
- Core semantics through public Server/API results;
- Common shared identifiers/types where accepted;
- Deployment secret/config paths as operational configuration;
- Google Drive API as an external provider contract.

GDrive adapter must not consume:

- Core internals directly for policy decisions;
- Storage internals or direct DB writes;
- Server private handler state;
- Worktree local filesystem behavior;
- Obsidian plugin internals;
- CI workflow internals.

## Downstream contracts exposed

Expected downstream consumers:

- Server/API/Core indirectly, through normalized incoming changes and external replica status;
- Deployment, for service configuration, OAuth token placement, and runbook needs;
- GitHub CI/tests, for mockable provider boundaries where scoped;
- operators, through safe status/doctor output.

Downstream consumers must not treat Drive as source of truth over Core.

## Forbidden dependency directions

GDrive adapter must not:

- decide final conflict/delete outcomes outside Core;
- directly write database rows;
- call Obsidian plugin internals;
- use Worktree as hidden metadata source;
- expose OAuth tokens or provider payloads in public output;
- hard-delete Drive files without accepted Core/API policy and guardrails;
- require live Google credentials for ordinary unit tests.

## Cross-component contracts

Important GDrive adapter contracts:

- Drive state is an external replica, not the source of truth;
- Drive changes are normalized before Core submission;
- full scan provides correctness; change feed/webhooks provide latency;
- disappearance becomes delete candidate until guardrails confirm intent;
- provider writes require echo suppression to avoid loops;
- status/doctor output is safe and summarized.

## Integration/fan-in ownership

Fan-in is required when:

- Server/API exposes adapter-facing endpoints;
- Core apply/result semantics change;
- persistent mapping/cursor storage ownership is accepted;
- Deployment provisions OAuth secrets/service runtime;
- CI adds provider-mock validation;
- delete/export behavior needs end-to-end verification.

These are integration gates. They do not block independent GDrive adapter work inside its component boundary.

## Dependency rules

- GDrive adapter owns provider mechanics, not sync authority.
- GDrive adapter uses public Core/API/Server contracts, not direct DB or Core internals.
- Provider-specific details stay inside the adapter unless safely summarized.
- OAuth/token handling stays operationally isolated and never tracked as real credentials.
- Tests should default to mocks/fixtures, not live provider calls.

## Contract-change notes

Current known contract questions:

1. Adapter-facing API surface
   - GDrive may need endpoints for submitting changes, reading state, and reporting status.
   - Missing endpoints are API/Server/Core fan-in points.

2. Mapping/cursor persistence
   - Adapter needs durable mapping/cursor state.
   - Ownership may be adapter-local storage or Storage-backed contracts; decide explicitly.

3. Delete guardrails
   - Adapter can detect provider disappearance.
   - Core/API owns final delete/tombstone semantics.

No serial implementation dependency is implied by this map.
