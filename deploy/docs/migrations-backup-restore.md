# Database Migrations, Backup, and Restore Runbook

## Scope

This runbook defines the DEP-P4 operator procedure for PostgreSQL metadata migrations and backup/restore coordination with the local object store.

It applies to the current Deployment surface:

```text
deploy/docker-compose.yml
postgres service
server service
postgres_data named volume
server_objects named volume
```

It is intentionally documentation-only. It does not add a migration runner, backup script, restore script, remote deployment automation, destructive cleanup, production credentials, or backup archives.

## Ownership and policy

Current migration execution owner:

```text
operator-run manual SQLx migration command
```

Current accepted policy:

- Storage owns migration contents and schema design.
- Server exposes an explicit internal migration helper, but the current server binary and compose service do not call it automatically.
- Deployment owns the operator sequence for when migrations are run relative to shutdown, backup, startup, and verification.
- Operators must run migrations explicitly after taking backups and while writes are stopped or quiesced.
- No repository file may contain production database URLs, passwords, dumps, object-store archives, or restored data.

Future accepted phases may replace the manual SQLx command with a dedicated CLI, server entry point, or deploy script. Until then, do not invent automatic migration behavior in compose or Docker images.

## Consistency model

Haze Sync metadata in PostgreSQL references content stored in the object store. A usable backup or restore point must preserve both sides from the same operational window:

```text
PostgreSQL metadata dump
object-store archive or snapshot
operator notes: code revision, migration status, backup timestamp, service state
```

A database-only backup can be useful for metadata debugging, but it is not a complete vault recovery point. An object-store-only backup can preserve blobs, but it is not enough to reconstruct metadata, revision state, operation logs, conflicts, or idempotency records.

For migration and restore safety, treat the server and all future adapters as writers. They must be stopped or quiesced before migration backups and before restoring either side.

## Pre-migration checklist

Before running migrations:

1. Confirm the target repository commit and deployment configuration.
2. Confirm the migration command source is the repository `migrations` directory.
3. Confirm all writers are stopped or quiesced:
   - current server service stopped;
   - future GDrive adapter service stopped;
   - future Worktree runtime stopped;
   - future Obsidian/plugin writes disabled.
4. Confirm backup output paths are outside the repository.
5. Confirm backup output paths are access-controlled and not world-readable.
6. Take a PostgreSQL metadata backup.
7. Take an object-store backup or snapshot from the same stopped/quiesced window.
8. Record sanitized notes only:
   - timestamp;
   - repository commit SHA;
   - migration source path;
   - backup file names without secrets;
   - whether services were stopped or quiesced.
9. Do not paste production database URLs, passwords, token values, absolute secret paths, raw provider payloads, or stack traces into public reports.

## Stop or quiesce writers

For the current local Compose scaffold, stop the server before backup and migration:

```bash
docker compose -f deploy/docker-compose.yml stop server
```

If future adapter services are added, stop or quiesce them before continuing. Do not rely on `/health` or `/ready` alone as proof that writes are stopped.

## Local PostgreSQL backup example

Use an operator-owned backup directory outside the repository:

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

Copy it out to the operator-owned backup directory:

```bash
docker compose -f deploy/docker-compose.yml cp \
  postgres:/tmp/haze_sync_metadata.dump \
  "$HAZE_SYNC_BACKUP_DIR/haze_sync_metadata.dump"
```

Remove the temporary dump from the container:

```bash
docker compose -f deploy/docker-compose.yml exec -T postgres \
  rm -f /tmp/haze_sync_metadata.dump
```

Do not commit the dump file or paste rendered database URLs into reports.

## Local object-store backup example

The current local server object store is a Docker named volume, not a production host path.

Archive it to the same operator-owned backup directory:

```bash
docker run --rm \
  --volume haze-sync_server_objects:/source:ro \
  --volume "$HAZE_SYNC_BACKUP_DIR:/backup" \
  alpine:3.20 \
  sh -c 'cd /source && tar -czf /backup/haze_sync_objects.tgz .'
```

If the Compose project name differs, the actual named volume may differ. Check with:

```bash
docker volume ls
```

Do not commit the archive. If the object-store volume does not exist yet, record that no object-store data was present for this backup point.

## Production-style PostgreSQL backup placeholder

For a production-like host, keep credentials outside shell history and outside the repository. Prefer host-local `.pgpass`, a secrets manager, or an operator-local environment file with restricted permissions.

Example shape only:

```bash
PGHOST=<db-host> \
PGPORT=<db-port> \
PGDATABASE=<db-name> \
PGUSER=<db-user> \
pg_dump \
  --format=custom \
  --file=/secure/operator/backups/haze-sync/<timestamp>/haze_sync_metadata.dump
```

Do not put passwords or full connection URLs in committed docs, reports, command transcripts, or issue comments.

## Production-style object-store backup placeholder

For a production-like host path, archive or snapshot the object-store directory from the same stopped/quiesced window as the PostgreSQL dump:

```bash
tar -C /srv/haze-sync/objects \
  -czf /secure/operator/backups/haze-sync/<timestamp>/haze_sync_objects.tgz \
  .
```

`/srv/haze-sync/objects` is a placeholder path. Use the accepted host directory layout from a future Deployment host-directory phase.

## Manual migration command

Run migrations only after backups are complete and writers remain stopped/quiesced.

Current manual SQLx command shape from the repository root:

```bash
DATABASE_URL=<operator-supplied-database-url> \
  sqlx migrate run \
  --source migrations
```

Rules:

- supply `DATABASE_URL` through an operator-local secret mechanism;
- do not commit or paste the URL;
- do not use `sqlx database drop`, reset commands, or destructive cleanup as part of this runbook;
- do not add migration execution to `deploy/docker-compose.yml` or `deploy/server.Dockerfile` until a later accepted contract scopes it.

For local Compose, the URL can target the local-only PostgreSQL bind. Keep the value in an untracked `.env` or operator shell session, not in the repository.

## Post-migration verification

After migrations complete:

1. Confirm the migration command exited successfully.
2. Start the server:

   ```bash
   docker compose -f deploy/docker-compose.yml up -d server
   ```

3. Check server process health:

   ```bash
   curl -fsS http://127.0.0.1:8080/health
   ```

4. Check runtime readiness:

   ```bash
   curl -fsS http://127.0.0.1:8080/ready
   ```

5. Confirm future adapters remain disabled until their deployment phases explicitly enable them.
6. Record sanitized results only. Do not paste secrets, full URLs, stack traces, provider payloads, or raw dumps.

`/health` only proves the HTTP process/router responds. `/ready` reports sanitized dependency readiness. Neither endpoint proves production rollout, provider sync readiness, backup integrity, or route-level write correctness.

## Restore prerequisites

Restore is a coordinated recovery operation, not a merge/update routine.

Before restore:

1. Stop all writers.
2. Confirm the target database is empty or intentionally replaceable.
3. Confirm the target object-store directory or volume is empty or intentionally replaceable.
4. Confirm the PostgreSQL dump and object-store archive come from the same backup window.
5. Confirm the target code revision is compatible with the backup's migration state.
6. Do not run destructive cleanup against production unless an operator has explicitly approved the target and backup set.

## Local restore order

For local Compose recovery into empty local volumes:

1. Stop services:

   ```bash
   docker compose -f deploy/docker-compose.yml down
   ```

2. Recreate PostgreSQL only:

   ```bash
   docker compose -f deploy/docker-compose.yml up -d postgres
   ```

3. Copy the database dump into the PostgreSQL container:

   ```bash
   docker compose -f deploy/docker-compose.yml cp \
     "$HAZE_SYNC_BACKUP_DIR/haze_sync_metadata.dump" \
     postgres:/tmp/haze_sync_metadata.dump
   ```

4. Restore into the target database:

   ```bash
   docker compose -f deploy/docker-compose.yml exec -T postgres \
     pg_restore \
       -U "${POSTGRES_USER:-haze_sync}" \
       -d "${POSTGRES_DB:-haze_sync}" \
       --clean \
       --if-exists \
       /tmp/haze_sync_metadata.dump
   ```

5. Restore object-store data into the matching local volume:

   ```bash
   docker run --rm \
     --volume haze-sync_server_objects:/target \
     --volume "$HAZE_SYNC_BACKUP_DIR:/backup:ro" \
     alpine:3.20 \
     sh -c 'cd /target && tar -xzf /backup/haze_sync_objects.tgz'
   ```

6. Remove temporary dump from the container:

   ```bash
   docker compose -f deploy/docker-compose.yml exec -T postgres \
     rm -f /tmp/haze_sync_metadata.dump
   ```

7. Start the server and run health/readiness checks:

   ```bash
   docker compose -f deploy/docker-compose.yml up -d server
   curl -fsS http://127.0.0.1:8080/health
   curl -fsS http://127.0.0.1:8080/ready
   ```

The `--clean --if-exists` restore flags are intended for an explicitly approved empty or replaceable target only. Do not run them against a live production database without a reviewed recovery plan.

## Production-style restore placeholder

Production restore must use an operator-reviewed plan with real target names supplied outside the repository. The shape is:

```bash
pg_restore \
  --dbname=<target-db-name> \
  --clean \
  --if-exists \
  /secure/operator/backups/haze-sync/<timestamp>/haze_sync_metadata.dump
```

Then restore the matching object-store archive or snapshot to the accepted object-store path for the same timestamp.

Never restore a database backup from one timestamp with an object-store archive from another timestamp unless an explicit incident plan documents why the mismatch is acceptable.

## Dry-run and checklist verification

Before treating a backup as usable, verify:

```text
[ ] Backup directory is outside the repository.
[ ] Database dump exists and is non-empty.
[ ] Object-store archive/snapshot exists, or absence is intentionally recorded.
[ ] Services were stopped or quiesced during capture.
[ ] Code revision and migration source were recorded.
[ ] No production URL/password/token appears in notes or reports.
[ ] Restore target was identified as empty or replaceable before destructive restore commands.
[ ] Restore uses DB and object-store artifacts from the same backup window.
[ ] /health and /ready were checked after restore where the server was started.
[ ] Any skipped check is recorded honestly.
```

A successful dry-run should be recorded with sanitized evidence only. Do not attach dumps, object-store archives, logs containing secrets, provider payloads, or full rendered configs to public PRs or reports.

## What this runbook deliberately does not do

This runbook does not:

```text
run migrations automatically
create deploy scripts
create backup archives in the repository
restore production hosts
rotate or generate credentials
define Storage schema contents
define object-store retention/cleanup
enable provider services
enable Worktree runtime
claim production readiness
```

Those require later accepted component contracts and deployment phases.
