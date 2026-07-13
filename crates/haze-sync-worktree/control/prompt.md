# W1-WT-P11-CLEAN — Clean review of hosted runtime contract extension

Before starting, name this worker chat exactly:

`worktree — W1 WT-P11 Hosted Runtime Contract Clean Review`

Component: worktree
Path: crates/haze-sync-worktree
Branch: component/worktree
PR: #49
Role: clean-code-reviewer
Phase: WT-P11-CLEAN

Do not merge PR #49, change draft state, rewrite history, modify sibling branches, or begin Server fan-in.

## Candidate

Review the complete WT-P11 contract extension through exact final SHA:

- accepted WT-P10 baseline: `1942946331e8362f19907ab6ad4eb779da70fd57`;
- original WT-P11 implementation SHA: `8a21497c845710a4205cb91b2af7d13b5346bd95`;
- final fixed code-bearing SHA: `61c24544a3fb9d785cb95ab2016f6d29f4661d3e`;
- FIX report commit: `21ac67c51bb2286b6fe26b890658f64320244067`;
- FIX report blob: `9ecb40e00d136db03ada66a5a01d8d549699ed90`;
- authoritative Component CI: run `29266803510`, number `1870`, attempt `1`, success.

Review range:

`1942946331e8362f19907ab6ad4eb779da70fd57..61c24544a3fb9d785cb95ab2016f6d29f4661d3e`

## Required review

Verify at minimum:

1. production watcher lifecycle is genuinely production-complete and Worktree-owned;
2. watcher hints are path-free and bounded;
3. overflow/closure/backend failure degrade safely and preserve authoritative full-scan behavior;
4. no unmanaged detached task/thread or lifecycle leak exists;
5. shutdown/drop behavior is deterministic and coarse-safe;
6. `run_manual_cycle` preserves at-most-one-cycle semantics;
7. busy/not-started/cancelling/shutdown outcomes are typed and coarse;
8. manual cycles use the same executor/cancellation/accounting path as automatic cycles;
9. counters, last-cycle cause/result and lifecycle/status accounting are consistent;
10. manual DryRun requires full scan and remains mutation/checkpoint-free;
11. automatic startup/periodic/watcher behavior remains unchanged;
12. budget/mode/full-scan validation is fail-closed;
13. Debug/Display/errors expose no roots, paths, backend internals or sensitive material;
14. tests honestly cover watcher lifecycle, coalescing, overflow, failure, manual success/busy/DryRun/cancellation/shutdown and no-overlap;
15. manifest dependency choice is minimal and justified;
16. no Server/Storage/Core/API/CLI/Deployment/migration/workflow/sibling-control changes occurred;
17. no later product/tooling commit invalidates final SHA.

No cleanup for its own sake. Modify only Worktree-owned WT-P11 code/tests/docs for a concrete defect. Any code-bearing correction requires new exact-SHA Component CI. Do not inspect failure artifacts unless Orchestrator changes the role to fixer-worker.

## Mandatory report

Write `crates/haze-sync-worktree/control/report.md` using `report-template.md` with:

- `REPORT_TYPE: CLEAN_CODE_REVIEW`;
- `phase_id: WT-P11-CLEAN`;
- `chat_name: worktree — W1 WT-P11 Hosted Runtime Contract Clean Review`.

Use one honest status: `CLEAN_ACCEPT`, `CLEAN_ACCEPT_PENDING_CI`, `CLEAN_NEEDS_FIX`, `CLEAN_BLOCKED_BY_CONTRACT`, `CLEAN_BLOCKED_BY_SCOPE`, or `CLEAN_BLOCKED_BY_TOOLING`.

Include exact range/SHA, findings for watcher lifecycle/manual scheduling/accounting/secrecy/tests/dependency choice, every correction, exact CI evidence, later-commit validation and whether Orchestrator may begin exact-SHA fan-in to Server.

Do not claim Server is reactivated or fan-in complete.
