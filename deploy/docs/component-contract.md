# Component Contract: deployment

## Responsibility

`deploy` owns deployment and operations scaffolding for running Haze Sync outside tests.

The deployment component is responsible for future deployable topology, local development service scaffolds, production runbooks, secret placement documentation, directory layout, reverse proxy/TLS guidance, backup/restore procedure, and operational safety checks.

Current implemented deployment surface is intentionally minimal:

- `deploy/docker-compose.yml` provides local PostgreSQL only;
- `.env.example` provides local placeholder configuration keys;
- root `README.md` documents compose syntax validation with `docker compose -f deploy/docker-compose.yml config`;
- no server container/service, no GDrive adapter service, no reverse proxy, no production secret files, and no deployment automation are implemented yet.

Deployment owns runtime packaging and operations documents. It does not own product behavior.

## Public interfaces

Current public files:

```text
deploy/docker-compose.yml
.env.example
README.md deployment/configuration snippets
```

Future public deployment surfaces may include:

```text
deploy/docker-compose.yml
Dockerfile or component-specific Dockerfiles
systemd unit files
reverse-proxy examples
.env.example updates
ops runbooks
backup/restore scripts
migration run procedure
secret placement guide
local smoke-test scripts
production checklist
```

Generated secrets, production `.env` files, OAuth token files, local vault data, object-store data, database volumes, logs, dumps, and backups must not be committed.

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
- migration execution policy must be explicit before production startup uses it;
- service dependencies and health checks must be documented;
- deployment commands must distinguish local development from production.

## Output contracts

Deployment outputs include:

- checked-in examples and runbooks;
- safe local compose scaffolds;
- future service definitions;
- future smoke-test commands;
- future backup/restore artifacts created locally but not committed.

Required output rules:

- tracked deployment outputs must not contain real secrets;
- runbooks must not instruct operators to paste tokens into public logs/reports;
- status/smoke output examples must be sanitized;
- backup/restore examples must preserve object-store/database consistency expectations;
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

Local operator-only commands may show host paths when necessary, but public docs/reports should prefer placeholders such as `/srv/haze-sync/...` and must not include real user secrets.

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

- Do not commit `.env`, production secret files, OAuth token files, database dumps, object-store data, worktree data, logs, backup archives, or provider payload snapshots.
- `.env.example` values must be empty or clearly local placeholders.
- Deployment docs must use placeholder secrets only.
- OAuth token files must be documented as host-local, permission-restricted files.
- Database and object-store backup paths must be local/operator-owned and ignored.
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
- permissions checklist for secret/object/worktree paths.

Current expected check:

```bash
docker compose -f deploy/docker-compose.yml config
```

This validates compose syntax only and does not prove production readiness.

## Contract change protocol

Request a contract change instead of silently broadening scope when implementation requires:

- committing real secrets or production `.env` files;
- changing server/config variable names without Server coordination;
- changing storage paths or migration policy without Storage/Server coordination;
- adding production deployment automation that mutates remote hosts;
- adding hard-delete/cleanup operations;
- adding provider credentials or OAuth token contents;
- making local-only compose behavior appear production-ready;
- editing CI workflow policy outside coordinated github-ci scope.
