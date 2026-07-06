# Component Documentation Guide

## Purpose

Component-local docs keep each component understandable without long prompts.

They contain component-specific requirements, boundaries, current state, implementation plan, decisions, and history. They should not copy the full development protocol into every component.

Use this guide to keep component docs consistent and compact.

## Required files

Standard component docs:

```text
docs/component-contract.md
docs/implementation-plan.md
docs/dependency-map.md
docs/decisions.md
docs/implementation-log.md
```

Standard control files:

```text
control/README.md
control/state.md
control/log/.gitkeep
```

Active files that should exist only while active work is scheduled:

```text
control/prompt.md
control/report.md
```

## `component-contract.md`

The component contract defines what the component owns and does not own.

Recommended sections:

```text
Responsibility
Public interfaces
Input contracts
Output contracts
Error contracts
Persistence/runtime ownership
Security and sensitive-output rules
Non-goals
Dependencies
Dependents
Invariants
Test obligations
Contract change protocol
```

Contract content should be specific to the component. Generic lifecycle rules should refer to `docs-process/docs/development-model.md` instead of being repeated.

A good contract answers:

- what this component owns;
- which public interfaces are stable or planned;
- what inputs it accepts;
- what outputs it produces;
- what errors must be safe;
- which persistence/runtime resources it owns;
- what it explicitly must not implement;
- which invariants must never be broken;
- which tests prove the contract.

## `implementation-plan.md`

The implementation plan breaks the component into implementation phases.

Recommended top-level structure:

```text
Current state
Target state
Implementation phases
Dependency gates
Known risks
Deferred work
Completion criteria
```

Each phase should state:

```text
phase id and title
goal
allowed scope
likely work
non-goals
contract-change triggers
acceptance criteria
```

Phase IDs should be stable and component-specific, for example:

```text
CORE-P3
API-P5
SRV-P7
GDA-P4
DOC-P2
```

Implementation phases are scoped to the component. If a phase needs another component, defer it to fan-in or mark the dependency explicitly.

## `dependency-map.md`

The dependency map explains what the component consumes and what consumes it.

Recommended sections:

```text
Component role in dependency graph
Upstream dependencies
Downstream dependents
Cross-component contracts
Integration/fan-in ownership
Dependency rules
Contract-change notes
```

It should distinguish:

- direct code dependencies;
- conceptual/API dependencies;
- provider/platform dependencies;
- forbidden dependency directions;
- unresolved contract questions.

## `decisions.md`

The decisions log records component-local architecture/process decisions.

Use this format:

```text
## YYYY-MM-DD — <decision title>

Decision:
Rationale:
Alternatives:
Consequences:
Affected contracts:
```

Decisions should be durable. Do not use this file as a scratchpad.

If a decision changes another component contract, record the requested change and suggested owner rather than silently rewriting the other component.

## `implementation-log.md`

The implementation log records completed passes in chronological order.

Use this format:

```text
### YYYY-MM-DD — <phase>

Agent:
Branch:
Prompt:
Report:
Commit(s):
Summary:
Status:
Follow-ups:
```

The log should be enough for a future worker to understand what happened without reading the whole chat.

It is not a replacement for `control/report.md` during active worker execution.

## What not to duplicate

Do not repeat the full global process model in every component.

Prefer references to canonical docs for:

- role definitions;
- full lifecycle sequence;
- branch policy;
- global status vocabulary;
- control-slot ownership;
- report template fields;
- global safety rules.

Component docs should include component-specific safety rules when the component has special risk, such as provider credentials, database connection strings, local filesystem paths, raw provider responses, or generated artifacts.

## Current state vs target state

Every component plan should separate current state and target state.

Current state must match the repository.

Target state may describe planned behavior, but it must not imply the behavior already exists.

Use wording like:

```text
Current state:
- implemented today: ...
- intentionally not implemented today: ...

Target state:
- V1-ready when: ...
```

Avoid claiming production behavior when the current code is still a placeholder.

## Non-goals

Every component should state non-goals.

Non-goals prevent accidental scope expansion. They are especially important for:

- Core vs adapters;
- API DTOs vs Server runtime;
- Storage schema vs Core policy;
- Worktree vs GDrive vs Obsidian;
- Deployment vs CI;
- Docs-process vs product behavior.

## Contract-change triggers

Each phase should list conditions that require a contract change.

Common triggers:

- direct DB access from a component that should use API/Server;
- provider behavior inside a non-provider component;
- product behavior inside docs/process branches;
- new public API shapes;
- hard-delete behavior;
- unsafe sensitive-output behavior;
- cross-component scope not covered by the prompt.

## Documentation density

Component docs should be full enough to continue development, but they should not become a copied global manual.

Keep:

- component ownership;
- implementation phases;
- local decisions;
- local dependency boundaries;
- local risks.

Move or reference:

- repeated lifecycle policy;
- generic status lists;
- global role definitions;
- report-template explanations;
- branch policy.

## Component baseline checklist

A component docs baseline is acceptable when:

```text
component-contract.md defines ownership and non-goals
implementation-plan.md has current/target state and phases
dependency-map.md shows upstream/downstream/cross-component rules
decisions.md records local design/process decisions
implementation-log.md records the planning baseline
current-state claims match repository code/docs
no active prompt/report files were created accidentally
no private local scratch was committed
```
