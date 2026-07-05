# Implementation Plan: github-ci

## Current state

`.github` is an active CI scaffold with two workflow files.

Implemented current-state surface:

- `.github/workflows/ci.yml`
  - runs on pull requests;
  - runs on pushes to `main` and `w01-ci-dev-tooling`;
  - uses read-only contents permission;
  - cancels in-progress runs for the same workflow/ref;
  - Rust workspace job:
    - install stable Rust with rustfmt and clippy;
    - `cargo fmt --check`;
    - `cargo check --workspace`;
    - `cargo test --workspace`;
    - `cargo clippy --workspace --all-targets -- -D warnings`;
  - Obsidian plugin job:
    - setup Node.js 20;
    - `npm install --no-audit --no-fund`;
    - `npm run --workspace haze-obsidian-plugin typecheck`;
    - `npm run --workspace haze-obsidian-plugin build`;
  - Docker Compose job:
    - `docker compose -f deploy/docker-compose.yml config`.
- `.github/workflows/component-ci.yml`
  - Rust-only workspace fmt/check/test/clippy;
  - runs on pushes to `main` and `process/**`;
  - runs on pull requests to `main`;
  - supports manual dispatch.

Current behavior intentionally does not:

- deploy production services;
- use production secrets;
- call Google Drive or other providers;
- run live server/provider E2E tests;
- upload build artifacts;
- publish releases;
- mutate remote hosts.

The component docs were scaffold-level before this planning pass.

## Target state

The target state for GitHub CI is a safe, fast-enough, secret-free validation layer for the repository and its component lifecycle.

The component is V1-ready when:

- required PR checks cover Rust workspace quality, plugin typecheck/build, and deployment scaffold syntax;
- component/process branches get appropriate validation without requiring production credentials;
- workflow triggers are explicit and not surprising;
- permissions are least-privilege;
- CI logs/artifacts remain secret-free;
- optional heavier integration/E2E checks are gated and clearly labeled;
- deployment validation remains validation only, not production rollout;
- branch/process documentation explains what CI does and does not prove.

## Implementation phases

### CI-P1 — Component contract and planning normalization

Status: completed by this Architect planning pass.

Goal:

```text
Replace scaffold CI docs with a real contract, dependency map, implementation
plan, decisions, and baseline implementation log.
```

Allowed scope:

```text
.github/docs/**
```

Completed deliverables:

- complete `component-contract.md`;
- complete `dependency-map.md`;
- complete `implementation-plan.md`;
- add initial component decisions;
- update implementation log with planning baseline.

Non-goals:

- no workflow behavior changes;
- no product code changes;
- no deployment automation;
- no required-check policy changes;
- no secrets or artifacts.

Acceptance:

- docs describe current CI behavior honestly;
- docs preserve CI/Deployment/Product boundaries;
- future workflow phases are implementable without hidden production automation.

### CI-P2 — Workflow trigger and branch policy normalization

Goal:

```text
Align workflow triggers with current branch/process policy while preserving safe
PR validation and avoiding unexpected deployment behavior.
```

Allowed scope:

```text
.github/workflows/**
.github/docs/**
```

Likely work:

- review `ci.yml` push branches and remove legacy branch names if appropriate;
- decide whether component branches should run full CI, Component CI, or PR-only checks;
- document trigger matrix for `main`, `process/**`, `component/**`, PRs, and manual dispatch;
- preserve concurrency cancellation where useful;
- keep permissions minimal.

Non-goals:

- no production deployment;
- no live provider tests;
- no secret use;
- no code changes outside `.github/**` unless explicitly scoped.

Contract-change triggers:

- changing required status checks;
- expanding triggers to secret-using deployment jobs;
- disabling PR validation;
- making component branches unvalidated without process decision.

Acceptance:

- trigger policy is documented and matches process policy;
- CI does not run surprising production actions;
- PR checks remain effective.

### CI-P3 — Rust workspace check hardening

Goal:

```text
Keep Rust workspace checks complete, reproducible, and aligned with workspace
lints and feature policy.
```

Allowed scope:

```text
.github/workflows/**
.github/docs/**
```

Likely work:

- audit Rust toolchain setup consistency across workflows;
- decide whether to pin toolchain or continue stable;
- add cache only if safe and useful;
- decide whether storage `test-support` feature checks belong in default CI or a separate job;
- add component-specific Rust job partitioning only if runtime becomes too slow;
- keep `cargo clippy --workspace --all-targets -- -D warnings` as quality gate unless explicitly changed.

Non-goals:

- no Rust code changes;
- no production DB dependency;
- no live provider tests;
- no secrets.

Contract-change triggers:

- dropping fmt/check/test/clippy coverage;
- requiring production services;
- changing feature policy without component coordination;
- introducing flaky network/provider tests.

Acceptance:

- Rust CI matches repository quality expectations;
- checks remain secret-free and actionable;
- any cache/partitioning is documented.

### CI-P4 — Obsidian plugin and npm workflow hardening

Goal:

```text
Keep plugin typecheck/build validation reliable and scoped to checked-in source,
without committing generated artifacts or secrets.
```

Allowed scope:

```text
.github/workflows/**
.github/docs/**
apps/haze-obsidian-plugin/package.json only if explicitly scoped with plugin component
```

Likely work:

- review Node version and npm workspace commands;
- decide whether lockfile policy needs update;
- add npm cache only if safe;
- ensure plugin build does not require production server URL/token;
- document artifact policy for generated plugin bundles;
- add TypeScript fixture compatibility checks when API fixtures exist.

Non-goals:

- no plugin feature implementation;
- no generated bundle upload unless accepted;
- no real server/provider integration;
- no token use.

Contract-change triggers:

- committing generated plugin artifacts against policy;
- requiring secrets for plugin checks;
- changing plugin package scripts without plugin coordination;
- dropping typecheck/build validation.

Acceptance:

- plugin checks are reliable and secret-free;
- artifact policy is explicit;
- API compatibility checks can be added later without surprise.

### CI-P5 — Deployment scaffold validation hardening

Goal:

```text
Validate deployment scaffolds without turning CI into production deployment.
```

Allowed scope:

```text
.github/workflows/**
.github/docs/**
deploy/** only if deployment component explicitly scopes it
```

Likely work:

- keep `docker compose -f deploy/docker-compose.yml config` syntax validation;
- add secret-free validation for `.env.example` placeholders if accepted;
- add reverse-proxy config syntax checks if examples are added;
- document that compose config validation does not start services or prove production readiness;
- keep production secrets out of CI.

Non-goals:

- no service startup in CI unless accepted as local-only integration;
- no production deploy;
- no remote host mutation;
- no real OAuth/provider credentials.

Contract-change triggers:

- adding CI jobs that deploy or mutate infrastructure;
- requiring production secrets;
- validating with real vault/provider data;
- editing Deployment-owned artifacts outside coordination.

Acceptance:

- deployment validation is useful and safe;
- local-vs-production distinction is clear;
- CI remains secret-free.

### CI-P6 — Secret, artifact, and generated-output policy checks

Goal:

```text
Add repository hygiene checks that prevent accidental secret/artifact leakage
without blocking accepted generated files.
```

Allowed scope:

```text
.github/workflows/**
.github/docs/**
scripts/** if accepted for repository checks
```

Likely work:

- add lightweight checks for forbidden files/patterns such as `.env`, token files, DB dumps, logs, vault contents, object-store data, generated bundles where not accepted;
- document allowed exceptions;
- consider secret scanning tooling only if false-positive risk is manageable;
- avoid printing matched secret values in logs.

Non-goals:

- no production secret storage;
- no destructive cleanup;
- no scanning external systems;
- no exposing suspected secret values in logs.

Contract-change triggers:

- blocking accepted generated artifacts without policy update;
- printing matched secrets;
- adding external services requiring secrets;
- changing repository ignore policy outside process docs.

Acceptance:

- accidental artifacts/secrets are harder to commit;
- logs remain safe;
- policy is documented.

### CI-P7 — Integration and E2E check staging

Goal:

```text
Stage heavier local integration/E2E checks only when components expose safe,
secret-free test support.
```

Allowed scope:

```text
.github/workflows/**
.github/docs/**
tests/e2e/** only if E2E component/fan-in scopes it
```

Likely work:

- define separate optional jobs for DB-backed integration tests using local Postgres service;
- run storage `test-support` checks when stable;
- add server route E2E tests with local object store/temp dirs;
- keep GDrive/provider tests fake-provider only unless manually gated;
- label slow/flaky/manual jobs clearly.

Non-goals:

- no real Google credentials in CI;
- no production DB;
- no real user vault;
- no deployment automation.

Contract-change triggers:

- requiring real provider credentials;
- making slow/flaky jobs required without process decision;
- uploading sensitive artifacts;
- mutating external services.

Acceptance:

- integration checks are staged and secret-free;
- required vs optional jobs are explicit;
- failures are actionable.

### CI-P8 — Release/package validation and manual workflows

Goal:

```text
Prepare release/package validation only after artifact and deployment policies are
accepted.
```

Allowed scope:

```text
.github/workflows/**
.github/docs/**
```

Likely work:

- add manual workflow for packaging binaries/plugins if accepted;
- validate package contents exclude secrets and local data;
- generate artifacts only from clean source and documented commands;
- keep publishing/deploying disabled unless future release contract accepts it;
- document manual approval requirements.

Non-goals:

- no automatic publish from main;
- no production deploy;
- no signing/secrets unless release policy exists;
- no generated artifact commits unless accepted.

Contract-change triggers:

- publishing releases;
- using signing/deploy secrets;
- uploading artifacts with embedded config/secrets;
- changing release branch/tag policy.

Acceptance:

- package validation is safe and manual;
- release automation remains opt-in;
- artifact contents are checked.

## Dependency gates

CI implementation depends on component contracts and process decisions:

- Rust components define workspace checks and feature/test-support policy.
- Obsidian plugin defines npm scripts and artifact policy.
- Deployment defines compose/reverse-proxy validation scope.
- Server/Storage define DB-backed integration readiness.
- GDrive adapter defines fake-provider vs real-provider CI scope.
- Process/docs define branch and required-check policy.

CI should not expand into release/deploy behavior until Deployment and process contracts accept it.

## Known risks

- CI can accidentally become deployment automation if secret-using jobs are added too early.
- Workflow logs can leak environment values or suspected secrets if checks are careless.
- Required checks can become too slow/flaky and block development.
- Missing component-branch triggers can delay feedback.
- Duplicate workflows can waste CI time or create inconsistent required checks.
- Compose validation can be mistaken for production readiness.
- Real provider tests are high-risk and should remain outside default CI.

## Deferred work

Deferred outside this Architect documentation/planning pass:

- run or observe GitHub Actions checks;
- execute CI-P2 through CI-P8 implementation/clean-code/CI/fixer phases;
- change workflow triggers;
- add caching or matrix partitioning;
- add secret/artifact scans;
- add DB-backed integration jobs;
- add release/package workflows;
- add deployment automation.

## Completion criteria for the component

`github-ci` is V1-ready when:

- workflow trigger policy matches branch/process policy;
- required checks cover Rust, plugin, and deployment scaffold validation;
- heavier integration/E2E jobs are staged and secret-free;
- CI logs/artifacts do not leak secrets or local data;
- workflow permissions are least-privilege;
- release/package/deployment workflows, if any, are manually gated and contract-backed;
- CI documentation explains what each check proves and does not prove.
