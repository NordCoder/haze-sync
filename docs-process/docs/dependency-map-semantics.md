# Dependency Map Semantics

## Purpose

A dependency map is not a serial development roadmap.

A dependency map exists to let components be developed independently without crossing ownership boundaries or creating hidden coupling.

The canonical interpretation is:

```text
components develop independently inside their own component branches
dependency maps define contract boundaries and fan-in points
dependency maps do not impose a global implementation order
```

## Core rule

A component may be developed independently against the currently documented contracts of other components.

If the component needs behavior that is already covered by another component contract, it may consume that contract.

If the component needs behavior that is not covered by another component contract, it must report a contract-change request or dependency blocker instead of implementing around the missing contract.

## What dependency maps answer

A dependency map answers:

```text
what this component consumes
what this component exposes
who depends on this component
which cross-component calls are allowed
which cross-component calls are forbidden
where fan-in is needed after independent work
which contract gaps must be reported
```

It does not answer:

```text
which component must be implemented first in all cases
which worker owns another component
which branch should be merged first automatically
whether production readiness is achieved
whether CI is green
```

## Independent development model

Each component branch may progress independently when work stays inside that component's ownership boundary.

Examples:

```text
Core may implement Core policy and service logic inside Core.
API may evolve API DTO helpers inside API.
Server may wire routes inside Server against API/Core contracts.
Storage may implement repository/persistence details inside Storage.
GDrive adapter may implement provider-facing planning inside GDrive adapter.
Obsidian plugin may implement plugin UI/client behavior inside the plugin component.
Deployment may implement runbooks and local scaffolds inside Deployment.
GitHub CI may implement workflow policy inside github-ci.
Docs-process may implement process docs inside docs-process.
```

These branches do not need to wait for each other merely because a dependency map mentions another component.

## Fan-in points

A fan-in point appears when independently developed component work must be connected, verified, or reconciled.

Common fan-in points:

```text
API shape consumed by Server or adapters
Core service behavior exposed through Server
Storage repository behavior consumed by Core
Deployment service wiring depending on Server/GDrive runtime contracts
Plugin/client behavior depending on Server/API routes
CI jobs depending on component test-support behavior
```

Fan-in is coordination. It is not a reason to block all local component work.

## Contract-change points

A contract-change point appears when a component needs another component to expose or change behavior.

Examples:

```text
GDrive adapter needs a Core/API endpoint not currently defined.
Server needs a Storage repository method not currently contracted.
Obsidian plugin needs an API response field not currently defined.
Deployment needs a runtime config surface not currently accepted.
CI needs a stable test-support feature not currently provided.
```

The correct response is:

```text
CONTRACT_CHANGE_REQUESTED or BLOCKED_BY_DEPENDENCY
```

The incorrect response is:

```text
reach into another component's internals
add duplicate local behavior to avoid the contract
silently change another component's public interface
```

## Allowed wording

Use wording like:

```text
This component can be developed independently inside its component boundary.
The dependency map records contracts to consume and fan-in points to coordinate.
This dependency does not imply a serial implementation order.
A missing upstream contract is reported as a contract-change request.
```

Avoid wording like:

```text
This component cannot start until X is fully implemented.
This dependency map defines the implementation order.
Worker must implement upstream component X first.
Fan-in means this component is blocked from local work.
```

## Dependency gates

The phrase `dependency gate` means:

```text
a cross-component condition that must be satisfied before integration, fan-in, or merge readiness
```

It does not mean:

```text
a prohibition on independent local development inside the component
```

When writing plans, prefer explicit wording:

```text
Integration gate
Fan-in gate
Contract gate
Merge-readiness gate
```

instead of ambiguous generic dependency wording.

## Worker behavior

A worker assigned to a component branch should:

1. Read its component contract and dependency map.
2. Implement inside the component boundary.
3. Use public contracts for other components.
4. Report missing contracts instead of bypassing them.
5. Mark fan-in needs in the report.
6. Avoid modifying sibling components unless explicitly scoped.

## Orchestrator behavior

The Orchestrator uses dependency maps to:

- schedule independent component work;
- detect cross-component contract gaps;
- create fan-in phases;
- decide merge order;
- invoke Architect when boundaries conflict;
- prevent hidden coupling.

The Orchestrator should not interpret every dependency edge as a serial ordering requirement.

## Relationship to component docs

Component-local dependency maps remain the source for component-specific dependency information.

This document defines how those maps should be interpreted.

If a component-local dependency map appears to imply strict global sequencing, interpret it through this document and revise the wording during the next component docs drift pass.
