# Database Migrations, Backup, and Restore Runbook

## Scope

This runbook defines the DEP-P4 operator procedure for PostgreSQL metadata migrations and coordinated backup/restore of PostgreSQL, object-store data, and Worktree data whenever the Worktree may contain authoritative or unreplicated content.

It applies to the current Deployment surface:

```text
deploy/docker-compose.yml
deploy/docker-compose.worktree.yml when explicitly selected
postgres service
server service
postgres_data named volume
server_objects named volume
operator-supplied Worktree host path when the override is used
```

It is intentionally documentation-only. It does not add a migration runner, backup script, restore script, remote deployment automation, destructive cleanup, production credentials, or backup archives.

Use `deploy/docs/worktree-compose.md` for the accepted opt-in Worktree bind, UID `10001` permissions, read-only default, mode gate, startup, and shutdown procedure.

## Ownership and policy

Current migration execution owner:

```text
human/operator running the manual SQLx migration command
```

Current accepted policy:

- Storage owns migration contents and schema design.
- Server exposes an explicit internal migration helper, but the current server binary and Compose service do not call it automatically.
- Deployment owns the operator sequence for shutdown/quiescence, backup, migration, restore, startup, and verification.
- Operators must run migrations explicitly after taking backups and while all writers are stopped or explicitly quiesced.
- Compose, Server startup, Docker entrypoints, and the Worktree override remain migration-free.
- Rollback is an operator-approved coordinated restore, not an automatic reset, drop, cleanup, or destructive fallback.
- No repository file may contain production database URLs, passwords, dumps, object-store archives, Worktree archives, vault contents, or restored data.

A future accepted phase may replace the manual SQLx command with a dedicated CLI, Server entry point, or deploy script. Until then, do not invent automatic migration behavior in Compose or Docker images.

## Consistency model

Haze Sync metadata in PostgreSQL references content stored in the object store. When a Worktree is mounted and may contain user-authored, authoritative, or otherwise unreplicated content, it is part of the same recovery set.

A usable complete backup or restore point must preserve all required state from one stopped/quiesced operational window:

```text
PostgreSQL metadata dump
object-store archive or snapshot
Worktree archive or snapshot when authoritative or unreplicated content may exist
sanitized operator notes: code revision, migration status, backup timestamp, service state, Worktree mode, and bind access mode
```

A database-only backup can be useful for metadata debugging, but it is not a complete vault recovery point. An object-store-only backup cannot reconstruct metadata, revision state, operation logs, conflicts, or idempotency records. A Worktree-only backup cannot restore PostgreSQL metadata or object-store content.

Treat every process capable of mutating PostgreSQL, object-store data, or the Worktree as a writer. This includes the Server, enabled Worktree runtime, future provider adapters, CLI write commands, and client/plugin writes. All such writers must be stopped or explicitly quiesced before backup, migration, or restore.

## Pre-migration checklist

Before running migrations:

1. Confirm the target repository commit and deployment configuration.
2. Confirm the migration command source is the repository `migrations` directory.
3. Confirm all writers are stopped or explicitly quiesced:
   - Server service stopped or quiesced;
   - Worktree runtime stopped or quiesced when the override is used;
   - GDrive adapter stopped when present;
   - CLI write activity stopped;
   - Obsidian/plugin writes disabled or quiesced.
4. Record whether the Worktree override is active, the accepted adapter mode, and whether the bind is read-only or writable.
5. Confirm backup output paths are outside the repository, object store, Worktree, logs, and runtime temp paths.
6. Confirm backup output paths are access-controlled and not world-readable.
7. Take a PostgreSQL metadata backup.
8. Take an object-store backup or snapshot from the same stopped/quiesced window.
9. Take a Worktree backup or snapshot from that same window whenever it may contain authoritative or unreplicated content.
10. Record sanitized notes only:
    - timestamp;
    - repository commit SHA;
    - migration source path;
    - backup file names without secrets;
    - whether each writer was stopped or quiesced;
    - Worktree mode and read-only/writable state without exposing sensitive host paths.
11. Do not paste production database URLs, passwords, token values, absolute secret paths, raw provider payloads, vault contents, or stack traces into public reports.

## Stop or quiesce writers

For base local Compose, stop the Server before backup and migration:

```bash
docker compose -f deploy/docker-compose.yml stop server
```

When the Worktree override is active, use the same file set that started the deployment:

```bash
docker compose \
  -f deploy/docker-compose.yml \
  -f deploy/docker-compose.worktree.yml \
  stop server
```

Stopping the Server stops Server-owned Worktree runtime activity. Confirm separately that no CLI, plugin, provider adapter, operator shell, or other process still writes to PostgreSQL, the object store, or the host Worktree path.

Do not rely on `/health`, `/ready`, or a sanitized Worktree status response alone as proof that all writers are quiesced.

## Local PostgreSQL backup example

Use an operator-owned backup directory outside the repository and runtime data roots:

```bash
export HAZE_SYNC_BACKUP_DIR=/secure/operator/backups/haze-sync/<timestamp>
mkdir -p "$HAZE_SYNC_BACKUP_DIR"
```

Create a custom-format PostgreSQL dump inside the local Compose PostgreSQL container:

```bash
docker compose -f deploy/docker-compose.yml exec -T postgres \
  pg_dump \
    -U "${POSTGRES_USER:-haze_sync}" \
    -d "${POSTGRES_DB:-haze_sync}" \
    --format=custom \
    --file=/tmp/haze_sync_metadata.dump
```

Copy it out:

```bash
docker compose -f deploy/docker-compose.yml cp \
  postgres:/tmp/haze_sync_metadata.dump \
  "$HAZE_SYNC_BACKUP_DIR/haze_sync_metadata.dump"
```

Remove the temporary dump:

```bash
docker compose -f deploy/docker-compose.yml exec -T postgres \
  rm -f /tmp/haze_sync_metadata.dump
```

Do not commit the dump or paste rendered database URLs into reports.

## Local object-store backup example

The current local Server object store is a Docker named volume.

Archive it to the same operator-owned recovery directory:

```bash
docker run --rm \
  --volume haze-sync_server_objects:/source:ro \
  --volume "$HAZE_SYNC_BACKUP_DIR:/backup" \
  alpine:3.20 \
  sh -c 'cd /source && tar -czf /backup/haze_sync_objects.tgz .'
```

If the Compose project name differs, inspect the actual named volume with `docker volume ls`. Do not commit the archive.

## Worktree backup placeholder

When the opt-in override is active and the Worktree may contain authoritative or unreplicated content, capture it from the same stopped/quiesced window as PostgreSQL and the object store.

Example shape only:

```bash
tar -C "${HAZE_SYNC_WORKTREE_HOST_PATH:?operator environment required}" \
  -czf "$HAZE_SYNC_BACKUP_DIR/haze_sync_worktree.tgz" \
  .
```

Do not place the backup inside the Worktree. Do not print or commit real host paths or vault contents. A read-only bind reduces Server write authority but does not make a Worktree backup unnecessary when external users or tools can modify the same directory.

## Production-style backup placeholders

Keep credentials outside shell history and the repository. Prefer host-local `.pgpass`, a secrets manager, or an operator-local environment file with restricted permissions.

PostgreSQL shape:

```bash
PGHOST=<db-host> \
PGPORT=<db-port> \
PGDATABASE=<db-name> \
PGUSER=<db-user> \
pg_dump \
  --format=custom \
  --file=/secure/operator/backups/haze-sync/<timestamp>/haze_sync_metadata.dump
```

Object-store shape:

```bash
tar -C /srv/haze-sync/objects \
  -czf /secure/operator/backups/haze-sync/<timestamp>/haze_sync_objects.tgz \
  .
```

Worktree shape, when required by its authority model:

```bash
tar -C /srv/haze-vault/worktree \
  -czf /secure/operator/backups/haze-sync/<timestamp>/haze_sync_worktree.tgz \
  .
```

These paths are generic placeholders. Use an approved host layout and do not commit real machine paths or backup artifacts.

## Manual migration command

Run migrations only after the coordinated backup set is complete and all writers remain stopped/quiesced.

Current manual SQLx command shape from the repository root:

```bash
DATABASE_URL=<operator-supplied-database-url> \
  sqlx migrate run \
  --source migrations
```

Rules:

- the human/operator is the sole migration execution owner under the accepted policy;
- Storage owns migration contents and schema design;
- Deployment owns this operational sequence;
- supply `DATABASE_URL` through an operator-local secret mechanism;
- do not commit or paste the URL;
- do not use `sqlx database drop`, reset commands, or destructive cleanup as a migration fallback;
- do not add migration execution to Compose files, Dockerfiles, entrypoints, startup commands, or healthchecks.

## Post-migration verification

After migrations complete:

1. Confirm the migration command exited successfully.
2. Start the Server with the same Compose file set used before shutdown.
3. Check `/health` for process response.
4. Check `/ready` for sanitized dependency readiness.
5. When the Worktree override is active, inspect the accepted sanitized Worktree status separately.
6. Confirm the adapter mode and bind access remain exactly as approved; do not automatically enable bidirectional or write-capable behavior.
7. Record sanitized results only.

Base startup:

```bash
docker compose -f deploy/docker-compose.yml up -d server
```

Opt-in Worktree startup:

```bash
docker compose \
  -f deploy/docker-compose.yml \
  -f deploy/docker-compose.worktree.yml \
  up -d server
```

Health and readiness:

```bash
curl -fsS http://127.0.0.1:8080/health
curl -fsS http://127.0.0.1:8080/ready
```

`/health` and `/ready` do not prove Worktree permission correctness, backup integrity, rollout approval, synchronization direction, or production readiness.

## Restore prerequisites

Restore is an operator-approved coordinated recovery operation, not a merge/update routine and not an automatic rollback.

Before restore:

1. Stop or explicitly quiesce every writer.
2. Confirm the target database is empty or intentionally replaceable.
3. Confirm the target object-store directory or volume is empty or intentionally replaceable.
4. When restoring Worktree data, confirm the target Worktree is intentionally replaceable and no external user or process is writing to it.
5. Confirm PostgreSQL, object-store, and required Worktree artifacts come from the same recovery window.
6. Confirm the target code revision is compatible with the backup migration state.
7. Confirm the Worktree mode and bind access to restore after data recovery.
8. Obtain explicit operator approval before any destructive replacement.
9. Do not use automatic `down -v`, database reset, schema drop, object deletion, Worktree cleanup, or destructive fallback behavior.

## Local restore order

For local recovery into explicitly approved empty or replaceable targets:

1. Stop all services and external writers.
2. Recreate PostgreSQL only.
3. Restore the PostgreSQL dump.
4. Restore matching object-store data.
5. Restore matching Worktree data when it belongs to the coordinated recovery set.
6. Remove temporary database dump material from the container.
7. Start the Server with the approved base or opt-in Compose file set.
8. Verify process health, readiness, Worktree status, mode, bind access, and rollout approval separately.

Representative PostgreSQL restore shape:

```bash
docker compose -f deploy/docker-compose.yml exec -T postgres \
  pg_restore \
    -U "${POSTGRES_USER:-haze_sync}" \
    -d "${POSTGRES_DB:-haze_sync}" \
    --clean \
    --if-exists \
    /tmp/haze_sync_metadata.dump
```

Representative object-store restore shape:

```bash
docker run --rm \
  --volume haze-sync_server_objects:/target \
  --volume "$HAZE_SYNC_BACKUP_DIR:/backup:ro" \
  alpine:3.20 \
  sh -c 'cd /target && tar -xzf /backup/haze_sync_objects.tgz'
```

Representative Worktree restore shape, only after explicit approval of the replaceable target:

```bash
tar -C "${HAZE_SYNC_WORKTREE_HOST_PATH:?operator environment required}" \
  -xzf "$HAZE_SYNC_BACKUP_DIR/haze_sync_worktree.tgz"
```

The `--clean --if-exists` flags and any Worktree replacement are destructive operations for explicitly approved targets only. They are not automatic rollback behavior.

## Production-style restore placeholder

Restore PostgreSQL, object-store data, and required Worktree data from the same coordinated recovery window. Never combine timestamps unless an explicit incident plan documents and approves the mismatch.

PostgreSQL shape:

```bash
pg_restore \
  --dbname=<target-db-name> \
  --clean \
  --if-exists \
  /secure/operator/backups/haze-sync/<timestamp>/haze_sync_metadata.dump
```

Then restore the matching object-store archive and, when required, the matching Worktree archive to approved target paths.

## Dry-run and checklist verification

Before treating a backup as usable, verify:

```text
[ ] Backup directory is outside the repository and runtime data roots.
[ ] Database dump exists and is non-empty.
[ ] Object-store archive/snapshot exists, or absence is intentionally recorded.
[ ] Worktree archive/snapshot exists when authoritative or unreplicated content may exist, or omission is explicitly justified.
[ ] Every writer was stopped or explicitly quiesced during capture.
[ ] Code revision, migration source, service state, Worktree mode, and bind access were recorded safely.
[ ] No production URL, password, token, vault content, or sensitive path appears in notes or reports.
[ ] Restore targets were approved as empty or replaceable before destructive commands.
[ ] PostgreSQL, object-store, and required Worktree artifacts come from the same recovery window.
[ ] /health, /ready, and Worktree status were checked separately after restore where applicable.
[ ] No automatic migration or automatic rollback behavior was introduced.
[ ] Any skipped check is recorded honestly.
```

Do not attach dumps, object-store archives, Worktree archives, logs containing secrets, provider payloads, vault contents, or rendered secret-bearing configs to public PRs or reports.

## What this runbook deliberately does not do

This runbook does not:

```text
run migrations automatically
change migration ownership away from the human/operator
create deploy scripts
create backup archives in the repository
restore production hosts automatically
rotate or generate credentials
define Storage schema contents
define object-store retention or cleanup
automatically enable provider services
automatically enable Worktree runtime or a write-capable mode
perform automatic rollback, down -v, reset, drop, deletion, or destructive fallback
claim production readiness
```

Those require separate accepted contracts, operator approvals, and deployment phases.
