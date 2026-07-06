# Dependency Map: docs-process

## Component role in dependency graph

`docs-process` is the process and documentation convention layer.

Docs-process owns tracked process guidance, component documentation conventions, dependency-map semantics, control-slot/report guidance, fan-in guidance, and documentation unification audits.

Docs-process does not own product runtime behavior, component implementation, GitHub workflow behavior, Deployment automation, active prompt/report execution, PR creation, or merge decisions.

## Independent development model

`docs-process` can be developed independently inside `docs-process/**` on process/docs branches.

The dependency map records process/documentation contracts and fan-in points. It does not impose a serial implementation order on product components, CI, Deployment, adapters, or clients.

Allowed independent work includes:

- canonical process model documentation;
- dependency-map semantics documentation;
- component docs guidance;
- control/report lifecycle guidance;
- fan-in and merge-readiness guidance;
- documentation unification audits;
- docs hygiene/check definitions when not changing CI behavior.

If docs-process needs to change component ownership, CI workflow behavior, deployment automation, product architecture, active control files, or merge policy, it reports a contract-change request or fan-in need instead of silently changing another component's responsibility.

## Upstream contracts consumed

Docs-process may consume:

- component-local contracts/plans/maps/decisions/logs as current ownership context;
- root README/system docs as repository-level documentation context;
- github-ci contracts for automated docs validation only when scoped;
- deployment docs for runbook/process boundary alignment;
- project-source process rules only when explicitly referenced and rewritten safely.

Docs-process must not consume:

- local project-source prompts as tracked repo docs by default;
- active prompt/report files as static documentation;
- private/local scratch;
- raw logs or unredacted operational output;
- product internals as process authority.

## Downstream contracts exposed

Expected downstream users/consumers:

- Orchestrator prompts and planning;
- implementation workers;
- clean-code reviewers;
- fixer workers;
- architects;
- component owners;
- human maintainers;
- future docs validation checks.

Downstream users rely on docs-process for process interpretation, not product component ownership.

## Forbidden dependency directions

Docs-process must not:

- rewrite component ownership silently;
- pre-create active `control/prompt.md` or `control/report.md` slots;
- archive active prompt/report files unless explicitly acting as Orchestrator;
- change CI workflows without github-ci coordination;
- change deployment automation without deployment coordination;
- change product code;
- treat dependency maps as serial roadmaps.

## Cross-component contracts

Important docs-process contracts:

- process docs define workflow conventions, not product behavior;
- dependency maps are contract-boundary maps, not global implementation order;
- component docs remain component-local source of truth for component-specific ownership;
- active control files are process state, not static docs;
- fan-in guidance distinguishes implementation acceptance from merge readiness;
- project-source context is not copied verbatim into tracked docs by default.

## Integration/fan-in ownership

Fan-in is required when:

- component docs need bulk drift cleanup;
- dependency-map wording is normalized across component branches;
- docs validation becomes a CI workflow;
- root/system docs index process docs;
- active control-slot policy affects branch fan-in;
- status/report vocabulary becomes enforced.

These are integration gates. They do not block independent docs-process work inside its component boundary.

## Dependency rules

- Docs-process owns conventions, not component-specific implementation.
- Component docs own component-specific contracts.
- github-ci owns workflow enforcement.
- deployment owns operational automation/runbooks.
- Orchestrator owns active control-slot scheduling and fan-in decisions.
- Architect is optional/manual for boundary or contract disputes.

## Contract-change notes

Current known contract questions:

1. Component docs drift pass
   - Component maps may need wording cleanup after dependency-map semantics were clarified.
   - This requires component-branch scope or an accepted fan-in branch.

2. Docs validation enforcement
   - Docs-process can define expectations.
   - github-ci must own workflow enforcement.

3. Control slot cleanup
   - Docs-process defines policy.
   - Orchestrator owns execution decisions.

No serial implementation dependency is implied by this map.
