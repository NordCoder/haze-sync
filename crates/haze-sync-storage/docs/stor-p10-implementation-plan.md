# STOR-P10 implementation plan: durable Worktree state

## Goal

Provide durable, versioned, per-instance Worktree state and exact-contiguous
export checkpoints required by the accepted Server/Worktree architecture without
moving runtime or filesystem semantics into Storage.

## Deliverables

1. Migration 0010:
   - empty-legacy precondition;
   - `worktree_instances`;
   - per-instance replacement `worktree_state`;
   - non-negative adapter cursor constraint.
2. Passive row models with root-fingerprint redaction.
3. Instance repository:
   - bind-or-verify;
   - load by adapter.
4. Path-state repository:
   - load one path;
   - bounded deterministic snapshot;
   - present/tombstoned upserts;
   - revision/hash-guarded observation update;
   - no hard delete.
5. Cursor repository:
   - lock/read;
   - safe initialization;
   - exact expected-to-next advancement;
   - regression/gap/stale/missing/overflow rejection;
   - safe summary output.
6. Test support for fresh, exact pre-P10 and exact current schemas.
7. Pure tests plus explicit mandatory PostgreSQL tests.
8. Component contract, dependency, migration and transaction documentation.

## Mini-phases

### P10.1 — Schema and compatibility

- Add migration with fail-before-drop legacy row guard.
- Update table/migration metadata.
- Extend test migration embedding and schema validation.

Acceptance: no default instance is invented; current/pre/current schema states are
deterministic.

### P10.2 — Models and safe errors

- Replace stale path-only Worktree row.
- Add versioned instance/path rows.
- Add redacted binding/row debug behavior.
- Add safe mismatch/version/cursor errors.

Acceptance: no raw root/fingerprint/cursor payload in public formatting.

### P10.3 — Worktree repositories

- Bind/verify instance.
- Load/upsert per-instance path states.
- Add bounded path-ordered snapshots.
- Add guarded observations.

Acceptance: adapter isolation, row consistency and caller-owned transaction
compatibility.

### P10.4 — Exact cursor repositories

- Preserve broad monotonic compatibility API.
- Add transaction-only lock/init/exact advance.
- Reject all unsafe transition classes.

Acceptance: only `N -> N+1` succeeds after locking expected `N`.

### P10.5 — Evidence and docs

- Pure validation/SQL tests in Component CI.
- Explicit ignored strict PostgreSQL tests for migrations, instances, snapshots,
  observations, rollback and races.
- Document the exact database command and infrastructure blocker honestly.

## Non-goals

- Server executor/runtime implementation;
- Worktree scanning, reconciliation policy, materialization or repair;
- Core policy changes;
- API/public status changes;
- production migration invocation/runbook execution;
- provider behavior;
- workflow/dependency/sibling changes;
- hard delete.

## Required checks

```bash
cargo fmt --all --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Mandatory PostgreSQL evidence:

```bash
HAZE_SYNC_TEST_DATABASE_URL='<dedicated test DB>' \
  cargo test -p haze-sync-storage --features test-support -- --ignored
```

A green Component CI run without PostgreSQL is necessary but not sufficient for
full STOR-P10 acceptance.
