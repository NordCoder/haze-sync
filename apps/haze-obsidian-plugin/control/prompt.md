# W1-OBS-P6 — Remote changes pull and safe materialization

Component: obsidian-plugin
Path: apps/haze-obsidian-plugin
Branch: component/obsidian-plugin
PR: #51
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

OBS-P5 implementation and clean-code review are accepted. Component CI evidence for the accepted source state is green.

- workflow: Component CI
- workflow_run_id: 29027985676
- run_number: 616
- conclusion: success

The next implementation phase is OBS-P6 from apps/haze-obsidian-plugin/docs/implementation-plan.md.

## Read

Read before editing:

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- implementation-worker-prompt.md from ChatGPT Project Sources
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

Implement OBS-P6: Remote changes pull and safe materialization.

Follow the implementation plan:

- fetch `/v1/changes` with cursor state through accepted abstractions;
- download file metadata/content for relevant revisions where scoped;
- verify content hash before applying;
- write through Obsidian APIs with local dirty checks;
- update base revision/cursor only after successful apply;
- represent tombstones/deletes safely;
- queue conflicts instead of overwriting dirty local files;
- prevent echo loops from plugin-applied changes.

## Allowed files

- apps/haze-obsidian-plugin/src/**
- apps/haze-obsidian-plugin/docs/**
- apps/haze-obsidian-plugin/control/report.md

## Non-goals

- No provider calls.
- No semantic markdown merge.
- No hard delete by default.
- No server route changes.
- No Worktree behavior.
- No sibling component changes.
- No workflow changes.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and implementation-worker-prompt.md. Product/source/docs commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to apps/haze-obsidian-plugin/control/report.md.

Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION.
