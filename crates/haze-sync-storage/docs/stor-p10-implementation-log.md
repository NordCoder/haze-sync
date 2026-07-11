# STOR-P10 implementation log

Date: 2026-07-11

Agent: Implementation Worker

Branch: `component/storage`

Prompt: `crates/haze-sync-storage/control/prompt.md`

Report: `crates/haze-sync-storage/control/report.md`

## Summary

Implemented the Storage side of the accepted durable Worktree architecture.
Migration 0010 refuses non-empty legacy path-only state before destructive SQL,
creates versioned adapter/root-fingerprint bindings, replaces path-only state with
per-instance present/tombstoned facts, and adds a database non-negative cursor
constraint.

Added passive repository primitives for instance bind/verify, path load/upsert,
bounded deterministic snapshots, revision/hash-guarded observations and
transaction-only exact-contiguous cursor advancement. The broad monotonic cursor
API remains source-compatible for other adapters. Root fingerprints and raw
external cursor JSON remain non-public/redacted.

Test support now recognizes fresh, exact pre-P10 and exact current schemas. Pure
checks run in Component CI. Mandatory PostgreSQL migration/state/cursor tests are
explicitly ignored in ordinary CI and require the strict dedicated-database
command; absence of that infrastructure is not converted into a passing result.

## Commits

See the `component/storage` history beginning after accepted baseline
`aa59064d641f4850f7c70fa615e638b52613dd95`.

## Status

`SELF_ACCEPT_PENDING_CI` until final source/docs Component CI completes.

Full lifecycle acceptance additionally requires execution of:

```bash
HAZE_SYNC_TEST_DATABASE_URL='<dedicated test DB>' \
  cargo test -p haze-sync-storage --features test-support -- --ignored
```

If the current environment cannot provide that database, report
`BLOCKED_BY_TOOLING` rather than claiming the PostgreSQL evidence passed.

## Preserved boundaries

- no Server runtime/executor code;
- no Worktree filesystem or reconciliation semantics;
- no Core/API/provider changes;
- no workflow/dependency/sibling edits;
- no raw root persistence;
- no hard delete;
- caller-owned transaction boundaries preserved.
