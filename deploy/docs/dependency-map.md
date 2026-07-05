# Dependency Map: deployment

## Component role in dependency graph

`deploy` is the operations and runtime placement layer.

Conceptual position:

```text
built artifacts + configuration + host paths + secrets
  -> deployment scaffolds/runbooks
  -> running Server / PostgreSQL / object store / Worktree / GDrive adapter
  -> operators and smoke checks
```

Deployment composes already-defined components into local or production-like runtime topology. It does not define product semantics.

## Upstream dependencies

### `haze-sync-server`

Deployment depends on Server for:

- binary/container startup behavior;
- environment/config variable names;
- listen address and readiness/health endpoints;
- database connection configuration;
- object-store path configuration;
- migration policy if Server owns migration execution;
- Worktree hosting behavior if Server hosts Worktree runtime.

### `haze-sync-storage`

Deployment depends on Storage for:

- PostgreSQL schema and migration files;
- database versioning expectations;
- backup/restore consistency expectations;
- object-store persistence layout expectations.

### `haze-gdrive-adapter`

Deployment depends on GDrive adapter for:

- adapter binary/container startup behavior;
- OAuth token file path expectations;
- server URL/auth configuration;
- adapter mode variables;
- scan/poll/backoff behavior;
- status/doctor/smoke behavior when implemented.

### `haze-sync-worktree`

Deployment depends on Worktree for:

- configured worktree root expectations;
- filesystem permissions;
- runtime hosting boundary;
- trash/temp/runtime directory expectations when implemented.

### `apps/haze-obsidian-plugin`

Deployment depends on Obsidian plugin docs for:

- server URL setup;
- adapter identity/token setup;
- client rollout caveats.

### `github-ci`

Deployment depends on CI only for validation hooks such as compose syntax checks. CI owns workflow policy; Deployment owns deploy artifacts/runbooks.

## Current direct runtime surfaces

Current tracked deployment surface:

```text
deploy/docker-compose.yml
.env.example
```

Current compose surface provides local PostgreSQL only.

## Disallowed dependency directions

Deployment must not directly own or implement:

```text
Core sync policy
API DTO/header/error vocabulary
Storage schema semantics
Server route behavior
GDrive provider logic
Worktree scanner/materializer behavior
Obsidian plugin behavior
GitHub Actions workflow policy except coordinated validation hooks
```

Deployment files must not import, embed, or vendor production secrets.

## Downstream dependents

Expected dependents:

- local developers;
- operators;
- runbooks;
- production VPS setup;
- smoke tests;
- incident response procedures;
- backup/restore workflows.

Downstream dependents rely on Deployment to distinguish local scaffolding from production deployment.

## Cross-component contracts

### Server ↔ Deployment

- Server owns runtime behavior and config parsing.
- Deployment supplies environment variables, service definitions, host paths, healthcheck wiring, and startup order.
- Deployment must not invent config keys without Server coordination.

### Storage ↔ Deployment

- Storage owns migrations/schema/object-store behavior.
- Deployment owns how PostgreSQL volumes, backup, restore, and object-store directories are provisioned.
- Backup/restore runbooks must treat DB and object store as consistency-linked.

### GDrive adapter ↔ Deployment

- GDrive adapter owns provider runtime behavior.
- Deployment owns process/service configuration, OAuth token file placement, permissions, and restart policy.
- Deployment must not commit OAuth tokens or provider payloads.

### Worktree ↔ Deployment

- Worktree owns filesystem adapter logic.
- Deployment owns host path provisioning and permissions for the worktree root.
- Deployment must not implement Worktree scanner/materializer behavior.

### Obsidian plugin ↔ Deployment

- Plugin owns client behavior.
- Deployment/runbooks may document server URL and token setup expectations.
- Deployment must not manage local user vault contents.

### CI ↔ Deployment

- CI may validate compose syntax and secret-free examples.
- Deployment owns deploy artifacts and runbooks.
- CI must not deploy production services or require production credentials unless a future explicit release/deploy workflow is accepted.

## Integration/fan-in ownership

The following work requires cross-component coordination:

- adding Server service to compose/systemd;
- adding GDrive adapter service;
- deciding migration execution policy;
- defining backup/restore consistency procedure;
- wiring Worktree runtime into Server deployment;
- adding reverse proxy/TLS examples;
- adding deployment validation to CI;
- documenting bootstrap/rollback across adapters.

## Dependency rules

- Do not commit real secrets or production `.env` files.
- Do not add production deployment automation without explicit scope.
- Do not make local scaffolds appear production-ready.
- Keep config keys aligned with owning components.
- Keep provider credentials outside the repo.
- Keep backup/log/dump artifacts outside the repo.
- Any deploy artifact addition must update this dependency map when it changes component boundaries.

## Contract-change notes

Current known contract questions:

1. Server production startup
   - Server currently has scaffold startup behavior and route/runtime wiring.
   - Deployment cannot finalize server service until Server production listener/config/migration policy is accepted.

2. Migration execution owner
   - Could be Server startup, CLI/manual command, or deployment script.
   - This requires Server/Storage/Deployment decision before production use.

3. GDrive token placement
   - Expected to be host-local secret file with restrictive permissions.
   - Exact path and service user need deployment decision.

4. Worktree runtime hosting
   - V1 may host Worktree in Server.
   - Deployment should not assume service topology until Server/Worktree fan-in is accepted.

5. Reverse proxy and TLS
   - Public access boundary must be documented before production exposure.
   - TLS/private keys must never be committed.

No immediate blocking contract change is required for the current documentation/planning pass.
