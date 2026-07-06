# Dependency Map: deployment

## Component role in dependency graph

`deploy` is the operations and runtime placement layer.

Deployment owns Docker Compose scaffolds, service placement documentation, host directory layout, runtime configuration examples, secret-file placement guidance, backup/restore/runbook planning, and production-readiness checklists.

Deployment does not own product runtime behavior, Core policy, API DTOs, Server route behavior, Storage schema contents, adapter/provider implementation, GitHub workflow policy, or client UI behavior.

## Independent development model

`deploy` can be developed independently inside the `component/deployment` branch.

The dependency map records operational contracts and fan-in points. It does not impose a serial implementation order on Server, Storage, GDrive adapter, Worktree, API, Core, CI, or clients.

Allowed independent work includes:

- local compose scaffolding;
- service/runbook documentation;
- host path and permission planning;
- backup/restore procedure documentation;
- configuration examples using placeholders;
- production-readiness checklists;
- operational docs for known runtime surfaces.

If Deployment needs service config, binary names, migrations, health endpoints, adapter token layout, or CI release artifacts not currently contracted, it reports a contract-change request or fan-in need instead of implementing another component's responsibility.

## Upstream contracts consumed

Deployment may consume:

- Server binary/config/health contracts;
- Storage migration/object-store requirements;
- Worktree host path requirements;
- GDrive adapter service/config/token-placement requirements;
- API/CLI status surfaces used in runbooks;
- GitHub CI artifact/check policy when release/deploy automation is explicitly scoped.

Deployment must not consume:

- raw product internals as operational contract;
- provider credentials as tracked files;
- CI workflow internals as deployment truth;
- component-private test fixtures;
- local user paths or machine-specific notes.

## Downstream contracts exposed

Expected downstream users/consumers:

- operators deploying Haze Sync;
- Server/GDrive adapter runtime processes through documented config layout;
- Storage migration/backup operators;
- Worktree host directory provisioning;
- CI/release checks when coordinated;
- docs and support workflows.

Product components should not depend on Deployment for runtime business semantics.

## Forbidden dependency directions

Deployment must not:

- change Core/API/Server product behavior as part of docs/process work;
- define Storage schema content;
- call provider APIs;
- own GitHub workflow policy unless explicitly coordinated with github-ci;
- commit real secrets or production env files;
- claim production readiness from syntax-only validation.

## Cross-component contracts

Important Deployment contracts:

- tracked examples use placeholders only;
- operational paths are documented and configurable;
- migration execution policy is explicit;
- backup/restore runbooks do not imply unimplemented tooling exists;
- compose syntax validation is not service startup or production readiness;
- deployment docs reflect current runtime capabilities honestly.

## Integration/fan-in ownership

Fan-in is required when:

- Server exposes new config/health/runtime behavior;
- Storage migrations or backup/restore requirements change;
- Worktree object paths/permissions are accepted;
- GDrive adapter service/token layout is implemented;
- CI/release workflows start producing deployment artifacts;
- system docs index operational runbooks.

These are integration gates. They do not block independent Deployment work inside its component boundary.

## Dependency rules

- Deployment owns operational packaging/runbooks, not product semantics.
- Deployment should document current behavior and mark future behavior explicitly.
- Deployment changes to CI workflows require github-ci coordination.
- Deployment examples must remain safe and placeholder-based.
- Production-readiness claims require explicit evidence beyond compose syntax.

## Contract-change notes

Current known contract questions:

1. Service set and topology
   - Current compose is local scaffold-oriented.
   - Adding server/gdrive/worktree production services requires component contracts.

2. Migration execution
   - Storage owns migration content.
   - Deployment owns when/how operators run it.

3. Secret layout
   - Deployment can document paths/permissions.
   - Adapter/server components own how secrets are loaded and validated.

No serial implementation dependency is implied by this map.
