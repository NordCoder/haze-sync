# Dependency Map: common

## Component role in dependency graph

`haze-sync-common` is the lowest-level shared Rust component.

It provides stable shared primitives, type foundations, lightweight helpers, constants, and cross-crate definitions that do not belong to a runtime authority component.

It must remain dependency-light and must not become a hidden implementation layer for Core, API, Storage, Server, adapters, CLI, Deployment, or CI.

## Independent development model

`haze-sync-common` can be developed independently inside the `component/common` branch.

The dependency map records contract boundaries and fan-in points. It does not impose a serial implementation order on other components.

Allowed independent work includes:

- shared identifiers and value types;
- safe parsing/formatting helpers;
- shared error categories when they do not expose component internals;
- feature-free utility code used by several Rust crates;
- test fixtures or helpers that remain generic and component-neutral.

If common work requires domain policy, storage behavior, API shape, server behavior, provider behavior, deployment behavior, or CI policy, it must report a contract-change request instead of absorbing that responsibility.

## Upstream contracts consumed

`haze-sync-common` should have no product-component upstream dependencies.

Allowed upstreams:

- Rust standard library;
- carefully justified third-party crates used only for generic types/helpers;
- workspace lint/tooling configuration.

Forbidden upstreams:

- `haze-sync-core`;
- `haze-sync-api`;
- `haze-sync-storage`;
- `haze-sync-server`;
- `haze-sync-worktree`;
- `haze-gdrive-adapter`;
- `haze-sync-cli`;
- Obsidian plugin code;
- Deployment or GitHub workflow behavior.

## Downstream contracts exposed

Expected downstream consumers:

- Core, for shared IDs/value types/error categories;
- API, for shared public-safe types where appropriate;
- Storage, for shared identifiers and value constraints;
- Server, for shared configuration/value helpers if accepted;
- Worktree, GDrive adapter, CLI, and tests where generic shared types are useful.

Downstream consumers must not rely on Common for runtime policy.

## Forbidden dependency directions

Common must not call or model:

- Core decision policy;
- Storage repository behavior;
- Server route behavior;
- HTTP request/response execution;
- provider APIs;
- filesystem watcher/runtime loops;
- OAuth/token handling;
- deployment layout;
- CI workflow policy.

## Cross-component contracts

Common contracts should be small, stable, and intentionally boring.

Breaking changes to Common types are fan-in points because many components may compile against those types.

If a component wants to add a type to Common, it must justify why the type is truly shared and not owned by a more specific component.

## Integration/fan-in ownership

Fan-in is required when:

- a shared type changes shape;
- a shared error category affects public API mapping;
- a shared identifier format changes;
- multiple components need the same helper and ownership is unclear;
- a helper risks embedding product policy in Common.

Fan-in does not block local Common development. It only marks integration or merge-readiness conditions.

## Dependency rules

- Keep Common free of runtime authority.
- Prefer narrower component ownership over adding broadly shared abstractions too early.
- Do not add provider-specific or deployment-specific concepts to Common unless multiple accepted contracts require them.
- Avoid feature flags that make Common behave differently for different components unless explicitly accepted.
- Shared public types must be safe to expose and must not encode internal/private state.

## Contract-change notes

Current known contract questions:

1. Shared ID/type ownership
   - Some identifiers may be shared by Core, API, and Storage.
   - They belong in Common only when they are truly cross-component and stable.

2. Shared error taxonomy
   - Common may define coarse public-safe error categories.
   - Component-specific error internals stay with owning components.

3. Test support
   - Generic test helpers may belong in Common.
   - Component-specific fixtures should stay with their component.

No serial implementation dependency is implied by this map.
