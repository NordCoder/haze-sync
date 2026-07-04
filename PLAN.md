# Haze Sync Stabilization Plan

Date: 2026-07-03

## Purpose

Bring the repository from the current W3 fan-in state to a clean, stable baseline
that is safe to extend with bootstrap and adapter runtime development.

This plan has two tracks:

1. Code cleanup: remove stale wording, accidental complexity, inconsistent route
   seams, and misleading placeholders.
2. Stability hardening: verify current behavior, add missing regression coverage,
   standardize server/runtime patterns, and close known W3 safety gaps.

The target is not production-complete sync. The target is a trustworthy
foundation for the next feature phases.

## Current State Summary

Haze Sync V1 is a centralized file-level sync system for:

- an Obsidian vault on iPhone/iPad through a custom plugin;
- a Google Drive replica for PC and external visibility;
- a VPS worktree for Haze agents;
- a Core metadata database and content-addressed object store as the source of
  truth.

Implemented today:

- Rust workspace and crate boundaries;
- shared domain/value/path/hash/auth primitives;
- Core normal file upsert semantics;
- conflict-saved planning primitives;
- tombstone/delete guard/idempotency primitives;
- API DTOs and passive route helpers;
- storage repositories and local object-store primitives;
- W2 file PUT/GET/changes server routes;
- W3 conflict/delete/admin route modules;
- CLI/doctor scaffolding;
- adapter/plugin placeholders.

Not implemented today:

- Google Drive runtime sync;
- Obsidian plugin runtime sync;
- worktree scanner/watcher/materializer;
- bootstrap flow;
- background jobs;
- retention cleanup and restore workflow;
- full `accept_conflict` resolution;
- production E2E sync scenario.

## Non-Goals For This Plan

- Do not implement Google Drive sync runtime.
- Do not implement Obsidian sync runtime.
- Do not implement worktree runtime.
- Do not add hard-delete cleanup.
- Do not implement semantic merge.
- Do not convert the project into a multi-user SaaS architecture.
- Do not make broad architectural rewrites without regression tests first.

## Phase 0 - Environment And Baseline

Goal: know exactly what is broken before editing behavior.

Tasks:

- Install or expose the expected Rust toolchain if missing.
- Confirm Node/npm availability for the Obsidian plugin scaffold.
- Record current git state with `git status --short` and `git log --oneline -12`.
- Run baseline checks:
  - `cargo fmt --check`
  - `cargo check --workspace`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `npm install --no-audit --no-fund`
  - `npm run --workspace haze-obsidian-plugin typecheck`
  - `npm run --workspace haze-obsidian-plugin build`
  - `docker compose -f deploy/docker-compose.yml config`
- Save failures as stabilization work items before adding features.

Exit criteria:

- The team has a concrete baseline result.
- Missing local tooling is documented separately from code failures.
- No feature work has started before baseline failures are understood.

## Phase 1 - Documentation And Repository Hygiene

Goal: make local documentation match the actual W3 state.

Tasks:

- Update `README.md` so it no longer describes the repository as only Wave 1.
- Clearly separate implemented behavior from intended V1 architecture.
- Add a short "current limitations" section for adapter runtimes, bootstrap,
  worktree materialization, and conflict resolution.
- Decide whether `haze-sync-docs-v2/` and `haze-sync-w3-handoff/` should remain
  untracked local context, be committed, or be moved outside the repo.
- Ensure `.gitignore` excludes local notes, prompt files, archives, dumps, logs,
  generated plugin output, credentials, and environment files.
- Remove or update stale comments that contradict implemented W3 behavior.

Exit criteria:

- A new contributor can read `README.md` and understand what works now.
- Documentation does not imply production sync exists.
- No secrets, local artifacts, or handoff-only files are accidentally staged.

## Phase 2 - Mechanical Code Cleanup

Goal: reduce friction without changing behavior.

Tasks:

- Run formatting after Rust tooling is available.
- Fix Clippy warnings without changing behavior.
- Remove unused imports, dead helper functions, and obsolete comments.
- Standardize naming where the same concept appears across crates:
  - revision id;
  - content hash;
  - adapter id;
  - tombstone id;
  - conflict id;
  - operation seq.
- Keep DTO conversion helpers small and local to their route/storage boundary.
- Avoid broad refactors in the same commit as behavior fixes.

Exit criteria:

- `cargo fmt --check` passes.
- `cargo clippy --workspace --all-targets -- -D warnings` passes.
- Cleanup commits are reviewable and behavior-neutral.

## Phase 3 - Route Composition Stabilization

Goal: replace fragile W3 route interception with explicit route composition.

Current risk:

W2 routes live in `crates/haze-sync-server/src/routes/v1.rs`, while W3 routes are
currently intercepted in `crates/haze-sync-server/src/routes/mod.rs` through
middleware/fallback logic.

Tasks:

- Add regression tests that lock current behavior for:
  - `GET /health`
  - `GET /ready`
  - `GET /v1/server-info`
  - `PUT /v1/files/*path`
  - `GET /v1/files/*path`
  - `GET /v1/changes`
  - `DELETE /v1/files/*path`
  - `GET /v1/conflicts`
  - `POST /v1/conflicts/{conflict_id}/resolve`
  - `GET /v1/admin/status`
  - `GET /v1/admin/adapters`
- Refactor route registration so W3 routes are explicitly composed under `/v1`.
- Preserve handler logic during the route composition refactor.
- Remove middleware/fallback route interception only after tests prove parity.
- Make path parameter syntax consistent with the Axum version in use.

Exit criteria:

- Route behavior is covered before and after refactor.
- W3 routes are registered explicitly.
- `routes/mod.rs` no longer hides business routes in fallback interception.

## Phase 4 - Auth And Runtime State Consistency

Goal: make route modules use one predictable auth/runtime dependency pattern.

Current risk:

Some routes support DB-backed auth, while W3 conflict routes currently accept
static principal auth and reject `AuthState::Database`.

Tasks:

- Define a shared authentication helper for route modules.
- Standardize behavior for:
  - missing token;
  - malformed bearer token;
  - disabled auth;
  - static principal auth;
  - DB-backed token lookup;
  - forbidden role.
- Ensure no route returns token hashes, raw tokens, OAuth data, DB URLs, provider
  payloads, local absolute paths, or stack traces.
- Add tests for auth behavior across files, conflicts, delete, and admin routes.

Exit criteria:

- Auth behavior is consistent across all server routes.
- DB-backed auth support is either implemented everywhere it is advertised or
  clearly documented as intentionally unavailable.
- Public errors remain sanitized.

## Phase 5 - Conflict Persistence Completeness

Goal: ensure `conflict_saved` is not just a Core outcome, but a durable server
state when PUT receives stale/unknown/null-base content.

Tasks:

- Inspect `put_file_route` and confirm the current stale-base path.
- If incomplete, implement a narrow durable path:
  - call `conflict_saved_planner`;
  - store incoming bytes in the object store;
  - insert incoming content metadata;
  - create a conflict row;
  - append `conflict_created` operation log;
  - return a safe `conflict_saved` response;
  - store idempotency response.
- Prevent recursive conflict source paths under `_haze_conflicts`.
- Ensure public JSON never includes raw file bytes.
- Add tests for conflict_saved persistence, idempotency replay, and idempotency
  mismatch.

Exit criteria:

- Stale/unknown/null-base writes preserve incoming content durably.
- Current file content is not overwritten.
- Conflict list can show the saved conflict.
- Changes feed can expose the conflict operation safely.

## Phase 6 - DELETE Hardening

Goal: make tombstone delete behavior dependable before adapter runtimes consume
it.

Tasks:

- Verify base revision semantics for delete.
- Verify idempotency storage, replay, and mismatch behavior.
- Verify per-path advisory lock use.
- Verify tombstone ID determinism and collision handling.
- Verify sync object state after delete:
  - current revision cleared;
  - deleted timestamp set;
  - content blobs retained;
  - file revisions retained.
- Add safe DB-backed opt-in tests for delete transaction behavior.
- Keep retention cleanup and restore out of scope for this phase.

Exit criteria:

- DELETE creates tombstone metadata only.
- No physical content or row deletion occurs.
- Repeated identical delete requests replay safely.
- Different requests with the same idempotency key fail safely.

## Phase 7 - Conflict Resolution Hardening

Goal: make current metadata-only conflict resolution reliable and keep unsafe
resolution paths explicitly deferred.

Tasks:

- Verify behavior for:
  - `accept_current`;
  - `keep_both`;
  - `mark_resolved`;
  - `accept_conflict`.
- Keep `accept_conflict` as not implemented until it can safely promote incoming
  conflict content into a new current revision.
- Add tests that resolved conflicts cannot be resolved twice.
- Add operation log tests for `conflict_resolved`.
- Document exact semantics of each supported resolution action.

Exit criteria:

- Metadata-only resolutions are transactional and idempotency-safe where needed.
- `accept_conflict` fails safely and predictably.
- The Conflict Center can rely on stable API semantics later.

## Phase 8 - Storage And DB Test Harness

Goal: add confidence around database behavior without requiring production
services.

Tasks:

- Keep default tests fake-backed, DTO-only, or dependency-free.
- Add opt-in DB tests gated by feature flag or environment variable.
- Cover repositories for:
  - objects;
  - revisions;
  - content blobs;
  - operation log;
  - tombstones;
  - conflicts;
  - idempotency;
  - adapter cursors.
- Add route-level DB tests for W3 conflict/delete paths.
- Ensure tests use disposable databases or transaction rollbacks.

Exit criteria:

- Default CI remains secret-free and provider-free.
- Opt-in DB tests can validate transaction behavior locally.
- Repository and route behavior have enough coverage for adapter development.

## Phase 9 - CLI And Doctor Stabilization

Goal: keep operational commands useful but passive and secret-safe.

Tasks:

- Add CLI smoke tests for existing commands.
- Ensure doctor output avoids secrets and local absolute sensitive paths.
- Keep doctor checks read-only.
- Document which checks are placeholders and which inspect real runtime state.
- Do not add repair, cleanup, provider calls, or destructive actions yet.

Exit criteria:

- CLI builds and smoke tests pass.
- Doctor output is safe to paste into an issue or PR.
- No command mutates production state unexpectedly.

## Phase 10 - Pre-Feature Readiness Gate

Goal: define the point at which new development can safely resume.

Required checks:

- `cargo fmt --check`
- `cargo check --workspace`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p haze-sync-storage --features test-support`
- `npm install --no-audit --no-fund`
- `npm run --workspace haze-obsidian-plugin typecheck`
- `npm run --workspace haze-obsidian-plugin build`
- `docker compose -f deploy/docker-compose.yml config`

Required state:

- README reflects implemented behavior.
- W3 routes are explicitly composed.
- Auth behavior is consistent or explicitly documented.
- `conflict_saved` PUT behavior is durable or explicitly tracked as the next
  blocker.
- DELETE tombstone behavior has regression coverage.
- Metadata-only conflict resolution has regression coverage.
- Default tests require no production secrets, providers, or network services.
- Known deferrals are listed in documentation.

Only after this gate should the project move to:

- bootstrap flow design and implementation;
- worktree runtime adapter;
- Obsidian plugin runtime sync;
- Google Drive adapter runtime sync;
- restore and retention lifecycle;
- production E2E sync scenarios.

## Follow-Up Code Cleanup Plan

Purpose: improve local code quality before heavy feature work continues, without
creating cross-component helper layers that future development will immediately
reshape.

The cleanup scope is deliberately intra-component:

- A `haze-sync-server` cleanup may move code between files inside
  `crates/haze-sync-server/src`, but should not introduce a shared helper crate.
- A route cleanup may create route-local support modules, but should not merge
  unrelated route concerns just to remove superficial duplication.
- A storage cleanup may improve repository files inside `haze-sync-storage`, but
  should not make server routes depend on test-only utilities.
- Test fixture cleanup should stay near the test type it supports unless two
  tests in the same component clearly need the same fixture.
- Do not chase perfect DRY for deterministic IDs, small error mappings, or local
  request helpers when the duplication is expected to evolve differently during
  bootstrap, worktree, plugin, and Google Drive runtime work.

### Parallel Cleanup Agent Protocol

Parallel cleanup is allowed only by bounded ownership scopes. Every subagent
must read this section and its assigned scope before editing.

Global rules for all cleanup agents:

- Work only inside the assigned write scope.
- Do not revert or rewrite changes made outside the assigned scope.
- Do not create cross-component helper modules, shared test fixtures, or generic
  utility layers.
- Preserve public API, database schema, route paths, JSON shapes, and CLI
  command behavior unless the assigned scope explicitly says otherwise.
- Prefer move-only refactors, import cleanup, local naming cleanup, and local
  helper extraction over behavior changes.
- Keep tests close to the component they exercise.
- Run the assigned local verification command before reporting completion.
- Report changed files and verification result in the final agent message.

Bounded scopes for parallel execution:

- `scope-server-routes`: owns `crates/haze-sync-server/src/routes/**` and may
  touch `crates/haze-sync-server/src/http/**` only if route-local error imports
  require it. This scope is responsible for Cleanup Phase A on server route file
  boundaries. It must account for any existing partial `v1` module split under
  `crates/haze-sync-server/src/routes/v1/` and finish it rather than reverting
  it. Local verification: `cargo fmt --check` and
  `cargo test -p haze-sync-server`.
- `scope-storage`: owns `crates/haze-sync-storage/src/**` and
  `crates/haze-sync-storage/tests/**`. It may clean repository/test harness
  locality and remove stale comments, but must not change migrations or server
  route code. Local verification: `cargo fmt --check`,
  `cargo test -p haze-sync-storage`, and
  `cargo test -p haze-sync-storage --features test-support`.
- `scope-api-contracts`: owns `crates/haze-sync-api/src/**` and
  `crates/haze-sync-api/tests/**`. It may clean DTO/route contract comments,
  local names, and test fixture readability while preserving serialized public
  contracts. Local verification: `cargo fmt --check` and
  `cargo test -p haze-sync-api`.
- `scope-cli`: owns `crates/haze-sync-cli/src/**`,
  `crates/haze-sync-cli/tests/**`, and `crates/haze-sync-cli/Cargo.toml`. It may
  clean command/doctor wording, imports, and smoke-test fixtures while keeping
  commands passive and secret-safe. Local verification: `cargo fmt --check` and
  `cargo test -p haze-sync-cli`.
- `scope-plugin`: owns `apps/haze-obsidian-plugin/**`, `package.json`, and
  `package-lock.json` only when plugin dependencies are affected. It may clean
  TypeScript structure, stale comments, and local naming without implementing
  runtime sync. Local verification: `npm run --workspace haze-obsidian-plugin
  typecheck` and `npm run --workspace haze-obsidian-plugin build`.
- `scope-docs-readme`: owns `README.md`, `PLAN.md`, `.gitignore`, and local
  repo documentation only. It may align documentation with implemented behavior
  and cleanup status. It must not edit Rust or TypeScript source. Local
  verification: documentation review plus `git diff --check -- README.md
  PLAN.md .gitignore`.

Recommended parallel batches:

- Batch 1: `scope-server-routes`, `scope-storage`, `scope-api-contracts`,
  `scope-cli`.
- Batch 2: `scope-plugin`, `scope-docs-readme`, then the full Cleanup Readiness
  Gate.

Do not run two agents with overlapping write scopes in the same batch.

### Cleanup Phase A - Server Route File Boundaries

Goal: reduce the largest `haze-sync-server` route files while preserving route
behavior exactly.

Targets:

- `crates/haze-sync-server/src/routes/v1.rs`
- `crates/haze-sync-server/src/routes/delete.rs`
- `crates/haze-sync-server/src/routes/conflicts.rs`

Tasks:

- Split each oversized route file into route-local modules only when there is a
  stable responsibility boundary:
  - handler entrypoints;
  - request/header parsing glue;
  - persistence orchestration;
  - response mapping;
  - local test fixtures.
- Keep public router construction and route paths unchanged.
- Move tests with the code they primarily exercise, or keep a route-local
  `tests` module when splitting would make test setup harder to read.
- Do not create shared `server::utils`, `common::ids`, or cross-route fixture
  modules in this phase.

Exit criteria:

- `v1.rs`, `delete.rs`, and `conflicts.rs` no longer contain unrelated handler,
  persistence, and test-fixture blocks in one long file.
- Route tests still pass without changing public behavior.
- Review diffs are mostly move-only plus import updates.

### Cleanup Phase B - Server Persistence Locality

Goal: make server persistence steps easier to read without changing storage
contracts or pushing orchestration into the wrong crate.

Tasks:

- Inside `haze-sync-server`, isolate the persistence flows for:
  - accepted file revision;
  - `conflict_saved`;
  - tombstone delete;
  - metadata-only conflict resolution.
- Prefer small route-local structs or functions that name the operation being
  performed over long inline SQL/action sequences in handlers.
- Use existing `haze-sync-storage` repositories where they already exist, such
  as `content_blobs`, instead of keeping duplicate route-local SQL.
- Keep Core policy decisions in `haze-sync-core`; keep SQL repository primitives
  in `haze-sync-storage`; keep request orchestration in `haze-sync-server`.
- Do not introduce a generic transaction framework or repository service layer.

Exit criteria:

- Main route handlers read as authentication, parsing, idempotency, transaction,
  orchestration, response.
- Persistence helper names describe business operations rather than SQL tables.
- Existing route-level DB tests still prove the transaction behavior.

### Cleanup Phase C - Test Fixture Compaction Within Components

Goal: reduce fixture noise without building global test infrastructure too early.

Tasks:

- In `haze-sync-server`, keep route test helpers local to the route module or
  route submodule they support.
- In `haze-sync-storage`, keep DB harness helpers inside the integration test
  file unless a second storage integration test needs them.
- Collapse repeated fixture setup inside a single module into small builders only
  when it removes meaningful setup noise.
- Keep fixture data explicit enough that test intent remains visible.
- Avoid a global test-support layer for route tests until bootstrap/runtime
  development shows the stable shared shape.

Exit criteria:

- Large test blocks are easier to scan.
- Fixture helpers do not hide important state transitions.
- No production code depends on test-only helpers.

### Cleanup Phase D - Comments And Placeholder Language Pass

Goal: remove comments that still describe earlier waves or placeholder behavior
that is now implemented.

Tasks:

- Review module headers in `haze-sync-api`, `haze-sync-core`,
  `haze-sync-storage`, `haze-sync-server`, and `haze-sync-cli`.
- Update comments that say behavior is "future" or "placeholder" when the
  behavior is now implemented.
- Keep explicit deferral comments for genuinely deferred work:
  - adapter runtimes;
  - bootstrap;
  - worktree materialization;
  - restore/retention cleanup;
  - full `accept_conflict`.
- Remove comments that only repeat the function name or obvious type signature.

Exit criteria:

- Comments describe current behavior or intentional deferrals.
- No stale wave/foundation wording remains in touched files.
- Public docs and module docs do not contradict the server routes.

### Cleanup Phase E - Local Naming And Error Consistency

Goal: tighten local readability without broad renames across crates.

Tasks:

- Within each component, standardize local names for:
  - current revision;
  - incoming revision;
  - materialized conflict path;
  - idempotency fingerprint;
  - operation sequence.
- Keep public DTO, DB, and API contract names stable unless there is a strong
  correctness reason to change them.
- Keep public error bodies sanitized and stable.
- Prefer small local conversion helpers over ad hoc parsing repeated several
  times in the same file.

Exit criteria:

- Local naming is consistent within each cleaned component.
- Public API contracts remain backwards-compatible.
- `cargo clippy --workspace --all-targets -- -D warnings` remains green.

### Cleanup Phase F - Cleanup Readiness Gate

Goal: prove the cleanup did not change behavior.

Required checks:

- `cargo fmt --check`
- `cargo check --workspace`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p haze-sync-storage --features test-support`
- `npm run --workspace haze-obsidian-plugin typecheck`
- `npm run --workspace haze-obsidian-plugin build`
- `docker compose -f deploy/docker-compose.yml config`

Required state:

- No behavior-only cleanup phase lands without tests already covering the moved
  code path.
- No new cross-component abstraction exists solely to remove duplication.
- Large files are smaller because responsibilities moved to local modules, not
  because logic was hidden in vague helpers.
- New development can continue with clearer route, persistence, and test
  boundaries.

## Recommended Branch Sequence

- `chore/stabilization-plan`
- `chore/docs-current-state`
- `chore/mechanical-cleanup`
- `test/server-route-regressions`
- `refactor/server-route-composition`
- `refactor/auth-route-consistency`
- `feat/conflict-saved-persistence`
- `test/delete-db-hardening`
- `test/conflict-resolution-hardening`
- `test/storage-db-harness`
- `chore/cli-doctor-smoke`

## Review Rules

- Keep route refactors separate from behavior changes.
- Keep tests in the same branch as the behavior they protect.
- Prefer small commits with one reason to exist.
- Do not change adapter runtime placeholders during stabilization unless needed
  to keep builds/tests honest.
- Do not commit secrets, generated artifacts, local logs, dumps, archives, or
  one-off AI scratch files.
