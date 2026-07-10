# Storage test support

## Scope

`haze-sync-storage` exposes test helpers only under `cfg(test)` or the explicit
`test-support` feature. The crate's default build does not compile or export the
`test_support` module.

Downstream production crates must depend on `haze-sync-storage` without enabling
`test-support` in their normal dependency set. A downstream crate that needs the
helpers for tests should enable the feature through a dev-dependency or a
separate test-harness package so its production dependency graph remains clean.

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
a CI-owned PostgreSQL service. Tests that explicitly use the optional
`connect_test_database_from_env` compatibility helper report the database path as
not run when `HAZE_SYNC_TEST_DATABASE_URL` is absent. A future CI-owned database
job should supply the variable and use the required or prepare helpers for
mandatory execution.

## Configuration contract

- `HAZE_SYNC_TEST_DATABASE_URL` is the only environment variable read
  implicitly by storage test support.
- General application `DATABASE_URL` configuration is ignored. This prevents a
  developer or CI job from accidentally reusing a runtime database.
- Malformed, non-Unicode, non-PostgreSQL, or unsafe configuration is always an
  error.
- The database name must contain a standalone `test` marker such as
  `haze_sync_test`, `test_haze_sync`, or `haze_sync_test42`.
- Names containing production markers such as `prod`, `production`, `live`,
  `primary`, `main`, or `default` are rejected even when they also contain a
  test marker.
- URL query parameters are allowlisted. TLS parameters plus
  `application_name` and `connect_timeout` are accepted; target/session-changing
  options such as `dbname` or `options` and duplicate keys are rejected.
- URLs and database names are redacted from `Debug`, `Display`, and public test
  support errors. Raw URLs are available only through the explicitly sensitive
  accessor used to construct the SQLx pool.

`connect_required_test_database_from_env` and
`prepare_test_database_from_env` require nonblank configuration. Missing
configuration or an unavailable database fails through a redacted error.

`connect_test_database_from_env` is a compatibility helper for tests whose name
and contract explicitly say "when real PostgreSQL is available". It returns
`Ok(None)` only when `HAZE_SYNC_TEST_DATABASE_URL` is absent or blank. Invalid
configuration and connection, setup, migration, or cleanup failures remain
errors and must not be converted into passing results.

## Database setup and isolation

`prepare_test_database_from_env` is the preferred setup helper for mandatory
PostgreSQL integration tests. It:

1. requires and validates `HAZE_SYNC_TEST_DATABASE_URL`;
2. connects without exposing connection details in errors;
3. serializes schema preparation with a transaction-scoped PostgreSQL advisory
   lock;
4. applies the embedded storage migrations only when no owned storage base
   tables exist;
5. accepts an already-complete initial storage schema made of base tables;
6. rejects a partial schema or views masquerading as owned tables instead of
   guessing which migrations are safe to replay.

Repository tests should isolate writes with caller-owned transactions and roll
them back. Each `PostgresTestContext` also supplies a bounded unique
`TestNamespace`; child identifiers are stable within that namespace and remain
within shared identifier length limits. Typed fixture helpers such as
`operation_id` produce values accepted by their corresponding Common types.

`clean_storage_tables` truncates every owned storage table and is intentionally
destructive. Use it only during exclusive harness setup. Normal parallel
repository tests should prefer namespaced fixtures plus transaction rollback.

## Filesystem setup and cleanup

`TestObjectRoot` creates an isolated temporary object-store root with `sha256`
and `tmp` directories. Partial layout creation is removed before returning an
error. Drop cleanup is best-effort. Tests that must prove cleanup should call
`TestObjectRoot::cleanup`, which reports failures through a path-redacted error
and leaves Drop cleanup enabled for a final best-effort retry when explicit
removal fails.

`TestObjectRoot::keep` is only for local debugging and deliberately transfers the
local path to the test caller. It must not be used in CI reports or public output.
