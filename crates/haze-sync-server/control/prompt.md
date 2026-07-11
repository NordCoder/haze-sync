# W1-SRV-P7A-CLEAN — Clean-code review of Worktree snapshot fan-in and Server composition

Before starting, name this worker chat exactly:

`server — W1 SRV-P7A Clean-Code Review`

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: clean-code-reviewer

Work only through the GitHub connector. Do not merge the PR, change its draft state, rebase, reset, rewrite history, force-push, modify `main`, modify `component/worktree`, or modify sibling branches.

## Accepted implementation evidence

Review the completed SRV-P7A implementation at the exact final code-bearing SHA:

- final_code_bearing_sha: `37706634fd8dd2d9b299a1c453718f2de63981d0`
- authoritative workflow: `Component CI`
- workflow_run_id: `29161721748`
- run_number: `1704`
- status: `completed`
- conclusion: `success`
- passed gates: cargo fmt, cargo check, cargo test, cargo clippy, Finalize CI diagnostics

Accepted Worktree fan-in source:

- source branch: `component/worktree`
- source SHA: `4f7bc748d9b901d7d5c3e43c845ba407c0c36e59`
- source CI run: `29152965199`, run number `1665`, conclusion `success`
- synchronized product inventory: `32/32` destination blobs identical to the exact source paths
- non-identical synchronized files: none
- Worktree `control/**`, prompt, report, state, and workflow files copied: none

The post-implementation fixer used exact diagnostics artifact `8250773210`, found only rustfmt differences, changed no behavior, and produced the green run above.

## Required reads

Before editing, read:

- project implementation manifest, report template, clean-code-reviewer prompt, and GitHub connector guidance;
- this active prompt and current Server control state;
- Server component contract, implementation plan, dependency map, decisions, implementation log, Cargo manifest, `src/main.rs`, configuration/state/readiness/routes, and `src/worktree_runtime.rs`;
- accepted Worktree runtime contract and public runtime types already synchronized into this branch;
- archived SRV-P7A implementation prompt;
- archived SRV-P7A report-recovery prompt and report;
- archived SRV-P7A-FIX implementation prompt and report;
- archived FIX-SRV-P7A-CI fixer prompt and report;
- the relevant Server and Worktree product diff for SRV-P7A.

Do not download or read CI diagnostics artifacts. The active code-bearing CI is green and this prompt does not authorize diagnostics access.

## Review scope

Review the complete SRV-P7A result, with primary attention to Server-owned code:

- `crates/haze-sync-server/Cargo.toml` dependency direction and necessity;
- `crates/haze-sync-server/src/main.rs` construction, ownership, startup, serve, shutdown, and error paths;
- `crates/haze-sync-server/src/worktree_runtime.rs` mode mapping, lifecycle state machine, safe status/debug output, tests, and complexity;
- factual accuracy of the SRV-P7A implementation-log entry;
- exact preservation of the accepted Worktree snapshot.

## Required review questions

Correctness and lifecycle:

1. Is `worktree_runtime.rs` definitely part of the active crate graph and compiled test target?
2. Is `ServerWorktreeRuntime` constructed from the existing configured mode and root before config ownership is moved?
3. Is `start` called exactly once?
4. Is the boundary retained for the entire serve lifetime?
5. Is `shutdown` attempted exactly once after both successful and failed serve completion?
6. Can an error path accidentally skip shutdown, double-start, double-shutdown, or restart after shutdown?
7. Are lifecycle failures mapped to stable, secret-safe startup errors?

Behavioral honesty:

8. Is Disabled completely inert?
9. Are enabled modes honestly unavailable with `CycleExecutorNotWired`, without appearing healthy or operational?
10. Is DryRun explicitly unsupported rather than silently mapped to another mode?
11. Is there any fake watcher, executor, scan/import/export loop, mutation, provider call, repair execution, or background task?
12. Does dependency-free router/state construction remain unchanged?

Clean code and tests:

13. Are module boundaries, naming, ownership, and helper abstractions proportionate and clear?
14. Is there unnecessary duplication or complexity in lifecycle orchestration?
15. Do tests prove exhaustive mode mapping, disabled inertness, explicit start/shutdown semantics, shutdown after success and failure, restart prevention, and redaction?
16. Do any tests overclaim real Worktree execution that remains deferred to SRV-P7B?
17. Are absolute roots, database URLs, tokens, raw filesystem errors, or internal debug payloads exposed through status, Debug, Display, logs, or errors?

Contract and fan-in:

18. Does Server remain composition-only rather than embedding Worktree policy or provider behavior?
19. Are all 32 accepted Worktree product files still exact and unchanged?
20. Were any Worktree control/workflow files introduced after the recovery audit?
21. Are real Core/API/Storage-backed cycle execution and hosted runtime behavior still clearly deferred to SRV-P7B?

## Allowed changes

You may make only narrowly justified clean-code or correctness changes inside:

- `crates/haze-sync-server/src/main.rs`
- `crates/haze-sync-server/src/worktree_runtime.rs`
- Server-local tests in those files
- `crates/haze-sync-server/docs/implementation-log.md` only for factual correction
- `crates/haze-sync-server/control/report.md`

If a required correction needs another Server-local product file, Cargo dependency change, public route/DTO change, or any sibling component change, do not expand silently. Report `CLEAN_BLOCKED_BY_SCOPE` or `CLEAN_BLOCKED_BY_CONTRACT` with the exact required change.

## Forbidden scope

- no changes under `crates/haze-sync-worktree/**`;
- no Core/API/Storage/Common/GDrive/Obsidian/CLI/Deployment changes;
- no Cargo dependency or feature changes;
- no new HTTP route or public DTO;
- no SRV-P7B cycle executor, watcher, polling loop, import/export runtime, persistence wiring, or provider integration;
- no hidden global or unbounded/background task;
- no hard delete or automatic destructive repair;
- no workflow changes;
- no test deletion or assertion weakening;
- no unrelated cleanup;
- do not archive control files.

## CI policy

- If you change product source, tests, or documentation, commit normally and require a new code-bearing Component CI run.
- Do not use CI skip for source, tests, docs, manifests, dependencies, or validation changes.
- If no product change is required, the existing green code-bearing run `29161721748` is valid evidence; only the final report-only commit may use `[skip ci]`.
- A skipped run is never CI evidence.

## Report

Write only `crates/haze-sync-server/control/report.md` using `report-template.md`.

Set:

- `REPORT_TYPE: CLEAN_CODE_REVIEW`
- `phase_id: SRV-P7A-CLEAN`
- `chat_name: server — W1 SRV-P7A Clean-Code Review`

Use an honest status from:

- `CLEAN_ACCEPT`
- `CLEAN_ACCEPT_PENDING_CI`
- `CLEAN_NEEDS_FIX`
- `CLEAN_BLOCKED_BY_CONTRACT`
- `CLEAN_BLOCKED_BY_SCOPE`
- `CLEAN_BLOCKED_BY_TOOLING`

The report must state:

- exact reviewed code-bearing SHA;
- exact changed files, if any;
- lifecycle and error-path findings;
- test adequacy;
- secrecy and non-goal assessment;
- Worktree snapshot/control-file preservation result;
- authoritative CI evidence;
- whether SRV-P7A is ready for Orchestrator progression to the next Server phase.

Do not implement SRV-P7B in this review.