# W1-SRV-WT-P11-FAN-IN — Synchronize accepted Worktree contract into Server

Before starting, name this worker chat exactly:

`server — W1 WT-P11 Exact-SHA Fan-In`

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: implementation-worker
Phase: SRV-WT-P11-FAN-IN

Do not merge PR #45, change draft state, rewrite history, modify sibling branches, or begin SRV-P7B4 hosted-runtime implementation before this fan-in is accepted.

## Accepted Worktree source

Synchronize the exact accepted Worktree product snapshot:

- source branch: `component/worktree`;
- accepted code-bearing SHA: `b38264ce2b09632a4c0bab0dd77319e1db239a3b`;
- final clean-review report commit: `b76f88369d079a9cb264bb4cdb9b6c62e361a9bc`;
- final clean-review report blob: `84fb7a399d708088c3b545efd0e2adbaad3f909e`;
- status: `CLEAN_ACCEPT`;
- authoritative Component CI run: `29272964159`, number `1881`, attempt `1`, success.

## Goal

Bring only the Worktree-owned product changes from accepted baseline `1942946331e8362f19907ab6ad4eb779da70fd57` through exact SHA `b38264ce2b09632a4c0bab0dd77319e1db239a3b` into `component/server`, preserving Server-owned work and all unrelated component snapshots.

## Required actions

1. Compare the accepted Worktree range and identify only Worktree product/dependency files required for WT-P11.
2. Apply the exact product content into `component/server` without merging the entire sibling branch and without copying Worktree control/log files.
3. Preserve all Server product/control history and accepted Storage/Server snapshots.
4. Do not modify Worktree semantics during fan-in.
5. Resolve only integration conflicts caused by existing Server fan-in state; record each resolution precisely.
6. Run authoritative DB-capable Component CI on the exact final code-bearing Server SHA.
7. Verify no API/CLI/Deployment/migration/workflow or unrelated sibling changes occurred.

Expected Worktree-owned product paths include only the exact accepted changes under:

- `crates/haze-sync-worktree/Cargo.toml`;
- `crates/haze-sync-worktree/src/lib.rs`;
- `crates/haze-sync-worktree/src/runtime.rs`;
- `crates/haze-sync-worktree/src/watcher.rs`;
- `crates/haze-sync-worktree/src/hosted_runtime.rs`;
- `crates/haze-sync-worktree/src/wt_p11_tests.rs`;
- workspace lockfile only if exact accepted dependency content requires it.

Do not assume this list overrides the exact accepted compare; use the compare as source of truth.

## Completion

Create a real code-bearing fan-in commit without CI skip.

Write `crates/haze-sync-server/control/report.md` with:

- `REPORT_TYPE: IMPLEMENTATION`;
- `phase_id: SRV-WT-P11-FAN-IN`;
- `chat_name: server — W1 WT-P11 Exact-SHA Fan-In`;
- honest status `SELF_ACCEPT`, `NEEDS_FIX`, `BLOCKED_BY_CONTRACT`, `BLOCKED_BY_SCOPE`, or `BLOCKED_BY_TOOLING`.

Report exact source SHA/range, copied product paths, conflict resolutions, final Server code-bearing SHA, exact CI evidence, scope verification and whether the fan-in is ready for functional integration review.

Formatting/style alone is non-blocking when exact-SHA CI is green. Do not begin SRV-P7B4 implementation or claim fan-in CLEAN_ACCEPT; Orchestrator controls the next gate.
