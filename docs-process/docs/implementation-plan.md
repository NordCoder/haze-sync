# Implementation Plan: docs-process

## Current state

`docs-process` is a new component-local documentation root for repository process and documentation conventions.

Before this pass:

- branch `component/docs-process` contained only the pre-existing `component-ci.yml` branch addition;
- no dedicated `docs-process/docs/**` files existed;
- process conventions existed in project-source context and across component-local docs;
- root README described product architecture, repository layout, checks, limitations, and configuration at a high level;
- component branches use `docs/**` and `control/**` conventions, but repository-tracked docs-process guidance was not established.

This pass creates the baseline documentation component.

## Target state

The target state for docs-process is a clear, tracked process/documentation guidance layer for Haze Sync.

The component is V1-ready when:

- component documentation structure and expected files are documented;
- control prompt/report/log lifecycle is documented without pre-creating active slots;
- branch lifecycle and component work boundaries are documented;
- report/status vocabulary is documented and aligned with implementation reports;
- system docs organization is clear and separated from component-local docs;
- docs review/check expectations are defined;
- process docs remain behavior-neutral and secret-safe;
- docs-process does not become an implementation, CI, deployment, or product-architecture owner.

## Implementation phases

### DOC-P1 — Component contract and planning normalization

Status: completed by this Architect planning pass.

Goal:

```text
Create the docs-process component-local documentation baseline and define its
scope as process/docs conventions, not product behavior.
```

Allowed scope:

```text
docs-process/docs/**
```

Completed deliverables:

- create `component-contract.md`;
- create `dependency-map.md`;
- create `implementation-plan.md`;
- create `decisions.md`;
- create `implementation-log.md`.

Non-goals:

- no product code changes;
- no component prompt/report creation;
- no CI workflow behavior changes;
- no deployment automation;
- no root/system docs rewrite unless explicitly scoped.

Acceptance:

- docs-process ownership is explicit;
- current scaffold state is documented honestly;
- future docs/process phases are implementable without product behavior changes.

### DOC-P2 — Component docs structure guide

Goal:

```text
Document the standard component documentation structure and expectations for
contract, plan, dependency map, decisions, and implementation log files.
```

Allowed scope:

```text
docs-process/docs/**
component docs only if explicitly scoped as consistency updates
```

Likely work:

- define purpose of:
  - `docs/component-contract.md`;
  - `docs/implementation-plan.md`;
  - `docs/dependency-map.md`;
  - `docs/decisions.md`;
  - `docs/implementation-log.md`.
- define current-state vs target-state wording expectations;
- define component-local ownership and non-goal sections;
- document when to request contract changes;
- add examples using sanitized placeholders.

Non-goals:

- no product behavior changes;
- no bulk edits across all components unless explicitly scoped;
- no active control prompt/report changes.

Contract-change triggers:

- changing required component doc file set;
- changing component ownership model;
- making docs-process responsible for product architecture decisions.

Acceptance:

- future workers can understand what each component doc file is for;
- examples are sanitized;
- guidance matches existing component docs.

### DOC-P3 — Control-slot lifecycle guide

Goal:

```text
Document active control slot semantics for `control/prompt.md`, `control/report.md`,
and `control/log/**` without changing active control files.
```

Allowed scope:

```text
docs-process/docs/**
```

Likely work:

- describe Orchestrator ownership of active prompts;
- describe Worker ownership of reports;
- describe archive ownership and timing;
- define what docs-process must not pre-create;
- document how control files differ from component docs;
- document safe report/log references.

Non-goals:

- no prompt/report creation;
- no archiving live control files;
- no workflow automation;
- no worker assignment generation.

Contract-change triggers:

- changing prompt/report ownership;
- allowing workers to archive active prompts/reports;
- committing local prompt templates as active files.

Acceptance:

- control lifecycle is unambiguous;
- docs do not mutate process state;
- active slot safety is preserved.

### DOC-P4 — Report/status vocabulary and lifecycle guide

Goal:

```text
Document report fields, status vocabulary, and implementation/review/fixer
lifecycle in a repository-safe form.
```

Allowed scope:

```text
docs-process/docs/**
```

Likely work:

- document report fields and intended meaning;
- document statuses such as `SELF_ACCEPT_PENDING_CI`, `BLOCKED_BY_CONTRACT`, `CONTRACT_CHANGE_REQUESTED`, clean-code/fixer statuses, and architect statuses;
- document allowed next-agent recommendations;
- document when to report blockers vs partial success;
- document safety/secrecy section expectations.

Non-goals:

- no changes to external project-source templates unless explicitly asked;
- no automated report generation;
- no PR/merge decision ownership.

Contract-change triggers:

- changing report template fields;
- changing lifecycle status semantics;
- changing who decides merge readiness.

Acceptance:

- reports can be interpreted consistently;
- status semantics do not conflict with component lifecycle;
- docs are safe to publish.

### DOC-P5 — Branch/process lifecycle guide

Goal:

```text
Document branch classes, component work boundaries, and allowed/forbidden branch
operations for this repository.
```

Allowed scope:

```text
docs-process/docs/**
```

Likely work:

- describe `main`, `component/*`, `process/*`, and future fan-in/review/fixer branches if accepted;
- document no-direct-main behavior;
- document no-PR/no-merge worker constraints where relevant;
- document base SHA and branch drift reporting expectations;
- document when component scope may broaden internally and when cross-component change is blocked.

Non-goals:

- no GitHub branch protection settings;
- no CI trigger changes unless coordinated with github-ci;
- no automated branch creation/deletion.

Contract-change triggers:

- changing branch policy;
- changing merge authority;
- changing CI required checks.

Acceptance:

- branch/process guidance matches current workflow;
- no runtime/product behavior changes occur.

### DOC-P6 — System docs organization and index guide

Goal:

```text
Define how repository-level system docs should be organized and how they relate
to component-local docs.
```

Allowed scope:

```text
docs-process/docs/**
docs/** only if explicitly scoped for system docs work
README.md only if explicitly scoped for docs index updates
```

Likely work:

- distinguish system docs from component docs;
- define index conventions for `docs/README.md`;
- define ownership for architecture, boundaries, safety semantics, integration rollout, operations docs;
- define how accepted component decisions flow into system docs;
- prevent stale local archive docs from being tracked accidentally.

Non-goals:

- no product architecture changes;
- no system docs rewrite unless explicitly scoped;
- no local archive ingestion by default.

Contract-change triggers:

- changing product architecture;
- making local handoff archives tracked source;
- duplicating conflicting component contracts in system docs.

Acceptance:

- docs layers are clear;
- system docs can be expanded without conflicting with component-local docs.

### DOC-P7 — Documentation validation and hygiene checks

Goal:

```text
Define or add lightweight documentation hygiene checks that reduce broken links,
secret leaks, and process drift.
```

Allowed scope:

```text
docs-process/docs/**
scripts/** or .github/** only if coordinated with github-ci and explicitly scoped
```

Likely work:

- define markdown link check expectations;
- define forbidden-file/path checks for prompts, reports, archives, logs, dumps, `.env`, tokens, and local scratch;
- define secret-safe example conventions;
- optionally add CI validation hooks through github-ci coordination;
- document manual review checklist.

Non-goals:

- no broad CI workflow changes without github-ci scope;
- no automatic deletion of files;
- no secret value printing in validation logs.

Contract-change triggers:

- adding CI required checks;
- printing suspected secret values;
- changing artifact policy;
- blocking accepted generated docs without policy update.

Acceptance:

- docs hygiene expectations are clear;
- any automated check is secret-safe and coordinated.

### DOC-P8 — Process docs consolidation and drift audit

Goal:

```text
Audit component docs and process docs for consistency after implementation waves
and record accepted process changes in tracked docs.
```

Allowed scope:

```text
docs-process/docs/**
component docs only if explicitly scoped for consistency corrections
```

Likely work:

- check that component docs use consistent status/lifecycle terminology;
- check that dependency boundaries do not contradict system docs;
- identify stale process wording;
- record accepted changes in decisions and implementation log;
- request contract changes for contradictions rather than silently rewriting ownership.

Non-goals:

- no product behavior changes;
- no component-wide rewrites without owner coordination;
- no active prompt/report edits.

Contract-change triggers:

- conflicting component contracts;
- process changes affecting workers/orchestrators/reviewers;
- merge readiness or CI policy changes.

Acceptance:

- docs-process remains aligned with current repository state;
- contradictions are reported explicitly.

## Dependency gates

Docs-process depends on:

- current component contracts and plans;
- implementation manifest/process rules when explicitly referenced;
- report template/status vocabulary when explicitly referenced;
- GitHub CI policy for any automated docs checks;
- Deployment docs for runbook boundaries;
- root README/system docs for repository-level docs index.

Docs-process must not consume private/local project-source files into the repository without explicit user scope.

## Known risks

- Process docs can drift from actual component lifecycle.
- Copying local project instructions into repository docs can accidentally expose private context or overconstrain future work.
- Docs-process can become a dumping ground for product architecture decisions unless ownership is kept narrow.
- Active prompt/report control slots can be confused with static docs.
- Bulk docs consistency edits can accidentally change component contracts without owner review.
- Documentation examples can leak secrets if copied from real logs/config.

## Deferred work

Deferred outside this Architect documentation/planning pass:

- create detailed component-docs guide;
- create control-slot lifecycle guide;
- create report/status vocabulary guide;
- create branch/process lifecycle guide;
- create system docs organization guide;
- add docs validation tooling;
- reconcile component docs after future implementation waves;
- update root README or system docs.

## Completion criteria for the component

`docs-process` is V1-ready when:

- component docs structure is documented;
- control-slot lifecycle is documented;
- report/status vocabulary is documented;
- branch/process lifecycle is documented;
- system docs organization is documented;
- docs hygiene checks or manual review checklist exist;
- process docs are secret-safe and behavior-neutral;
- docs-process does not own product code, CI workflow behavior, deployment automation, or runtime sync semantics.
