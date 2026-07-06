# Implementation Plan: docs-process

## Current state

`docs-process` is the repository process and documentation-convention component.

Current tracked surface:

```text
docs-process/README.md
docs-process/docs/component-contract.md
docs-process/docs/implementation-plan.md
docs-process/docs/dependency-map.md
docs-process/docs/decisions.md
docs-process/docs/implementation-log.md
docs-process/docs/development-model.md
docs-process/docs/dependency-map-semantics.md
docs-process/docs/component-docs-guide.md
docs-process/docs/control-slots-and-reports.md
docs-process/docs/fan-in-and-merge-readiness.md
docs-process/docs/unification-audit.md
```

This process branch unifies the documentation model around protocol v2:

```text
component-centric development
worker scope = component
separate self-review phase disabled
Architect review optional/manual
implementation -> clean-code-review -> CI -> fixer loop if needed
```

Docs-process remains behavior-neutral. It does not own product code, CI workflow behavior, deployment automation, active prompt/report execution, PR creation, or merge decisions.

## Target state

Docs-process is V1-ready when:

- the canonical development model is tracked and clear;
- dependency maps are interpreted as contract-boundary maps, not serial roadmaps;
- component docs conventions are tracked and reduce duplication;
- active control-slot and report lifecycle is tracked;
- fan-in and merge-readiness rules are tracked;
- outdated model fragments are explicitly superseded;
- process docs are safe, repository-relative, and sufficient for future workers;
- future docs/process changes can proceed without re-reading local project-source prompts.

## Canonical documents

```text
docs-process/docs/development-model.md
  Current source of truth for roles, lifecycle, branch policy, source order,
  status vocabulary, check honesty, secrecy rules, and contract-change protocol.

docs-process/docs/dependency-map-semantics.md
  Current source of truth for dependency map interpretation: independent component
  development, contract boundaries, fan-in points, and contract-change blockers.

docs-process/docs/component-docs-guide.md
  Rules for component-local docs and how to avoid duplicating global process policy.

docs-process/docs/control-slots-and-reports.md
  Active prompt/report slot lifecycle and report field semantics.

docs-process/docs/fan-in-and-merge-readiness.md
  Rules for branch fan-in, control-slot hygiene, workflow duplicates, CI evidence,
  and merge-readiness classification.

docs-process/docs/unification-audit.md
  Records the contradictions and duplicated model fragments superseded by this pass.
```

## Completed phases

### DOC-P1 — Component contract and planning normalization

Status: completed.

Delivered baseline docs-process component docs:

```text
component-contract.md
implementation-plan.md
dependency-map.md
decisions.md
implementation-log.md
```

### DOC-P2 — Canonical development model

Status: completed in `process/docs-unification`.

Delivered:

```text
docs-process/README.md
docs-process/docs/development-model.md
```

Resolved contradictions around:

- old wave model vs component-centric model;
- mandatory Architect review vs optional/manual Architect review;
- separate self-review vs implementation worker internal verification;
- tiny-task scope vs component scope;
- process/scaffold branches changing product code by default.

### DOC-P3 — Dependency map semantics

Status: completed in `process/docs-unification`.

Delivered:

```text
docs-process/docs/dependency-map-semantics.md
```

Purpose:

- define dependency maps as contract-boundary maps;
- explicitly reject serial-roadmap interpretation;
- preserve independent component development;
- distinguish fan-in gates from local component work blockers;
- define worker and Orchestrator behavior around missing contracts.

### DOC-P4 — Component docs guide

Status: completed in `process/docs-unification`.

Delivered:

```text
docs-process/docs/component-docs-guide.md
```

Purpose:

- define component doc file roles;
- reduce repeated global process boilerplate;
- preserve component-specific contracts and non-goals;
- keep current-state and target-state claims distinct;
- keep dependency-map wording aligned with independent component development.

### DOC-P5 — Control slots and reports guide

Status: completed in `process/docs-unification`.

Delivered:

```text
docs-process/docs/control-slots-and-reports.md
```

Purpose:

- define active prompt/report ownership;
- prevent idle active slot pre-creation;
- define report field semantics;
- define check/scope/contract honesty requirements.

### DOC-P6 — Fan-in and merge-readiness guide

Status: completed in `process/docs-unification`.

Delivered:

```text
docs-process/docs/fan-in-and-merge-readiness.md
```

Purpose:

- define fan-in evidence;
- handle active control files;
- handle duplicate `component-ci.yml` workflow additions;
- distinguish docs-only, mixed, and product branches;
- define when to invoke Architect manually.

### DOC-P7 — Documentation unification audit

Status: completed in `process/docs-unification`.

Delivered:

```text
docs-process/docs/unification-audit.md
```

Purpose:

- record what was unified;
- list superseded model fragments;
- record unresolved fan-in decisions;
- define precedence when docs conflict.

## Remaining phases

### DOC-P8 — System docs index alignment

Goal:

```text
Align root/system docs with the canonical process model without mixing process docs
and product architecture docs.
```

Allowed scope:

```text
docs-process/docs/**
docs/** only if explicitly scoped
README.md only if explicitly scoped
```

Non-goals:

- no product architecture change;
- no component contract rewrite;
- no local archive ingestion.

Acceptance:

- `docs/**` remains system/product docs;
- `docs-process/**` remains process docs;
- README links do not imply unmerged branch docs are on `main`.

### DOC-P9 — Docs validation and hygiene checks

Goal:

```text
Define or add lightweight validation for docs structure, links, forbidden files, and
sensitive examples.
```

Allowed scope:

```text
docs-process/docs/**
scripts/** only if explicitly scoped
.github/** only with github-ci coordination
```

Non-goals:

- no workflow behavior change without github-ci scope;
- no automatic deletion;
- no printing sensitive matches in logs.

Acceptance:

- docs hygiene is enforceable manually or by coordinated CI;
- validation remains behavior-neutral.

### DOC-P10 — Component docs drift audit after fan-in

Goal:

```text
After component docs branches are merged or staged, audit them against the canonical
process model and dependency-map semantics, then remove duplicated generic process
boilerplate where useful.
```

Allowed scope:

```text
docs-process/docs/**
component docs only when explicitly scoped
```

Non-goals:

- no silent component ownership changes;
- no active control prompt/report edits by docs-process;
- no product-code changes.

Acceptance:

- component docs remain full enough for workers;
- generic lifecycle duplication is reduced;
- dependency maps do not imply global serial implementation order;
- contradictions are reported or fixed with owner scope.

## Dependency gates

Docs-process depends on:

- component contracts for component-specific ownership;
- github-ci for automated docs checks;
- deployment for runbook boundaries;
- root README/system docs for repository-level index wording;
- Orchestrator for fan-in and active control-slot decisions.

These are integration/fan-in gates. They do not block independent docs-process work inside `docs-process/**`.

Docs-process must not ingest local/private project-source files into the repository without explicit user scope.

## Known risks

- Old archived prompts or reports may still contain superseded model wording.
- Component docs may still repeat generic process rules until a component docs drift audit runs.
- Component dependency maps may still use ambiguous dependency-gate wording until the drift audit normalizes them.
- Active `control/prompt.md` and `control/report.md` files in component branches may be merged accidentally without fan-in cleanup.
- Multiple branches may carry different copies of `component-ci.yml`.
- Process docs can become product-architecture docs unless roots remain separated.

## Completion criteria

`docs-process` is V1-ready when:

- canonical process model is tracked;
- dependency map semantics are tracked;
- component docs guide is tracked;
- control/report guide is tracked;
- fan-in guide is tracked;
- unification audit is tracked;
- system docs/index alignment is complete when scoped;
- docs hygiene checks or manual checklist are defined;
- docs-process remains behavior-neutral and does not own product code, CI workflows, deployment automation, or runtime sync semantics.
