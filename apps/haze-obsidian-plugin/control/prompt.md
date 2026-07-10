# W1-OBS-P7 — Conflict center and user actions

Component: obsidian-plugin
Path: apps/haze-obsidian-plugin
Branch: component/obsidian-plugin
PR: #51
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

OBS-P6 implementation and clean-code review are accepted. Component CI evidence for the accepted source state is green.

- workflow: Component CI
- workflow_run_id: 29038540630
- run_number: 740
- conclusion: success

The next implementation phase is OBS-P7 from apps/haze-obsidian-plugin/docs/implementation-plan.md.

## Read

Read implementation-manifest.md, report-template.md, implementation-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, relevant source, and PR diff.

Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Implement OBS-P7: Conflict center and user actions.

Follow the plan:

- list open conflicts from Server through accepted abstractions;
- show original path, conflict path or materialized copy, status, source adapter, and safe timestamps;
- provide supported actions: accept_current, accept_conflict, keep_both, mark_resolved;
- explain destructive implications before actions where necessary;
- refresh local base state after server-confirmed resolution;
- avoid raw server internals or token details in UI/state.

## Allowed files

- apps/haze-obsidian-plugin/src/**
- apps/haze-obsidian-plugin/docs/**
- apps/haze-obsidian-plugin/control/report.md

## Non-goals

No local-only conflict resolution bypassing Server, semantic merge editor, new conflict policy vocabulary, server route changes, workflow changes, or sibling component changes.

## CI trigger policy

Product/source/docs commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to apps/haze-obsidian-plugin/control/report.md. Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION and phase_id to OBS-P7.
