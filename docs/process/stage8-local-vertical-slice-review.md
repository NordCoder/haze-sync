# Stage 8 — Worktree + Obsidian Local Vertical Slice Review

Date: 2026-07-23

## Verdict

```text
STAGE_8_WORKTREE_LOCAL_SLICE: PASS
STAGE_8_OBSIDIAN_LIVE_CLIENT_SLICE: PASS
R6_2_LOCAL_VERTICAL_SLICE: PASS
OBSIDIAN_DESKTOP_DEVICE_ACCEPTANCE: NOT_RUN
OBSIDIAN_IOS_ACCEPTANCE: NOT_RUN
REAL_VAULT_ACCEPTANCE: NOT_RUN
V1_COMPLETION: BLOCKED
ROLLOUT_READINESS: BLOCKED
PR_68_STATE: KEEP_DRAFT
```

Stage 8 closes the repository-local Worktree + Obsidian vertical slice against an actual PostgreSQL database and an actual Haze Sync Server process. It does not close installed desktop/mobile acceptance or real-vault rollout.

## Accepted topology

The CI slice starts exactly these services:

```text
postgres
server
```

The Server is configured with:

- Worktree adapter mode `bidirectional`;
- an explicit ephemeral host bind mounted read-write at `/var/lib/haze-sync/worktree`;
- Server runtime UID `10001`;
- PostgreSQL-backed Storage and migrations;
- separately provisioned Obsidian, admin, and Worktree adapter credentials.

The ephemeral Worktree bind is used only for CI. It is removed after the scenario, including files created by container UID 10001.

## Accepted end-to-end path

The successful slice executes this sequence:

1. wait for PostgreSQL readiness;
2. apply all SQL migrations;
3. register Obsidian, admin, and Worktree adapters;
4. start the real Server with the hosted Worktree runtime enabled bidirectionally;
5. wait for `/health`, `/ready`, and Worktree hosted-runtime readiness;
6. use the production Obsidian HTTP client to query Server info;
7. use the production Obsidian HTTP client to create `stage8-local-vertical-slice.md`;
8. wait for an automatic Worktree export to materialize the Markdown file in the host bind;
9. wait until the exporting Worktree cycle has completed and checkpointed;
10. atomically replace the local Markdown file with a host-side edit;
11. allow watcher-triggered delivery with periodic full-cycle fallback;
12. read the imported revision through the production Obsidian HTTP client;
13. verify content hash and revision advancement;
14. verify the Worktree-authored upsert appears in the live changes feed;
15. verify Worktree lifecycle, readiness, counters, service set, UID, and secret-safe logs;
16. tear down containers, volumes, and ephemeral host data.

## Production contracts exercised

The slice uses production code paths rather than test-only request stubs:

- `HazeSyncApiClient.getServerInfo`;
- `HazeSyncApiClient.putFile`;
- `HazeSyncApiClient.getFile`;
- `HazeSyncApiClient.getChanges`;
- PostgreSQL-backed Server routes;
- Server-hosted Worktree runtime;
- automatic Worktree export cycles;
- automatic watcher/periodic Worktree import cycles;
- Worktree status admin endpoint;
- Core revision, hash, and operation-log semantics.

## Worktree lifecycle contract

In non-dry-run modes, the manual `sync-once` surface is intentionally unavailable. Stage 8 therefore validates automatic production cycles rather than calling the dry-run-only manual trigger.

Accepted final status includes:

```text
configured_mode = bidirectional
host_lifecycle = running
readiness = ready
readiness_reason = running
manual_availability = unavailable
cycles_failed = 0
cycles_completed >= 2
```

## Fixture permission contract

The local host mutation must be readable by Server UID 10001. The accepted CI fixture uses normal Markdown file mode `0644` inside an ephemeral operator-controlled bind.

An earlier `0600` fixture owned by the runner was correctly unreadable to UID 10001 and was safely skipped by the Worktree scanner without recording a runtime cycle failure. That behavior was not treated as a product defect; the fixture was corrected to match the supported host-file permission model.

## Security evidence

The slice verifies:

- Server runs as UID 10001;
- Worktree bind is explicit and read-write only for this opt-in test;
- PostgreSQL password is absent from Server logs;
- Obsidian bearer token is absent from Server logs;
- admin bearer token is absent from Server logs;
- Worktree bearer token is absent from Server logs;
- no real credentials or user vault data are used;
- all temporary containers, volumes, files, and UID-owned directories are removed.

## Validation evidence

Successful isolated exact-runtime verifier:

```text
base runtime head: 001ff8f9dec18d9b6a12a257477783c6591cfbe0
verifier head:     6374214c007c039c8ca8640466b29f7fef67b116
Hardening run:     29991120879 — success
Node run:          29991120869 — success
```

The verifier branch differs from the integration runtime tree only by one inert dispatch marker. The successful Hardening run includes:

- locked Cargo metadata/check/test/clippy;
- existing deployment Compose smoke;
- Worktree + Obsidian local vertical slice;
- successful diagnostics finalization.

## Failures resolved during Stage 8

1. Docker Compose omits `read_only` when a bind is read-write. The topology assertion now treats an absent field as `false` and still rejects `true`.
2. Worktree manual synchronization is unavailable outside dry-run by contract. The test now follows automatic production cycles.
3. The host mutation originally raced the exporting cycle. The test now waits for the completed export checkpoint before editing the local file.
4. The host mutation originally used runner-owned mode `0600`, making it unreadable to Server UID 10001. The non-secret Markdown fixture now uses mode `0644`.
5. Cleanup now removes UID-owned ephemeral runtime directories through the CI runner's privileged cleanup path.

## Explicitly not accepted

Stage 8 does not prove:

- installed Obsidian desktop plugin behavior inside the Obsidian application;
- iOS foreground/background behavior;
- mobile transport recovery;
- user-visible conflict presentation;
- real user vault compatibility;
- GDrive live-provider synchronization;
- production deployment or rollout readiness.

## Remaining completion boundaries

The candidate remains blocked on:

1. full R6-1 GDrive live-cycle composition and provider acceptance;
2. installed desktop and iOS Obsidian acceptance;
3. R6-3 operational CLI bootstrap/preflight/recovery surfaces;
4. R6-4 executed backup, restore, release artifact, and rollback evidence;
5. R6-5 staged real-vault rollout acceptance.

## Review state

PR #68 must remain open and draft. Stage 8 is accepted as repository-local integration evidence only and must not be represented as V1 completion or rollout readiness.
