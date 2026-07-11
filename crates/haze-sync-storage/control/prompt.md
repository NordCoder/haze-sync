# W1-STOR-P10-DB-VERIFY — Live PostgreSQL verification for durable Worktree state

Before starting, name this worker chat exactly:

`storage — W1 STOR-P10 PostgreSQL Verification`

Component: storage
Path: crates/haze-sync-storage
Branch: component/storage
PR: #47
Role: implementation-worker
Phase: STOR-P10-DB-VERIFY

Work only through the GitHub connector. Do not merge the PR, change draft state, rebase, reset, rewrite history, force-push, modify `main`, or modify sibling branches.

## Trigger

STOR-P10 implementation and artifact correction are complete, and ordinary Component CI is green, but lifecycle acceptance remains blocked because mandatory strict PostgreSQL tests were explicitly ignored and never executed.

Accepted evidence:

- STOR-P10 implementation SHA before fixer: `63d80764933cba5f23fb43bad44201a75e1dc16a`
- final post-fix code-bearing SHA: `abca69058390983894465cef9d66c38960fae4c7`
- fixer status: `FIX_COMPLETE`
- authoritative green Component CI: run `29167108216`, run number `1792`, conclusion `success`
- missing command/evidence: `cargo test -p haze-sync-storage --features test-support -- --ignored` against a dedicated reachable `HAZE_SYNC_TEST_DATABASE_URL`

Archived fixer evidence is under `crates/haze-sync-storage/control/log/20260711-204500Z-W1-FIX-STOR-P10-CI-*` and is pinned by immutable original blob SHAs.

## Goal

Provide real, repeatable, secret-free PostgreSQL acceptance evidence for STOR-P10 migration, instance binding, path-state, transaction, rollback, and exact cursor behavior. Ordinary workspace tests with ignored DB tests are not sufficient.

## Required approach

Inspect the current shared Component CI workflow and Storage strict test-support contract. Implement the smallest safe branch-local CI/test-harness change that:

1. provisions an ephemeral PostgreSQL service in GitHub Actions with non-secret test-only credentials;
2. waits for database readiness deterministically;
3. exposes a test-only `HAZE_SYNC_TEST_DATABASE_URL` to the strict Storage verification command;
4. executes exactly the mandatory ignored PostgreSQL tests with `--features test-support -- --ignored`;
5. fails the workflow when any strict DB test is skipped, cannot connect, times out, or fails;
6. preserves ordinary fmt/check/test/clippy/finalizer gates;
7. uploads normal diagnostics on failure;
8. does not commit real credentials or depend on repository secrets;
9. does not silently make DB verification optional.

Prefer an explicit Storage-only verification step/job conditioned on `component/storage` rather than imposing unnecessary database work on unrelated component branches. Keep any shared workflow change conservative and documented.

## Mandatory verification coverage

The live PostgreSQL run must prove the existing STOR-P10 tests for:

- migration from supported pre-P10 schemas;
- fail-closed handling of incompatible or non-empty legacy Worktree state;
- first bind and identical rebind;
- root fingerprint and version mismatch rejection;
- adapter-instance isolation;
- present/tombstoned invariants;
- deterministic bounded snapshot ordering;
- caller-owned transaction rollback;
- cursor initialization and exact N-to-N+1 advancement;
- regression, gap, stale expected value, overflow and concurrent race rejection;
- no false path-state or cursor claims after rollback;
- safe redacted errors.

Do not weaken, unignore, delete, or rewrite tests merely to obtain green CI. If a live test reveals a genuine Storage defect, apply the smallest Storage-owned correction and document it precisely.

## Preservation requirements

Preserve:

- migration `0010_worktree_durable_state.sql` fail-before-destructive semantics;
- versioned adapter/root-fingerprint binding with no raw-root persistence;
- per-instance present/tombstoned state;
- bounded deterministic snapshots;
- caller-transaction-owned repositories;
- exact contiguous cursor advancement;
- no hard delete or implicit cleanup;
- passive Storage ownership;
- no Server runtime, Worktree filesystem, Core policy, API DTO, provider or deployment behavior.

## Allowed files

- `.github/workflows/component-ci.yml` only for the minimum DB-capable verification wiring;
- `.github/scripts/**` only if a small reusable readiness/verification helper is strictly necessary;
- STOR-P10 Storage tests/test-support files only when live execution proves a defect or harness issue;
- STOR-P10 docs/implementation log for factual verification instructions;
- `crates/haze-sync-storage/control/report.md`.

Forbidden:

- Server, Worktree, Core, API, CLI or Deployment source changes;
- production database URLs, secrets or external managed database dependencies;
- migration redesign without a live-test-proven defect;
- optional/silent DB test skipping;
- destructive cleanup, hard delete or production migration execution policy;
- unrelated workflow refactoring;
- archiving control files.

## CI acceptance

The final code-bearing/tooling SHA must have a green Component CI run that visibly includes successful execution of the strict ignored PostgreSQL command. A green ordinary workspace run without that command is not acceptance evidence.

## Report

Write `crates/haze-sync-storage/control/report.md` using `report-template.md`.

Set:

- `REPORT_TYPE: IMPLEMENTATION`
- `phase_id: STOR-P10-DB-VERIFY`
- `chat_name: storage — W1 STOR-P10 PostgreSQL Verification`

Use one honest status:

- `SELF_ACCEPT`
- `SELF_ACCEPT_PENDING_CI`
- `SELF_NEEDS_FIX`
- `BLOCKED_BY_TOOLING`
- `BLOCKED_BY_SCOPE`
- `BLOCKED_BY_CONTRACT`

The report must include exact workflow/test-harness changes, PostgreSQL image/version and readiness method, the exact strict command executed, which ignored tests actually ran, failures/corrections, final code-bearing SHA, authoritative CI run, secrecy assessment, and whether STOR-P10 is ready for mandatory clean-code review.