# Dependency Map: github-ci

## Component role in dependency graph

`.github` is the repository automation and validation layer.

Conceptual position:

```text
repository source + workflow definitions
  -> GitHub Actions checks
  -> PR/main/process feedback and repository quality gates
```

CI validates source quality and repository scaffolds. It does not own runtime deployment or product behavior.

## Upstream dependencies

### Rust workspace

CI depends on Rust workspace structure and commands:

```bash
cargo fmt --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Rust components own their code, tests, features, and lints. CI wires accepted checks.

### Obsidian plugin workspace

CI depends on npm workspace/plugin commands:

```bash
npm install --no-audit --no-fund
npm run --workspace haze-obsidian-plugin typecheck
npm run --workspace haze-obsidian-plugin build
```

The Obsidian plugin component owns package scripts and TypeScript behavior.

### Deployment scaffold

CI depends on Deployment for compose validation:

```bash
docker compose -f deploy/docker-compose.yml config
```

Deployment owns compose content and production rollout docs. CI validates syntax only unless expanded by a future contract.

### Branch/process policy

CI depends on process decisions for:

- workflow triggers;
- required status checks;
- branch classes such as `main`, `process/**`, `component/**`, PRs, and manual dispatch;
- release/deploy gating.

### GitHub Actions platform

CI depends on:

- hosted Ubuntu runners;
- GitHub-provided tokens with configured permissions;
- public package registries for Rust/npm dependencies;
- Actions such as `actions/checkout`, `actions/setup-node`, and toolchain setup actions.

## Downstream dependents

Expected downstream dependents:

- implementation workers and reviewers;
- pull requests;
- merger/orchestrator decisions;
- local developer check expectations;
- deployment validation runbooks;
- future release/package workflows.

Downstream users rely on CI to give safe, reproducible feedback without requiring production credentials.

## Cross-component contracts

### Rust components ↔ CI

- Rust components own code/test/lint expectations.
- CI runs workspace checks and may later partition by component.
- Components must not require production secrets for default tests.

### Obsidian plugin ↔ CI

- Plugin owns npm scripts and generated artifact policy.
- CI runs typecheck/build and may add fixture compatibility checks.
- CI must not commit/upload generated bundles unless accepted.

### Deployment ↔ CI

- Deployment owns deployment artifacts and runbooks.
- CI validates deployment scaffolds such as compose config.
- CI must not deploy production services or require deployment secrets by default.

### Server/Storage ↔ CI

- Server/Storage own DB-backed test support and migration behavior.
- CI may add local Postgres integration jobs only after test-support contracts are stable.
- CI must not use production DB URLs.

### GDrive adapter ↔ CI

- GDrive adapter owns fake-provider and real-provider test strategy.
- CI default jobs must use fake provider or no provider credentials.
- Real Google credentials require a separate manual/secured contract and should not be default.

### Process/docs ↔ CI

- Process/docs own branch/control expectations.
- CI should align triggers and required checks with process policy.
- CI workflow changes that affect merge readiness require explicit process review.

## Integration/fan-in ownership

The following work requires coordination outside CI-only leaf phases:

- changing required checks for PR merge readiness;
- adding DB-backed integration tests using Storage/Server test support;
- adding API fixture checks for Obsidian plugin;
- adding deployment validation beyond syntax;
- adding release/package/publish workflows;
- adding secret scanning with repository policy changes;
- adding real-provider/manual workflows.

## Dependency rules

- Default CI jobs must be secret-free.
- CI must not deploy or mutate remote hosts without explicit release/deploy contract.
- CI must not use production Google OAuth, bearer tokens, DB URLs, TLS keys, or vault data.
- Workflow permissions should be least-privilege.
- CI logs/artifacts must not include sensitive data.
- CI should document required vs optional/manual jobs.
- Workflow changes that alter merge readiness should update docs and implementation log.

## Contract-change notes

Current known contract questions:

1. Trigger policy for component branches
   - Current `ci.yml` runs on PRs and pushes to `main`/legacy `w01-ci-dev-tooling`.
   - Current `component-ci.yml` runs on `main`, `process/**`, PRs to `main`, and manual dispatch.
   - Whether `component/**` branches should trigger CI directly is a process decision.

2. Required status checks
   - Required checks are repository settings outside this file set.
   - Workflow changes that rename/remove jobs can affect merge readiness.

3. Storage test-support in CI
   - Storage has optional test-support feature.
   - Whether it belongs in default CI or optional integration job needs component/test stability review.

4. Integration/E2E jobs
   - DB-backed and fake-provider E2E tests should be staged after supporting components stabilize.
   - Real-provider tests should not be default.

5. Release/package workflows
   - No release workflow is currently accepted.
   - Generated artifact and publishing policy must be decided first.

No immediate blocking contract change is required for the current documentation/planning pass.
