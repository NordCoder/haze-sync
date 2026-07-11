# Durable Worktree state and export cursor

## Ownership

Storage owns only durable persistence primitives:

- a versioned Worktree runtime-instance binding keyed by `AdapterId`;
- a non-public SHA-256 fingerprint supplied for an already-normalized root;
- versioned per-instance path state keyed by `(adapter_id, VaultPath)`;
- exact, locked, contiguous advancement of `adapter_cursors.last_core_seq`;
- safe row validation and redacted repository errors.

Server owns transaction timing and composes path-state writes with cursor
advancement. Worktree owns root normalization, scanning, reconciliation,
materialization, echo handling, delete planning and runtime scheduling. Core owns
revision, conflict and delete policy.

Raw roots, root fingerprints and raw external cursor JSON are not public status or
API fields.

## Schema version 1

`worktree_instances` binds one stable adapter identity to:

- `root_fingerprint`: canonical `sha256:<64 lowercase hex>`;
- `state_format_version = 1`;
- creation and update timestamps.

Binding is create-or-verify. Rebinding an existing adapter to another root
fingerprint or unsupported version fails closed. Storage never derives the
fingerprint from a path and never stores the raw root.

`worktree_state` is keyed by `(adapter_id, path)` and stores:

- `state_kind`: `present` or `tombstoned`;
- `state_format_version = 1`;
- required last-applied authoritative revision;
- required content hash for `present`, no content hash for `tombstoned`;
- optional reconciliation observation version 1, non-negative byte size and
  optional modification timestamp;
- creation and update timestamps.

Observation updates are guarded by adapter, path, expected revision and expected
content hash. A stale observation therefore updates no row and cannot overwrite
newer authoritative state.

Snapshot reads are adapter-scoped, path-ordered and bounded. Pagination uses the
last returned normalized `VaultPath` as an exclusive cursor. Storage does not
hard-delete Worktree state.

## Migration from the path-only schema

Migration `0010_worktree_durable_state.sql` accepts the historical path-only
`worktree_state` table only when it is empty. Non-empty rows cannot be assigned to
an adapter identity or normalized-root fingerprint without inventing facts.
Therefore the migration raises before `drop table` and PostgreSQL rolls back the
whole migration.

Operator procedure for a non-empty legacy table:

1. stop the Worktree runtime;
2. export or inspect the legacy rows privately;
3. choose the correct adapter identity and normalized root;
4. migrate rows through an explicitly reviewed operator migration or clear them
   only after confirming they are reproducible cache-like state;
5. rerun repository migrations;
6. bind the runtime instance before the first cycle.

Storage test support recognizes exactly three states:

- no owned tables: apply all migrations;
- exact pre-P10 schema with an empty legacy table: apply only migration 0010;
- exact P10 schema: validate and continue.

Partial schemas, altered Worktree columns, a missing cursor constraint and
non-empty legacy Worktree state fail with safe errors.

## Exact-contiguous export cursor

The Worktree export checkpoint remains `adapter_cursors.last_core_seq`.

The strict API is transaction-only:

1. initialize the cursor at zero when explicitly requested;
2. lock the row with `SELECT ... FOR UPDATE`;
3. require the persisted value to equal the caller's expected current sequence;
4. require `next_sequence == expected_current + 1`;
5. reject negative values, equality/regression, gaps, stale expected values,
   missing cursors and integer overflow;
6. update only the sequence and success/update timestamps;
7. return a safe summary without raw external cursor JSON.

The older broad monotonic API remains for source compatibility with adapters that
persist non-Worktree progress. Worktree export must use the exact-contiguous API.

## Transaction and crash invariants

Server must use one caller-owned transaction for the durable checkpoint of each
exported operation:

1. lock and read the cursor;
2. materialize exactly one authoritative operation through Worktree-owned
   filesystem code;
3. persist the resulting present/tombstoned path state;
4. advance the cursor from `N` to exactly `N + 1`;
5. commit.

Rollback leaves both path state and cursor unchanged. A crash after filesystem
materialization but before commit replays the same operation; Worktree may report
`AlreadyCurrent`, after which Server persists state and advances the checkpoint.
The cursor is never advanced beyond an unprocessed sequence.

## Tests

Pure tests run in normal workspace CI. Mandatory PostgreSQL evidence is explicit
and never silently skipped:

```bash
export HAZE_SYNC_TEST_DATABASE_URL='<dedicated test database URL>'
cargo test -p haze-sync-storage --features test-support -- --ignored
```

The ignored tests require the dedicated database and cover:

- fresh, pre-P10 and current migration states;
- rejection of non-empty legacy path-only rows;
- instance bind replay and mismatch;
- per-adapter path isolation and deterministic snapshots;
- guarded observations and transaction rollback;
- cursor initialization, missing/stale/gap/regression handling;
- rollback and concurrent exact-advance races.
