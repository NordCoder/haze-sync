# Decisions: core

## 2026-07-05 — T0-P1 documents current Core contract as a process test

Decision:
Treat T0-P1 as a documentation-only process test that replaces generic Core scaffold docs with a current component contract, implementation plan, dependency map, implementation-log entry, control state update, and report.

Rationale:
The component-centric workflow needs an accurate active contract before future implementation, clean-code review, CI, and fixer passes can safely operate on `component/core`. The current Core source already exposes post-W3 pure service-layer primitives, while the component docs were still scaffold placeholders. Documenting the current state reduces ambiguity without changing product behavior.

Alternatives:
Leave scaffold docs in place until the next feature task, or mix documentation with source changes. Leaving scaffolds would make later worker scope and contract checks weaker. Mixing source work into this prompt would exceed the stated process-test goal.

Consequences:
Future Core agents should treat these docs as the component-local baseline and update them when Core behavior changes. This decision does not change Rust source, route wiring, storage ownership, adapter behavior, or runtime behavior.

Affected contracts:
`crates/haze-sync-core/docs/component-contract.md`, `crates/haze-sync-core/docs/implementation-plan.md`, `crates/haze-sync-core/docs/dependency-map.md`, `crates/haze-sync-core/docs/implementation-log.md`, and `crates/haze-sync-core/control/state.md`.
