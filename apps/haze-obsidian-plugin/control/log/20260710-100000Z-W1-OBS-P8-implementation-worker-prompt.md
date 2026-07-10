# W1-OBS-P8 — Sync runner, offline/backoff, and status UX

Component: obsidian-plugin
Path: apps/haze-obsidian-plugin
Branch: component/obsidian-plugin
PR: #51
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

OBS-P7 implementation and clean-code review are accepted. Clean-code Component CI is green.

- code_bearing_sha: c9caf95ad2e4261b825c07965f44ee7aedf87e19
- workflow: Component CI
- workflow_run_id: 29080003119
- run_number: 906
- conclusion: success

The next implementation phase is OBS-P8 from apps/haze-obsidian-plugin/docs/implementation-plan.md.

## Read

Read implementation-manifest.md, report-template.md, implementation-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, current source, accepted API/Server contracts, and PR diff. Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Implement OBS-P8: Sync runner, offline/backoff, and status UX.

Follow the plan:

- implement an explicit manual sync trigger;
- add optional interval/event-triggered sync only with explicit lifecycle ownership and cleanup;
- model offline and retry/backoff state predictably;
- expose sanitized status-bar or settings/status summaries;
- prevent concurrent overlapping sync runs;
- cancel/stop pending work on plugin unload;
- make mobile and background limitations explicit in UI/docs;
- coordinate existing push, pull, materialization, and conflict-refresh capabilities without bypassing Server/Core semantics.

## Allowed files

- apps/haze-obsidian-plugin/src/**
- apps/haze-obsidian-plugin/docs/**
- apps/haze-obsidian-plugin/control/report.md

## Non-goals

No guaranteed mobile background sync, hidden telemetry, provider integration, server runtime changes, destructive repair automation, workflow changes, dependency changes, or sibling component changes.

## CI trigger policy

Product/source/docs commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to apps/haze-obsidian-plugin/control/report.md. Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION and phase_id to OBS-P8.
