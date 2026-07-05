# Decisions: server

## 2026-07-05 — T0-P3 remains documentation plus tiny cleanup

Decision:
T0-P3 is treated as a component documentation and process-control test with at most one behavior-preserving server-local route cleanup. It is not a broad route refactor or runtime behavior phase.

Rationale:
The T0-P3 implementation prompt asked to replace generic scaffold docs with useful current-state server documentation and permitted only one tiny safe cleanup if obvious. Server route modules already contain W2/W3 wiring across file, changes, conflict, delete, and admin surfaces, so a broad cleanup would be higher risk and belongs in a separately scoped implementation or fan-in phase.

Alternatives:
- Refactor route registration or split oversized route modules now; rejected because the T0-P3 implementation prompt explicitly forbade rewriting `routes/mod.rs` or `routes/v1.rs` and framed the work as a process test.
- Make no source cleanup; allowed, but a small route doc-comment clarification was safe and behavior-preserving.

Consequences:
The component docs now describe the current runtime wiring, ownership boundaries, safety rules, and deferred work without changing public behavior. Any substantial route decomposition, startup wiring, observability, or provider-runtime integration remains deferred to future scoped phases.

Affected contracts:
Server component contract only. No cross-component contract changes requested.
