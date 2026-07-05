# Haze Sync

Haze Sync V1 is the current working name for a custom centralized file-level
sync system for an Obsidian vault, a VPS worktree, and a Google Drive replica.

## Architecture target

The intended V1 architecture is planned around:

- custom Sync Core;
- PostgreSQL metadata database;
- content-addressed object store;
- operation log;
- conflict/delete policy engine;
- a planned built-in worktree adapter;
- a planned separate Google Drive API adapter;
- a planned custom Obsidian plugin adapter.

CouchDB/LiveSync is not the V1 implementation architecture.

Core mental model:

- Core is truth.
- Worktree is a materialized view.
- Google Drive is an external replica.
- iPhone vault is an external replica.

## Current repository state

The repository is no longer just the Wave 1 shell. Current `main` already
contains the W2 normal file flow plus the W3 fan-in for conflict, delete, and
admin/status surfaces.

Implemented today:

- Rust workspace and crate boundaries;
- shared domain/value/path/hash/auth primitives;
- storage schema, migrations, repositories, and local object-store primitives;
- Core normal file upsert semantics;
- conflict-saved planning primitives;
- tombstone/delete guard/idempotency primitives;
- API DTOs and passive route helpers;
- W2 server routes for `PUT/GET /v1/files/*path`, `GET /v1/changes`, and
  `GET /v1/server-info`;
- W3 server route modules for conflicts, delete, admin/status, and doctor
  scaffolding;
- CI/dev tooling and the Obsidian plugin scaffold.

This is still a partial server-side sync foundation, not a production-complete
sync product.

## Repository layout

~~~text
crates/
  haze-sync-server/      Axum server, readiness, W2 routes, and W3 fan-in routes
  haze-sync-core/        core revision/conflict/delete primitives and services
  haze-sync-api/         HTTP/API DTOs, route contracts, and auth primitives
  haze-sync-storage/     schema metadata, repositories, object store, test support
  haze-sync-common/      shared domain, value, hash, path, adapter, security primitives
  haze-sync-worktree/    built-in worktree adapter placeholder
  haze-gdrive-adapter/   Google Drive adapter binary placeholder
  haze-sync-cli/         CLI/doctor foundation
apps/
  haze-obsidian-plugin/  Obsidian plugin skeleton
.github/workflows/       CI checks for Rust, plugin, and compose validation
deploy/
  docker-compose.yml     local Postgres scaffold
docs/                    system-level architecture, safety, and integration docs
migrations/              initial storage schema migrations
tests/e2e/               E2E test scaffold only
~~~

## System documentation

Repository-level system documentation lives in [`docs/`](docs/README.md).

Start with:

- [`docs/system-architecture.md`](docs/system-architecture.md) for the product/runtime architecture;
- [`docs/component-boundaries.md`](docs/component-boundaries.md) for component ownership and dependency rules;
- [`docs/safety-and-sync-semantics.md`](docs/safety-and-sync-semantics.md) for global sync safety semantics;
- [`docs/integration-testing-rollout.md`](docs/integration-testing-rollout.md) for integration, testing, bootstrap, and rollout strategy.

Component-local documentation lives inside each component directory and should describe that component's contract, implementation plan, dependency map, implementation log, and decisions.

## Development commands

These commands mirror the repository CI checks and should not require production secrets, Google Drive OAuth, or externally running provider services.

Rust workspace:

~~~bash
cargo fmt --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
~~~

Optional storage test-support feature checks:

~~~bash
cargo test -p haze-sync-storage --features test-support
~~~

Obsidian plugin workspace:

~~~bash
npm install --no-audit --no-fund
npm run --workspace haze-obsidian-plugin typecheck
npm run --workspace haze-obsidian-plugin build
~~~

The repository currently uses npm workspaces and does not require a committed npm lockfile for these scaffold checks.

Deployment scaffold validation:

~~~bash
docker compose -f deploy/docker-compose.yml config
~~~

This validates the local Compose file syntax only; it does not start PostgreSQL or any future sync services.

## HTTP server state

The server can still build a dependency-light Axum router, but it is no longer
limited to a Wave 1 route shell.

Current route behavior:

- `GET /health` returns a local `ok` response.
- `GET /ready` returns sanitized readiness based on configured database/object
  store state.
- `GET /v1/server-info` returns static protocol metadata and advertised
  capabilities.
- `PUT /v1/files/*path`, `GET /v1/files/*path`, and `GET /v1/changes` are wired
  to the current W2 server/runtime path.
- `DELETE /v1/files/*path`, `GET /v1/conflicts`,
  `POST /v1/conflicts/{conflict_id}/resolve`, `GET /v1/admin/status`, and
  `GET /v1/admin/adapters` are present through the accepted W3 fan-in path.
- W3 routes are composed explicitly under `/v1`; route fallback interception is
  no longer part of the active server wiring.

## Current limitations

Not implemented today:

- Google Drive runtime sync;
- Obsidian plugin runtime sync;
- worktree scanner/watcher/materialization runtime;
- bootstrap/import flow;
- background retention/cleanup workers;
- restore workflow;
- full `accept_conflict` resolution path;
- production end-to-end sync scenario.

Current capability caveats:

- `server-info` advertises protocol capabilities ahead of full production
  completeness.
- Some W3 routes return safe placeholder behavior when DB/object-store runtime
  dependencies are absent.
- Conflict resolution is intentionally partial: metadata-only actions exist, but
  full conflict-content acceptance is deferred.
- `accept_current`, `keep_both`, and `mark_resolved` currently resolve conflict
  metadata without changing stored file content; `accept_conflict` returns
  `501 not_implemented`.
- `haze-sync status`, `haze-sync adapters list`, and `haze-sync doctor` are
  currently read-only CLI surfaces. `doctor` emits an offline placeholder
  summary only; live repair/provider/destructive actions are intentionally not
  implemented yet.

## Continuous integration

The GitHub Actions CI workflow runs Rust formatting, checking, tests, and Clippy across the workspace; installs npm workspace dependencies; typechecks and builds the Obsidian plugin; and validates the local Docker Compose configuration.

CI must not use repository secrets, production credentials, Google Drive OAuth, or deployment automation.

## Local handoff docs

`haze-sync-docs-v2/` and `haze-sync-w3-handoff/` are local development context
archives. They are useful during stabilization, but they are not part of the
tracked product source tree and should stay ignored unless there is an explicit
decision to version them.

## Configuration

Copy `.env.example` for local development and fill in local-only values. Do not
commit real `.env` files, raw tokens, OAuth credentials, generated build
outputs, or local handoff notes.
