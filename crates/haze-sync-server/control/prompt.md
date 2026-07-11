# W1-SRV-P7A — Worktree snapshot fan-in and Server composition boundary

Before starting, name this worker chat exactly:

`server — W1 SRV-P7A Worktree Snapshot Fan-In`

Component: server
Primary path: crates/haze-sync-server
Fan-in source path: crates/haze-sync-worktree
Branch: component/server
PR: #45
Role: implementation-worker

Work only through the GitHub connector. Do not merge the PR, change its draft state, rebase, reset, rewrite history, force-push, modify `main`, or modify the `component/worktree` branch.

## Context

The Server component has completed SRV-P1 through SRV-P6 and the Storage `test-support` production-isolation correction. Its last accepted Server code-bearing evidence is:

- server_baseline_sha: `dc53f8dbe08da56d129fc3898cec262149c69f38`
- workflow: `Component CI`
- workflow_run_id: `29127012776`
- run_number: `1616`
- conclusion: `success`

The Worktree component has completed WT-P1 through WT-P9, WT-P9C clean-code correction, and its final artifact-based formatting fix. Its accepted product snapshot is:

- source_branch: `component/worktree`
- source_code_bearing_sha: `4f7bc748d9b901d7d5c3e43c845ba407c0c36e59`
- workflow: `Component CI`
- workflow_run_id: `29152965199`
- run_number: `1665`
- conclusion: `success`

Important branch fact: `component/server` still contains the Wave-0 placeholder Worktree crate. Do not implement SRV-P7 against that placeholder and do not claim Worktree hosting until the accepted product snapshot is explicitly fanned into this branch.

## Phase goal

Perform the first bounded SRV-P7 mini-phase:

1. synchronize the accepted Worktree product snapshot into `component/server` without changing Worktree semantics;
2. add an explicit Server-owned composition boundary for the Worktree runtime;
3. prove disabled/inert behavior, mode mapping, lifecycle ownership, and safe status handling;
4. do not fabricate a working import/export runtime where current public contracts are insufficient.

This is `SRV-P7A`, not the entire SRV-P7 runtime/E2E phase.

## Required reads

Before editing, read:

- project implementation manifest, report template, implementation-worker prompt, and GitHub connector guidance;
- current Server control state/prompt;
- Server component contract, implementation plan SRV-P7 section, implementation log, dependency map, decisions, Cargo manifest, startup, config, state, readiness, routes, and tests;
- Worktree component contract, implementation plan WT-P8/WT-P9 sections, dependency map, runtime/doctor docs, Cargo manifest, and public source at exact SHA `4f7bc748d9b901d7d5c3e43c845ba407c0c36e59`;
- PR #49 changed-file inventory or equivalent repository evidence sufficient to enumerate the accepted Worktree product files;
- the current placeholder Worktree files on `component/server`.

## Part A — exact Worktree snapshot fan-in

Bring the accepted Worktree product files from exact source SHA `4f7bc748d9b901d7d5c3e43c845ba407c0c36e59` into `component/server`.

Required product scope:

- `crates/haze-sync-worktree/Cargo.toml`
- `crates/haze-sync-worktree/src/**`
- `crates/haze-sync-worktree/docs/**`
- `Cargo.lock` only when required by the synchronized crate/dependency graph

Do not copy Worktree `control/**`, branch-specific reports/prompts/state, or unrelated root/component files.

For synchronized files:

- preserve exact accepted content unless a compile-required integration adjustment is explicitly justified;
- record source and destination blob identity where the connector exposes it;
- list any destination file that cannot be copied exactly and explain why;
- do not introduce semantic Worktree changes inside this phase;
- if a semantic Worktree correction is required, stop and report `BLOCKED_BY_CONTRACT` rather than silently editing accepted Worktree behavior.

## Part B — Server composition boundary

Within Server ownership:

- add `haze-sync-worktree` as a normal Server dependency;
- introduce a clearly named Server-owned composition/lifecycle module or equivalent boundary;
- map the existing Server/Common adapter mode exhaustively to Worktree `WorktreeMode` without changing either component's mode semantics;
- consume the existing Server worktree root/config explicitly and avoid hidden globals;
- keep Worktree scanner, importer, materializer, trash, doctor, repair, watcher, and scheduler logic inside `haze-sync-worktree`;
- keep startup/shutdown ownership inside Server;
- ensure disabled mode is inert and performs no scan, watcher, import, export, filesystem mutation, or background work;
- expose only safe count/category/status data, never local absolute paths, database URLs, tokens, raw filesystem errors, or internal payloads;
- preserve dependency-free router/state construction and existing route semantics.

The accepted Worktree runtime is host-driven and does not spawn tasks itself. Do not create hidden background work merely to claim integration. If a Server polling task is introduced, it must be explicit, configuration-gated, cancellation-aware, documented, bounded, and tested. Otherwise keep SRV-P7A at explicit construction/lifecycle/status composition and defer the concrete executor loop to a later SRV-P7 mini-phase.

## Truthfulness rule

Do not use no-op watcher/executor implementations that make an enabled runtime appear healthy or operational.

If current public Worktree/API/Core/Storage contracts are insufficient to construct a real enabled cycle executor, implement the honest composition boundary that is possible, keep enabled runtime unavailable/not-started rather than falsely ready, and report the exact missing contract for SRV-P7B.

Do not add a new public HTTP DTO or route shape unless an existing API-owned contract already supports it. Safe internal status or existing admin DTO integration is allowed only when semantically honest and backward-compatible.

## Required tests

Add focused tests for at least:

- exact and exhaustive adapter-mode mapping;
- disabled mode remaining inert;
- explicit lifecycle ownership and no implicit start;
- safe/redacted `Debug` or status output;
- dependency-free Server state/router behavior remaining unchanged;
- enabled-but-unavailable composition being reported honestly if no real executor is wired;
- synchronized Worktree crate compiling and its existing tests remaining present.

Do not delete or weaken existing Worktree or Server tests.

## Allowed files

- `crates/haze-sync-worktree/Cargo.toml`
- `crates/haze-sync-worktree/src/**`
- `crates/haze-sync-worktree/docs/**`
- `crates/haze-sync-server/Cargo.toml`
- `crates/haze-sync-server/src/**`
- `crates/haze-sync-server/docs/**`
- `Cargo.lock` only if required
- `crates/haze-sync-server/control/report.md`

## Forbidden scope

- no writes to `component/worktree`;
- no Worktree `control/**` fan-in;
- no Core/API/Storage/GDrive/Obsidian/CLI/Deployment source changes;
- no provider API calls;
- no hard delete;
- no automatic destructive repair;
- no plan-carried durable authorization;
- no watcher-only correctness;
- no hidden globals or unbounded background loops;
- no new database schema/migrations;
- no workflow changes;
- no PR merge/readiness/lifecycle changes;
- no unrelated cleanup.

## CI and reporting

Product/source/test/docs/dependency commits must run CI normally. CI skip is permitted only for a final report-only commit.

Write only `crates/haze-sync-server/control/report.md` using `report-template.md`.

Set:

- `REPORT_TYPE: IMPLEMENTATION`
- `phase_id: SRV-P7A`

Your report must include:

- exact Worktree source SHA used;
- synchronized file inventory and any non-identical files;
- Server composition design;
- whether enabled execution is genuinely wired or honestly deferred;
- tests and CI evidence;
- cross-component boundary assessment;
- exact next recommended gate.

Use an honest implementation status. Do not claim `SELF_ACCEPT` if accepted Worktree files are missing, enabled runtime is falsely represented, required scope escaped, or authoritative CI is red/pending.
