# Archived active prompt

component: obsidian-plugin
archived_at: 2026-07-10T10:00:00Z
wave: W1
phase: OBS-P7C
agent_role: clean-code-reviewer
source_path: apps/haze-obsidian-plugin/control/prompt.md
source_sha: a88670dea80f895add3570931eb3d125012dd7dc

# W1-OBS-P7C — Obsidian conflict center clean-code review

Component: obsidian-plugin
Path: apps/haze-obsidian-plugin
Branch: component/obsidian-plugin
PR: #51
Role: clean-code-reviewer

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

OBS-P7 implementation is complete and its code-bearing Component CI run is green.

- code_bearing_sha: 6fd375f07ae942befd171181fcadfe72c52dc531
- workflow: Component CI
- workflow_run_id: 29067731941
- run_number: 849
- conclusion: success

## Read

Read implementation-manifest.md, report-template.md, clean-code-reviewer-prompt.md, chatgpt-gh-connector.md, component docs/control files, relevant source, and PR diff. Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Review OBS-P7 conflict center and supported server-backed actions.

Focus areas:

- safe conflict DTO validation and rendering;
- supported action vocabulary and destructive-action confirmation;
- idempotency-key stability and retry behavior;
- server-confirmed-only local base-state updates;
- disabled/dry-run mutation guards;
- sanitization of server-provided display values and generic UI errors;
- separation of controller logic from Obsidian modal rendering;
- preservation of all OBS-P7 non-goals.

## Allowed files

- apps/haze-obsidian-plugin/src/**
- apps/haze-obsidian-plugin/docs/**
- apps/haze-obsidian-plugin/control/report.md

## Forbidden changes

No local-only resolution, semantic merge editor, new conflict policy vocabulary, server route changes, provider behavior, hard delete, background sync loop, workflow changes, dependency changes, or sibling component changes.

## CI trigger policy

Source/doc clean-code commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to apps/haze-obsidian-plugin/control/report.md. Use report-template.md. Set REPORT_TYPE to CLEAN_CODE_REVIEW and phase_id to OBS-P7C.
