# Core file operations integration tests

This directory documents the W2-P7 normal file-operation test foundation.

## Current W2-P7 state

W2-P7 does not wire production Core, storage repositories, object-store runtime, HTTP routes, or database migrations together. That wiring belongs to W2-F1.

The executable default coverage currently lives in:

```text
crates/haze-sync-core/tests/core_file_ops.rs
```

Those tests are fake-backed and exercise the pure W2-P4 `RevisionService` contract through test fixtures only. They do not require PostgreSQL, object-store configuration, providers, network access, `DATABASE_URL`, or `HAZE_SYNC_TEST_DATABASE_URL`.

Run default tests with:

```bash
cargo test -p haze-sync-core --test core_file_ops
cargo test -p haze-sync-core
```

## Covered fake-backed scenarios

The default fake-backed tests cover the W2-P7 normal file-operation scenarios that are practical before W2-F1:

- upload bytes, read the same bytes from the fake object fixture, and verify SHA-256;
- upload two revisions and verify parent/current revision metadata;
- build a changes-feed page containing upsert operations;
- ignore duplicate same-content uploads without adding a revision or operation;
- serialize same-path concurrent writes in the fake harness so one write is accepted and the competing null-base different-content write does not corrupt fake state.

These tests are not end-to-end product behavior. They are contract and harness tests for the currently available pure Core boundary.

## Ignored real DB specification placeholders

The same test file also contains ignored tests named with the `real_db_` prefix for:

- upload file then download same bytes and verify hash;
- upload two revisions;
- changes feed includes upsert operations;
- same-content duplicate is ignored;
- concurrent writes to the same path do not corrupt DB state.

Run ignored placeholders explicitly with:

```bash
cargo test -p haze-sync-core --test core_file_ops -- --ignored --nocapture
```

Before W2-F1, these placeholders skip safely by default and do not touch a database. If `HAZE_SYNC_ENABLE_W2F1_CORE_FILEOPS_TESTS` is set before W2-F1 wiring exists, they fail with an explicit message rather than giving a false green signal.

## Future W2-F1 wiring expectation

When W2-F1 wires Core services to storage repositories and the object store, these ignored placeholders should be replaced or completed with real integration tests that use the existing storage test-support helpers.

Real PostgreSQL test runs must remain opt-in only. Use a dedicated test database through `HAZE_SYNC_TEST_DATABASE_URL`; `DATABASE_URL` may only be accepted after the same safety validation. Test helpers must reject unsafe database names and must not print raw database URLs.

Do not point these tests at a production or personal database. They must never require Google Drive, Obsidian, worktree watchers, provider SDKs, external network calls, server route wiring, or CI secrets.
