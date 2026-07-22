# Worktree Compose Override Runbook

## Scope

`deploy/docker-compose.worktree.yml` is an explicit opt-in override for mounting an operator-supplied Worktree directory into the existing `server` service.

The base `deploy/docker-compose.yml` remains local-only, has no Worktree host bind, and keeps `HAZE_SYNC_WORKTREE_ADAPTER_MODE=disabled`. The override does not add a service, migration runner, startup hook, secret, background task, reverse proxy, public bind, or automatic rollout behavior.

## Fixed topology

```text
operator-supplied host directory
  -> bind mount, read-only by default
  -> /var/lib/haze-sync/worktree in haze-sync-server
```

Accepted configuration keys remain unchanged:

```text
HAZE_SYNC_WORKTREE_PATH
HAZE_SYNC_WORKTREE_ADAPTER_MODE
```

The container target is always:

```text
/var/lib/haze-sync/worktree
```

## Required operator environment

The override requires:

```text
HAZE_SYNC_WORKTREE_HOST_PATH=<operator-supplied-host-path>
```

Place the real value in an untracked `.env` file or operator environment. Do not commit production paths, vault contents, credentials, database URLs, tokens, dumps, archives, or machine-specific home paths.

Safe defaults are:

```text
HAZE_SYNC_WORKTREE_ADAPTER_MODE=disabled
HAZE_SYNC_WORKTREE_BIND_READ_ONLY=true
```

The host-path variable has no tracked default. Compose must fail interpolation when the override is used without it.

## Validation and invocation

Validate the base topology:

```bash
docker compose -f deploy/docker-compose.yml config
```

It must contain no Worktree host bind and must keep the adapter disabled.

Validate the opt-in topology with a safe temporary path:

```bash
HAZE_SYNC_WORKTREE_HOST_PATH=/tmp/haze-sync-worktree-check \
HAZE_SYNC_WORKTREE_BIND_READ_ONLY=true \
HAZE_SYNC_WORKTREE_ADAPTER_MODE=disabled \
docker compose \
  -f deploy/docker-compose.yml \
  -f deploy/docker-compose.worktree.yml \
  config
```

Do not paste rendered configuration into public reports when the operator environment contains secrets or sensitive paths.

Start the existing PostgreSQL and Server services with the override:

```bash
docker compose \
  -f deploy/docker-compose.yml \
  -f deploy/docker-compose.worktree.yml \
  up --build -d postgres server
```

The override does not start migrations. The operator must run the documented manual SQLx migration procedure separately.

## Permissions and UID 10001

The Server container runs as numeric UID `10001`.

For the default read-only bind, UID `10001` must be able to traverse the parent directories and read the required Worktree files. Do not use world-writable permissions to bypass ownership problems.

Before changing the bind to writable, verify all of the following:

- the selected adapter mode is explicitly approved;
- host ownership and group membership are compatible with UID `10001`;
- required directory and file modes are narrowly scoped;
- the Worktree is not nested inside the object store, backup root, repository checkout, secret root, log root, or runtime temp root;
- backup and recovery handling for user-authored or unreplicated Worktree content is accepted;
- operators understand which process has write authority.

## Read-only and write-access gate

The tracked default is:

```text
HAZE_SYNC_WORKTREE_BIND_READ_ONLY=true
```

An operator may set the untracked value to `false` only after the permission and rollout review above. Writable mounting alone does not authorize a write-capable adapter mode.

Likewise, changing `HAZE_SYNC_WORKTREE_ADAPTER_MODE` does not automatically change the mount to writable. Mode and filesystem access are independent gates and must both be reviewed.

Do not default to `export_only`, `bidirectional`, or another write-capable rollout. Begin from `disabled` and advance only through an explicit operator-approved plan using the accepted mode vocabulary.

## Runtime and status checks

Keep these checks distinct:

1. **Process health** — `/health` proves the Server HTTP process responds.
2. **Readiness** — `/ready` reports sanitized Server dependency readiness.
3. **Worktree status** — use the accepted Server/API status surface to inspect Worktree runtime state without exposing secrets or raw paths publicly.
4. **Rollout approval** — an operator decision that permissions, backup state, mode, and recovery plan are acceptable.

A healthy or ready process does not prove that Worktree permissions, mode, synchronization direction, backup coverage, or production rollout are safe.

The Server owns runtime startup, shutdown, readiness, and status semantics. Deployment only wires configuration and documents operator sequencing.

CLI-P6A may be treated as an accepted command contract. Do not assume or claim a proven live HTTP transport where that transport remains deferred.

## Safe rollout order

Use this minimum sequence:

1. Keep the base topology and adapter mode `disabled`.
2. Prepare and inspect the host path without changing repository files.
3. Validate the override with a read-only bind and disabled mode.
4. Start the Server and check process health and readiness.
5. Inspect sanitized Worktree status.
6. Confirm backup coverage and rollback inputs.
7. Approve a non-disabled mode separately.
8. Approve writable access separately when the chosen mode requires it.
9. Enable only one bounded rollout change at a time and verify status after each change.

There is no automatic bidirectional enablement.

## Migration, backup, and recovery sequencing

The migration execution owner remains the human/operator using the documented manual SQLx command. Storage owns migration contents. Deployment owns only operational sequencing.

Before backup, migration, or restore:

- stop all writers or place them in an explicitly accepted quiesced state;
- record the code revision, migration state, Server state, adapter mode, and whether the Worktree bind is read-only or writable;
- capture PostgreSQL metadata, object-store data, and Worktree data from one coordinated recovery window when Worktree content may be authoritative or unreplicated;
- keep backup artifacts outside the object store, Worktree, repository, logs, and runtime temp paths.

Startup remains migration-free. Compose does not run `sqlx migrate`, change entrypoints, or add hidden migration services.

Rollback is an operator-approved coordinated restore. Do not add automatic `down -v`, database reset, schema drop, object deletion, Worktree cleanup, or destructive fallback behavior.

## Shutdown

Stop the Compose services without deleting data:

```bash
docker compose \
  -f deploy/docker-compose.yml \
  -f deploy/docker-compose.worktree.yml \
  down
```

Confirm Server shutdown and writer quiescence before taking coordinated backups or beginning restore work.

## Safety boundaries

Never commit:

```text
production .env files
real host paths tied to a machine or user
vault or Worktree contents
database URLs or passwords
bearer or OAuth tokens
certificate private material
database dumps
object-store archives
backup archives
logs containing operational data
```

Sanitize status and command output before sharing it. Do not expose raw provider payloads, file contents, database errors, stack traces, secret paths, bearer values, or machine-specific paths.

## Evidence boundary

Compose interpolation and syntax validation prove only that the configuration renders. They do not prove target-host permissions, migration correctness, backup consistency, Worktree behavior, public access safety, or production readiness.
