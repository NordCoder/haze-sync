# Component Contract: github-ci

## Responsibility

`.github` owns GitHub Actions workflows and repository automation policy for Haze Sync.

The component is responsible for checked-in CI definitions, workflow trigger policy, job matrix/check selection, secret-free repository validation, and CI documentation.

Current implemented CI surface:

- `.github/workflows/ci.yml`
  - pull request checks;
  - push checks for `main` and `w01-ci-dev-tooling`;
  - Rust workspace fmt/check/test/clippy;
  - Obsidian plugin npm install/typecheck/build;
  - Docker Compose config validation for `deploy/docker-compose.yml`;
  - read-only contents permissions;
  - concurrency cancellation per workflow/ref.
- `.github/workflows/component-ci.yml`
  - Rust-only workspace fmt/check/test/clippy;
  - push checks for `main` and `process/**`;
  - pull requests to `main`;
  - manual dispatch.

CI validates repository quality. It must not deploy production services or require production credentials.

## Public interfaces

Current public workflow files:

```text
.github/workflows/ci.yml
.github/workflows/component-ci.yml
```

Current documentation/control files:

```text
.github/docs/component-contract.md
.github/docs/implementation-plan.md
.github/docs/dependency-map.md
.github/docs/decisions.md
.github/docs/implementation-log.md
```

Future public CI surfaces may include:

```text
workflow templates
component-specific check jobs
secret scanning checks
docs-link/check scripts
deployment validation hooks
release packaging checks
artifact policy checks
```

Workflow logs and artifacts are public/repository-visible operational outputs and must not contain secrets.

## Input contracts

CI inputs include:

- repository source tree;
- branch/ref and pull request metadata;
- checked-in workflow YAML;
- public package registries used by Rust/npm tooling;
- local placeholder config files such as `.env.example` and `deploy/docker-compose.yml`;
- test code and fixtures.

Required input rules:

- CI must not require production secrets, Google OAuth credentials, bearer tokens, real vault contents, production DB URLs, SSH keys, TLS private keys, or remote host access;
- CI should use generated/ephemeral local values only where needed;
- workflow triggers must be explicit and documented;
- dependency installation should avoid unnecessary audit/fund/network-noise where already accepted;
- checked-in workflows must use least permissions practical for the job.

## Output contracts

CI outputs include:

- check status;
- workflow logs;
- optional artifacts if future workflows add them;
- failure diagnostics.

Required output rules:

- logs must not expose secrets or production values;
- failures should be actionable and tied to repository-local checks;
- workflow artifacts must not contain secrets, vault contents, DB dumps, provider payloads, OAuth token files, or generated private build outputs;
- CI status should not be described as production sync readiness.

## Error contracts

CI failure output must be safe and scoped.

Workflow logs must not expose:

- bearer tokens;
- OAuth tokens;
- token hashes;
- Idempotency-Key values;
- production database URLs;
- TLS private keys;
- provider payloads;
- raw vault contents;
- production host paths;
- deployment secrets.

CI may show repository-relative paths, compiler errors, test names, and safe placeholder configuration values.

## Persistence/runtime ownership

CI owns validation behavior, not runtime deployment.

CI may own:

- workflow trigger definitions;
- formatting/check/test/lint job wiring;
- npm typecheck/build job wiring;
- compose syntax validation;
- future repository secret-scan or artifact-policy checks;
- workflow documentation.

CI does not own:

- product sync behavior;
- Server runtime startup policy;
- Storage schema semantics;
- Deployment production rollout;
- GDrive provider credentials;
- Worktree runtime behavior;
- Obsidian plugin product logic;
- production release or remote deploy automation unless explicitly accepted by future contract.

## Security and secrecy rules

- Use least GitHub token permissions practical.
- Do not add production secrets to CI for normal checks.
- Do not run workflows that deploy or mutate remote hosts unless explicitly authorized by future release/deploy contract.
- Do not upload artifacts containing secrets, logs with tokens, DB dumps, provider payloads, vault contents, or build outputs not accepted by project policy.
- Do not print environment values that may contain secrets.
- CI checks should be reproducible from tracked source and public dependencies where practical.

## Non-goals

GitHub CI must not implement:

- product runtime behavior;
- migrations against production DB;
- live Google Drive provider tests with real credentials;
- production deployment;
- remote SSH actions;
- secret generation/rotation;
- release publishing unless a future contract accepts it;
- component process orchestration beyond workflow validation.

## Dependencies

See `dependency-map.md`.

## Dependents

See `dependency-map.md`.

## Invariants

- CI is secret-free by default.
- CI validates repository quality, not production readiness.
- Workflow triggers and permissions are explicit.
- Main PR checks should cover Rust workspace, plugin typecheck/build, and compose config unless intentionally changed.
- Deployment automation is not enabled by default.
- CI failures should be actionable from repository source, not dependent on private infrastructure.

## Test obligations

CI component changes should be verified by:

- YAML syntax review;
- GitHub compare/review of workflow changes;
- successful workflow execution when available;
- local equivalent commands where shell is available:

```bash
cargo fmt --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm install --no-audit --no-fund
npm run --workspace haze-obsidian-plugin typecheck
npm run --workspace haze-obsidian-plugin build
docker compose -f deploy/docker-compose.yml config
```

Future workflow checks should include tests for secret-free examples and generated artifact policy when introduced.

## Contract change protocol

Request a contract change instead of silently broadening scope when implementation requires:

- adding production secrets to CI;
- adding deployment/remote host mutation;
- changing required status checks or trigger policy;
- changing Rust/npm/compose validation scope;
- adding live provider tests with real OAuth credentials;
- uploading artifacts that may contain sensitive data;
- editing Deployment-owned runtime artifacts as part of CI work without coordination.
