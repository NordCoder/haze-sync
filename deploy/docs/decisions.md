# Decisions: deployment

## 2026-07-10 — DEP-P6 uses Caddy as the placeholder public TLS boundary

Decision:

Deployment provides one tracked, secret-free Caddy example at `deploy/reverse-proxy/Caddyfile`. Public HTTPS terminates at Caddy, which proxies to Haze Sync Server over `127.0.0.1:8080`. The tracked public listener blocks `/health` and `/ready`, preserves Server-owned authentication for protected V1 routes, and enforces the accepted `52,428,800`-byte upload limit.

Rationale:

A single documented proxy model is easier to validate and less ambiguous than parallel nginx/Caddy/Traefik examples. Caddy supports automatic HTTPS without tracked certificate paths, keeps the upstream loopback-only, and provides a configuration validation command. Server remains the application authentication authority rather than duplicating bearer-token policy in deployment configuration.

Alternatives:

- Track several proxy examples with drifting security behavior.
- Expose Server directly on a public interface.
- Inject a bearer credential at the proxy.
- Expose readiness publicly by default.
- Commit certificate/private-key paths or material.
- Change the proxy body-size limit independently of Server/API.

Consequences:

- Caddy 2.10 or newer is required because the example uses `request_body`.
- Operators must replace `sync.example.com`, configure DNS, and validate the Caddyfile before rollout.
- Public inbound access is limited to TCP 443 and optional TCP 80; Server, PostgreSQL, and Caddy admin ports remain non-public.
- `/v1/server-info` stays publicly proxied; protected V1 routes continue to require Server bearer authentication and role checks.
- Public health endpoints return `404` in the tracked example; Caddy checks upstream `/health` over loopback.
- Certificate material remains in Caddy-managed host-local protected storage and outside the repository.
- Syntax/config validation is not production readiness evidence.

Affected contracts:

- deploy/reverse-proxy/Caddyfile;
- deploy/docs/public-access.md;
- Server route/auth/health contracts;
- API upload-limit contract;
- deployment component contract and Server compose runbook.

## 2026-07-09 — DEP-P5 separates host path classes

Decision:

Production-style Deployment guidance keeps persistent application data, user-visible worktree data, non-secret configuration, secrets, logs, backups, and runtime temp state in separate roots. The documented placeholder layout is:

```text
/srv/haze-sync/objects
/srv/haze-vault/worktree
/etc/haze-sync
/opt/haze-sync/secrets
/var/log/haze-sync
/var/backups/haze-sync
/run/haze-sync
/var/tmp/haze-sync
```

The paths are examples, not automatically created resources. `HAZE_SYNC_OBJECT_STORE_PATH` and `HAZE_SYNC_WORKTREE_PATH` remain the accepted Server config keys.

Rationale:

Separating path classes reduces accidental recursive backups, self-ingestion, secret disclosure, unsafe cleanup, and permission broadening. It also lets operators give the Server, vault users, and backup operators only the access each role requires.

Alternatives:

- Store runtime data under the repository checkout.
- Put object store, worktree, secrets, logs, and backups under one writable root.
- Make persistent directories world-writable to avoid UID/GID coordination.
- Add host bind mounts before Worktree and container identity contracts are accepted.

Consequences:

- Host deployment guidance must preserve path separation and least privilege.
- Future container bind mounts must account for the Server image UID `10001` or explicitly coordinate another runtime identity.
- The worktree write model remains deferred until the Worktree contract accepts it.
- Backups must be operator-owned and outside service data roots.
- Real secrets and runtime artifacts remain untracked.

Affected contracts:

- deploy/docs/host-directory-layout.md;
- Server object-store and Worktree path config;
- migration/backup/restore procedure;
- future Worktree runtime and secret placement phases;
- `.env.example` local placeholder guidance.

## 2026-07-09 — DEP-P4 uses operator-run manual SQLx migrations

Decision:

Until a later accepted entry point or deploy script exists, migrations are operator-owned and run manually with SQLx from the repository `migrations` directory after pre-migration backups and while writers are stopped or quiesced.

Rationale:

The accepted server binary deliberately does not auto-run migrations, and Deployment must not invent an automatic migration runner. A manual SQLx command keeps execution explicit while preserving Storage ownership of migration contents and Server ownership of runtime internals.

Alternatives:

- Add automatic migration execution to compose or the server image.
- Add a deployment script before script behavior is accepted.
- Treat server startup as migration success evidence.

Consequences:

- Operators must run the documented migration command explicitly.
- Backup and object-store consistency checks must happen before migration.
- Deployment can later replace this procedure only through an accepted CLI, Server entry point, or deploy-script phase.

Affected contracts:

- deploy/docs/migrations-backup-restore.md;
- Server startup policy;
- Storage migration ownership;
- backup/restore runbooks.

## 2026-07-05 — Deployment owns operations scaffolding, not product behavior

Decision:

`deploy` owns deployment artifacts, runbooks, service topology examples, secret placement guidance, and operational checks. It does not own Core policy, Server route behavior, Storage schema, adapter provider behavior, Worktree logic, Obsidian plugin behavior, or CI workflow policy.

Rationale:

Deployment composes already-defined components into runtime environments. If deployment files start defining product behavior, they become an unsafe parallel implementation path.

Alternatives:

- Put runtime product behavior into deploy scripts.
- Let deployment scripts mutate database/provider state directly.
- Treat deployment as owner of Server/GDrive config semantics.

Consequences:

- Deploy artifacts must follow component contracts.
- Cross-component changes require fan-in coordination.
- Runbooks should call accepted binaries/APIs rather than implement behavior directly.

Affected contracts:

- component contract;
- dependency map;
- Server/Storage/GDrive/Worktree integration phases;
- github-ci validation boundaries.

## 2026-07-05 — Current compose is a local PostgreSQL and Server scaffold

Decision:

`deploy/docker-compose.yml` is a local development scaffold that starts PostgreSQL and `haze-sync-server`. It is not production deployment and does not start the GDrive adapter, Worktree runtime, Obsidian runtime, reverse proxy/TLS, migration runner, or backup jobs.

Rationale:

Local Compose now supports Server smoke workflows but still uses placeholder configuration, local-only host binds, Docker named volumes, disabled adapters, and no production access or secret flow. Treating it as production-ready would hide missing migrations, host path provisioning, provider services, Worktree authority, backups, and reverse proxy/TLS setup.

Alternatives:

- Present current compose as production deployment.
- Add all services before Server/GDrive/Worktree contracts are ready.
- Remove local compose until production deployment exists.

Consequences:

- Documentation must label compose validation and startup as local/prod-like smoke evidence only.
- Future GDrive/Worktree/public-access wiring needs dedicated phases.
- Local placeholder passwords must not become production values.
- Production-style host paths remain documented but unwired until later accepted phases.

Affected contracts:

- docker-compose scaffold;
- Server packaging;
- local/server compose runbooks;
- deployment plan;
- CI compose validation.

## 2026-07-05 — Tracked deployment files never contain real secrets

Decision:

Deployment examples may include placeholder variable names and safe dummy values, but real `.env` files, database URLs, OAuth tokens, bearer tokens, TLS private keys, provider payloads, backups, dumps, logs, vault data, and object-store data must stay out of the repository.

Rationale:

Deployment is the area most likely to accumulate real operational data. Repository history is not a safe secret store.

Alternatives:

- Commit local `.env` examples with realistic secrets.
- Commit OAuth token JSON for convenience.
- Commit sample production configs with private keys removed later.

Consequences:

- `.env.example` must remain placeholder-only.
- Secret placement must be documented as host-local or secret-manager-backed.
- CI/deployment checks should not require production credentials.

Affected contracts:

- `.env.example`;
- deployment docs;
- GDrive secret layout;
- Server config docs;
- CI secret policy.

## 2026-07-05 — Migration execution policy must be explicit

Decision:

Deployment must not silently decide whether migrations run automatically at server startup, through CLI/manual commands, or through deploy scripts. Migration execution owner must be accepted before production service wiring relies on it.

Rationale:

Migrations can break startup or corrupt state if run unexpectedly. Operators need a clear pre-deploy backup and migration process.

Alternatives:

- Always auto-run migrations on server startup.
- Never document migration behavior.
- Let compose run migrations implicitly through ad hoc commands.

Consequences:

- Server packaging can be local-only until migration policy is accepted.
- Backup/restore runbooks must coordinate with migration procedure.
- Production readiness checklist must include migration state.

Affected contracts:

- Server startup;
- Storage migrations;
- deployment runbooks;
- backup/restore procedure.

## 2026-07-05 — Host paths and permissions are deployment-owned

Decision:

Deployment owns documentation for host path layout and permissions for object store, worktree, secrets, logs, and backups, while the semantics of those directories remain owned by Server/Storage/Worktree/GDrive.

Rationale:

Operators need a concrete filesystem layout, but deployment should not implement object-store, worktree, or provider behavior itself.

Alternatives:

- Let each component invent host paths independently.
- Put object-store/worktree semantics into deployment scripts.
- Use tracked repository directories for runtime data.

Consequences:

- Path examples must be placeholders and secret-safe.
- Runtime data directories must be ignored/untracked.
- Component config keys must align with path layout.

Affected contracts:

- Server object-store config;
- Worktree root config;
- GDrive secret path;
- backup/restore docs;
- `.env.example`.

## 2026-07-05 — Deployment automation is not CI/CD by default

Decision:

Deployment validation may be checked by CI, but CI must not deploy production services, mutate remote hosts, or require production credentials unless a future explicit release/deploy workflow is accepted.

Rationale:

The current project uses GitHub connector and branch-based implementation controls. Production deployment automation has higher operational risk and requires separate authorization.

Alternatives:

- Add automatic deployment from main.
- Store production SSH/secrets in CI immediately.
- Treat compose validation as deployment.

Consequences:

- `github-ci` remains separate owner of workflow policy.
- Deployment can define artifacts/runbooks and validation hooks.
- Production rollout remains manual/documented until explicitly accepted.

Affected contracts:

- github-ci component;
- deployment docs;
- release/runbook process;
- secret policy.
