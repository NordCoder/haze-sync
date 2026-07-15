# W1-DEP-P5A — Worktree runtime/config deployment fan-in

Before starting, name this worker chat exactly:

`deployment — W1 DEP-P5A Worktree Runtime Fan-In`

Component: deployment
Path: deploy
Branch: component/deployment
PR: #52
Role: implementation-worker
Phase: DEP-P5A-WORKTREE-RUNTIME-CONFIG-FAN-IN

Do not merge, change draft state, rewrite history, modify sibling branches, or perform unrelated cleanup.

## Accepted baseline

- synchronized Deployment SHA: `54e0b8b84e06e7475dc99ea25b22ddd248bb98c2`;
- exact main ancestor: `c1e69a664388b0cba028170e8398b9088218957d`;
- migration architecture report blob: `737a3395945d94479c19b27d771384b57c267d06`;
- migration policy status: `ARCHITECT_ACCEPT`;
- post-sync Component CI run `29400618638`, number `1954`, success.

Accepted product contracts:

- Server Worktree runtime and HTTP surface SHA `50461354c18ddc4d2e47202d9303b4358a27ee45`;
- API-P8 SHA `56ae94570441d68715f34b5d54381a0fc4d7c231`;
- CLI-P6A SHA `d33fa105398d9731bfc1b7927e98d5d085c6fe59`;
- accepted Server config keys:
  - `HAZE_SYNC_WORKTREE_PATH`;
  - `HAZE_SYNC_WORKTREE_ADAPTER_MODE`;
- accepted container Worktree path: `/var/lib/haze-sync/worktree`.

## Fixed architecture decisions

The worker must implement these decisions, not redesign them:

1. The base local Compose topology remains safe and disabled by default.
2. Worktree hosting is opt-in through an explicit Compose override, not silently enabled in the base file.
3. The opt-in override requires an operator-supplied host path. No repository-relative or production-looking default host path is allowed.
4. The host path maps exactly to `/var/lib/haze-sync/worktree` in the Server container.
5. The bind mount is read-only by default. Write access requires an explicit operator override after permissions and rollout mode are reviewed.
6. `HAZE_SYNC_WORKTREE_ADAPTER_MODE` remains `disabled` by default and must be explicitly changed through an untracked operator environment.
7. Do not default to `export_only`, `bidirectional`, or any write-capable rollout.
8. The sole migration execution owner remains the human/operator running the documented manual SQLx command.
9. Server, Compose and Docker startup remain migration-free.
10. Storage owns schema/migration contents. Deployment owns only operational sequencing, configuration examples and runbooks.
11. All writers must be stopped or explicitly quiesced during coordinated backup, migration and restore windows.
12. Rollback remains operator-approved coordinated restore; no automatic down/reset/drop/destructive fallback.

## Required deliverables

### 1. Opt-in Compose override

Add one focused override file under `deploy/` for Worktree hosting.

It must:

- extend only the existing `server` service;
- require `HAZE_SYNC_WORKTREE_HOST_PATH` when the override is explicitly used;
- mount that source to `/var/lib/haze-sync/worktree`;
- preserve the accepted `HAZE_SYNC_WORKTREE_PATH` container value;
- consume `HAZE_SYNC_WORKTREE_ADAPTER_MODE` without renaming it;
- keep mode default `disabled`;
- keep the bind read-only by default through an explicit boolean placeholder;
- add no migration command, entrypoint, healthcheck side effect, background service, adapter service or secret value;
- not alter PostgreSQL, object-store or public bind topology.

Use a clear file name such as `deploy/docker-compose.worktree.yml`. Do not add multiple competing topology variants.

### 2. Environment placeholders

Update `.env.example` with local placeholder keys only:

- `HAZE_SYNC_WORKTREE_HOST_PATH=`;
- a boolean read-only bind control, defaulting to the safe read-only value;
- retain `HAZE_SYNC_WORKTREE_ADAPTER_MODE=disabled`.

Comments must state:

- the host path is required only when the Worktree override is used;
- real paths belong in an untracked `.env` or operator environment;
- production paths, vault contents and secrets must not be committed;
- write-capable modes require explicit operator approval and compatible permissions.

Do not add tokens, database URLs, credentials or real machine paths.

### 3. Worktree deployment runbook

Add or update a focused Deployment runbook that documents:

- base Compose remains Worktree-disabled;
- exact opt-in Compose invocation shape;
- required host path and container target;
- container UID `10001` permission implications;
- default read-only mount and explicit write-access gate;
- accepted mode vocabulary without inventing new modes;
- safe rollout order beginning from `disabled`;
- no automatic bidirectional enablement;
- Server-owned runtime startup/shutdown and public status endpoints;
- process health, readiness, Worktree status and rollout approval as distinct checks;
- stop/quiesce requirements before backup/migration/restore;
- coordinated PostgreSQL/object-store/worktree recovery-window rule;
- no implicit migrations and no automatic rollback;
- sanitized output and secret handling.

Do not claim that CLI-P6A has a concrete live HTTP transport if it remains deferred. It may be referenced as an accepted command contract, not as proven deployment transport.

### 4. Existing documentation alignment

Update only the minimum affected Deployment documentation so it no longer incorrectly says Worktree hosting is wholly unavailable after this phase.

At minimum reconcile:

- `deploy/docker-compose.yml` comments;
- `deploy/docs/host-directory-layout.md`;
- `deploy/docs/migrations-backup-restore.md` where Worktree writer/recovery sequencing is described;
- local/server Compose docs if their startup examples need the opt-in override.

Preserve explicit local-only and non-production language.

Do not perform a broad documentation rewrite. The architect noted older implementation-plan current-state drift; update it only if a narrow factual correction is necessary for this phase.

## Safety and scope

Allowed:

- `deploy/docker-compose.worktree.yml` or one equivalent focused override;
- `deploy/docker-compose.yml` comments/minimal alignment;
- `.env.example` placeholders;
- focused `deploy/docs/**` changes;
- Deployment control report.

Forbidden:

- Server, Storage, Worktree, API, CLI, GDrive or Obsidian product changes;
- migration files or migration execution;
- automatic migration runner or startup hook;
- real secrets, production `.env`, tokens, URLs, dumps or archives;
- creation/chmod of real host paths;
- remote deployment, SSH, DNS or cloud automation;
- new runtime services;
- cleanup, retention or destructive restore automation;
- automatic write-capable or bidirectional mode;
- workflow changes;
- unrelated Deployment cleanup.

## Validation

Validate, where tooling permits:

1. Base `docker compose -f deploy/docker-compose.yml config` still succeeds and remains Worktree-disabled without a host bind.
2. Base plus Worktree override validates when supplied only safe temporary placeholder values.
3. Override requires a host path instead of silently choosing one.
4. Resolved Worktree target is exactly `/var/lib/haze-sync/worktree`.
5. Default resolved bind is read-only and mode is disabled.
6. Explicit write-bind configuration is possible only through the documented operator override.
7. No migration command/entrypoint/startup hook was introduced.
8. No tracked secret or real host path was introduced.
9. Existing Caddy and Deployment checks remain green where available.
10. Authoritative Component CI succeeds on the exact final code-bearing SHA.

If Docker/Caddy tooling is unavailable, report that honestly; do not fabricate validation. CI success remains required.

## Report

Write `deploy/control/report.md` with:

- `REPORT_TYPE: IMPLEMENTATION`;
- `phase_id: DEP-P5A-WORKTREE-RUNTIME-CONFIG-FAN-IN`;
- `chat_name: deployment — W1 DEP-P5A Worktree Runtime Fan-In`;
- status `SELF_ACCEPT`, `NEEDS_FIX`, `BLOCKED_BY_CONTRACT`, `BLOCKED_BY_SCOPE`, or `BLOCKED_BY_TOOLING`.

Record:

- exact changed paths;
- override behavior and resolved mount/mode defaults;
- documentation alignment;
- preservation of migration ownership policy;
- validation evidence;
- secrecy checks;
- final code-bearing SHA and exact CI run.

Do not claim CLEAN_ACCEPT or merge readiness. A focused Deployment functional/security review follows.
