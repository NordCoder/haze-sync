# W1-CLI-P6A-PRE-SYNC — Synchronize CLI branch with accepted main

Before starting, name this worker chat exactly:

`cli — W1 CLI-P6A Main Sync`

Component: cli
Path: crates/haze-sync-cli
Branch: component/cli
PR: #48
Role: implementation-worker
Phase: CLI-P6A-PRE-SYNC

The previous `CLI-P6A-BLOCKED-BY-API-SERVER` hold is resolved. This prompt is executable and supersedes older blocked snapshots.

Do not merge the PR, change draft state, rewrite history, rebase, modify sibling branches, or begin CLI-P6A product work in this phase.

## Current coordinates

- CLI branch head before sync: `d923ad4d66416344ea166097fe1e53728701652b`;
- current exact main SHA: `c1e69a664388b0cba028170e8398b9088218957d`;
- merge base: `9ee3ced989bf60a71d0d7b37ff046118b0b2d1a2`;
- CLI is 139 commits behind main and has 12 CLI-local commits;
- PR #48 is open, draft, mergeable and unmerged.

Accepted downstream dependencies:

- API-P8 code-bearing SHA `56ae94570441d68715f34b5d54381a0fc4d7c231`, CLEAN_ACCEPT;
- Server Worktree HTTP integration SHA `50461354c18ddc4d2e47202d9303b4358a27ee45`, CLEAN_ACCEPT;
- Server final clean report blob `e3271abaf3d667f9ffd4f4ff0652e5d26892b9e5`;
- Server DB-capable CI run `29326558901`, number `1940`, success.

## Task

1. Merge exact main SHA `c1e69a664388b0cba028170e8398b9088218957d` normally into `component/cli`.
2. Do not rebase, squash, cherry-pick the full main history, force-push or rewrite CLI-local history.
3. Preserve all existing CLI-local commits.
4. Resolve conflicts minimally and only to preserve current accepted main plus CLI-local behavior.
5. Do not implement status or sync-once commands yet.
6. Do not copy API/Server files manually; the exact accepted product surface arrives through the main merge.
7. Create a real merge/code-bearing commit without CI skip.
8. Obtain authoritative Component CI success on the exact post-sync SHA.

## Verification

Confirm:

- exact main SHA is a merge parent/ancestor;
- CLI-local history remains present;
- no rebase/history rewrite occurred;
- no CLI-P6A product files were added or changed beyond conflict resolution;
- PR remains open, draft and unmerged;
- exact post-sync Component CI is green.

## Report

Write `crates/haze-sync-cli/control/report.md` with:

- `REPORT_TYPE: IMPLEMENTATION`;
- `phase_id: CLI-P6A-PRE-SYNC`;
- `chat_name: cli — W1 CLI-P6A Main Sync`;
- status `SELF_ACCEPT`, `NEEDS_FIX`, `BLOCKED_BY_CONTRACT`, `BLOCKED_BY_SCOPE`, or `BLOCKED_BY_TOOLING`.

Record pre-sync head, exact main SHA, merge SHA/parents, conflict paths, history-preservation evidence, absence of CLI-P6A product work, and exact CI run details.

Do not claim CLEAN_ACCEPT or start CLI-P6A implementation. Orchestrator will open the product slot after validating this synchronization.
