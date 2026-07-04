# Agentic Development Protocol

## Status

- Protocol version: 2
- Development model: component-centric
- Architect gate: optional/manual for now
- Default lifecycle: `implementation -> self-review -> clean-code-review -> CI -> fixer loop`

## Entities

### Agent Execution

One run of one agent against one active control prompt.

### Component

A bounded repository area with its own contract, implementation plan, implementation log, and control folder.

### Component Contract

The requirements, public interfaces, invariants, dependency boundaries, non-goals, and test obligations for one component.

### Implementation Phase

A set of Agent Executions implementing part of one Component plan. A phase is not required to be a separate branch.

### Wave

A batch of component Implementation Phases that may run in parallel subject to the dependency graph.

### Control Folder

The component-local active-slot interface between Orchestrator and agents.

## Component structure

Each component should contain:

~~~text
docs/component-contract.md
docs/implementation-plan.md
docs/implementation-log.md
docs/dependency-map.md
docs/decisions.md
control/README.md
control/state.md
control/log/.gitkeep
~~~

`control/prompt.md` and `control/report.md` are active files. They should not be pre-created in an idle scaffold.

## Control active-slot invariant

At any time, a component control folder may have only one active prompt and one active/latest report:

~~~text
control/prompt.md
control/report.md
control/state.md
control/log/
~~~

Rules:

- Orchestrator creates the next active `control/prompt.md`.
- Orchestrator archives old `prompt.md` and `report.md` into `control/log/`.
- Workers read `control/prompt.md`.
- Workers write `control/report.md`.
- Workers do not archive control files unless explicitly instructed.
- Workers do not search old prompts unless the active prompt points to them.

Recommended archive naming:

~~~text
YYYYMMDD-HHMMSSZ-<wave>-<phase>-<agent-role>-prompt.md
YYYYMMDD-HHMMSSZ-<wave>-<phase>-<agent-role>-report.md
~~~

## Roles

### Orchestrator

Owns wave planning, control-slot management, prompt creation, report triage, CI interpretation, and next-step decisions.

### Implementation Worker

Writes code inside the assigned component scope according to the active control prompt.

### Clean-Code Reviewer

A mandatory quality gate after implementation self-accept and before CI merge readiness.

Responsibilities:

- clean-code refactoring;
- code review;
- bug hunt;
- simplicity/KISS review;
- contract conformance review;
- test honesty review;
- local fixes inside the component scope.

### Fixer Worker

Reads failing CI logs and fixes the minimum cause of failure inside the appropriate component scope.

### Architect

Optional/manual for the current project stage. Invoke Architect when contracts, dependencies, or component boundaries are suspected to be wrong.
