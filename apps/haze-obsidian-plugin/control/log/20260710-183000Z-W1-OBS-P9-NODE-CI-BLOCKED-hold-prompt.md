# W1-OBS-P9-NODE-CI-BLOCKED — PR merge-conflict validation gate

Component: obsidian-plugin
Path: apps/haze-obsidian-plugin
Branch: component/obsidian-plugin
PR: #51
Role: none

This is a hold notice, not an executable worker prompt.

## Implemented state

The Obsidian-specific Node validation job has been added to `.github/workflows/component-ci.yml` without changing the existing Rust job.

- workflow_code_bearing_sha: 161b412bb57546c28bf0aa7fb0d9408f5b74536d
- required checks: npm ci, plugin test, typecheck, and build
- diagnostics integration: existing ci-run.sh, ci-finalize.sh, and one-day artifact convention
- product or package changes: none

## Blocker

PR #51 currently reports:

- state: open
- draft: true
- mergeable: false
- merge_commit_sha: null

No pull-request workflow run or status checks were created for the workflow change. Therefore the Node commands and Rust regression job remain unexecuted for this head.

Resolving the branch conflict requires an explicitly authorized integration action. Do not merge, rebase, update the branch from main, rewrite history, or change PR lifecycle state from this hold notice.

## Unblock condition

One of the following must happen:

1. the user explicitly authorizes the required branch integration/conflict-resolution operation, after which Component CI must run and exact Rust/Node job and step conclusions must be inspected; or
2. an accepted external validation path produces independently observable successful results for npm ci, plugin test, typecheck, and build, while the workflow change is separately validated through an authorized integration path.

If a created Node run fails, assign a scoped fixer using that run's exact diagnostics evidence. Do not launch a component worker from this hold notice.
