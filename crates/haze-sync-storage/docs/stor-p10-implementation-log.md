# STOR-P10 implementation log

Date: 2026-07-11

Agent: Implementation Worker

Branch: `component/storage`

Prompt: `crates/haze-sync-storage/control/prompt.md`

Report: `crates/haze-sync-storage/control/report.md`

## Implementation summary

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

Test support recognizes fresh, exact pre-P10 and exact current schemas. Pure
checks run in Component CI. Mandatory PostgreSQL migration/state/cursor tests are
explicitly ignored in ordinary tests and require the strict dedicated-database
command; absence of that infrastructure is not converted into a passing result.

## CI correction

The first STOR-P10 workflow run exposed only formatter differences and a stale
migration-count smoke assertion. `FIX-STOR-P10-CI` corrected those issues. The
post-fix source SHA `abca69058390983894465cef9d66c38960fae4c7` passed ordinary
Component CI run `29167108216`.

## PostgreSQL verification phase

`STOR-P10-DB-VERIFY` adds a Storage-only Component CI job with:

- `postgres:16-alpine` as an ephemeral service;
- synthetic non-secret credentials and a dedicated test-named database;
- container health checking plus a bounded explicit `pg_isready` loop;
- `RUST_TEST_THREADS=1` for exclusive shared-database harness operations;
- the exact command:

  ```bash
  cargo test -p haze-sync-storage --features test-support -- --ignored
  ```

- evidence validation requiring all four mandatory ignored test names to finish
  with `ok`;
- the standard diagnostics wrapper/finalizer and a distinct PostgreSQL-job
  artifact name on failure.

This verification job is conditioned on `component/storage`; unrelated component
branches do not provision PostgreSQL.

## Commits

See the `component/storage` history beginning after accepted baseline
`aa59064d641f4850f7c70fa615e638b52613dd95`.

## Status

`SELF_ACCEPT_PENDING_CI` until a code-bearing workflow run proves both:

1. the ordinary Rust workspace job is green;
2. the Storage PostgreSQL verification job actually executes all mandatory
   ignored tests and is green.

A green ordinary job alone is not acceptance evidence.

## Preserved boundaries

- no Server runtime/executor code;
- no Worktree filesystem or reconciliation semantics;
- no Core/API/provider changes;
- no production database secrets or external managed services;
- no raw root persistence;
- no hard delete;
- caller-owned transaction boundaries preserved.
