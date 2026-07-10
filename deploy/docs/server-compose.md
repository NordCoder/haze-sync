# Server Compose Runbook

## Purpose

This runbook documents the DEP-P3 local server service wiring in `deploy/docker-compose.yml`.

The server service is intended for local and prod-like smoke workflows only. It is not a full production deployment and must not be used as evidence that provider sync, Worktree runtime, public TLS rollout, migrations, backups, host permissions, or real-vault rollout are ready.

Use:

- `deploy/docs/migrations-backup-restore.md` for database migration, backup, and restore sequencing;
- `deploy/docs/host-directory-layout.md` for production-style path, ownership, permission, and backup boundaries;
- `deploy/docs/public-access.md` for the placeholder Caddy/TLS/firewall boundary.

The tracked Caddyfile is not started by Compose.

## Packaging decision

DEP-P3 chooses container packaging inside Deployment scope:

```text
deploy/server.Dockerfile
```

Rationale:

- the accepted Server dependency starts from explicit environment configuration;
- the Server binary owns listener startup, object-store root preparation, PostgreSQL pool creation, router construction, and graceful shutdown;
- Docker Compose can wire PostgreSQL and Server without changing Server code;
- container health checks can use the accepted `/health` route;
- production binary/systemd and running reverse-proxy/TLS services remain separate deployment work.

The Docker build uses the repository `Cargo.lock` through `cargo build --release --locked -p haze-sync-server` so dependency resolution cannot silently change during image builds.

`deploy/server.Dockerfile.dockerignore` keeps local secrets, `.env` files, `.git`, build outputs, dependency caches, logs, dumps, backups, archives, and OS/editor noise out of the server image build context while preserving tracked placeholder examples such as `.env.example`.

The runtime image uses the non-root `haze-sync` account with numeric UID `10001`. A future host bind mount must grant only the required access to that identity or explicitly coordinate another accepted runtime user/group mapping.

## Service topology

Current local topology:

```text
host 127.0.0.1:${POSTGRES_PORT:-5432} -> postgres:5432
host 127.0.0.1:${HAZE_SYNC_HTTP_PORT:-8080} -> server:8080
server -> postgres:5432 over the Compose network
server -> server_objects named volume at /var/lib/haze-sync/objects
```

The Server listens on `0.0.0.0:8080` inside the container only. The host-side port mapping is fixed to `127.0.0.1`.

The DEP-P6 production-style public topology keeps the same loopback host boundary:

```text
public TCP 443 -> Caddy -> 127.0.0.1:8080 Server
```

Do not widen the Compose Server port bind to make the proxy work.

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

Production-style host placeholders remain the same accepted Server keys:

```text
HAZE_SYNC_OBJECT_STORE_PATH=/srv/haze-sync/objects
HAZE_SYNC_WORKTREE_PATH=/srv/haze-vault/worktree
```

Those values are documentation only in DEP-P5; Compose does not bind those host paths.

## Volumes

Current named volumes:

```text
postgres_data
server_objects
```

`postgres_data` stores local PostgreSQL state. `server_objects` stores local server object-store data.

These are local Docker named volumes, not production host directories. DEP-P5 documents production-style host layout and permissions, but it intentionally does not replace the named volumes or add Worktree bind mounts.

## Startup

From the repository root:

```bash
docker compose -f deploy/docker-compose.yml up --build -d postgres server
```

The server service waits for the PostgreSQL healthcheck through Compose `depends_on.condition: service_healthy`.

The server image and compose service do not run migrations. Apply migrations through `deploy/docs/migrations-backup-restore.md` before treating the service as schema-ready.

## Health and readiness

Trusted local process/router health:

```bash
curl -fsS http://127.0.0.1:8080/health
```

Trusted local runtime readiness:

```bash
curl -fsS http://127.0.0.1:8080/ready
```

Expected meaning:

- `/health` means the local HTTP process/router responds.
- `/ready` means the Server's sanitized readiness check sees required configured runtime dependencies as ready.
- Neither endpoint proves migrations, schema state, route-level write readiness, adapter credentials, Worktree runtime, public TLS rollout, backup readiness, host permission correctness, or production sync readiness.

The compose healthcheck uses `/health` so it verifies server process liveness without requiring migrations or provider/runtime sync.

The tracked public Caddy example blocks both `/health` and `/ready` from public clients while using loopback `/health` for active upstream checks.

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

Current deployment intentionally does not add:

```text
GDrive adapter service
Worktree runtime service or bind mount
Obsidian plugin runtime
running reverse-proxy service or certificate provisioning
migration runner
automatic backups
provider credential flows
remote deployment automation
host mutation/provisioning scripts
```

The repository now contains a placeholder-only Caddy configuration and public-access runbook. Starting it, provisioning DNS/certificates, and changing firewall rules remain operator-owned future rollout work.
