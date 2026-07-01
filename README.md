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

This repository currently contains only the Wave 0 foundation skeleton. The crates and plugin app compile as placeholders; real sync, storage, Drive, worktree, auth, and conflict behavior belong to later phases.

## Repository layout

~~~text
crates/
  haze-sync-server/      server binary placeholder
  haze-sync-core/        sync core library placeholder
  haze-sync-api/         HTTP/API contract surface placeholder
  haze-sync-storage/     storage library placeholder
  haze-sync-common/      shared types/utilities placeholder
  haze-sync-worktree/    built-in worktree adapter library placeholder
  haze-gdrive-adapter/   Google Drive adapter binary placeholder
  haze-sync-cli/         CLI binary placeholder
apps/
  haze-obsidian-plugin/  Obsidian plugin skeleton
deploy/
  docker-compose.yml     local Postgres scaffold
migrations/              migration scaffold only
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

## Continuous integration

The GitHub Actions CI workflow runs Rust formatting, checking, tests, and Clippy across the workspace; installs npm workspace dependencies; typechecks and builds the Obsidian plugin; and validates the local Docker Compose configuration.

CI must not use repository secrets, production credentials, Google Drive OAuth, or deployment automation.

## Configuration

Copy `.env.example` for local development and fill in local-only values. Do not commit real `.env` files, raw tokens, OAuth credentials, or generated build outputs.
