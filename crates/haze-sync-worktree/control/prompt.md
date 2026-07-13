# W1-WT-P11-CI-FIX — Fix exact-SHA WT-P11 CI failure

Before starting, name this worker chat exactly:

`worktree — W1 WT-P11 CI Fix`

Component: worktree
Path: crates/haze-sync-worktree
Branch: component/worktree
PR: #49
Role: fixer-worker
Phase: WT-P11-CI-FIX

Do not merge, rewrite history, modify sibling branches, or begin clean review/Server fan-in.

## Failed candidate

- code-bearing SHA: `8a21497c845710a4205cb91b2af7d13b5346bd95`;
- implementation report commit: `0873d1ea23627f8f7ac62ee3437fbc3ad92600bf`;
- report blob: `f8ab6904d5a6b4555fc9e0f39ed30eacefa69bce`;
- Component CI run: `29265781943`, number `1865`, attempt `1`, conclusion `failure`.

Visible wrapper evidence: fmt/check/test/clippy succeeded; diagnostics finalizer failed; artifact upload succeeded. Do not infer root cause from wrapper evidence.

## Mandatory artifact

- artifact ID: `8285369808`;
- name: `ci-diag__component-worktree__wf-component-ci__run-29265781943__attempt-1`;
- digest: `sha256:8703d14197a2a867bd633f9f25d8168f99d3b50a209fdd3d130c4ad6165a0c50`;
- head SHA: `8a21497c845710a4205cb91b2af7d13b5346bd95`;
- expired: false at rotation;
- expires: `2026-07-14T16:19:06Z`.

Download and read summary, manifest and every `failed_checks` log. Raw job logs are fallback-only if the artifact is missing/corrupt/insufficient.

Fix only the exact artifact-proven failure. Allowed scope: WT-P11 Worktree watcher/runtime/tests/manifest and Worktree control report. Forbidden: Server/Storage/Core/API/CLI/Deployment, migrations, workflows, sibling control files, unrelated cleanup.

Preserve WT-P11 semantics unless the artifact proves them wrong: production path-free watcher lifecycle; scheduler-accounted manual cycles; no-overlap; DryRun non-mutation; safe coarse outcomes.

Produce a real code-bearing fix commit when code/test/tooling changes, obtain exact-SHA Component CI, then write `crates/haze-sync-worktree/control/report.md` with:

- `REPORT_TYPE: FIX`;
- `phase_id: WT-P11-CI-FIX`;
- `chat_name: worktree — W1 WT-P11 CI Fix`;
- honest status `FIX_COMPLETE`, `FIX_NEEDS_MORE_WORK`, `FIX_BLOCKED_BY_LOGS`, `FIX_BLOCKED_BY_CONTRACT`, `FIX_BLOCKED_BY_SCOPE`, or `FIX_BLOCKED_BY_TOOLING`.

Record artifact files read, root cause, changed paths, final SHA and exact CI evidence. Do not claim CLEAN_ACCEPT or reactivate Server.
