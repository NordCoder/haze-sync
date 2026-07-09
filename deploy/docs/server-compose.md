# Server Compose Runbook

## Purpose

This runbook documents the DEP-P3 local server service wiring in `deploy/docker-compose.yml`.

The server service is intended for local and prod-like smoke workflows only. It is not a full production deployment and must not be used as evidence that provider sync, Worktree runtime, reverse proxy/TLS, migrations, backups, or real-vault rollout are ready.

## Packaging decision

DEP-P3 chooses container packaging inside Deployment scope:

```text
deploy/server.Dockerfile
```

Rationale:

- the accepted Server dependency now starts from explicit environment configuration;
- the Server binary owns listener startup, object-store root preparation, PostgreSQL pool creation, router construction, and graceful shutdown;
- Docker Compose can wire PostgreSQL and Server without changing Server code;
- container health checks can use the accepted `/health` route;
- production binary/systemd and reverse-proxy/TLS examples remain separate future deployment phases.

## Service topology

Current local topology:

```text
host 127.0.0.1:${POSTGRES_PORT:-5432} -> postgres:5432
host 127.0.0.1:${HAZE_SYNC_HTTP_PORT:-8080} -> server:8080
server -> postgres:5432 over the Compose network
server -> server_objects named volume at /var/lib/haze-sync/objects
```

The Server listens on `0.0.0.0:8080` inside the container only. The host-side port mapping is fixed to `127.0.0.1`.

## Server environment

The compose service passes only documented Server config variables:

```text
HAZE_SYNC_LISTEN_ADDR=0.0.0.0:8080
HAZE_SYNC_DATABASE_URL=<local Compose-network PostgreSQL URL>
HAZE_SYNC_OBJECT_STORE_PATH=/var/lib/haze-sync/objects
HAZE_SYNC_WORKTREE_PATH=/var/lib/haze-sync/worktree
HAZE_SYNC_WORKTREE_ADAPTER_MODE=disabled
HAZE_GDRIVE_ADAPTER_MODE=disabled
HAZE_OBSIDIAN_ADAPTER_MODE=disabled
```

The local Compose-network database URL is derived from placeholder PostgreSQL variables. Use an untracked `.env` file for local overrides. Never commit production database URLs or passwords.

## Volumes

Current named volumes:

```text
postgres_data
server_objects
```

`postgres_data` stores local PostgreSQL state. `server_objects` stores local server object-store data.

These are local Docker named volumes, not production host-directory guidance. Host path layout, permissions, backup boundaries, and restore order belong to later Deployment phases.

## Startup

From the repository root:

```bash
docker compose -f deploy/docker-compose.yml up --build -d postgres server
```

The server service waits for the PostgreSQL healthcheck through Compose `depends_on.condition: service_healthy`.

The server image and compose service do not run migrations. Apply migrations through the accepted operator procedure once the migration deployment phase defines it.

## Health and readiness

Process/router health:

```bash
curl -fsS http://127.0.0.1:8080/health
```

Runtime readiness:

```bash
curl -fsS http://127.0.0.1:8080/ready
```

Expected meaning:

- `/health` means the local HTTP process/router responds.
- `/ready` means the Server's sanitized readiness check sees required configured runtime dependencies as ready.
- Neither endpoint proves migrations, schema state, route-level write readiness, adapter credentials, Worktree runtime, reverse proxy/TLS, backup readiness, or production sync readiness.

The compose healthcheck uses `/health` so it verifies server process liveness without requiring migrations or provider/runtime sync.

## Shutdown

Stop services without deleting local state:

```bash
docker compose -f deploy/docker-compose.yml down
```

Intentional local reset:

```bash
docker compose -f deploy/docker-compose.yml down -v
```

`down -v` deletes local PostgreSQL and server object-store volumes. Do not run destructive reset commands against production services.

## Deferred production behavior

DEP-P3 intentionally does not add:

```text
GDrive adapter service
Worktree runtime service or bind mount
Obsidian plugin runtime
reverse proxy / TLS
migration runner
automatic backups
provider credential flows
remote deployment automation
```

Those surfaces require later component contracts and Deployment phases.
