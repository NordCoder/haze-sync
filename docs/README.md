# Haze Sync system documentation

This directory contains repository-level documentation for the Haze Sync system.

System documentation describes how the product behaves as a whole: architecture, runtime topology, global safety rules, integration sequence, test gates, deployment assumptions, and operational behavior.

Component documentation lives inside each component and describes the local contract, local implementation plan, dependency map, implementation log, and component decisions.

## Documents

- [`system-architecture.md`](system-architecture.md) — product architecture, runtime model, current implementation status, and V1 boundaries.
- [`component-boundaries.md`](component-boundaries.md) — component responsibilities, allowed dependency directions, and cross-component boundary decisions.
- [`safety-and-sync-semantics.md`](safety-and-sync-semantics.md) — global sync invariants, base revision behavior, conflict handling, delete policy, adapter rules, and security constraints.
- [`integration-testing-rollout.md`](integration-testing-rollout.md) — component-parallel development model, wiring strategy, test gates, bootstrap order, rollout, and operational readiness.

## Documentation boundaries

Repository-level documentation should answer system questions:

```text
What is the product?
How do components compose?
Where are runtime boundaries?
Which invariants are global?
Which integration gates prove the system works?
```

Component-level documentation should answer component questions:

```text
What does this component own?
What interfaces does it expose?
What inputs and outputs does it accept?
Which local phases implement it?
Which dependencies are allowed?
What must remain out of scope?
```

The two layers should not duplicate each other in detail. System docs may name component responsibilities and dependency directions, but detailed phase plans belong to component docs.

## Current product baseline

Haze Sync V1 is a centralized file-level sync system for:

- an Obsidian vault;
- a VPS worktree used by agents;
- a Google Drive replica;
- server-side metadata, object storage, conflict handling, delete safety, and operation history.

The runtime mental model is:

```text
Core metadata + object store = authoritative sync state
Worktree = materialized filesystem view for agents
Google Drive = external replica
Obsidian/iPhone vault = external replica
```

The system is intentionally not CouchDB/LiveSync-based in V1.

## Current repository baseline

The repository already contains the server-side W2/W3 foundation: shared primitives, storage schema/repositories, object-store primitives, normal Core upsert behavior, conflict/delete/idempotency primitives, API DTO/helper surfaces, server route surfaces, CLI/doctor scaffolding, CI, deployment scaffold, and Obsidian plugin scaffold.

It is still not a production-complete sync product. The main missing product areas are runtime adapters, bootstrap/import, full worktree materialization/import, full Obsidian sync behavior, Google Drive import/export, production E2E, and rollout hardening.
