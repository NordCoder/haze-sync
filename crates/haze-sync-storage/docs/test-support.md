# Storage test support

## Scope

`haze-sync-storage` exposes test helpers only under `cfg(test)` or the explicit
`test-support` feature. The crate's default build does not compile or export the
`test_support` module.

Downstream production crates must depend on `haze-sync-storage` without enabling
`test-support` in their normal dependency set. Tests should enable it through a
dev-dependency or separate test-harness package.

The helpers are local/CI storage infrastructure. They do not contact providers,
load OAuth credentials, access external vaults, choose production migration
policy, or create/drop PostgreSQL databases.

## Commands

Run pure tests without PostgreSQL:

```bash
cargo test -p haze-sync-storage
```

Run historical feature-gated tests with a dedicated database:

```bash
export HAZE_SYNC_TEST_DATABASE_URL='<dedicated postgres/postgresql test database URL>'
cargo test -p haze-sync-storage --features test-support
```

Run mandatory STOR-P10 migration, durable-state and exact-cursor evidence:

```bash
export HAZE_SYNC_TEST_DATABASE_URL='<dedicated postgres/postgresql test database URL>'
RUST_TEST_THREADS=1 \
  cargo test -p haze-sync-storage --features test-support -- --ignored
```

The STOR-P10 tests are explicitly ignored in ordinary workspace CI because they
require an exclusive real database. They use strict required/prepare helpers and
therefore fail when configuration, connection, migration, setup, cleanup or an
assertion fails. They do not convert unavailable infrastructure into a pass.

Useful compile checks:

```bash
cargo check -p haze-sync-storage
cargo check -p haze-sync-storage --features test-support
```

## Component CI PostgreSQL gate

The `component/storage` branch has a dedicated `Storage PostgreSQL verification`
job in Component CI. It does not run on unrelated component branches.

The job:

1. starts `postgres:16-alpine` with synthetic test-only user, password and database
   values committed in the workflow;
2. uses the service-container health check and an additional bounded
   `docker exec ... pg_isready` loop before testing;
3. binds a test-only localhost URL through `HAZE_SYNC_TEST_DATABASE_URL`;
4. sets `RUST_TEST_THREADS=1` because the strict harness performs exclusive table
   cleanup in one dedicated database;
5. executes exactly:

   ```bash
   cargo test -p haze-sync-storage --features test-support -- --ignored
   ```

6. verifies the command log contains successful execution of all four mandatory
   tests:
   - pre-P10 migration plus non-empty legacy rejection;
   - fresh/current schema idempotence;
   - durable instance/path-state isolation and rollback;
   - exact cursor progression, rollback and concurrent loser rejection;
7. runs the standard diagnostics finalizer and uploads a job-specific diagnostics
   artifact on readiness, test or evidence failure.

The credentials are intentionally non-secret, ephemeral and reachable only on the
GitHub Actions runner. The job does not use repository secrets or an external
managed database. A green ordinary Rust workspace job without a green Storage
PostgreSQL verification job is not STOR-P10 acceptance evidence.

## Configuration contract

- `HAZE_SYNC_TEST_DATABASE_URL` is the only environment variable read implicitly.
- General application `DATABASE_URL` is ignored.
- Missing, malformed, non-Unicode, non-PostgreSQL or unsafe configuration is an
  error for strict helpers.
- The database name must contain a standalone `test` marker such as
  `haze_sync_test`, `test_haze_sync`, or `haze_sync_test42`.
- Production markers such as `prod`, `production`, `live`, `primary`, `main`, or
  `default` are rejected even with a test marker.
- URL query parameters are allowlisted. TLS parameters plus `application_name`
  and `connect_timeout` are accepted; target/session-changing options such as
  `dbname` or `options` and duplicate keys are rejected.
- URLs and database names are redacted from `Debug`, `Display`, and errors. Raw
  URLs are available only through the sensitive accessor used by SQLx.

`connect_required_test_database_from_env` and
`prepare_test_database_from_env` require nonblank configuration and return
redacted errors.

`connect_test_database_from_env` is retained only for compatibility tests whose
name and contract explicitly say “when real PostgreSQL is available”. It returns
`Ok(None)` only when the dedicated variable is absent or blank. Invalid
configuration and connection/setup/migration/cleanup failures remain errors.

## Database setup and migration states

`prepare_test_database_from_env` serializes setup with a transaction-scoped
PostgreSQL advisory lock and accepts only:

1. no owned tables — apply all migrations;
2. the exact pre-STOR-P10 owned table set with the exact legacy
   `worktree_state` columns and no legacy rows — apply only migration 0010;
3. the exact current table/column set including `worktree_instances`, versioned
   per-instance `worktree_state`, and the non-negative cursor constraint — no-op.

A non-empty legacy path-only table returns
`LegacyWorktreeStateNotEmpty` before destructive migration SQL. Storage cannot
infer an adapter identity or root fingerprint for those rows. Partial tables,
altered Worktree columns, views masquerading as tables, or missing required
constraints return `IncompleteStorageSchema`.

Repository tests should use caller-owned transactions and rollback. Each
`PostgresTestContext` supplies a bounded `TestNamespace` for deterministic unique
fixtures.

`clean_storage_tables` truncates every current owned table, including
`worktree_state` before `worktree_instances`, and is intentionally destructive.
Use it only during exclusive harness setup.

## Filesystem setup and cleanup

`TestObjectRoot` creates an isolated temporary object-store root with `sha256` and
`tmp` directories. Partial layout creation is removed before returning an error.
Drop cleanup is best-effort. `TestObjectRoot::cleanup` reports path-redacted
failures and keeps Drop cleanup enabled for one final best-effort retry.

`TestObjectRoot::keep` is for local debugging only and must not be used in CI
reports or public output.
