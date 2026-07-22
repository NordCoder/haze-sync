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

## 2026-07-05 — Core remains a pure decision layer

Decision:

`haze-sync-core` owns sync decisions and safety semantics, not runtime execution. Core may expose traits and pure value models, but concrete persistence, HTTP routing, filesystem writes, provider calls, background jobs, and CLI commands remain outside the component.

Rationale:

Core is the only conflict arbiter. Keeping the arbiter pure makes it testable, deterministic, and reusable by Server/Storage/API/adapters without hidden side effects or runtime coupling.

Alternatives:

- Let Core own SQLx repositories and transactions.
- Let Core own Axum route handlers.
- Let adapters implement conflict/delete decisions locally.
- Move safety decisions into Server.

Consequences:

- Storage/Server must provide transactions, locks, durability, and runtime composition.
- API must map Core semantic outcomes into public DTOs and HTTP statuses.
- Adapters must submit facts and obey Core/API outcomes.
- Core tests can remain provider-free and database-free.

Affected contracts:

- Core component contract;
- Storage repositories and transaction plan;
- Server runtime wiring plan;
- API route/error mapping plan;
- adapter behavior plans.

## 2026-07-05 — Conflict preservation is represented before it is persisted

Decision:

Core may represent `conflict_saved` outcomes and conflict-preservation plans, including incoming content bytes held in memory for downstream fan-in, but it does not persist conflict records, write conflict-copy blobs, or append durable conflict operations.

Rationale:

The safety decision must happen in Core, but persistence requires Storage/Server transaction boundaries and object-store durability. Keeping the plan in Core prevents adapters/API/storage from inventing conflict policy while still preserving ownership boundaries.

Alternatives:

- Persist conflict rows directly from Core.
- Return only a rejection and force downstream components to decide how to preserve incoming content.
- Let stale writes overwrite current content.

Consequences:

- Downstream fan-in must consume `conflict_saved` plans transactionally.
- Public serialization must continue to skip raw incoming bytes.
- `accept_conflict` full behavior requires future Core/API/Storage/Server coordination.

Affected contracts:

- revision_service;
- conflict_saved_planner;
- conflict_service;
- storage conflict repository plan;
- API conflict route mapping;
- Server file-write fan-in.

## 2026-07-05 — Delete safety is pure policy; deletion side effects live elsewhere

Decision:

Core owns tombstone metadata models, retention validation, and mass-delete guard decisions. It does not move files to trash, call providers, delete blobs, hard-delete rows, or run cleanup jobs.

Rationale:

Delete safety must be global and consistent, but physical delete/trash/provider behavior is component-specific. Keeping Core side-effect-free avoids accidental data loss and keeps adapter runtime behavior testable in isolation.

Alternatives:

- Put hard-delete cleanup in Core.
- Let each adapter decide mass-delete thresholds.
- Let Storage delete rows immediately.

Consequences:

- Worktree/GDrive/Obsidian/Storage/Server must implement side effects only after Core/API-safe decisions.
- Retention cleanup requires a separate future contract.
- Delete guard defaults become shared safety semantics.

Affected contracts:

- tombstone_service;
- delete_guard;
- Storage tombstone repository;
- Server DELETE route;
- adapter delete reconciliation;
- CLI/doctor safety checks.

## 2026-07-05 — Doctor models are passive and redacted

Decision:

Core doctor types represent check results and aggregate summaries only. Live checks are performed by Server/CLI/Storage/adapters and supplied to Core as already safe facts.

Rationale:

Doctor output needs a common semantic shape, but live checks require DB pools, object-store access, filesystem paths, provider calls, and credentials. Those dependencies do not belong in Core.

Alternatives:

- Let Core run live database and object-store checks.
- Let every downstream component invent its own doctor status model.
- Include raw diagnostics in Core doctor details.

Consequences:

- CLI/Server must honestly mark skipped checks.
- Core doctor outputs remain safe for JSON/API/CLI/reporting.
- Real doctor completeness is achieved through downstream integration phases, not Core alone.

Affected contracts:

- doctor module;
- CLI doctor plan;
- Server admin/doctor routes;
- Storage object/blob checks;
- GDrive/Worktree mapping/drift checks.

## 2026-07-05 — Core output models are semantic contracts, not API DTO ownership

Decision:

Core may expose serializable semantic models, but API owns public HTTP DTO shape, status mapping, header extraction, and route contracts.

Rationale:

Core needs testable and mappable outcomes. However, public HTTP compatibility, route naming, request extraction, auth, and response envelopes are API/Server responsibilities. Blurring that boundary makes component-parallel development harder.

Alternatives:

- Move all public DTOs into Core.
- Let API duplicate Core semantic enums by hand.
- Let Server map raw internal Core types directly without API-owned contracts.

Consequences:

- API should preserve Core semantics but can wrap them in HTTP-specific shapes.
- Core must avoid embedding HTTP status codes or route-specific fields.
- Compatibility fixtures may be useful later to keep mapping stable.

Affected contracts:

- Core public models;
- API DTOs and route helpers;
- Server route implementation;
- adapter/client expectations.
