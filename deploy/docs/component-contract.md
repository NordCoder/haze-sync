# Component Contract: deployment

## Responsibility

`deploy` owns deployment and operations scaffolding for running Haze Sync outside tests.

The deployment component is responsible for deployable topology, local development service scaffolds, production-style runbooks, secret placement documentation, directory layout, reverse proxy/TLS guidance, backup/restore procedure, and operational safety checks.

Current implemented deployment surface is intentionally limited but no longer PostgreSQL-only:

- `deploy/docker-compose.yml` provides local PostgreSQL and `haze-sync-server` services;
- `deploy/server.Dockerfile` provides non-root Server container packaging;
- `.env.example` provides local placeholder configuration and Compose host-port keys;
- `deploy/docs/local-compose.md` and `deploy/docs/server-compose.md` document local startup, health/readiness, and local-only boundaries;
- `deploy/docs/migrations-backup-restore.md` documents explicit manual migration, backup, and restore sequencing;
- `deploy/docs/host-directory-layout.md` documents production-style path, ownership, permission, and backup boundaries;
- no GDrive adapter service, Worktree runtime bind mount, reverse proxy/TLS, production secret files, cleanup automation, or remote deployment automation is implemented yet.

Deployment owns runtime packaging and operations documents. It does not own product behavior.

## Public interfaces

Current public deployment files include:

```text
deploy/docker-compose.yml
deploy/server.Dockerfile
deploy/server.Dockerfile.dockerignore
.env.example
deploy/docs/local-compose.md
deploy/docs/server-compose.md
deploy/docs/migrations-backup-restore.md
deploy/docs/host-directory-layout.md
README.md deployment/configuration snippets
```

Future public deployment surfaces may include:

```text
systemd unit files
reverse-proxy examples
backup/restore scripts only when explicitly accepted
secret placement integrations
local smoke-test scripts
production checklist
release/deployment artifacts when coordinated with github-ci
```

Generated secrets, production `.env` files, OAuth token files, local vault data, object-store data, database volumes, logs, dumps, backups, and runtime temp data must not be committed.

## Input contracts

Deployment inputs include:

- explicit environment variables;
- local `.env` files kept outside commits;
- deployment host paths;
- built binaries/images;
- PostgreSQL instance/volume;
- object-store directory;
- worktree directory;
- GDrive OAuth token file path;
- reverse proxy/TLS configuration;
- operator commands from runbooks.

Required input rules:

- production secrets must come from host secret files, environment, or a future accepted secret manager;
- placeholder values in tracked examples must not be usable production credentials;
- host paths must be explicit and documented;
- persistent data, config, secrets, logs, backups, and runtime temp paths must remain distinguishable;
- migration execution policy must be explicit before production startup uses it;
- service dependencies and health checks must be documented;
- deployment commands must distinguish local development from production-style guidance.

## Output contracts

Deployment outputs include:

- checked-in examples and runbooks;
- safe local compose scaffolds;
- service definitions and packaging artifacts accepted inside Deployment scope;
- smoke-test commands;
- host path and permission guidance;
- backup/restore artifacts created locally by operators but never committed.

Required output rules:

- tracked deployment outputs must not contain real secrets;
- runbooks must not instruct operators to paste tokens into public logs/reports;
- status/smoke output examples must be sanitized;
- backup/restore examples must preserve object-store/database consistency expectations;
- path examples must keep persistent data, secrets, logs, backups, and temp state separated;
- local-only scaffolds must be labeled local-only.

## Error contracts

Deployment docs and scripts must keep failure output safe.

They must not expose:

- production database URLs;
- database passwords;
- bearer tokens;
- OAuth refresh/access tokens;
- token hashes;
- Idempotency-Key values;
- local absolute secret paths in public examples;
- raw provider payloads;
- raw vault contents;
- stack traces in public reports.

Local operator-only commands may show generic host paths when necessary, but public docs/reports should prefer placeholders such as `/srv/haze-sync/...` and must not include real user secrets or machine-specific home paths.

## Persistence/runtime ownership

Deployment owns runtime placement and operational lifecycle documentation.

Deployment may own:

- compose/systemd/reverse-proxy examples;
- service dependency ordering;
- host directory layout;
- file permission guidance;
- backup/restore procedure;
- migration execution runbook;
- local development startup procedure;
- production checklist.

Deployment does not own:

- Core sync policy;
- API DTOs or server routes;
- Storage schema design;
- Server runtime internals;
- GDrive provider behavior;
- Worktree scanner/materializer logic;
- Obsidian plugin behavior;
- CI policy beyond deployment-specific checks when coordinated with github-ci;
- secret generation/rotation semantics unless a security/admin contract accepts them.

## Security and secrecy rules

- Do not commit `.env`, production secret files, OAuth token files, database dumps, object-store data, worktree data, logs, backup archives, runtime temp data, or provider payload snapshots.
- `.env.example` values must be empty or clearly local placeholders.
- Deployment docs must use placeholder secrets only.
- OAuth token files must be documented as host-local, permission-restricted files.
- Database, object-store, and worktree backup paths must be local/operator-owned and untracked.
- Persistent data paths must not be nested inside the repository checkout.
- Backup paths must not be nested inside object-store, worktree, log, or temp paths.
- Production reverse proxy/TLS examples must not include private keys.
- Deployment automation must not run destructive cleanup by default.
- Smoke checks must not require production secrets or real provider credentials unless explicitly marked production-only.

## Non-goals

Deployment must not implement:

- product sync behavior;
- server route handlers;
- Core policy;
- Storage repositories/migrations schema design;
- provider adapter logic;
- Worktree scanner/materializer behavior;
- Obsidian plugin code;
- CI workflow ownership except deployment validation hooks coordinated with github-ci;
- automatic production rollout/remote SSH actions through the repo;
- hard-delete cleanup without explicit retention contract.

## Dependencies

See `dependency-map.md`.

## Dependents

See `dependency-map.md`.

## Invariants

- Tracked deployment files never contain real secrets.
- Local development scaffolds are not production readiness proof.
- Production startup/migration/backup/restore behavior must be explicit.
- Database and object-store consistency must be preserved in backup/restore guidance.
- Persistent data, config, secrets, logs, backups, and runtime temp paths remain separated.
- Server, GDrive adapter, Worktree, and Obsidian behavior remain owned by their components.
- Deployment must distinguish local compose validation from service startup and production sync.

## Test obligations

Deployment tests/checks should eventually cover:

- `docker compose -f deploy/docker-compose.yml config`;
- absence of production secrets in tracked examples;
- documented environment variables match Server/GDrive/Obsidian expected config;
- service healthcheck syntax;
- backup/restore dry-run procedure where practical;
- reverse proxy config syntax if examples are added;
- local smoke-test runbook commands;
- permissions checklist for secret/object/worktree paths;
- validation that approved persistent paths are not world-writable or nested inside tracked/runtime-temp paths.

Current expected executable check:

```bash
docker compose -f deploy/docker-compose.yml config
```

Read-only host permission checks are documented in `deploy/docs/host-directory-layout.md` but are not proof of production readiness by themselves.

## Contract change protocol

Request a contract change instead of silently broadening scope when implementation requires:

- committing real secrets or production `.env` files;
- changing server/config variable names without Server coordination;
- changing storage paths or migration policy without Storage/Server coordination;
- enabling Worktree runtime or choosing its authority/write model without Worktree coordination;
- adding production deployment automation that mutates remote hosts;
- adding hard-delete/cleanup operations;
- adding provider credentials or OAuth token contents;
- making local-only compose behavior appear production-ready;
- editing CI workflow policy outside coordinated github-ci scope.
