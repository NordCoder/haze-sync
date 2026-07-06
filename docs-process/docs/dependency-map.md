# Dependency Map: docs-process

## Component role in dependency graph

`docs-process` is the process and documentation convention layer.

Conceptual position:

```text
component docs + process rules + reports + branch lifecycle
  -> docs-process guidance
  -> future workers/reviewers/orchestrators/architects
```

Docs-process documents how work is organized. It does not execute product behavior, CI, deployment, or sync policy.

## Upstream dependencies

### Component documentation

Docs-process depends on component-local docs for current ownership boundaries:

```text
crates/*/docs/**
apps/haze-obsidian-plugin/docs/**
deploy/docs/**
.github/docs/**
```

Each component owns its own contract and implementation plan. Docs-process may document conventions across those files but must not silently change component ownership.

### Process sources

Docs-process may depend on process rules supplied by project source context when prompts explicitly scope that work.

Relevant concepts include:

- component-centric development;
- component contract;
- implementation plan;
- dependency map;
- implementation log;
- decisions log;
- control prompt/report slots;
- report template/status vocabulary;
- implementation/review/fixer lifecycle;
- branch policy.

Local project-source files are not automatically repository artifacts.

### Root README and system docs

Docs-process may coordinate repository-level documentation indexes and system docs when explicitly scoped.

Root README owns high-level repository introduction and current state.

System docs, when tracked, should summarize accepted architecture and integration boundaries without replacing component contracts.

### GitHub CI

Docs-process depends on github-ci only for automated docs validation hooks.

CI owns workflow behavior. Docs-process owns what documentation checks should mean.

### Deployment

Docs-process depends on deployment docs only for runbook/process boundary alignment.

Deployment owns operational artifacts; docs-process owns documentation conventions.

## Downstream dependents

Expected dependents:

- Orchestrator prompts and planning;
- implementation workers;
- clean-code reviewers;
- fixers;
- architects;
- component owners;
- human maintainers reviewing docs and reports;
- future docs validation checks.

Downstream users rely on docs-process to explain how to read and update repository process docs safely.

## Cross-component contracts

### Component docs ↔ Docs-process

- Components own their contracts/plans/maps/decisions/logs.
- Docs-process owns conventions for those file types.
- Docs-process must not rewrite component ownership without explicit scope.

### Control folders ↔ Docs-process

- Control folders contain active prompts/reports and archived logs.
- Docs-process may document control lifecycle.
- Docs-process must not pre-create or archive active prompts/reports unless explicitly assigned by process role.

### GitHub CI ↔ Docs-process

- GitHub CI owns workflows.
- Docs-process may define documentation validation expectations.
- Automated docs checks require github-ci coordination.

### Deployment ↔ Docs-process

- Deployment owns runbooks and operational topology.
- Docs-process may define style/organization conventions.
- Operational behavior changes stay in deployment.

### Product components ↔ Docs-process

- Product components own runtime behavior and product contracts.
- Docs-process may document how product docs should be structured.
- Docs-process must not introduce product behavior, API changes, storage changes, or adapter semantics.

## Integration/fan-in ownership

The following work requires coordination outside docs-process-only phases:

- changing component docs file requirements;
- changing control prompt/report lifecycle;
- adding docs checks to GitHub workflows;
- changing root README/system docs architecture claims;
- changing branch policy or required checks;
- changing report/status vocabulary;
- bulk-editing component contracts.

## Dependency rules

- Keep docs-process changes behavior-neutral by default.
- Use repository-relative paths in public docs.
- Do not ingest local/private project source files into repo without explicit scope.
- Do not add active prompts/reports as docs artifacts.
- Do not edit CI workflows without github-ci coordination.
- Do not edit deployment automation without deployment coordination.
- Do not alter product component contracts without explicit owner/fan-in scope.

## Contract-change notes

Current known contract questions:

1. Component root for docs-process
   - This pass uses `docs-process/docs/**` as the isolated process component root.
   - This avoids conflict with future product/system docs in `docs/**`.

2. Tracked process source of truth
   - Project-source manifest/report templates currently live outside the repository context.
   - Deciding how much to mirror into repo docs requires explicit scope.

3. System docs ownership
   - System docs under `docs/**`, if added, may be coordinated by docs-process or a separate docs task.
   - Product architecture claims must reflect accepted component contracts.

4. Automated docs checks
   - Markdown/link/secret hygiene checks would affect CI.
   - They require github-ci coordination.

5. Report/status vocabulary stability
   - Once tracked in repo, changing status vocabulary affects worker/report interpretation.

No immediate blocking contract change is required for the current documentation/planning pass.
