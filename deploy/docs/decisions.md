# Decisions: deployment

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

## 2026-07-05 — Current compose is local PostgreSQL scaffold only

Decision:

`deploy/docker-compose.yml` is currently a local development PostgreSQL scaffold. It is not production deployment and does not start Haze Sync Server, GDrive adapter, Worktree runtime, or any sync service.

Rationale:

The current compose file only provisions PostgreSQL with local bind address and placeholder defaults. Treating it as production-ready would hide missing services, secrets, migrations, object-store, worktree, backup, and reverse proxy setup.

Alternatives:

- Present current compose as production deployment.
- Add all services before Server/GDrive/Worktree contracts are ready.
- Remove local compose until production deployment exists.

Consequences:

- Documentation must label compose validation as syntax/local dependency validation only.
- Future Server/GDrive service wiring needs dedicated phases.
- Local placeholder passwords must not become production values.

Affected contracts:

- docker-compose scaffold;
- README deployment validation command;
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
