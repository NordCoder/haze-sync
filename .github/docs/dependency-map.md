# Dependency Map: github-ci

## Component role in dependency graph

`.github` is the repository automation and validation layer.

GitHub CI owns workflow definitions, validation job structure, trigger policy, least-privilege workflow permissions, and repository-quality checks executed by GitHub Actions.

GitHub CI does not own product runtime behavior, component implementation, Deployment automation, branch protection settings, PR/merge decisions, provider behavior, or local developer environments.

## Independent development model

`github-ci` can be developed independently inside the `component/github-ci` branch.

The dependency map records workflow validation contracts and fan-in points. It does not impose a serial implementation order on Rust crates, plugin code, Deployment, docs-process, or product components.

Allowed independent work includes:

- workflow trigger normalization;
- Rust fmt/check/test/clippy job definitions;
- plugin Node/typecheck/build validation;
- compose config validation as syntax/scaffold validation;
- permissions/concurrency hardening;
- docs validation hooks when coordinated;
- workflow documentation.

If CI needs stable commands, fixtures, service containers, deployment artifacts, docs validation scripts, or component-specific test support not currently contracted, it reports a contract-change request or fan-in need instead of implementing another component's product behavior.

## Upstream contracts consumed

GitHub CI may consume:

- workspace Cargo commands and crate layout;
- Obsidian plugin npm scripts;
- Deployment compose file paths for syntax validation;
- docs-process validation semantics when docs checks are scoped;
- test-support contracts exposed by components;
- GitHub Actions platform behavior.

GitHub CI must not consume:

- real provider credentials;
- deployment secrets;
- component-private local notes;
- production host paths;
- runtime assumptions not expressed as tests/commands;
- PR/merge authority.

## Downstream contracts exposed

Expected downstream consumers:

- Orchestrator, for CI status interpretation;
- workers, for check expectations;
- component branches, for repository validation feedback;
- Deployment/release planning when explicitly scoped;
- docs-process, for future docs validation enforcement.

Downstream consumers must not treat CI green as production readiness unless the workflow explicitly proves that readiness.

## Forbidden dependency directions

GitHub CI must not:

- change product behavior to satisfy workflow convenience;
- run live provider calls by default;
- print sensitive values in logs;
- assume deployment/startup readiness from compose syntax alone;
- decide PR/merge readiness by itself;
- own branch protection settings unless explicitly scoped.

## Cross-component contracts

Important GitHub CI contracts:

- workflow permissions are least-privilege;
- checks are explicit and reproducible;
- CI status is validation evidence, not production-readiness proof;
- component-specific checks require component-owned commands/contracts;
- docs/process checks require docs-process coordination;
- deployment checks distinguish syntax from operational rollout.

## Integration/fan-in ownership

Fan-in is required when:

- component commands change CI expectations;
- docs-process adds docs validation semantics;
- Deployment adds service/artifact validation requirements;
- plugin package scripts change;
- Rust workspace layout changes;
- release/deploy workflows are introduced.

These are integration gates. They do not block independent GitHub CI work inside its component boundary.

## Dependency rules

- GitHub CI owns workflow validation, not product behavior.
- Component tests/commands should be owned by the relevant component.
- CI may run those commands but should not hide component contract gaps.
- Secret-dependent checks must be opt-in/manual and safe.
- Workflow changes that affect required checks require Orchestrator/github-ci coordination.

## Contract-change notes

Current known contract questions:

1. Canonical `component-ci.yml`
   - Multiple component branches may carry similar workflow additions.
   - github-ci should own the canonical workflow before broad fan-in.

2. Docs validation
   - docs-process may define semantics.
   - github-ci owns workflow enforcement when explicitly scoped.

3. Deployment validation
   - Deployment owns operational readiness.
   - CI can validate syntax/commands but must not overclaim rollout safety.

No serial implementation dependency is implied by this map.
