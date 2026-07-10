# Archived active prompt

component: obsidian-plugin
archived_at: 2026-07-10T11:00:00Z
wave: W1
phase: OBS-P8C
agent_role: clean-code-reviewer
source_path: apps/haze-obsidian-plugin/control/prompt.md
source_sha: 28f0b065f6344a9e8d2aa11df01fed87237a54c6

# W1-OBS-P8C — Obsidian sync-runner clean-code review

Component: obsidian-plugin
Path: apps/haze-obsidian-plugin
Branch: component/obsidian-plugin
PR: #51
Role: clean-code-reviewer

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

OBS-P8 implementation is complete and its final code-bearing Component CI is green.

- code_bearing_sha: 87801f93c34f7ef14411af3c8d9b071462676f96
- workflow_run_id: 29082606406
- run_number: 995
- conclusion: success

## Read

Read process sources, component docs/control files, current source/docs, accepted API/Server contracts, and PR diff. Do not read diagnostics artifacts unless a future fixer prompt instructs it.

## Task

Review OBS-P8 explicit sync runner, offline/backoff state, optional automation, and sanitized status UX.

Focus on serialized execution, trigger coalescing, timer ownership, unload cancellation, mode/dry-run behavior, safe progress persistence, unreadable-file handling, delete absence checks, conflict-stop behavior, sanitized status output, and honest mobile/background limitations.

## Allowed files

- apps/haze-obsidian-plugin/src/**
- apps/haze-obsidian-plugin/docs/**
- apps/haze-obsidian-plugin/control/report.md

## Boundaries

No provider integration, Server runtime changes, hard deletion, destructive repair automation, workflow/dependency changes, or sibling changes.

## CI trigger policy

Source/docs clean-code commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only apps/haze-obsidian-plugin/control/report.md. Use REPORT_TYPE CLEAN_CODE_REVIEW and phase_id OBS-P8C.
