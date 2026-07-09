# W1-OBS-P5C — Obsidian base revision and mutation planner clean-code review

Component: obsidian-plugin
Path: apps/haze-obsidian-plugin
Branch: component/obsidian-plugin
PR: #51
Role: clean-code-reviewer

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

OBS-P5 implementation completed and its code-bearing commit has green Component CI.

- workflow: Component CI
- workflow_run_id: 29024064696
- run_number: 587
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

Review OBS-P5 base revision store and upload/delete planner.

Focus areas:

- per-vault-path base revision/hash metadata;
- idempotency key generation for local operations;
- upload/delete request planning with base revision or explicit null base;
- content hash handling before upload planning;
- outcome handling for same-content, accepted, conflict-saved, rejected, unauthorized, and server-unavailable cases;
- base-state updates only after accepted/server-confirmed outcomes;
- preservation of non-goals: no automatic conflict resolution, no hard delete, no direct DB access, no provider behavior, no background sync loop, no sibling changes.

## Allowed files

- apps/haze-obsidian-plugin/src/**
- apps/haze-obsidian-plugin/docs/**
- apps/haze-obsidian-plugin/control/report.md

## Forbidden changes

- Do not add upload/download execution.
- Do not add automatic conflict resolution.
- Do not add hard delete.
- Do not add direct DB access.
- Do not add provider behavior.
- Do not change workflow files.
- Do not change sibling components.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and clean-code-reviewer-prompt.md. Source/doc clean-code commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to apps/haze-obsidian-plugin/control/report.md.

Use report-template.md. Set REPORT_TYPE to CLEAN_CODE_REVIEW.
