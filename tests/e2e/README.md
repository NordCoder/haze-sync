# E2E tests

End-to-end tests are intentionally scaffold-only in Wave 1. Future phases will add local, mock Google Drive, conflict/delete, and rollout acceptance tests.

## Storage test harness

The reusable storage harness lives in `crates/haze-sync-storage/src/test_support` and is explicitly test-only:

- compiled for crate tests or with the `test-support` feature;
- not production server startup behavior;
- not a production migration runner;
- not adapter, Core, or API business logic.

Future E2E and integration tests should prefer these helpers instead of duplicating database setup, unsafe cleanup, or temporary object-root code.

## Real PostgreSQL tests

Real database tests must be opt-in. Configure one of these environment variables before running ignored or feature-gated tests:

~~~bash
export HAZE_SYNC_TEST_DATABASE_URL="postgres://haze_sync:haze_sync_local_placeholder@127.0.0.1:5432/haze_sync_test"
# DATABASE_URL is accepted as a fallback only after the same safety checks.
~~~

Safety rules enforced by the harness:

- the URL must use `postgres://` or `postgresql://`;
- the database name must include `test`;
- obviously unsafe names such as `postgres`, `template0`, `template1`, `haze_sync`, `prod`, `production`, `main`, or `live` are rejected;
- database URLs are redacted in formatting and diagnostics;
- helpers never create or drop databases.

Suggested local flow for future storage integration tests:

~~~bash
createdb haze_sync_test
cargo test -p haze-sync-storage --features test-support -- --ignored
~~~

If no test database URL is configured, tests should skip by handling `connect_test_database_from_env()` returning `Ok(None)`. Do not make normal unit tests depend on a local Postgres service.

## Migrations and cleanup

The harness embeds the repository migrations and can apply them to a fresh, explicitly test-named database. This is for tests only and does not record migration history.

The cleanup helper truncates known Haze Sync metadata tables in dependency-safe order after the database URL has passed test-name safety checks. It never drops databases or schemas.

## Temporary object roots and identifiers

Use `TestObjectRoot` for filesystem/object-store tests that need an isolated object root with `sha256/` and `tmp/` directories.

Use `TestNamespace` or `unique_test_id()` for test-owned adapter ids, operation ids, request ids, and vault-relative paths. Keep generated fixture paths under `_haze_tests/` unless a scenario explicitly requires another safe path.
