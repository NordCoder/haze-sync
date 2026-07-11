# W1-SRV-P7B1 — Real Worktree cycle executor contract fan-in

Before starting, name this worker chat exactly:

`server — W1 SRV-P7B1 Worktree Cycle Executor`

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: implementation-worker

Work only through the GitHub connector. Do not merge the PR, change its draft state, rebase, reset, rewrite history, force-push, modify `main`, modify `component/worktree`, or modify sibling branches.

## Accepted baseline

SRV-P7A is fully accepted:

- implementation: `SELF_ACCEPT`
- CI fixer: `FIX_COMPLETE`
- clean-code review: `CLEAN_ACCEPT`
- final reviewed code-bearing SHA: `37706634fd8dd2d9b299a1c453718f2de63981d0`
- authoritative Component CI run: `29161721748`
- run number: `1704`
- conclusion: `success`
- passed gates: cargo fmt, cargo check, cargo test, cargo clippy, Finalize CI diagnostics

Accepted Worktree fan-in source:

- source branch: `component/worktree`
- source SHA: `4f7bc748d9b901d7d5c3e43c845ba407c0c36e59`
- source CI run: `29152965199`, run number `1665`, conclusion `success`
- synchronized product files: `32/32` exact destination/source blob identity
- non-identical synchronized files: none
- Worktree `control/**`, prompt, report, state, and workflow files copied: none

The active Server boundary currently:

- maps all shared adapter modes explicitly;
- keeps Disabled inert;
- keeps DryRun unsupported;
- constructs, starts, retains, and shuts down the composition boundary safely;
- reports enabled modes as `Unavailable(CycleExecutorNotWired)`;
- does not yet execute a real Core/API/Storage-backed Worktree cycle.

## Why this is a bounded mini-phase

This phase is `SRV-P7B1`, not the whole of broad SRV-P7B. Its purpose is to determine and implement the smallest real cycle-execution bridge supported by the already accepted public contracts.

Do not combine this with an unbounded scheduler, watcher implementation, public status route, deployment topology, CLI integration, provider integration, or new configuration vocabulary.

## Required reads

Before editing, read:

- project implementation manifest, report template, implementation-worker prompt, and GitHub connector guidance;
- current Server control state and this prompt;
- archived SRV-P7A implementation, recovery, fixer, and clean-review prompt/report evidence;
- Server component contract, implementation plan SRV-P7 section, dependency map, decisions, implementation log, Cargo manifest, `src/main.rs`, `src/worktree_runtime.rs`, config, state, readiness, route transaction helpers, and existing Core/API/Storage integration code;
- accepted Worktree contract/docs and all public scanner, import-planning, guarded-delete, reconciliation, materialization, trash, echo-guard, and runtime-cycle interfaces already synchronized into this branch;
- Core/API/Storage public contracts used by existing PUT, DELETE, changes, current-revision, content retrieval, conflict preservation, operation-log, and object-store paths.

Do not read or download CI diagnostics artifacts unless a new CI run fails and the Orchestrator later assigns a fixer prompt with exact artifact metadata.

## Part A — Mandatory contract-feasibility audit

Before product edits, prove whether the current accepted contracts can support a real Server-owned implementation of `WorktreeRuntimeCycle` without architectural violations.

Verify explicitly:

1. how a cycle obtains authoritative Worktree state and Core revisions;
2. how local put/delete facts can reuse accepted Core/API/Storage transaction semantics instead of duplicating route policy;
3. how bounded authoritative exports can retrieve revision metadata and bytes;
4. how Worktree applied-state/reconciliation state is loaded and persisted, if persistence is required;
5. whether the synchronous `WorktreeRuntimeCycle::run_cycle` contract can safely compose the current asynchronous SQLx/Tokio Server/Storage interfaces;
6. whether existing config provides every value needed for a real executor and deterministic invocation;
7. whether implementation would require nested runtimes, `block_on`, route self-calls, fake clients, in-memory production-only state, hidden globals, or duplicated transaction/policy code.

If the accepted contracts are insufficient, do not improvise. Write an implementation report with:

- `STATUS: BLOCKED_BY_CONTRACT`;
- the exact missing or incompatible contract;
- exact source locations proving the mismatch;
- the minimum owner component and API change required;
- why any tempting workaround would be unsafe or architecturally invalid;
- no speculative product implementation.

A truthful contract blocker is an acceptable outcome for this mini-phase.

## Part B — Implementation, only if current contracts are sufficient

If and only if Part A proves the existing contracts are sufficient, implement the smallest real Server-owned cycle executor bridge.

Required properties:

1. Implement a real `WorktreeRuntimeCycle` adapter in Server-owned code.
2. Reuse accepted Core/API/Storage services or extracted transaction boundaries; do not invoke Server HTTP routes internally.
3. Do not reimplement Core base-revision, conflict, delete-guard, idempotency, or overwrite policy inside the executor.
4. For modes that observe/import local state, require an authoritative full scan before planning any local facts.
5. Respect `max_import_actions`, `max_delete_candidates`, and `max_export_actions` exactly.
6. Never submit more imports than were planned.
7. Preserve conflict-saving and tombstone semantics through the accepted Core/API/Storage path.
8. For export-capable modes, materialize only authoritative bounded revisions/changes and preserve dirty local changes through Worktree’s accepted deferral/import contract.
9. Keep Disabled inert and DryRun unsupported.
10. Return only accepted safe `WorktreeRuntimeCycleFailure` categories; do not expose DB URLs, absolute roots, tokens, raw SQLx/I/O errors, request bodies, or internal debug payloads.
11. Do not create a fake watcher or executor.
12. Do not claim hosted periodic execution unless a real, explicit, bounded host invocation is implemented and tested.
13. If only a deterministic explicit one-cycle bridge is possible with current config, keep periodic/background hosting honestly deferred to `SRV-P7B2` and report that limitation precisely.
14. Preserve the accepted 32-file Worktree snapshot exactly; no edits under `crates/haze-sync-worktree/**`.

## Async/sync hard rule

The Worktree runtime cycle trait is synchronous while Server/Storage may use async SQLx/Tokio interfaces.

Forbidden workarounds:

- creating a nested Tokio runtime;
- calling `Handle::block_on` from an async runtime thread;
- using ad hoc blocking around async DB operations;
- spawning detached tasks and returning a fabricated synchronous summary;
- using test doubles or in-memory repositories in production wiring;
- calling localhost HTTP routes as an internal client;
- weakening the Worktree runtime contract.

If this boundary cannot be implemented correctly with current contracts, return `BLOCKED_BY_CONTRACT`.

## Tests

If implementation proceeds, add focused Server-local tests proving:

- full scan precedes local fact planning where required;
- mode permissions are enforced for import and export work;
- all three budgets are hard bounds;
- import counts cannot exceed planned counts;
- Core/API/Storage failures map to safe cycle categories;
- conflict/tombstone outcomes are preserved;
- dirty local content is not overwritten by exports;
- Disabled performs no filesystem, DB, object-store, or mutation work;
- DryRun remains unsupported;
- no absolute root, DB URL, token, raw SQLx/I/O error, or request body appears in status/errors/debug output;
- any explicit invocation lifecycle is deterministic, cancellable where applicable, and does not overlap cycles;
- dependency-free router/state tests remain unchanged.

Do not weaken or delete existing tests.

## Allowed files

- `crates/haze-sync-server/src/worktree_runtime.rs`
- a new Server-local module under `crates/haze-sync-server/src/` only when narrowly required for the real executor bridge
- `crates/haze-sync-server/src/main.rs` only for explicit bounded invocation/composition changes
- existing Server-local transaction/service helpers only when necessary to expose reusable, behavior-preserving internal boundaries
- Server-local tests for those files
- `crates/haze-sync-server/docs/implementation-log.md`
- `crates/haze-sync-server/docs/decisions.md` only for a factual decision record
- `crates/haze-sync-server/control/report.md`

If implementation requires changes outside this list, report `BLOCKED_BY_SCOPE` or `BLOCKED_BY_CONTRACT`; do not expand silently.

## Forbidden scope

- no changes under `crates/haze-sync-worktree/**`;
- no Core/API/Storage/Common/GDrive/Obsidian/CLI/Deployment source changes;
- no Cargo dependency or feature changes unless an existing accepted dependency is merely used;
- no migrations or schema changes;
- no new HTTP route or public DTO;
- no public readiness/status contract change;
- no new environment variable or configuration field;
- no watcher implementation;
- no unbounded or detached background task;
- no provider calls;
- no hard delete or automatic destructive repair;
- no workflow changes;
- no unrelated cleanup;
- do not archive control files.

## CI policy

If product source, tests, or docs change, commit normally and require a new code-bearing Component CI run.

Required gates:

- cargo fmt;
- cargo check;
- cargo test;
- cargo clippy with warnings denied;
- Finalize CI diagnostics.

CI skip is permitted only for the final report-only commit. A skipped run is never code-bearing evidence.

If CI fails, do not inspect raw job logs or guess. Record the exact failing run and stop; the Orchestrator will retrieve exact diagnostics artifact metadata and assign a fixer.

## Report

Write `crates/haze-sync-server/control/report.md` using `report-template.md`.

Set:

- `REPORT_TYPE: IMPLEMENTATION`
- `phase_id: SRV-P7B1`
- `chat_name: server — W1 SRV-P7B1 Worktree Cycle Executor`

Use an honest status, including as applicable:

- `SELF_ACCEPT`
- `SELF_ACCEPT_PENDING_CI`
- `SELF_NEEDS_FIX`
- `BLOCKED_BY_CONTRACT`
- `BLOCKED_BY_SCOPE`
- `BLOCKED_BY_TOOLING`

The report must include:

- complete Part A contract-feasibility findings;
- exact contracts and source paths inspected;
- whether sync/async composition is valid;
- exact changed files and behavior, if implementation proceeded;
- exact tests and mode/budget guarantees;
- exact final code-bearing SHA and authoritative CI run, if any;
- secrecy and non-goal assessment;
- whether periodic/background hosting remains deferred to `SRV-P7B2`;
- the next exact owner/agent required.

Do not recommend clean-code review for a contract-blocked outcome.