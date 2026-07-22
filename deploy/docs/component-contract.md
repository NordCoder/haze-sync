# Component Contract: deployment

## Responsibility

`deploy` owns deployment and operations scaffolding for running Haze Sync outside tests.

The deployment component is responsible for deployable topology, local development service scaffolds, production-style runbooks, secret placement documentation, directory layout, reverse proxy/TLS guidance, backup/restore procedure, and operational safety checks.

Current implemented deployment surface is intentionally limited:

- `deploy/docker-compose.yml` provides local PostgreSQL and `haze-sync-server` services;
- `deploy/server.Dockerfile` provides non-root Server container packaging;
- `.env.example` provides local placeholder configuration and Compose host-port keys;
- `deploy/docs/local-compose.md` and `deploy/docs/server-compose.md` document local startup, health/readiness, and local-only boundaries;
- `deploy/docs/migrations-backup-restore.md` documents explicit manual migration, backup, and restore sequencing;
- `deploy/docs/host-directory-layout.md` documents production-style path, ownership, permission, and backup boundaries;
- `deploy/reverse-proxy/Caddyfile` and `deploy/docs/public-access.md` provide a placeholder-only Caddy/TLS/public-access example;
- no running reverse-proxy service, production certificate material, GDrive adapter service, Worktree runtime bind mount, cleanup automation, or remote deployment automation is implemented yet.

Deployment owns runtime packaging and operations documents. It does not own product behavior, API authentication semantics, or Server route policy.

## Public interfaces

Current public deployment files include:

```text
deploy/docker-compose.yml
deploy/server.Dockerfile
deploy/server.Dockerfile.dockerignore
.env.example
deploy/reverse-proxy/Caddyfile
deploy/docs/local-compose.md
deploy/docs/server-compose.md
deploy/docs/migrations-backup-restore.md
deploy/docs/host-directory-layout.md
deploy/docs/public-access.md
README.md deployment/configuration snippets
```

Future public deployment surfaces may include:

```text
systemd unit files
running proxy service wiring
backup/restore scripts only when explicitly accepted
secret placement integrations
local smoke-test scripts
production checklist
release/deployment artifacts when coordinated with github-ci
```

Generated secrets, production `.env` files, OAuth token files, certificate private keys, local vault data, object-store data, database volumes, logs, dumps, backups, and runtime temp data must not be committed.

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
- accepted Server/API route, authentication, health, and upload-limit contracts;
- operator commands from runbooks.

Required input rules:

- production secrets must come from host secret files, environment, Caddy-managed protected storage, or a future accepted secret manager;
- placeholder values in tracked examples must not be usable production credentials;
- host paths must be explicit and documented;
- persistent data, config, secrets, logs, backups, and runtime temp paths must remain distinguishable;
- migration execution policy must be explicit before production startup uses it;
- service dependencies and health checks must be documented;
- public proxy and firewall boundaries must keep Server/PostgreSQL off untrusted interfaces;
- proxy body-size limits must remain aligned with accepted Server/API limits;
- deployment commands must distinguish local development from production-style guidance.

## Output contracts

Deployment outputs include:

- checked-in examples and runbooks;
- safe local compose scaffolds;
- service definitions and packaging artifacts accepted inside Deployment scope;
- placeholder-only reverse-proxy configurations;
- smoke-test commands;
- host path and permission guidance;
- backup/restore artifacts created locally by operators but never committed.

Required output rules:

- tracked deployment outputs must not contain real secrets or certificate private keys;
- runbooks must not instruct operators to paste tokens into public logs/reports;
- status/smoke output examples must be sanitized;
- backup/restore examples must preserve object-store/database consistency expectations;
- path examples must keep persistent data, secrets, logs, backups, and temp state separated;
- reverse-proxy examples must preserve Server authentication and accepted body-size limits;
- local-only scaffolds must be labeled local-only;
- tracked proxy examples must be syntax-checkable with documented tooling/version assumptions.

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
- request bodies or uploaded file bytes;
- TLS private keys;
- stack traces in public reports.

Local operator-only commands may show generic host paths and public placeholder hostnames when necessary, but public docs/reports should not include real user secrets or machine-specific home paths.

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
- public port/firewall topology guidance;
- production checklist.

Deployment does not own:

- Core sync policy;
- API DTOs, authorization vocabulary, or upload contract values;
- Server route registration, authentication execution, or runtime internals;
- Storage schema design;
- GDrive provider behavior;
- Worktree scanner/materializer logic;
- Obsidian plugin behavior;
- CI policy beyond deployment-specific checks when coordinated with github-ci;
- secret/certificate generation or rotation semantics unless a security/admin contract accepts them.

## Security and secrecy rules

- Do not commit `.env`, production secret files, OAuth token files, certificate private keys, database dumps, object-store data, worktree data, logs, backup archives, runtime temp data, or provider payload snapshots.
- `.env.example` values must be empty or clearly local placeholders.
- Deployment docs must use placeholder secrets and hostnames only.
- OAuth token files must be documented as host-local, permission-restricted files.
- Database, object-store, and worktree backup paths must be local/operator-owned and untracked.
- Persistent data paths must not be nested inside the repository checkout.
- Backup paths must not be nested inside object-store, worktree, log, or temp paths.
- Production reverse-proxy examples must not include certificates or private keys.
- Public traffic must terminate at the intended proxy; Server and PostgreSQL must not be publicly reachable for the documented topology.
- The proxy must not inject bearer credentials or bypass Server authorization.
- Public health exposure must be explicit; readiness should remain private by default.
- Deployment automation must not run destructive cleanup by default.
- Smoke checks must not require production secrets or real provider credentials unless explicitly marked production-only.

## Non-goals

Deployment must not implement:

- product sync behavior;
- server route handlers or authentication semantics;
- Core policy;
- Storage repositories/migrations schema design;
- provider adapter logic;
- Worktree scanner/materializer behavior;
- Obsidian plugin code;
- CI workflow ownership except deployment validation hooks coordinated with github-ci;
- automatic production rollout/remote SSH actions through the repo;
- DNS/cloud-provider automation;
- hard-delete cleanup without explicit retention contract.

## Dependencies

See `dependency-map.md`.

## Dependents

See `dependency-map.md`.

## Invariants

- Tracked deployment files never contain real secrets or certificate private keys.
- Local development scaffolds and syntax validation are not production readiness proof.
- Production startup/migration/backup/restore behavior must be explicit.
- Database and object-store consistency must be preserved in backup/restore guidance.
- Persistent data, config, secrets, logs, backups, and runtime temp paths remain separated.
- Public access terminates at a reverse proxy while Server/PostgreSQL stay off untrusted interfaces.
- Server remains the authentication/authorization authority for protected API routes.
- Proxy body-size limits remain coordinated with the accepted Server/API upload contract.
- Server, GDrive adapter, Worktree, and Obsidian behavior remain owned by their components.
- Deployment must distinguish local compose validation from service startup and production sync.

## Test obligations

Deployment tests/checks should eventually cover:

- `docker compose -f deploy/docker-compose.yml config`;
- absence of production secrets in tracked examples;
- documented environment variables match Server/GDrive/Obsidian expected config;
- service healthcheck syntax;
- backup/restore dry-run procedure where practical;
- Caddyfile formatting and configuration validation;
- proxy upload-limit alignment with Server/API contract;
- local smoke-test runbook commands;
- permissions checklist for secret/object/worktree paths;
- validation that approved persistent paths are not world-writable or nested inside tracked/runtime-temp paths.

Current expected executable checks when the required tools are available:

```bash
docker compose -f deploy/docker-compose.yml config
caddy fmt --diff deploy/reverse-proxy/Caddyfile
caddy validate --config deploy/reverse-proxy/Caddyfile --adapter caddyfile
```

The tracked Caddyfile requires Caddy 2.10 or newer because it uses `request_body`. Syntax/config validation does not prove DNS, certificate issuance, firewall correctness, authentication, upload success, or production readiness.

Read-only host permission checks are documented in `deploy/docs/host-directory-layout.md` but are not proof of production readiness by themselves.

## Contract change protocol

Request a contract change instead of silently broadening scope when implementation requires:

- committing real secrets, production `.env` files, certificates, or private keys;
- changing Server/API authentication semantics or route exposure without coordination;
- changing the accepted upload/body-size contract without Server/API coordination;
- changing server/config variable names without Server coordination;
- changing storage paths or migration policy without Storage/Server coordination;
- enabling Worktree runtime or choosing its authority/write model without Worktree coordination;
- adding production deployment automation that mutates remote hosts;
- adding hard-delete/cleanup operations;
- adding provider credentials or OAuth token contents;
- making local-only compose or proxy validation appear production-ready;
- editing CI workflow policy outside coordinated github-ci scope.
