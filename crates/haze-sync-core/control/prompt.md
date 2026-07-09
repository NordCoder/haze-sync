# W1-CORE-P4 — Conflict preservation and resolution primitives

Component: core
Path: crates/haze-sync-core
Branch: component/core
PR: #43
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

CORE-P3 implementation, CI fixer, and clean-code review are accepted. Component CI evidence for the accepted source state is green.

- workflow: Component CI
- workflow_run_id: 29011346160
- run_number: 536
- conclusion: success

The next implementation phase is CORE-P4 from crates/haze-sync-core/docs/implementation-plan.md.

## Read

Read before editing:

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- implementation-worker-prompt.md from ChatGPT Project Sources
- chatgpt-gh-connector.md from ChatGPT Project Sources
- crates/haze-sync-core/docs/component-contract.md
- crates/haze-sync-core/docs/implementation-plan.md
- crates/haze-sync-core/docs/implementation-log.md
- crates/haze-sync-core/docs/dependency-map.md
- crates/haze-sync-core/control/prompt.md
- crates/haze-sync-core/control/report.md
- relevant current repository code and PR diff

Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Implement CORE-P4: Conflict preservation and resolution primitives.

Follow the implementation plan:

- test conflict-copy path generation for nested paths, extensions, unsafe adapter IDs, timestamps, and recursive conflict-area input;
- verify `_haze_conflicts/open/**` materialization remains compatible with VaultPath rules;
- define pure decision primitives for accepted conflict resolution actions where Core owns them: accept_current, accept_conflict, keep_both, mark_resolved;
- represent resolution effects as storage/API-neutral plans, not persisted writes;
- clarify which actions are metadata-only and which require new current revision creation.

## Allowed files

- crates/haze-sync-core/src/conflict_service/**
- crates/haze-sync-core/src/policy_engine/**
- crates/haze-sync-core/src/conflict_saved_planner/**
- crates/haze-sync-core/docs/**
- crates/haze-sync-core/control/report.md

## Non-goals

- No conflict route wiring.
- No conflict row repository implementation.
- No object-store writes.
- No API DTO ownership.
- No Obsidian conflict UI behavior.
- No sibling component changes.
- No workflow changes.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and implementation-worker-prompt.md. Product, test, dependency, contract, workflow, and implementation commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-core/control/report.md.

Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION.
