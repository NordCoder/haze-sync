# Local Compose Runbook

## Scope

`deploy/docker-compose.yml` is a local development scaffold for dependency startup only.

It currently starts:

```text
PostgreSQL 16
```

It does not start:

```text
haze-sync-server
haze-gdrive-adapter
worktree runtime
Obsidian plugin
reverse proxy / TLS
migration runner
backup / restore jobs
provider credential flows
```

Compose syntax validation or successful PostgreSQL startup is not production readiness evidence.

## Local safety boundary

The PostgreSQL service binds to `127.0.0.1` only. Do not widen this bind address in the local scaffold. Public or remote exposure belongs to a later reverse-proxy/TLS deployment phase after Server auth and production access boundaries are accepted.

The compose file uses local placeholder defaults. Override them only through an untracked `.env` file when needed. Do not commit real passwords, database URLs, OAuth tokens, provider payloads, TLS keys, logs, dumps, vault data, object-store data, or backup archives.

## Compose variables

The current compose scaffold reads these local PostgreSQL variables:

```text
POSTGRES_DB
POSTGRES_USER
POSTGRES_PASSWORD
POSTGRES_PORT
```

`POSTGRES_PORT` changes only the local host port. The container port remains `5432`, and the host bind address remains fixed to `127.0.0.1`.

`.env.example` also includes Server and adapter placeholders so local configuration names stay visible, but `deploy/docker-compose.yml` does not consume those values until later Server/GDrive/Worktree service wiring phases.

## Syntax validation

From the repository root, run:

```bash
docker compose -f deploy/docker-compose.yml config
```

This validates Docker Compose syntax and interpolation only. It does not prove that Haze Sync Server, adapters, migrations, or production sync are ready.

## Optional local PostgreSQL smoke

Start the local PostgreSQL dependency:

```bash
docker compose -f deploy/docker-compose.yml up -d postgres
```

Check service state:

```bash
docker compose -f deploy/docker-compose.yml ps
```

Run the PostgreSQL readiness probe inside the container:

```bash
docker compose -f deploy/docker-compose.yml exec postgres pg_isready -U "$POSTGRES_USER" -d "$POSTGRES_DB"
```

When no local `.env` overrides are loaded, use the documented local placeholders:

```bash
docker compose -f deploy/docker-compose.yml exec postgres pg_isready -U haze_sync -d haze_sync
```

Stop the local dependency without deleting the named database volume:

```bash
docker compose -f deploy/docker-compose.yml down
```

Delete the local named database volume only when intentionally resetting local development state:

```bash
docker compose -f deploy/docker-compose.yml down -v
```

Do not run destructive reset commands against production services.

## Deferred object-store and worktree binds

Object-store and worktree bind directories are intentionally not mounted by this phase.

They remain deferred because Server startup, object-store path handling, Worktree runtime ownership, backup boundaries, and permission guidance are not yet accepted as deployment service wiring contracts. Later deployment phases may add those mounts after the Server and Worktree contracts are ready enough to avoid creating misleading or unused host directories.
