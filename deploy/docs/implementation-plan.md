# Implementation Plan: deployment

## Current state

`deploy` is a local deployment scaffold, not a production deployment.

Implemented current-state surface:

- `deploy/docker-compose.yml` with local PostgreSQL service only;
- local PostgreSQL service binds to `127.0.0.1:5432`;
- placeholder local PostgreSQL defaults through environment interpolation;
- persistent named Docker volume `postgres_data`;
- PostgreSQL healthcheck;
- comments stating future Server and GDrive adapter service wiring is deferred;
- `.env.example` with local placeholder configuration keys;
- root README command for compose syntax validation.

Current behavior intentionally does not:

- start Haze Sync Server;
- start GDrive adapter;
- provision Worktree/object-store directories;
- configure reverse proxy/TLS;
- run migrations;
- perform backups/restores;
- manage production secrets;
- deploy to remote hosts;
- start sync behavior.

The component docs were scaffold-level before this planning pass.

## Target state

The target state for Deployment is a safe local and VPS operations layer for Haze Sync V1.

Deployment is V1-ready when:

- local development compose can start required local dependencies safely;
- production deployment topology is documented and reproducible;
- Haze Sync Server and GDrive adapter service wiring is explicit;
- secret placement and permissions are documented without committing secrets;
- database migration policy is explicit;
- object-store and worktree host paths are provisioned safely;
- reverse proxy/TLS configuration is documented;
- backup/restore procedure preserves database/object-store consistency;
- smoke checks distinguish syntax validation, service readiness, and production sync readiness;
- runbooks explain bootstrap, rollback, restart, and safe shutdown behavior.

## Implementation phases

### DEP-P1 — Component contract and planning normalization

Status: completed by this Architect planning pass.

Goal:

```text
Replace scaffold deployment docs with a real contract, dependency map,
implementation plan, decisions, and baseline implementation log.
```

Allowed scope:

```text
deploy/docs/**
```

Completed deliverables:

- complete `component-contract.md`;
- complete `dependency-map.md`;
- complete `implementation-plan.md`;
- add initial deployment decisions;
- update implementation log with planning baseline.

Non-goals:

- no product code changes;
- no compose service changes;
- no production automation;
- no CI workflow changes;
- no secret files.

Acceptance:

- docs describe current deployment scaffold honestly;
- docs preserve Deployment/Server/Storage/GDrive/Worktree/CI boundaries;
- future deployment phases are implementable without product-code ownership.

### DEP-P2 — Local development compose hardening

Goal:

```text
Harden local compose scaffolding so developers can run dependencies without
mistaking local-only services for production deployment.
```

Allowed scope:

```text
deploy/docker-compose.yml
deploy/docs/**
.env.example if local placeholders need documented alignment
```

Likely work:

- verify PostgreSQL service healthcheck and local-only bind address;
- align `.env.example` variables with compose and server config names;
- add optional local object-store/worktree bind directories only when Server/Worktree contracts are ready;
- add comments labeling local placeholders and non-production defaults;
- add documented `docker compose config` and optional local startup smoke commands.

Non-goals:

- no production secrets;
- no reverse proxy/TLS;
- no remote deploy;
- no real Google OAuth;
- no server/adapters until startup contracts are stable.

Contract-change triggers:

- changing config variable names consumed by Server/GDrive/Obsidian;
- adding non-local bind addresses;
- adding real credentials;
- making local defaults production-like.

Acceptance:

- local compose is clear and safe;
- syntax validation passes;
- local-only limitations remain explicit.

### DEP-P3 — Server packaging and service wiring

Goal:

```text
Add explicit deployment wiring for `haze-sync-server` only after Server startup,
config loading, migration policy, and readiness behavior are accepted.
```

Allowed scope:

```text
deploy/**
Dockerfile or server packaging files if accepted
docs/runbooks if introduced
```

Likely work:

- decide binary vs container packaging path;
- define server environment variables and volumes;
- wire PostgreSQL connection and object-store path;
- expose local/proxy-bound listen address safely;
- add readiness/liveness checks;
- document startup and shutdown;
- avoid auto-running migrations until policy is accepted.

Non-goals:

- no Server code changes unless cross-component fan-in scopes them;
- no GDrive adapter service;
- no Worktree runtime unless Server/Worktree fan-in is ready;
- no production TLS/private keys.

Contract-change triggers:

- Server lacking production listener/config contract;
- migration policy unresolved;
- exposing server publicly without auth/TLS guidance;
- embedding secrets into images or tracked files.

Acceptance:

- server service can be started from explicit config;
- health/readiness behavior is documented;
- no secrets are committed.

### DEP-P4 — Database migrations, backup, and restore runbook

Goal:

```text
Define how operators apply migrations and back up/restore PostgreSQL metadata in
coordination with object-store data.
```

Allowed scope:

```text
deploy/docs/**
deploy/scripts/** only if explicitly accepted
```

Likely work:

- document migration execution owner: CLI, Server, manual `sqlx`, or deploy script;
- define pre-migration backup steps;
- define database backup command examples using placeholders;
- define restore order and consistency warnings;
- document stopped-service or quiesced-sync requirements;
- add dry-run/checklist style verification.

Non-goals:

- no automatic migration runner unless Server/Storage contract accepts it;
- no production DB URLs;
- no backup archives committed;
- no hard-delete cleanup.

Contract-change triggers:

- changing migration policy;
- requiring production credentials in repo;
- restoring database without object-store consistency guidance;
- adding destructive cleanup scripts.

Acceptance:

- migration/backup/restore process is explicit and secret-safe;
- operators can understand consistency requirements;
- no generated artifacts are tracked.

### DEP-P5 — Object-store, worktree, and host directory provisioning

Goal:

```text
Define host directory layout and permissions for object store, worktree,
secrets, logs, and backups.
```

Allowed scope:

```text
deploy/docs/**
deploy/scripts/** only if explicitly accepted
.env.example if placeholders need alignment
```

Likely work:

- document expected directories such as `/srv/haze-sync/objects`, `/srv/haze-vault/worktree`, and `/opt/haze-sync/secrets` as placeholders/examples;
- define ownership/permission expectations;
- distinguish data, config, secret, log, backup, and runtime temp paths;
- document what must be backed up and what must never be committed;
- coordinate Worktree root and object-store root with Server config.

Non-goals:

- no actual host mutation through GitHub connector;
- no Worktree runtime behavior;
- no object-store cleanup job;
- no real secret files.

Contract-change triggers:

- changing Server/Worktree/GDrive expected config keys;
- storing secrets under tracked paths;
- using local absolute user paths in committed examples;
- adding cleanup/retention automation.

Acceptance:

- operator path layout is clear;
- permission model is secret-safe;
- backup boundaries are documented.

### DEP-P6 — Reverse proxy, TLS, and public access boundary

Goal:

```text
Document safe public access topology for Haze Sync Server without committing TLS
secrets or provider credentials.
```

Allowed scope:

```text
deploy/docs/**
deploy/reverse-proxy/** if examples are accepted
```

Likely work:

- choose/example reverse proxy model such as nginx/Caddy/Traefik;
- document TLS termination and local upstream address;
- document max upload/body-size considerations;
- document auth requirement and no-public-open deployment expectation;
- add config syntax checks if examples are committed;
- document firewall/port expectations.

Non-goals:

- no TLS private keys;
- no public DNS automation;
- no cloud provider scripts;
- no server auth model changes.

Contract-change triggers:

- exposing unauthenticated routes beyond health where unsafe;
- changing upload size limits without Server/API coordination;
- committing cert/private key material;
- adding remote host automation.

Acceptance:

- public access runbook is explicit and secure by default;
- reverse proxy examples are placeholders and syntax-checkable if tracked;
- secrets remain out of repo.

### DEP-P7 — GDrive adapter service and OAuth secret layout

Goal:

```text
Add deployment wiring for the GDrive adapter after adapter config, token handling,
and persistence boundary are accepted.
```

Allowed scope:

```text
deploy/**
```

Likely work:

- define adapter service process/container;
- document OAuth token file path and permissions;
- pass server URL and adapter auth token safely;
- configure adapter mode and scan/poll cadence;
- define restart/backoff policy;
- document dry-run/import-only/export-only/bidirectional rollout stages;
- document provider credential bootstrap steps without committing tokens.

Non-goals:

- no Google API code;
- no real OAuth token contents;
- no direct DB access decision hidden in deployment;
- no production provider credentials in CI.

Contract-change triggers:

- adapter config not stable;
- mapping/cursor persistence boundary unresolved for chosen topology;
- OAuth token files tracked in repo;
- bidirectional mode enabled without bootstrap/runbook readiness.

Acceptance:

- adapter service can be deployed with explicit secrets and modes;
- dry-run/import/export rollout is documented;
- no credentials are committed.

### DEP-P8 — Bootstrap, rollout, rollback, and operational runbooks

Goal:

```text
Document safe first-vault bootstrap and operational rollback procedures across
Server, Worktree, GDrive, and Obsidian clients.
```

Allowed scope:

```text
deploy/docs/**
```

Likely work:

- define bootstrap stages:
  - GDrive import-only;
  - Core verification;
  - Worktree export/materialization;
  - Obsidian pull-only/manual verification;
  - bidirectional enablement one adapter at a time.
- define stop/rollback procedure;
- define safe backup checkpoints;
- define smoke tests after each stage;
- define operator decision points and blockers.

Non-goals:

- no product behavior changes;
- no automatic bootstrap script unless explicitly scoped;
- no real vault/provider data;
- no destructive repair automation.

Contract-change triggers:

- bootstrap requiring unavailable Server/API/GDrive/Worktree features;
- bypassing Core safety modes;
- enabling bidirectional sync without tests;
- omitting backup/rollback steps.

Acceptance:

- rollout is staged and reversible;
- operators know when to stop;
- no step depends on untracked secrets in repo.

### DEP-P9 — Production readiness and observability checklist

Goal:

```text
Prepare production readiness checks that prove the deployment is configured,
observable, recoverable, and secret-safe.
```

Allowed scope:

```text
deploy/docs/**
deploy/scripts/** only if accepted
```

Likely work:

- define readiness checklist for server, DB, object store, worktree, GDrive adapter, Obsidian client, backups, and reverse proxy;
- define log locations and redaction expectations;
- define service restart and update procedure;
- define incident checklist for mass delete guard, auth failures, provider outage, DB failure, object-store corruption, and conflict spike;
- define what evidence is safe to paste into reports.

Non-goals:

- no live production monitoring service unless scoped;
- no remote automation;
- no hidden telemetry;
- no secrets in reports.

Contract-change triggers:

- requiring metrics/logging not implemented by Server/GDrive;
- exposing sensitive logs;
- adding production secrets to CI;
- adding cleanup/destructive automation.

Acceptance:

- readiness checklist is actionable and safe;
- operators can diagnose without leaking secrets;
- production gaps are explicit.

## Dependency gates

Deployment implementation depends on stable upstream contracts:

- Server must define production startup, config, readiness, and migration policy before server service wiring is production-ready.
- Storage must define schema/migration/backup expectations before backup/restore runbooks are final.
- GDrive adapter must define config, secret, mode, and persistence boundary before service deployment is final.
- Worktree must define runtime hosting and path layout before worktree directory provisioning is final.
- Obsidian plugin must define server URL/token setup before client setup docs are final.
- GitHub CI owns workflow policy; Deployment may only coordinate validation hooks.

## Known risks

- Local compose scaffolds can be mistaken for production readiness.
- Placeholder passwords or empty `.env.example` values can accidentally become production values.
- Migration policy can cause startup/data-loss risk if hidden or inconsistent.
- Database/object-store backups can be inconsistent if taken while sync is mutating state.
- OAuth tokens and private keys are high-risk leak sources.
- Reverse proxy examples can accidentally expose unauthenticated or oversized public routes.
- Deployment scripts can become destructive remote automation if scope is not controlled.

## Deferred work

Deferred outside this Architect documentation/planning pass:

- run compose validation or observe CI;
- execute DEP-P2 through DEP-P9 implementation/clean-code/CI phases;
- add server/GDrive services to compose;
- add Dockerfiles/systemd/reverse proxy examples;
- define migration runner policy;
- define backup/restore runbook;
- define production secret layout;
- add real deployment smoke tests;
- implement remote deployment automation.

## Completion criteria for the component

`deploy` is V1-ready when:

- local development compose is safe and documented;
- server and GDrive services have explicit non-secret configuration;
- production path/permission/secret layout is documented;
- migration/backup/restore policy is explicit;
- reverse proxy/TLS guidance is safe;
- rollout/rollback/bootstrap runbooks are staged and reversible;
- smoke/readiness checks distinguish syntax, service, and sync readiness;
- no tracked deployment file contains real secrets, tokens, vault data, logs, dumps, or backup artifacts.
