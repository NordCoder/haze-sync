# Development Model

## Status

This document is the canonical tracked development model for Haze Sync.

```text
protocol version: 2
development model: component-centric
separate self-review phase: disabled
architect gate: optional/manual
process/scaffold product-code changes: forbidden unless explicitly scoped
```

## Core rule

A worker is bounded by component.

A component is the allowed scope. It is not necessarily one function, one file, or one small subtask.

If the prompt is narrow but the required change still belongs to the same component, the worker may make the broader internal component change and must explain the scope expansion in the report.

If the required change crosses component boundaries or requires changing another component contract, the worker must stop or report a contract/dependency blocker instead of implementing around the boundary.

## Source order for workers

Workers read sources in this order:

```text
1. canonical process model, when referenced by the prompt
2. assigned component contract
3. assigned component implementation plan
4. assigned component implementation log
5. assigned component dependency map
6. active component control prompt
7. current repository code
```

Old prompts, old reports, failed branches, failed attempts, local handoff archives, and project-source files are read-only context unless the active prompt explicitly says otherwise.

## Component structure

Every component should have:

```text
docs/component-contract.md
docs/implementation-plan.md
docs/implementation-log.md
docs/dependency-map.md
docs/decisions.md
control/README.md
control/state.md
control/log/.gitkeep
```

`control/prompt.md` and `control/report.md` are active-slot files. They must not be pre-created in idle component scaffolds.

Process-only components may use an isolated root such as `docs-process/docs/**` when that avoids conflict with product/system documentation.

## Control slot invariant

Each component control folder has one active prompt slot and one active report slot:

```text
control/prompt.md
control/report.md
control/state.md
control/log/
```

Rules:

- Orchestrator creates the next active `control/prompt.md`.
- Orchestrator archives old `prompt.md` and `report.md` into `control/log/` before writing a new prompt.
- Workers read `control/prompt.md` unless it explicitly references other control files.
- Workers write `control/report.md` unless explicitly instructed otherwise.
- Workers do not archive prompt/report files.
- Workers do not search old prompts unless the active prompt points to them.
- Active prompt/report files are process state, not static docs.

Recommended archive naming:

```text
YYYYMMDD-HHMMSSZ-<wave>-<phase>-<agent-role>-prompt.md
YYYYMMDD-HHMMSSZ-<wave>-<phase>-<agent-role>-report.md
```

## Roles

### Orchestrator

Owns planning, component scheduling, dependency ordering, control-slot management, prompt creation, report triage, CI interpretation, fan-in planning, and next-step decisions.

Orchestrator may write prompts. Orchestrator should not implement product code unless explicitly instructed.

### Implementation Worker

Implements the assigned phase inside the assigned component scope.

Implementation Worker reads the active component prompt, changes code/docs within scope, performs final internal verification, and writes the implementation report.

There is no separate self-review phase. The implementation report is written after the worker's own internal verification.

### Clean-Code Reviewer

Mandatory quality gate after implementation accept and before CI merge readiness.

Clean-Code Reviewer is both cleaner and reviewer. It may edit code inside the same component scope to fix found bugs, reduce duplication, improve naming, simplify structure, split large functions/files where justified, and verify component-contract compliance.

Clean-Code Reviewer must preserve behavior unless it is fixing a found bug.

### Fixer Worker

Reads failing CI logs and fixes the minimum cause of failure inside the appropriate component scope.

Fixer must not delete tests merely to make CI pass. If the CI failure requires a contract or cross-component change, report a contract blocker.

### Architect

Optional/manual in the current project stage.

Invoke Architect when contracts, dependency graph, component boundaries, or high-risk design decisions require review.

Architect review is not a mandatory automatic phase after every wave.

## Lifecycle

The active lifecycle is:

```text
implementation -> clean-code-review -> CI -> fixer loop if needed
```

Expanded:

```text
1. Orchestrator writes one active component prompt.
2. Implementation Worker performs the implementation phase.
3. Worker writes `control/report.md`.
4. Orchestrator triages the report.
5. Clean-Code Reviewer runs when implementation self-accepts.
6. CI runs or is observed.
7. Fixer Worker runs only for CI failures or scoped defects.
8. Repeat fixer loop until CI is green or blocked.
9. Architect is invoked manually only when needed.
```

## Wave and phase semantics

A wave is a scheduling convenience: a set of implementation phases that may run in parallel when dependencies allow.

An implementation phase is a bounded unit of work within one component branch.

The current model does not require a separate Architect gate after every wave. Architect review can be scheduled manually after high-risk fan-in, contract drift, or dependency changes.

## Branch policy

Rules:

- Do not write directly to `main`.
- Process migrations use `process/*` branches.
- Component work uses `component/*` branches.
- One component branch may contain multiple implementation, clean-code-review, and fixer passes.
- Workers must not modify sibling branches.
- Force-push/history rewrite is forbidden unless explicitly authorized.
- Product-code changes in process/scaffold migrations are forbidden unless explicitly scoped.

## Status vocabulary

Implementation statuses:

```text
SELF_ACCEPT
SELF_ACCEPT_PENDING_CI
SELF_NEEDS_FIX
BLOCKED_BY_CONTRACT
BLOCKED_BY_DEPENDENCY
BLOCKED_BY_TOOLING
```

Clean-Code Reviewer statuses:

```text
CLEAN_ACCEPT
CLEAN_ACCEPT_PENDING_CI
CLEAN_NEEDS_FIX
CLEAN_BLOCKED_BY_CONTRACT
CLEAN_BLOCKED_BY_SCOPE
CLEAN_BLOCKED_BY_TOOLING
```

Fixer statuses:

```text
FIX_COMPLETE
FIX_NEEDS_MORE
FIX_BLOCKED_BY_LOGS
FIX_BLOCKED_BY_CONTRACT
FIX_BLOCKED_BY_TOOLING
```

CI statuses:

```text
CI_UNKNOWN
CI_PENDING
CI_GREEN
CI_RED
```

Architect statuses:

```text
ARCHITECT_ACCEPT
ARCHITECT_NEEDS_CHANGES
ARCHITECT_CHANGED_CONTRACTS
ARCHITECT_BLOCKED
```

Docs/process planning may use `ARCHITECT_ACCEPT_PENDING_REVIEW` in implementation logs as a transitional planning status, but worker reports should use the canonical status vocabulary above unless the active prompt explicitly extends it.

## Check honesty

Agents must not claim checks passed unless they actually ran locally or observed CI metadata.

If working through a connector without shell execution, reports must say which shell commands were not run and why.

Docs-only passes should not imply Rust/npm/compose checks passed unless those checks were executed or observed.

## Safety and secrecy

Agents must not commit:

- real secrets;
- production `.env` files;
- OAuth tokens;
- token hashes in public reports;
- database URLs;
- generated logs/dumps/archives;
- local notes;
- build outputs;
- prompt/report files outside component control folders.

Public outputs must not expose:

- raw SQLx errors;
- stack traces;
- local absolute paths;
- provider payloads;
- bearer tokens;
- OAuth tokens;
- token hashes;
- Idempotency-Key values.

## Contract-change protocol

Request a contract change when the current contract would cause impossible implementation, serious complexity, unsafe behavior, or major technical debt.

A contract-change request reports:

```text
requested change
rationale
affected components
risk if unchanged
suggested owner
```

Orchestrator either writes follow-up prompts or invokes Architect manually.

## Superseded wording

The following older model fragments are superseded by this document:

- mandatory Architect review after every wave;
- separate self-review phase after implementation;
- worker scope limited to one file, function, or tiny task;
- workers archiving active prompt/report files;
- CI interpreted as production deployment readiness;
- process/scaffold migrations freely changing product code.
