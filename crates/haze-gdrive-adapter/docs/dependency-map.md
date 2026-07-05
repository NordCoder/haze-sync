# Dependency Map: gdrive-adapter

## Component role in dependency graph

`haze-gdrive-adapter` is the Google Drive external replica adapter.

Conceptual position:

```text
Google Drive folder subtree
  -> GDrive adapter scan/change-feed/import planner
  -> Haze Sync Server public API / Core outcomes
  -> GDrive adapter export planner
  -> Google Drive provider mutations
```

The adapter is an external replica client. It is not Core, Server, Storage, Worktree, or Obsidian plugin.

## Upstream dependencies

### Project contract dependencies

- `haze-sync-common`
  - conceptual path/hash/adapter/mode primitives;
  - validation vocabulary for normalized values.
- `haze-sync-api`
  - public HTTP DTOs;
  - headers for auth, content hash, base revision, idempotency;
  - public errors and route contracts.
- `haze-sync-core`
  - authoritative semantics observed through API outcomes:
    - base revision;
    - conflict-saved;
    - tombstone/delete;
    - idempotency;
    - operation changes.
- `haze-sync-server`
  - HTTP runtime endpoint for adapter requests;
  - status/admin behavior if consumed by adapter.
- `haze-sync-storage`
  - possible mapping/cursor persistence shape through `gdrive_mapping` and adapter cursor rows, but direct access is not assumed.

### Provider/platform dependencies

Future implementation may depend on:

- Google OAuth/token libraries or direct HTTP client support;
- Google Drive API client or REST client;
- async runtime/network libraries;
- retry/backoff/time libraries;
- serialization for provider-safe internal DTOs;
- test fake-provider utilities.

Any provider dependency must be introduced in a scoped implementation phase and reflected here.

### Current direct dependencies

Current code is a placeholder binary and has no direct dependencies beyond workspace/lints.

## Disallowed direct dependencies

GDrive adapter must not directly depend on:

```text
apps/haze-obsidian-plugin
haze-sync-worktree
Obsidian plugin internals
Worktree filesystem runtime
SQLx/direct database access unless explicitly accepted
Axum server route registration
CLI parser/command UX as adapter logic
```

Direct `haze-sync-storage` repository use is not assumed by default. If adapter direct DB access is chosen, it requires explicit architecture decision and documentation update.

## Downstream dependents

Expected downstream dependents:

- deployment/runbooks that launch the adapter process;
- Server/admin/status surfaces that summarize adapter state, if integration is scoped;
- E2E tests using fake or real provider setup;
- operational doctor checks.

No Core correctness should depend on GDrive adapter internals.

## Cross-component contracts

### API/Server ↔ GDrive adapter

- API owns route/header/DTO/error vocabulary.
- Server exposes the HTTP runtime and auth execution.
- GDrive adapter sends authenticated requests to configured Server URL.
- Adapter must preserve idempotency, content hash, and base/null-base semantics.
- Adapter must not rely on private Server internals.

### Core ↔ GDrive adapter

- Core owns sync policy.
- Adapter submits Drive facts and obeys outcomes.
- Adapter must not implement alternative conflict/delete/idempotency policy.

### Common ↔ GDrive adapter

- Common owns canonical Rust primitive validation.
- Adapter should reuse Common directly if it is a Rust client or mirror semantics through API validation.
- Path normalization drift can cause wrong-file updates and must be tested.

### Storage ↔ GDrive adapter

- Storage owns `gdrive_mapping` and cursor table shapes if those are used.
- Runtime persistence boundary is unresolved by default:
  - preferred clean boundary: adapter uses Server/API-mediated mapping/cursor operations if available;
  - direct Storage repository use requires explicit architecture decision.
- Adapter must not write DB tables opportunistically without contract.

### Worktree ↔ GDrive adapter

No direct dependency is allowed.

Both are replicas coordinated through Core/API, not through direct file/provider synchronization.

### Obsidian plugin ↔ GDrive adapter

No direct dependency is allowed.

Obsidian plugin must not talk to GDrive adapter directly for sync correctness. Coordination happens through Server/Core/API.

### Deployment ↔ GDrive adapter

Deployment owns service installation, secret placement, environment files, permissions, and process supervision.

GDrive adapter owns runtime behavior once launched with explicit config.

## Integration/fan-in ownership

The following work belongs outside GDrive-only leaf phases unless explicitly scoped:

- Storage schema/repository changes for `gdrive_mapping`;
- Server API additions for adapter mapping/cursor persistence;
- deployment service units or secret provisioning;
- Core policy changes;
- API DTO/header changes;
- end-to-end tests requiring live server and provider credentials.

## Dependency rules

- Consume Haze Sync through public Server/API unless direct internal integration is explicitly accepted.
- Do not call Obsidian or Worktree internals.
- Do not write database/storage directly by default.
- Do not decide conflict/delete policy locally.
- Keep OAuth tokens and provider payloads redacted.
- Treat Drive change feed as latency, full scan as correctness.
- Any dependency addition must be justified by a GDrive phase and reflected here.

## Contract-change notes

Current known contract questions:

1. Mapping/cursor persistence boundary
   - Storage has `gdrive_mapping`/cursor shapes.
   - Adapter needs persistent mapping/cursor state.
   - Whether access is Server/API-mediated or direct Storage repository use is unresolved.

2. Google Docs/Sheets/Slides support
   - V1 system docs exclude Docs/Sheets/Slides conversion.
   - Supporting those requires new product contract.

3. Shared drives and shortcuts
   - V1 should stay conservative.
   - Supporting shared drives/shortcuts requires mapping/path semantics review.

4. Delete behavior
   - Drive disappearance is delete candidate, not immediate tombstone.
   - Mass delete guard and manual unlock policy need Core/API/Server coordination.

5. Deployment/secret layout
   - OAuth token storage path and permissions belong to deployment/runbook decisions.
   - Adapter config must match those decisions.

No immediate blocking contract change is required for the current documentation/planning pass.
