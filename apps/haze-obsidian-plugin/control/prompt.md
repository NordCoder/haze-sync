# W1-OBS-P5 — Base revision store and upload/delete planner

Component: obsidian-plugin
Path: apps/haze-obsidian-plugin
Branch: component/obsidian-plugin
PR: #51
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

OBS-P4 implementation and clean-code review are accepted. Component CI is green for the accepted source head.

- workflow: Component CI
- workflow_run_id: 29011302542
- run_number: 534
- conclusion: success

The next implementation phase is OBS-P5 from apps/haze-obsidian-plugin/docs/implementation-plan.md.

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

Implement OBS-P5: Base revision store and upload/delete planner.

Follow the implementation plan:

- store per-vault-path last known Core revision/hash metadata;
- generate idempotency keys for local operations;
- compute or preserve content hashes before upload planning;
- prepare upload requests with current base revision or explicit null base;
- prepare delete requests with current base revision or explicit null base;
- model same-content, accepted, conflict-saved, rejected, unauthorized, and server-unavailable outcomes;
- update local base state only after accepted/server-confirmed outcomes.

## Allowed files

- apps/haze-obsidian-plugin/src/**
- apps/haze-obsidian-plugin/docs/**
- apps/haze-obsidian-plugin/control/report.md

## Non-goals

- No automatic conflict resolution.
- No hard delete.
- No direct DB access.
- No provider behavior.
- No full background sync loop unless explicitly scoped.
- No sibling component changes.
- No workflow changes.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and implementation-worker-prompt.md. Product, test, dependency, contract, workflow, and implementation commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to apps/haze-obsidian-plugin/control/report.md.

Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION.
