# W1-OBS-P6C — Obsidian remote materialization clean-code review

Component: obsidian-plugin
Path: apps/haze-obsidian-plugin
Branch: component/obsidian-plugin
PR: #51
Role: clean-code-reviewer

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

OBS-P6 implementation is complete. Component CI for the code-bearing commit is green.

- workflow: Component CI
- workflow_run_id: 29034944309
- run_number: 698
- conclusion: success

## Read

Read before editing:

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- clean-code-reviewer-prompt.md from ChatGPT Project Sources
- chatgpt-gh-connector.md from ChatGPT Project Sources
- apps/haze-obsidian-plugin/docs/component-contract.md
- apps/haze-obsidian-plugin/docs/implementation-plan.md
- apps/haze-obsidian-plugin/docs/implementation-log.md
- apps/haze-obsidian-plugin/docs/dependency-map.md
- apps/haze-obsidian-plugin/control/prompt.md
- apps/haze-obsidian-plugin/control/report.md
- relevant current repository code and PR diff

Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Review OBS-P6 remote changes pull and safe materialization.

Focus areas:

- change cursor and pull-state persistence;
- hash and revision verification before materialization;
- local dirty-file detection before writes;
- Obsidian Vault API write boundaries;
- base revision and cursor updates only after successful apply/no-op/tombstone representation;
- tombstone handling without hard delete;
- conflict queueing instead of overwriting dirty local files;
- echo suppression for plugin-applied remote writes;
- preservation of non-goals: no provider calls, no semantic merge, no hard delete, no server route changes, no Worktree behavior, no background sync loop.

## Allowed files

- apps/haze-obsidian-plugin/src/**
- apps/haze-obsidian-plugin/docs/**
- apps/haze-obsidian-plugin/control/report.md

## Forbidden changes

- Do not add provider calls.
- Do not add semantic markdown merge.
- Do not add hard delete.
- Do not change server routes.
- Do not add Worktree behavior.
- Do not change workflow files.
- Do not change sibling components.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and clean-code-reviewer-prompt.md. Source/doc clean-code commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to apps/haze-obsidian-plugin/control/report.md.

Use report-template.md. Set REPORT_TYPE to CLEAN_CODE_REVIEW.
