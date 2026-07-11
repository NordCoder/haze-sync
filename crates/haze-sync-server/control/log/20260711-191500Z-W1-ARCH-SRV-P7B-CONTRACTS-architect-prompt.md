# W1-ARCH-SRV-P7B — Resolve the Worktree async execution contract

Before starting, name this worker chat exactly:

`server — W1 SRV-P7B Architecture Decision`

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: architect

Work only through the GitHub connector. Do not merge the PR, change its draft state, rebase, reset, rewrite history, force-push, modify `main`, modify sibling branches, or implement product code.

## Trigger

SRV-P7B1 completed its mandatory feasibility audit and returned `BLOCKED_BY_CONTRACT`. The archived evidence is:

- prompt: `crates/haze-sync-server/control/log/20260711-184500Z-W1-SRV-P7B1-implementation-worker-prompt.md`
- report: `crates/haze-sync-server/control/log/20260711-184500Z-W1-SRV-P7B1-implementation-worker-report.md`

The blocker is architectural, not a local implementation defect:

1. Worktree runtime scheduling and `WorktreeRuntimeCycle::run_cycle` are synchronous.
2. Authoritative Server/Storage operations are asynchronous SQLx/Tokio operations.
3. Correct PUT/DELETE/conflict/idempotency/export behavior is currently embedded in private async route orchestration rather than reusable internal services.
4. Durable Worktree applied/reconciliation state and export-cursor contracts are not accepted.
5. Runtime budgets and invocation policy are absent from accepted configuration.

Changing only Server implementation code is insufficient. Forbidden workarounds include nested Tokio runtimes, `Handle::block_on`, ad hoc blocking, internal HTTP self-calls, detached tasks with fabricated synchronous summaries, duplicated route policy, and fake in-memory production state.

## Accepted baseline

SRV-P7A remains fully accepted:

- implementation: `SELF_ACCEPT`
- fixer: `FIX_COMPLETE`
- clean review: `CLEAN_ACCEPT`
- final code-bearing SHA: `37706634fd8dd2d9b299a1c453718f2de63981d0`
- Component CI run: `29161721748`, run number `1704`, conclusion `success`

Accepted Worktree snapshot:

- source: `component/worktree@4f7bc748d9b901d7d5c3e43c845ba407c0c36e59`
- source CI: `29152965199`, run number `1665`, conclusion `success`
- synchronized product files: `32/32` exact blob identity

Do not reopen accepted SRV-P7A behavior except where the new contract decision explicitly requires a later owner-component change.

## Required reads

Read before deciding:

- project implementation manifest, report template, GitHub connector guidance, and architecture/worker process rules;
- current Server state and this prompt;
- the archived SRV-P7B1 prompt/report;
- all archived SRV-P7A implementation, recovery, fixer, and clean-review evidence;
- Server component contract, implementation plan, dependency map, decisions, implementation log, Cargo manifest, config, startup, state, readiness, `worktree_runtime.rs`, route transaction helpers, repository use, and object-store integration;
- Worktree component contract, runtime-service docs, dependency map, decisions, scanner/import/delete/reconciliation/materialization/echo/trash public interfaces, and runtime traits;
- Storage contracts, repository traits, migrations/tables related to operation log, revisions, conflicts, idempotency, tombstones, adapter/worktree state, and any existing worktree-state schema;
- Core/API contracts governing base revisions, conflicts, deletes, idempotency, changes, and public/internal DTO ownership;
- CLI and Deployment blockers that depend on accepted Server runtime/operator fan-in.

Do not read CI diagnostic artifacts. This is an architecture pass, not a fixer pass.

## Required architecture decision

Produce one preferred target architecture. Do not return a menu of equally ranked options.

You must resolve all four decision areas below.

### Decision A — Async Worktree runtime boundary

Choose the exact async-compatible contract and ownership model that replaces or supplements the incompatible synchronous cycle boundary.

Evaluate at minimum:

- making the Worktree runtime service and cycle executor natively async;
- keeping synchronous filesystem primitives but introducing an async host/executor orchestration layer;
- preserving a synchronous scheduler only if it can remain correct without blocking async authoritative operations.

The chosen design must specify:

- exact trait/type ownership by component;
- proposed method signatures at contract level;
- whether `poll`, `run_cycle`, or both become async-compatible;
- cancellation and shutdown semantics;
- at-most-one-cycle/no-overlap guarantees;
- watcher-hint and periodic full-scan correctness;
- safe failure and count-only status mapping;
- compatibility/migration path for existing Worktree tests and public types;
- why the rejected designs are inferior or unsafe.

Do not solve this with blocking or hidden task spawning.

### Decision B — Reusable Server application-service boundary

Define the internal async service layer that both HTTP routes and Worktree execution will use.

Specify concrete service operations and contracts for at least:

- import/create-or-update from a normalized local Worktree fact;
- guarded local delete/tombstone submission;
- bounded authoritative changes/export retrieval;
- revision metadata and content retrieval needed for materialization;
- conflict-saving outcomes;
- idempotency ownership and key derivation for non-HTTP Worktree operations;
- transaction, advisory-lock, object-store, operation-log, and commit ownership.

The architecture must guarantee that:

- routes become transport adapters over reusable services rather than the only owners of correct behavior;
- Core policy is not duplicated in Server;
- Worktree never calls HTTP handlers internally;
- Worktree execution receives typed outcomes compatible with its accepted planning/materialization contracts;
- route behavior and public API shapes remain backward-compatible.

Name the proposed Server modules/types and identify which existing private route helpers should be extracted, retained, or deleted in later implementation phases.

### Decision C — Durable Worktree state and cursor persistence

Choose the owner and contract for:

- last-applied Worktree path/revision/content-hash state;
- reconciliation state;
- authoritative export cursor/checkpoint;
- runtime counters/status that must survive restart, if any;
- atomicity between accepted Core mutations/materialization and state advancement.

Determine whether the existing Storage schema is sufficient. If not, define the minimum migration/schema change and repository interfaces without implementing them.

Specify:

- owning component;
- data model and keys;
- serialization/versioning policy;
- load/save/update transaction semantics;
- crash/restart behavior;
- cursor advancement rules;
- whether state is per adapter instance, vault, root, or another identity;
- cleanup/retention policy;
- tests required to prove no skipped export, duplicate unsafe import, or false last-applied state.

Production-only in-memory state is not acceptable.

### Decision D — Runtime policy, configuration, and invocation

Define accepted ownership and defaults for:

- import/delete/export budgets;
- periodic correctness interval;
- watcher hint debounce and hint budget;
- explicit one-cycle/manual invocation, if retained;
- adapter identity and durable-state binding;
- startup behavior;
- cancellation and graceful shutdown;
- readiness/status semantics;
- disabled, read-only, import-only, export-only, bidirectional, and DryRun behavior.

Choose whether values are:

- explicit config fields;
- deterministic documented defaults;
- or a bounded mixture of both.

No hidden unbounded background work is allowed. Any hosted loop must be explicit, cancellable, bounded, observable through safe status, and owned by a named component.

## Required phased execution plan

Convert the architecture into small owner-aligned implementation phases. At minimum decide whether separate phases are required for:

1. Worktree async runtime contract change;
2. Server reusable application-service extraction;
3. Storage durable state/cursor repository and migration;
4. Server real bounded Worktree cycle executor;
5. Server hosted scheduling/startup/shutdown integration;
6. safe status/readiness integration;
7. CLI operator commands;
8. Deployment runtime/config fan-in.

For each phase provide:

- phase ID and exact suggested chat name;
- owner component and branch;
- role;
- prerequisites;
- allowed files;
- forbidden scope;
- deliverables;
- test/CI requirements;
- clean-code review gate;
- downstream unblocks.

The order must prevent two branches from independently defining the same contract. Contract-owner phases must precede consumers.

## Required documentation changes

This architect may update only Server-owned documentation on `component/server`:

- `crates/haze-sync-server/docs/component-contract.md`
- `crates/haze-sync-server/docs/implementation-plan.md`
- `crates/haze-sync-server/docs/dependency-map.md`
- `crates/haze-sync-server/docs/decisions.md`
- `crates/haze-sync-server/docs/implementation-log.md`
- `crates/haze-sync-server/control/report.md`

Use Server docs to record the accepted cross-component decision and the required owner-component follow-up phases. Do not modify Worktree, Storage, Core, API, Common, CLI, Deployment, source code, Cargo manifests, migrations, workflows, or sibling control files in this branch.

If the correct decision cannot be made from current evidence, return `ARCHITECT_BLOCKED` and state the exact missing evidence. Do not invent contracts.

## Architecture invariants

The final decision must preserve:

- Core ownership of sync/conflict/delete policy;
- API ownership of public HTTP DTOs and parsing;
- Storage ownership of durable persistence primitives;
- Worktree ownership of scanning, planning, reconciliation, materialization, echo suppression, trash, and runtime scheduling primitives;
- Server ownership of runtime composition, application-service transactions, and host lifecycle;
- no provider behavior inside generic Server routes;
- no hard delete;
- no automatic destructive repair;
- no raw secrets, DB URLs, tokens, absolute roots, raw SQLx/I/O errors, or internal payloads in status/errors/logs;
- dependency-free router tests and existing public route behavior;
- accepted Worktree snapshot behavior until an explicit Worktree-owner contract phase changes it.

## CI policy

Documentation changes must run Component CI normally. Do not use CI skip for contract, plan, dependency-map, decision, or implementation-log changes.

Only the final report-only commit may use `[skip ci]`. A skipped run is not architecture validation evidence.

If docs CI fails, record the exact run and stop. Do not inspect raw logs or guess; Orchestrator will route an artifact-based fixer if required.

## Report

Write `crates/haze-sync-server/control/report.md` using `report-template.md`.

Set:

- `REPORT_TYPE: ARCHITECT_REVIEW`
- `phase_id: ARCH-SRV-P7B-CONTRACTS`
- `chat_name: server — W1 SRV-P7B Architecture Decision`

Use one honest status:

- `ARCHITECT_ACCEPT`
- `ARCHITECT_CHANGED_CONTRACTS`
- `ARCHITECT_NEEDS_CHANGES`
- `ARCHITECT_BLOCKED`

The report must include:

- the chosen target architecture, not only the problem statement;
- exact contract decisions A–D;
- rejected alternatives and rationale;
- exact owner components for every contract;
- compatibility and migration strategy;
- persistence and crash-recovery semantics;
- security/secrecy assessment;
- phased implementation order with exact suggested chats;
- which prompts the Orchestrator should activate next and which components remain blocked;
- changed documentation files;
- CI evidence for documentation changes, if any.

Do not implement source code in this architecture phase.
