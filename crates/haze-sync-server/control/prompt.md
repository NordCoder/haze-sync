# W1-SRV-P7B3-FAN-IN-CLEAN — Clean review of exact-SHA Worktree/Storage fan-in

Before starting, name this worker chat exactly:

`server — W1 SRV-P7B3 Fan-In Clean-Code Review`

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: clean-code-reviewer
Phase: SRV-P7B3-FAN-IN-CLEAN

Work through the GitHub connector. Do not merge PR #45 into main, change draft state, rebase, reset, rewrite history, force-push, modify sibling branches, or begin the bounded Worktree executor.

## Accepted implementation candidate

Review the completed exact-SHA fan-in implementation:

- pre-phase Server head: `b5ec0e1089d1c50f0b121f35a4499bca4864ffa1`;
- initial fan-in candidate: `6aecbf0678e631236cd3001cd694c8033def5dd6`;
- final code-bearing candidate: `4f8b3d9219961409847b12e393d9a38dc6377dea`;
- implementation report commit: `ae996b4811dde20a1bf535f85f214f6f61d5b53f`;
- archived implementation report index: `crates/haze-sync-server/control/log/20260713-085500Z-W1-SRV-P7B3-EXACT-SHA-FAN-IN-RETRY-implementation-worker-report.md`;
- authoritative Component CI: run ID `29235942761`, run number `1840`, attempt `1`, conclusion `success` on exact SHA `4f8b3d9219961409847b12e393d9a38dc6377dea`.

Accepted owner snapshots that must remain authoritative:

- Worktree WT-P10: `1942946331e8362f19907ab6ad4eb779da70fd57`;
- Storage STOR-P10: `66b6a1f554aae1d1b774cc88560d46dd140c7a54`;
- Server SRV-P7B2 semantics: `647dce7b624d67663632808906896cb6745ea7e7`.

## Review goal

Determine whether the fan-in candidate cleanly and correctly integrates the exact accepted Worktree and Storage contracts into the Server integration line without importing sibling lifecycle state, changing owner policy, or starting future runtime phases.

A `CLEAN_ACCEPT` is required before the Orchestrator may activate `SRV-P7B3 Bounded Worktree Executor`.

## Required review

Review the complete implementation range:

`b5ec0e1089d1c50f0b121f35a4499bca4864ffa1..4f8b3d9219961409847b12e393d9a38dc6377dea`

Verify at minimum:

1. the transferred Worktree product files match the accepted WT-P10 owner snapshot for every transferred path;
2. the transferred Storage product, test, documentation and migration files match the accepted STOR-P10 owner snapshot for every transferred path;
3. no sibling `control/**` or sibling workflow was imported;
4. no whole sibling branch merge or unrelated historical content entered the Server integration line;
5. `ServerApplicationServices` and thin transport/auth/DTO route boundaries remain intact;
6. Core remains the sole policy authority;
7. Worktree retains filesystem and scheduler ownership;
8. Storage retains schema and repository ownership, including passive caller-owned transaction behavior;
9. the normal Server dependency on `haze-sync-storage` does not enable `test-support`;
10. Server dev/test scope enables Storage `test-support` only where required;
11. isolated Server and Storage PostgreSQL CI coverage and remaining workspace coverage remain intact;
12. the Server-local `WorktreeMode::DryRun` correction is exhaustive, fail-closed, secret-safe and does not create a manual operation or hosted runtime;
13. no executor, host task, scheduler, background runtime, API-P8 DTO, CLI behavior, Deployment behavior, nested runtime, `block_on`, internal HTTP call, fake repository or hard delete was introduced;
14. tests are honest and cover the changed Server compatibility behavior;
15. public errors, status and Debug output remain redacted and path/token/DB-safe;
16. no product or tooling commit after `4f8b3d9219961409847b12e393d9a38dc6377dea` invalidates the candidate.

## Owner snapshot protection

The accepted Worktree and Storage files are owner-controlled snapshots.

Do not refactor, clean up or otherwise modify files under:

- `crates/haze-sync-worktree/**`;
- `crates/haze-sync-storage/**`;
- `migrations/0010_worktree_durable_state.sql`.

If review finds a concrete owner-component defect or snapshot mismatch, do not silently repair it on the Server branch. Report `CLEAN_BLOCKED_BY_SCOPE` or `CLEAN_BLOCKED_BY_CONTRACT` with exact paths, owner SHA and evidence so the Orchestrator can route the issue to the correct owner.

## Allowed corrections

You may edit Server-owned code, tests or documentation inside `crates/haze-sync-server/**` only when a concrete review finding requires correction and the change remains inside the fan-in integration boundary.

Do not implement `ServerWorktreeCycleExecutor` or any SRV-P7B3 executor behavior in this review.

If you make any executable, test, dependency, workflow, contract or implementation-document change:

- create a new code-bearing commit without CI skip;
- require authoritative DB-capable Component CI on that exact final SHA;
- report the new exact SHA and run evidence.

If no executable correction is needed, the existing successful CI run `29235942761` on `4f8b3d9219961409847b12e393d9a38dc6377dea` remains the authoritative candidate evidence, and the final report-only commit may use `[skip ci]`.

Do not read CI diagnostics artifacts unless a new code-bearing review correction produces a failing run and the active role is explicitly changed by the Orchestrator. A clean-code reviewer must not perform fixer work from failure artifacts.

## Mandatory report

Write `crates/haze-sync-server/control/report.md` using `report-template.md`.

Set:

- `REPORT_TYPE: CLEAN_CODE_REVIEW`;
- `phase_id: SRV-P7B3-FAN-IN-CLEAN`;
- `chat_name: server — W1 SRV-P7B3 Fan-In Clean-Code Review`.

Use one honest status:

- `CLEAN_ACCEPT`;
- `CLEAN_ACCEPT_PENDING_CI`;
- `CLEAN_NEEDS_FIX`;
- `CLEAN_BLOCKED_BY_CONTRACT`;
- `CLEAN_BLOCKED_BY_SCOPE`;
- `CLEAN_BLOCKED_BY_TOOLING`.

The report must include:

- exact reviewed range and final reviewed code-bearing SHA;
- exact owner SHAs and snapshot-parity assessment;
- complete findings for integration boundaries, DryRun handling, tests and secrecy;
- every correction, if any;
- exact CI run ID, number, attempt, SHA and job conclusions;
- confirmation that later commits are control-only or a precise invalidation finding;
- whether the fan-in is `CLEAN_ACCEPT` and ready for Orchestrator activation of the bounded executor phase.

Do not claim that the bounded executor phase is active. Only the Orchestrator may rotate the next control slot.
