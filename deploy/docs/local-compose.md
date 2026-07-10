# Local Compose Runbook

## Scope

`deploy/docker-compose.yml` is a local development scaffold for dependency and server startup smoke workflows.

It currently starts:

```text
PostgreSQL 16
haze-sync-server
```

It does not start:

```text
haze-gdrive-adapter
worktree runtime
Obsidian plugin
reverse proxy / TLS
migration runner
backup / restore jobs
provider credential flows
```

Compose syntax validation or successful local service startup is not production readiness evidence.

Use:

- `deploy/docs/migrations-backup-restore.md` for explicit migration, backup, and restore sequencing;
- `deploy/docs/host-directory-layout.md` for production-style path, ownership, permission, and backup boundaries.

Compose does not run migrations automatically and DEP-P5 does not add host bind mounts.

## Local safety boundary

PostgreSQL and server HTTP ports bind to `127.0.0.1` on the host only. Do not widen those bind addresses in the local scaffold. Public or remote exposure belongs to a later reverse-proxy/TLS deployment phase after Server auth and production access boundaries are accepted.

The server listens on `0.0.0.0:8080` inside its container so Docker can publish the port to the host. This does not change the host-side local-only bind in compose.

The compose file uses local placeholder defaults. Override them only through an untracked `.env` file when needed. Do not commit real passwords, database URLs, OAuth tokens, provider payloads, TLS keys, logs, dumps, vault data, object-store data, or backup archives.

## Compose variables

The current compose scaffold reads these local PostgreSQL and host-port variables:

```text
POSTGRES_DB
POSTGRES_USER
POSTGRES_PASSWORD
POSTGRES_PORT
HAZE_SYNC_HTTP_PORT
```

`POSTGRES_PORT` changes only the local host database port. The container port remains `5432`, and the host bind address remains fixed to `127.0.0.1`.

`HAZE_SYNC_HTTP_PORT` changes only the local host HTTP port. The server container port remains `8080`, and the host bind address remains fixed to `127.0.0.1`.

The server service uses documented Server config variables inside the Compose network:

```text
HAZE_SYNC_LISTEN_ADDR
HAZE_SYNC_DATABASE_URL
HAZE_SYNC_OBJECT_STORE_PATH
HAZE_SYNC_WORKTREE_PATH
HAZE_SYNC_WORKTREE_ADAPTER_MODE
HAZE_GDRIVE_ADAPTER_MODE
HAZE_OBSIDIAN_ADAPTER_MODE
```

The Compose-provided database URL is a local placeholder for the internal Compose network. Do not paste production database URLs into tracked files or reports.

`.env.example` keeps relative `./data/objects` and `./data/worktree` placeholders for host-based local runs. Production-style host path examples are documented separately and are not injected into local Compose.

## Packaging path

DEP-P3 uses container packaging for local deployment smoke workflows:

```text
deploy/server.Dockerfile
```

The image builds the `haze-sync-server` binary from the Rust workspace, installs only minimal runtime support required for TLS roots and the local HTTP healthcheck, runs as a non-root `haze-sync` user, and does not bake secrets into the image.

The container user has numeric UID `10001`. A future bind-mount phase must coordinate host ownership with that UID or explicitly choose another accepted runtime identity. Do not make host data directories world-writable to bypass ownership checks.

Migrations are not run by the image or compose service. Migration execution remains an explicit operator action documented in `deploy/docs/migrations-backup-restore.md`.

## Syntax validation

From the repository root, run:

```bash
docker compose -f deploy/docker-compose.yml config
```

This validates Docker Compose syntax and interpolation only. It does not prove that migrations, adapters, Worktree runtime, reverse proxy/TLS, host permissions, backups, or production sync are ready.

When local `.env` contains real local-only secrets, do not paste full rendered `docker compose config` output into public reports.

## Optional local startup smoke

Start PostgreSQL and the local server:

```bash
docker compose -f deploy/docker-compose.yml up --build -d postgres server
```

Check service state:

```bash
docker compose -f deploy/docker-compose.yml ps
```

Check local process health:

```bash
curl -fsS http://127.0.0.1:8080/health
```

Check readiness:

```bash
curl -fsS http://127.0.0.1:8080/ready
```

`/health` only proves that the HTTP process/router responds. `/ready` reports sanitized database and object-store readiness. Neither endpoint proves migrations, provider credentials, adapter loops, Worktree runtime, reverse proxy/TLS, backups, host permission correctness, or production sync readiness.

Run the PostgreSQL readiness probe inside the container:

```bash
docker compose -f deploy/docker-compose.yml exec postgres pg_isready -U "$POSTGRES_USER" -d "$POSTGRES_DB"
```

When no local `.env` overrides are loaded, use the documented local placeholders:

```bash
docker compose -f deploy/docker-compose.yml exec postgres pg_isready -U haze_sync -d haze_sync
```

## Shutdown and local reset

Stop local services without deleting named volumes:

```bash
docker compose -f deploy/docker-compose.yml down
```

Delete local named volumes only when intentionally resetting local development state:

```bash
docker compose -f deploy/docker-compose.yml down -v
```

`down -v` deletes the local PostgreSQL and server object-store volumes. Do not run destructive reset commands against production services.

## Host paths and deferred bind mounts

The current server object store remains a Docker named volume. DEP-P5 documents `/srv/haze-sync/objects` as a production-style host placeholder but does not wire it into Compose.

The accepted Server path keys remain:

```text
HAZE_SYNC_OBJECT_STORE_PATH
HAZE_SYNC_WORKTREE_PATH
```

Worktree runtime is still disabled, and no worktree host bind is added. `/srv/haze-vault/worktree` is documented only as a future production-style placeholder with ownership and backup boundaries.

A future explicit phase may add bind mounts only after container UID/GID behavior, Worktree write authority, backup behavior, and target-host permissions are validated.
