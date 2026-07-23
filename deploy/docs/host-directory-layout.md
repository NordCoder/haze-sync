# Host Directory Layout and Permissions

## Scope

This DEP-P5 runbook defines production-style host path placeholders, ownership expectations, permission boundaries, backup classification, and validation checks for Haze Sync.

It is documentation only. It does not create directories, users, groups, bind mounts, secrets, backup archives, cleanup jobs, or Worktree runtime behavior on a host.

The base local Compose scaffold still uses Docker named volumes, has no Worktree host bind, and keeps Worktree runtime disabled. The accepted opt-in `deploy/docker-compose.worktree.yml` override may bind an operator-supplied host path to `/var/lib/haze-sync/worktree`; it does not provision the source path automatically and remains read-only and disabled by default. See `deploy/docs/worktree-compose.md`.

## Accepted Server path keys

Deployment consumes the existing Server configuration keys without renaming them:

```text
HAZE_SYNC_OBJECT_STORE_PATH
HAZE_SYNC_WORKTREE_PATH
```

Production-style placeholder values:

```text
HAZE_SYNC_OBJECT_STORE_PATH=/srv/haze-sync/objects
HAZE_SYNC_WORKTREE_PATH=/srv/haze-vault/worktree
```

The Server defaults remain suitable for host-local development:

```text
HAZE_SYNC_OBJECT_STORE_PATH=./data/objects
HAZE_SYNC_WORKTREE_PATH=./data/worktree
```

Do not replace accepted Server config keys with deployment-specific aliases.

## Identity model

Use dedicated identities rather than personal accounts.

Example identities:

```text
service user: haze-sync
service group: haze-sync
shared vault group: haze-vault
backup operator: root or a dedicated backup account
```

These names are examples. Operators may choose different names, but the resulting access model must preserve the same boundaries.

Any group granted read access to secret files must be dedicated to the exact runtime identities that require those secrets. Do not reuse a broad operator, login, or shared application group for secret access.

The current server container runs as the non-root user `haze-sync` with numeric UID `10001`. An accepted Worktree host bind must therefore either:

- grant the container UID `10001` the required read access and, only when explicitly approved, write access; or
- explicitly coordinate a different runtime user/group mapping in a later deployment phase.

Do not make persistent directories world-writable to work around UID/GID mismatches.

## Directory classes

| Path placeholder | Class | Primary consumer | Suggested owner/group | Suggested directory mode | Recovery role |
| --- | --- | --- | --- | --- | --- |
| `/srv/haze-sync/objects` | persistent application data | Server object store | `haze-sync:haze-sync` | `0750` | required with PostgreSQL metadata |
| `/srv/haze-vault/worktree` | user-visible worktree data | opt-in Worktree runtime and authorized vault users | `<vault-owner>:haze-vault` | `2770` when shared-group writes are required | back up when it may contain authoritative or unreplicated content |
| `/etc/haze-sync` | non-secret configuration | operator and service | `root:haze-sync` | `0750` | back up sanitized configuration where useful |
| `/opt/haze-sync/secrets` | secret material | operator and explicitly authorized service processes | `root:<dedicated-secret-group>` | prefer directory `0700` and files `0600`; use `0750`/`0640` only when dedicated group access is required | back up separately through an encrypted/secret-manager process |
| `/var/log/haze-sync` | service logs | service and log operator | `haze-sync:haze-sync` or logging agent group | `0750` | not required for state recovery |
| `/var/backups/haze-sync` | database/object-store/worktree backup sets | backup operator | `root:<backup-group>` | `0700` or narrowly scoped `0750` | recovery source; copy off-host according to operator policy |
| `/run/haze-sync` | short-lived runtime state | service | `haze-sync:haze-sync` | `0750` or stricter | never back up |
| `/var/tmp/haze-sync` | optional large temporary workspace | service | `haze-sync:haze-sync` | `0700` | never back up |

Permission values are conservative examples, not a substitute for host-specific security review. Group-readable secret modes are acceptable only when the group is dedicated and contains no unrelated identities.

## Separation invariants

Keep path classes physically and logically separate:

- object-store data must not live inside the repository checkout;
- the worktree must not be nested inside the object store;
- the object store must not be nested inside the worktree;
- backups must not be stored inside the object store, worktree, repository, or runtime temp directories;
- secrets must not be stored under public web roots, logs, worktree data, or tracked configuration directories;
- logs and runtime temp files must not share the object-store path;
- production `.env` files, when used, must remain untracked and permission-restricted;
- symlinks that escape an approved root require explicit operator review.

These boundaries reduce accidental recursive backups, credential disclosure, self-ingestion, and cleanup mistakes.

## Object-store permissions

`/srv/haze-sync/objects` is persistent Server-owned data.

Expected properties:

- writable by the Server runtime identity;
- not writable by unrelated interactive users;
- not world-readable or world-writable;
- located on storage with enough capacity and stable filesystem semantics;
- included in the same stopped/quiesced recovery window as PostgreSQL metadata;
- excluded from source-control checkouts and public diagnostic bundles.

The current Compose service uses the container path `/var/lib/haze-sync/objects` backed by the `server_objects` named volume. DEP-P5 does not replace that volume with `/srv/haze-sync/objects`.

A future bind-mount phase may map a host path explicitly, for example:

```text
host /srv/haze-sync/objects -> container /var/lib/haze-sync/objects
```

That mapping must not be added until ownership and numeric UID/GID behavior are validated on the target host.

## Worktree permissions

`/srv/haze-vault/worktree` is the production-style placeholder for an operator-supplied `HAZE_SYNC_WORKTREE_HOST_PATH`. The accepted opt-in override maps that source exactly to the Server container target:

```text
operator-supplied host path -> /var/lib/haze-sync/worktree
```

The base Compose file has no Worktree bind and keeps `HAZE_SYNC_WORKTREE_ADAPTER_MODE=disabled`. When the override is used:

- the host source must be supplied through an untracked `.env` or operator environment;
- the bind is read-only by default;
- UID `10001` must be able to traverse the parent directories and read required files;
- write access requires a separate explicit operator decision, compatible ownership and permissions, and an approved write-capable rollout mode;
- changing the adapter mode does not independently authorize a writable bind, and changing the bind to writable does not independently authorize a write-capable mode;
- the directory must not be exposed to unrelated system users;
- the directory must not be assumed safely reconstructible from another source.

When shared writes are explicitly accepted, a setgid directory such as mode `2770` can preserve a shared vault group on newly created entries. The exact file umask and write model must remain coordinated with the accepted Worktree contract.

Back up the worktree whenever it may contain user-authored, authoritative, or otherwise unreplicated content. Capture it in the same stopped/quiesced recovery window as PostgreSQL metadata and object-store data. See `deploy/docs/worktree-compose.md` and `deploy/docs/migrations-backup-restore.md`.

## Configuration and secrets

Use separate roots for non-secret configuration and secret material:

```text
/etc/haze-sync
/opt/haze-sync/secrets
```

Non-secret configuration may include service settings, public endpoint names, or paths. Secret material may include database credentials, OAuth token files, private keys, or future provider credentials.

Rules:

- never place real secrets in tracked `.env.example`;
- never commit production `.env`, OAuth token files, TLS private keys, database URLs, or credential exports;
- secret files should be readable only by the operator and the exact runtime identity that needs them;
- prefer owner-only `0700` directories and `0600` files when one identity can consume the secret;
- use group-readable `0750` directories and `0640` files only with a dedicated secret group containing the exact authorized identities;
- secret directories must not be writable by the service unless a future rotation contract explicitly requires it;
- do not print secret file contents or rendered database URLs in reports, logs, or CI artifacts;
- prefer a secret manager or encrypted operator backup over copying plaintext secrets into general backup archives.

DEP-P5 documents placement only. It does not define secret generation, rotation, or provider-specific loading semantics.

## Logs

`/var/log/haze-sync` is a placeholder for file-based logs when a future host deployment chooses them.

Rules:

- prefer structured, sanitized logs;
- never log database URLs, passwords, OAuth tokens, bearer tokens, raw provider payloads, vault contents, or secret file contents;
- do not rely on logs as a recovery source;
- keep logs outside object-store and worktree paths;
- define rotation and retention only in a later accepted operations phase.

A systemd deployment may use the journal instead of this directory. DEP-P5 does not choose a logging backend.

## Backups

`/var/backups/haze-sync` is the production-style placeholder for operator-owned recovery sets.

A complete recovery set may contain:

```text
PostgreSQL metadata dump
object-store archive or snapshot
worktree archive or snapshot when required by its authority model
sanitized manifest: timestamp, code revision, migration state, service state
```

Secret backups must be handled separately and encrypted or secret-manager-backed. Logs and runtime temp directories are not part of the normal recovery set.

The service runtime should not have general write access to the backup root. This prevents a compromised service from silently replacing recovery artifacts.

Use `deploy/docs/migrations-backup-restore.md` for stopped/quiesced backup and restore sequencing.

## Runtime temp paths

Use `/run/haze-sync` for short-lived runtime files such as PID files, Unix sockets, or small transient state when a future host service requires them.

Use `/var/tmp/haze-sync` only for larger temporary work that must survive a process restart but not a host recovery.

Rules:

- treat both paths as disposable;
- never store canonical object data, worktree data, secrets, or backups there;
- clear stale temp state only through an explicitly reviewed operational procedure;
- do not add automatic cleanup or retention jobs in DEP-P5.

## What must be backed up

| Path/data | Back up? | Notes |
| --- | --- | --- |
| PostgreSQL metadata | yes | coordinate with object store and stopped/quiesced writers |
| `/srv/haze-sync/objects` | yes | capture from the same recovery window as PostgreSQL |
| `/srv/haze-vault/worktree` | conditional but usually yes | required whenever user-authored, authoritative, or unreplicated data may exist; capture in the same recovery window |
| `/etc/haze-sync` | optional | only sanitized non-secret configuration; verify values before reuse |
| `/opt/haze-sync/secrets` | separately | encrypted/secret-manager process, not general data archive |
| `/var/log/haze-sync` | no for recovery | retain only for operations/security policy |
| `/run/haze-sync` | no | ephemeral |
| `/var/tmp/haze-sync` | no | disposable temporary data |

## What must never be committed

Never commit:

```text
production .env files
real database URLs or passwords
OAuth token files
bearer tokens or token hashes
TLS private keys
object-store contents
worktree/vault contents
database dumps
backup archives
logs containing operational data
runtime temp files
provider payload snapshots
host-specific permission dumps containing sensitive paths or identities
```

Tracked docs may contain only generic system path examples and placeholder identities.

## Provisioning checklist

Before starting a host deployment:

```text
[ ] Dedicated service identity exists.
[ ] Object-store root is writable by the service and not world-accessible.
[ ] Worktree root ownership matches the accepted Worktree mode.
[ ] Config and secret roots are separate.
[ ] Secret files are not service-writable unless explicitly required.
[ ] Any group-readable secret uses a dedicated group with only authorized identities.
[ ] Backup root is operator-owned and outside data/config/temp paths.
[ ] Runtime temp paths contain no persistent data.
[ ] HAZE_SYNC_OBJECT_STORE_PATH matches the approved object-store root.
[ ] HAZE_SYNC_WORKTREE_PATH remains /var/lib/haze-sync/worktree in the container.
[ ] HAZE_SYNC_WORKTREE_HOST_PATH is supplied only through an untracked operator environment when the override is used.
[ ] The Worktree bind defaults read-only; any write access is explicitly approved.
[ ] Container bind permissions account for UID 10001 or an explicitly coordinated runtime user.
[ ] No path points into the repository checkout.
[ ] No real secret or runtime artifact is tracked by Git.
```

## Read-only validation examples

Run these checks only on an authorized target host. They print path metadata, not file contents.

Inspect ownership and modes, including missing expected roots:

```bash
for path in \
  /srv/haze-sync/objects \
  /srv/haze-vault/worktree \
  /etc/haze-sync \
  /opt/haze-sync/secrets \
  /var/log/haze-sync \
  /var/backups/haze-sync \
  /run/haze-sync \
  /var/tmp/haze-sync
do
  if [ -e "$path" ]; then
    stat -c '%U:%G %a %n' "$path"
  else
    printf 'MISSING %s\n' "$path"
  fi
done
```

Check all approved roots for world-writable directories while safely skipping roots that are not provisioned yet:

```bash
for root in \
  /srv/haze-sync \
  /srv/haze-vault \
  /etc/haze-sync \
  /opt/haze-sync \
  /var/log/haze-sync \
  /var/backups/haze-sync \
  /run/haze-sync \
  /var/tmp/haze-sync
do
  [ -e "$root" ] || continue
  find "$root" -xdev -type d -perm -0002 -print
done
```

Any `MISSING` line or world-writable path requires operator review. Do not attach recursive directory listings, secret file names, ACL dumps, or raw configuration to public reports.

## Non-goals preserved

DEP-P5 and DEP-P5A do not:

```text
create host users or groups
create or chmod host directories
add a Worktree bind to the base Compose file
automatically enable Worktree runtime or a write-capable mode
change Server config keys
implement object-store behavior
add cleanup or retention automation
create secrets
create backup archives
change CI workflows
claim production readiness
```
