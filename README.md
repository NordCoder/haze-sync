# Haze Sync

Haze Sync V1 is a custom centralized file-level sync system for an Obsidian vault, a VPS worktree, and a Google Drive replica.

## V1 architecture baseline

Haze Sync V1 is built around:

- custom Sync Core;
- PostgreSQL metadata database;
- content-addressed object store;
- operation log;
- conflict/delete policy engine;
- built-in worktree adapter;
- separate Google Drive API adapter;
- custom Obsidian plugin adapter.

CouchDB/LiveSync is not the V1 implementation architecture.

Core mental model:

- Core is truth.
- Worktree is a materialized view.
- Google Drive is an external replica.
- iPhone vault is an external replica.

The repository currently contains the Wave 1 foundation: shared domain/value primitives, storage schema metadata and migrations, API DTO/contracts/auth primitives, server config and route shell, local content-addressed object store primitives, gated storage test support, and CI/dev tooling. Real Core sync behavior, storage repositories, adapters, conflict/delete engines, provider clients, and production runtime wiring belong to later phases.

## Repository layout

~~~text
crates/
  haze-sync-server/      Axum route shell and server config primitives
  haze-sync-core/        sync core library placeholder for later Core services
  haze-sync-api/         HTTP/API DTOs, route contracts, and auth primitives
  haze-sync-storage/     schema metadata, passive models, object store, test support
  haze-sync-common/      shared domain, value, hash, path, adapter, security primitives
  haze-sync-worktree/    built-in worktree adapter placeholder
  haze-gdrive-adapter/   Google Drive adapter binary placeholder
  haze-sync-cli/         CLI binary placeholder
apps/
  haze-obsidian-plugin/  Obsidian plugin skeleton
.github/workflows/       CI checks for Rust, plugin, and compose validation
deploy/
  docker-compose.yml     local Postgres scaffold
migrations/              initial storage schema migrations
tests/e2e/               E2E test scaffold only
~~~

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

## HTTP route shell

The Wave 1 server route shell can build an Axum router without database pools, object-store roots, provider clients, config secrets, or adapter tokens.

Current route behavior:

- `GET /health` returns a local `ok` response.
- `GET /ready` returns an explicit safe `not_ready` placeholder until later runtime dependency checks are wired.
- `GET /v1/server-info` returns static safe protocol metadata.
- File, changes, and conflict mutation/read routes are registered as explicit `not_implemented` placeholders until Core phases wire real behavior.

## Continuous integration

The GitHub Actions CI workflow runs Rust formatting, checking, tests, and Clippy across the workspace; installs npm workspace dependencies; typechecks and builds the Obsidian plugin; and validates the local Docker Compose configuration.

CI must not use repository secrets, production credentials, Google Drive OAuth, or deployment automation.

## Configuration

Copy `.env.example` for local development and fill in local-only values. Do not commit real `.env` files, raw tokens, OAuth credentials, or generated build outputs.
