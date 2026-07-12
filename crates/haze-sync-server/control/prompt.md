# W1-SRV-P7B3-EXACT-SHA-FAN-IN-RETRY — Integrate accepted Worktree and Storage snapshots

Before starting, name this worker chat exactly:

`server — W1 SRV-P7B3 Exact-SHA Fan-In Retry`

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: implementation-worker
Phase: SRV-P7B3-EXACT-SHA-FAN-IN-RETRY

Work through the GitHub connector. Do not merge PR #45 into main, change draft state, rebase, reset, rewrite history, force-push, modify sibling branches, or implement the real Worktree executor in this phase.

## Why this is a retry

The previous exact-SHA fan-in slot produced no verifiable repository result:

- `component/server` remained at Orchestrator control commit `dbb50d98da88fd01cca5a88e0e0de30bfc6be131`;
- `crates/haze-sync-server/control/report.md` returned `404`;
- no new code-bearing fan-in SHA existed;
- no new DB-capable Component CI run existed.

A chat response without branch advance and a committed report does not complete this phase.

## Accepted owner snapshots

Synchronize these exact accepted product snapshots into the Server integration branch:

- WT-P10 Worktree SHA: `1942946331e8362f19907ab6ad4eb779da70fd57`;
- STOR-P10 Storage SHA: `66b6a1f554aae1d1b774cc88560d46dd140c7a54`;
- preserve SRV-P7B2 Server SHA semantics from `647dce7b624d67663632808906896cb6745ea7e7`.

Authoritative evidence:

- Worktree CI `29167289593`, run `1799`, success;
- Storage CI `29185466870`, run `1833`, success, including strict PostgreSQL verification and five mandatory evidence checks;
- Server CI `29186058268`, run `1835`, success;
- all later Worktree owner-branch commits after its accepted SHA are control-only;
- Storage cross-branch report commit `13a0c80123061848235676b2a7dc7c3a3c644dee` is `CLEAN_ACCEPT`;
- Server clean report commit `053eea1496bf9b541b462a82989b4cd956ed7276` is `CLEAN_ACCEPT`.

## Exact integration scope

Transfer only accepted non-control owner product content required by the snapshots.

Worktree scope:

- `crates/haze-sync-worktree/Cargo.toml` when changed by the accepted snapshot;
- `crates/haze-sync-worktree/src/**`;
- Worktree-owned non-control docs and tests required to preserve the accepted contract.

Storage scope:

- `migrations/0010_worktree_durable_state.sql`;
- `crates/haze-sync-storage/src/**`;
- `crates/haze-sync-storage/tests/**`;
- Storage-owned non-control docs required for the accepted contract;
- manifest/lock updates strictly required for compilation.

Do not transfer:

- sibling `control/**` files;
- sibling `.github/workflows/component-ci.yml`;
- stale sibling Server/Core/API/Common/CLI/Deployment files;
- unrelated historical component changes outside the exact accepted owner snapshots.

Resolve integration conflicts by preserving:

- current accepted Server application services and thin-route boundaries;
- normal Server dependency on `haze-sync-storage` without `test-support`;
- Server dev-dependency enabling `haze-sync-storage/test-support` only for tests;
- isolated Server and Storage PostgreSQL CI services and complete workspace coverage;
- Core as the sole policy authority;
- Worktree filesystem/scheduler ownership;
- Storage schema/repository ownership.

Minimal Server-local compile/test compatibility corrections are allowed only when directly required by the accepted owner contracts. Do not implement `ServerWorktreeCycleExecutor`, hosted runtime, new API DTOs, CLI behavior or Deployment behavior.

## Required verification

Create a real code-bearing fan-in commit and run Component CI on that exact SHA.

Acceptance requires:

1. exact accepted Worktree product snapshot present;
2. exact accepted Storage product and migration snapshot present;
3. current SRV-P7B2 application-service behavior preserved;
4. no sibling control files or stale workflow imported;
5. cargo fmt succeeds;
6. cargo check succeeds;
7. isolated Server PostgreSQL tests succeed;
8. isolated Storage PostgreSQL tests, including strict STOR-P10 evidence, succeed;
9. remaining workspace tests succeed;
10. cargo clippy with warnings denied succeeds;
11. diagnostics finalizer succeeds;
12. no secrets, raw paths, DB URLs or internal errors exposed.

If CI fails, use the produced diagnostics artifact and report the exact failure. Do not claim completion from local reasoning alone.

## Mandatory committed report

Write `crates/haze-sync-server/control/report.md` using `report-template.md`.

Set:

- `REPORT_TYPE: IMPLEMENTATION`
- `phase_id: SRV-P7B3-EXACT-SHA-FAN-IN-RETRY`
- `chat_name: server — W1 SRV-P7B3 Exact-SHA Fan-In Retry`

Use one honest status:

- `SELF_ACCEPT`
- `IMPLEMENTATION_NEEDS_FIX`
- `IMPLEMENTATION_BLOCKED_BY_CONTRACT`
- `IMPLEMENTATION_BLOCKED_BY_SCOPE`
- `IMPLEMENTATION_BLOCKED_BY_TOOLING`

The report must include:

- exact final code-bearing fan-in SHA;
- source owner SHAs;
- exact transferred paths and conflict resolutions;
- proof that sibling control/workflow files were not imported;
- Server application-service and dependency-gating preservation assessment;
- final DB-capable CI run ID, run number, attempt and job conclusions;
- any corrections;
- whether the fan-in is ready for mandatory clean-code review.

The phase is incomplete until both branch advance and committed report are visible on `component/server`.