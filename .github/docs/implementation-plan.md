# Implementation Plan: github-ci

## Current state

`.github` is an active CI scaffold with four workflow files.

Implemented current-state surface:

- `.github/workflows/component-ci.yml`
  - Rust workspace fmt/check/test/clippy.
  - Runs automatically on pushes to `component/**` and `process/**`.
  - Supports manual dispatch.
  - Does not run on pull requests, avoiding duplicate component-branch PR runs.
- `.github/workflows/ci.yml`
  - Full repository validation: Rust workspace, Obsidian plugin typecheck/build, and Docker Compose config validation.
  - Runs automatically on pushes to `main`.
  - Supports manual dispatch.
  - Does not run automatically on component branch pushes.
- `.github/workflows/rust.yml`
  - Legacy standalone Rust workspace validation.
  - Manual dispatch only.
- `.github/workflows/obsidian-plugin.yml`
  - Legacy standalone Obsidian plugin typecheck/build validation.
  - Manual dispatch only.

Current automatic trigger policy:

```text
component/** push -> Component CI only
process/** push   -> Component CI only
main push         -> full CI
manual dispatch   -> available for targeted workflows
```

Current behavior intentionally does not:

- deploy production services;
- use production secrets;
- call Google Drive or other providers;
- run live server/provider E2E tests;
- upload build artifacts;
- publish releases;
- mutate remote hosts.

## Target state

The target state for GitHub CI is a safe, fast-enough, secret-free validation layer for the repository and its component lifecycle.

The component is V1-ready when:

- required checks cover Rust workspace quality, plugin typecheck/build, and deployment scaffold syntax at the appropriate lifecycle point;
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

Status: completed by Orchestrator CI alignment pass.

Goal:

```text
Align workflow triggers with current branch/process policy while preserving safe
validation and avoiding unexpected deployment behavior.
```

Completed trigger policy:

```text
component/** push -> Component CI only
process/** push   -> Component CI only
main push         -> full CI
Rust standalone   -> manual only
Obsidian standalone -> manual only
```

Allowed scope:

```text
.github/workflows/**
.github/docs/**
```

Completed work:

- removed legacy push branch names from automatic full CI paths;
- made `Component CI` the only automatic workflow for component/process branch pushes;
- kept full CI automatic on `main` pushes;
- kept legacy standalone Rust and Obsidian workflows manual-only;
- documented the trigger matrix.

Non-goals:

- no production deployment;
- no live provider tests;
- no secret use;
- no product code changes outside `.github/**`.

Contract-change triggers:

- changing required status checks;
- expanding triggers to secret-using deployment jobs;
- disabling all validation for component/process branches;
- making CI mutate remote infrastructure.

Acceptance:

- trigger policy is documented and matches process policy;
- CI does not run surprising production actions;
- component/process branch pushes run Component CI only.

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
- allowed exceptions are documented;
- checks remain safe and low-noise.
