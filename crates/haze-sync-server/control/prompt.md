# W1-SRV-P7B3-EXECUTOR-CLEAN — Clean review of bounded Worktree executor

Before starting, name this worker chat exactly:

`server — W1 SRV-P7B3 Executor Clean-Code Review`

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: clean-code-reviewer
Phase: SRV-P7B3-EXECUTOR-CLEAN

Work through the GitHub connector. Do not merge PR #45, change draft state, rewrite history, modify sibling branches, or begin SRV-P7B4 hosted runtime work.

## Accepted implementation candidate

Review the completed bounded executor implementation:

- baseline before executor implementation: `230a381dc37c300d6b2c17252f2c7ec163634be1`;
- final code-bearing candidate: `a08739cc8146b4f224475c83fa0652b60782db82`;
- implementation report commit: `543dd9cebe3210b82e8a075f6bdb17eadc5f47b3`;
- implementation report blob: `dab3fa8b2626c931011d39fcba5bf5a022a760e8`;
- authoritative Component CI: run `29245434633`, number `1857`, attempt `1`, success on exact candidate SHA.

Accepted prerequisites remain immutable:

- Worktree WT-P10: `1942946331e8362f19907ab6ad4eb779da70fd57`;
- Storage STOR-P10: `66b6a1f554aae1d1b774cc88560d46dd140c7a54`;
- Server application services: `647dce7b624d67663632808906896cb6745ea7e7`.

## Review goal

Determine whether `ServerWorktreeCycleExecutor` is correct, bounded, cancellation-safe, replay-safe, transactionally honest, secret-safe and cleanly separated from scheduler/host ownership.

A `CLEAN_ACCEPT` is required before SRV-P7B4 may start.

## Required review

Review the complete executor implementation range:

`230a381dc37c300d6b2c17252f2c7ec163634be1..a08739cc8146b4f224475c83fa0652b60782db82`

Verify at minimum:

1. exactly one requested cycle is executed and no task is spawned or detached;
2. request mode, full-scan requirement and all three budgets are enforced fail-closed;
3. cancellation is checked before work, between bounded phases and between individual actions;
4. synchronous filesystem work uses awaited bounded blocking execution only;
5. every authoritative mutation/read uses `ServerApplicationServices`, not routes or internal HTTP;
6. Core policy is not duplicated;
7. Storage repositories remain passive and Server owns transaction timing;
8. durable instance/root binding and state-format validation are correct;
9. state pagination is bounded and deterministic;
10. imports and guarded deletes are deterministic and idempotent across replay;
11. exports are ordered, content is verified, filesystem effects precede durable checkpointing correctly, and cursor advancement is exact-contiguous;
12. crash windows after authoritative import and after materialization are recoverable without duplicate destructive effects;
13. DryRun performs planning only and grants no mutation/checkpoint/materialization permission;
14. summaries contain counts only and failures use accepted coarse categories;
15. Debug, Display and errors expose no roots, tokens, DB URLs, SQLx errors, cursor payloads or idempotency material;
16. tests cover limits, cancellation, replay, rollback, stale cursor/binding/version failures and redaction honestly;
17. accepted Worktree/Storage snapshots and migrations remain unchanged;
18. no hosted scheduler, startup task, manual channel, public API/status/readiness, CLI or Deployment behavior was introduced;
19. no product/tooling commit after the candidate invalidates the review.

## Allowed corrections

You may modify Server-owned code/tests/docs only when a concrete review finding requires it and the change remains inside SRV-P7B3 executor scope.

Do not modify Worktree, Storage, migrations, Core, API, CLI, Deployment, sibling control files or workflows. Route owner-contract defects to the owner component with exact evidence.

Any executable/test/dependency/workflow/contract/doc correction requires a new code-bearing commit and authoritative DB-capable Component CI on that exact final SHA. If no correction is needed, run `29245434633` remains authoritative and the report-only commit may use `[skip ci]`.

Do not read failure diagnostics artifacts unless Orchestrator rotates the role to fixer-worker.

## Mandatory report

Write `crates/haze-sync-server/control/report.md` using `report-template.md`.

Set:

- `REPORT_TYPE: CLEAN_CODE_REVIEW`;
- `phase_id: SRV-P7B3-EXECUTOR-CLEAN`;
- `chat_name: server — W1 SRV-P7B3 Executor Clean-Code Review`.

Use one honest status:

- `CLEAN_ACCEPT`;
- `CLEAN_ACCEPT_PENDING_CI`;
- `CLEAN_NEEDS_FIX`;
- `CLEAN_BLOCKED_BY_CONTRACT`;
- `CLEAN_BLOCKED_BY_SCOPE`;
- `CLEAN_BLOCKED_BY_TOOLING`.

Include exact reviewed range/SHA, findings for boundedness/cancellation/transactions/replay/secrecy/tests, every correction, exact CI evidence, owner-snapshot protection, later-commit validation and whether SRV-P7B4 may be activated.

Do not claim SRV-P7B4 is active. Only Orchestrator may rotate the next slot.
