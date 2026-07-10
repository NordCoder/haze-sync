# Storage test support

## Scope

`haze-sync-storage` exposes test helpers only under `cfg(test)` or the explicit
`test-support` feature. Default production builds do not compile or export the
`test_support` module.

The helpers are for local or CI-owned storage tests. They do not contact provider
services, load OAuth credentials, access external vaults, choose production
migration policy, or create/drop PostgreSQL databases.

## Commands

Run pure/unit storage tests without a database:

```bash
cargo test -p haze-sync-storage
```

Compile and run feature-gated PostgreSQL tests only after supplying a dedicated
test database URL:

```bash
export HAZE_SYNC_TEST_DATABASE_URL='<postgres-or-postgresql URL ending in a dedicated test database name>'
cargo test -p haze-sync-storage --features test-support
```

Useful compile-only checks are:

```bash
cargo check -p haze-sync-storage
cargo check -p haze-sync-storage --features test-support
```

The default Component CI workflow runs workspace format/check/test/clippy without
the `test-support` feature. Therefore database-backed tests are explicitly
**not run** by that workflow unless a future CI-owned job opts into the feature
and supplies the required test-only environment.

## Configuration contract

- `HAZE_SYNC_TEST_DATABASE_URL` is the only environment variable read
  implicitly by storage test support.
- General application `DATABASE_URL` configuration is ignored. This prevents a
  developer or CI job from accidentally reusing a runtime database.
- Missing, blank, non-Unicode, malformed, non-PostgreSQL, or unsafe
  configuration is an error.
- The database name must contain a standalone `test` marker such as
  `haze_sync_test`, `test_haze_sync`, or `haze_sync_test42`.
- Names containing production markers such as `prod`, `production`, `live`,
  `primary`, `main`, or `default` are rejected even when they also contain a
  test marker.
- URLs and database names are redacted from `Debug`, `Display`, and public test
  support errors. Raw URLs are available only through the explicitly sensitive
  accessor used to construct the SQLx pool.

Once `test-support` is enabled, missing configuration or an unavailable database
must fail the test run. A test must not convert connection, setup, migration, or
cleanup failure into a passing result. The only supported no-database path is to
run without the feature, which records those tests as not compiled/not run.

## Database setup and isolation

`prepare_test_database_from_env` is the preferred setup helper. It:

1. requires and validates `HAZE_SYNC_TEST_DATABASE_URL`;
2. connects without exposing connection details in errors;
3. serializes schema preparation with a transaction-scoped PostgreSQL advisory
   lock;
4. applies the embedded storage migrations only when no owned storage tables
   exist;
5. accepts an already-complete initial storage schema;
6. rejects a partial schema instead of guessing which migrations are safe to
   replay.

Repository tests should isolate writes with caller-owned transactions and roll
them back. Each `PostgresTestContext` also supplies a bounded unique
`TestNamespace`; child identifiers are stable within that namespace and remain
within shared identifier length limits.

`clean_storage_tables` truncates every owned storage table and is intentionally
destructive. Use it only during exclusive harness setup. Normal parallel
repository tests should prefer namespaced fixtures plus transaction rollback.

## Filesystem setup and cleanup

`TestObjectRoot` creates an isolated temporary object-store root with `sha256`
and `tmp` directories. Drop cleanup is best-effort. Tests that must prove cleanup
should call `TestObjectRoot::cleanup`, which reports failures through a
path-redacted error.

`TestObjectRoot::keep` is only for local debugging and deliberately transfers the
local path to the test caller. It must not be used in CI reports or public output.
